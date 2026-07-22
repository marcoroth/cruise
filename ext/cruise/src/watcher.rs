//! The platform-agnostic file watcher.
//!
//! Knows nothing about Ruby or the C ABI: it watches paths, applies the
//! configured kind/glob filters, and hands out filtered `(path, kind)` pairs
//! one at a time via [`Watcher::next_event`].

use std::any::Any;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use globset::{Glob, GlobSet, GlobSetBuilder};
use notify::event::{CreateKind, ModifyKind, RemoveKind};
use notify::{EventKind, RecursiveMode};
use notify_debouncer_full::{new_debouncer, DebounceEventResult};

/// Outcome of a single [`Watcher::next_event`] call.
pub enum WaitStatus {
  Event(String, String),
  Timeout,
  Disconnected,
}

/// A live filesystem watcher.
///
/// Holds the debouncer alive (dropping it stops watching), the channel of
/// incoming events, the configured filters, and a queue of already-expanded
/// `(path, kind)` pairs waiting to be handed out one at a time.
pub struct Watcher {
  _debouncer: Box<dyn Any>,
  receiver: Receiver<notify::Event>,
  glob_set: Option<GlobSet>,
  only_kinds: Vec<String>,
  pending: VecDeque<(String, String)>,
}

impl Watcher {
  pub fn new(paths: &[String], debounce: f64, globs: &[String], only_kinds: Vec<String>) -> Result<Self, String> {
    let debounce_duration = Duration::from_secs_f64(debounce);
    let glob_set = build_glob_set(globs)?;

    let (sender, receiver) = std::sync::mpsc::channel::<notify::Event>();

    let mut debouncer = new_debouncer(debounce_duration, None, move |result: DebounceEventResult| {
      if let Ok(events) = result {
        for debounced_event in events {
          let _ = sender.send(debounced_event.event);
        }
      }
    })
    .map_err(|error| format!("Failed to create watcher: {error}"))?;

    for path in paths {
      let watch_path = PathBuf::from(path);

      if !watch_path.exists() {
        return Err(format!("Path does not exist: {}", watch_path.display()));
      }

      debouncer
        .watch(&watch_path, RecursiveMode::Recursive)
        .map_err(|error| format!("Failed to watch path: {error}"))?;
    }

    Ok(Watcher {
      _debouncer: Box::new(debouncer),
      receiver,
      glob_set,
      only_kinds,
      pending: VecDeque::new(),
    })
  }

  /// Blocks up to `timeout` for the next filesystem event that passes the
  /// configured kind and glob filters.
  ///
  /// A single notify event can carry several paths, so events are expanded into
  /// individual `(path, kind)` pairs and queued; each call returns the next one.
  pub fn next_event(&mut self, timeout: Duration) -> WaitStatus {
    if let Some((path, kind)) = self.pending.pop_front() {
      return WaitStatus::Event(path, kind);
    }

    match self.receiver.recv_timeout(timeout) {
      Ok(event) => {
        self.enqueue(event);

        match self.pending.pop_front() {
          Some((path, kind)) => WaitStatus::Event(path, kind),
          None => WaitStatus::Timeout,
        }
      }

      Err(RecvTimeoutError::Timeout) => WaitStatus::Timeout,
      Err(RecvTimeoutError::Disconnected) => WaitStatus::Disconnected,
    }
  }

  fn enqueue(&mut self, event: notify::Event) {
    let kind = event_kind_to_string(&event.kind);

    if !self.only_kinds.is_empty() && !self.only_kinds.iter().any(|allowed| allowed == kind) {
      return;
    }

    for path in event.paths {
      if let Some(ref globs) = self.glob_set {
        if !globs.is_match(&path) {
          continue;
        }
      }

      self.pending.push_back((path.to_string_lossy().to_string(), kind.to_string()));
    }
  }
}

fn build_glob_set(patterns: &[String]) -> Result<Option<GlobSet>, String> {
  if patterns.is_empty() {
    return Ok(None);
  }

  let mut builder = GlobSetBuilder::new();

  for pattern in patterns {
    let glob = Glob::new(pattern).map_err(|error| format!("Invalid glob pattern '{}': {}", pattern, error))?;
    builder.add(glob);
  }

  let set = builder.build().map_err(|error| format!("Failed to build glob set: {}", error))?;

  Ok(Some(set))
}

fn event_kind_to_string(kind: &EventKind) -> &'static str {
  match kind {
    EventKind::Create(CreateKind::File) => "created",
    EventKind::Create(CreateKind::Folder) => "created",
    EventKind::Create(_) => "created",
    EventKind::Modify(ModifyKind::Data(_)) => "modified",
    EventKind::Modify(ModifyKind::Name(_)) => "renamed",
    EventKind::Modify(_) => "modified",
    EventKind::Remove(RemoveKind::File) => "removed",
    EventKind::Remove(RemoveKind::Folder) => "removed",
    EventKind::Remove(_) => "removed",
    EventKind::Access(_) => "accessed",
    EventKind::Any | EventKind::Other => "changed",
  }
}
