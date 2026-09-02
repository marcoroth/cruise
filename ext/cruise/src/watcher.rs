//! The platform-agnostic file watcher.
//!
//! Knows nothing about Ruby or the C ABI. It watches paths, applies the
//! configured kind/glob filters on the debouncer's background thread, and
//! pushes matching `(path, kind)` pairs onto a shared queue. A self-pipe is
//! written to on every batch so a consumer can wait on a file descriptor
//! (`IO#wait_readable`) instead of blocking a thread — which is what makes the
//! watcher cooperate with Ruby's fiber scheduler.

use std::any::Any;
use std::collections::VecDeque;
use std::os::fd::RawFd;
use std::os::raw::{c_int, c_void};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use globset::{Glob, GlobSet, GlobSetBuilder};
use notify::event::{CreateKind, ModifyKind, RemoveKind};
use notify::{ErrorKind, EventKind, RecursiveMode};
use notify_debouncer_full::{new_debouncer, DebounceEventResult};

const MAX_QUEUED_ERRORS: usize = 64;

struct Shared {
  pending: Mutex<VecDeque<(String, String)>>,
  errors: Mutex<VecDeque<String>>,
  glob_set: Option<GlobSet>,
  only_kinds: Vec<String>,
  write_fd: RawFd,
}

impl Shared {
  fn enqueue(&self, event: notify::Event) -> bool {
    let kind = event_kind_to_string(&event.kind);

    if !self.only_kinds.is_empty() && !self.only_kinds.iter().any(|allowed| allowed == kind) {
      return false;
    }

    let mut queue = self.pending.lock().unwrap();
    let mut added = false;

    for path in event.paths {
      if let Some(ref globs) = self.glob_set {
        if !globs.is_match(&path) {
          continue;
        }
      }

      queue.push_back((path.to_string_lossy().into_owned(), kind.to_string()));
      added = true;
    }

    added
  }

  fn enqueue_error(&self, message: String) {
    let mut errors = self.errors.lock().unwrap();

    while errors.len() >= MAX_QUEUED_ERRORS {
      errors.pop_front();
    }

    errors.push_back(message);
  }

  fn wake(&self) {
    let byte = [1u8];
    let _ = unsafe { libc::write(self.write_fd, byte.as_ptr() as *const c_void, 1) };
  }
}

impl Drop for Shared {
  fn drop(&mut self) {
    unsafe { libc::close(self.write_fd) };
  }
}

/// A live filesystem watcher.
///
/// Holds the debouncer alive (dropping it stops watching) and the shared queue
/// the consumer drains. The read end of the self-pipe is handed to the caller
/// by [`Watcher::new`] and owned by them from that point on.
pub struct Watcher {
  _debouncer: Box<dyn Any>,
  shared: Arc<Shared>,
}

impl Watcher {
  pub fn new(paths: &[String], debounce: f64, globs: &[String], only_kinds: Vec<String>) -> Result<(Self, RawFd), String> {
    let glob_set = build_glob_set(globs)?;
    let (read_fd, write_fd) = make_pipe()?;

    let shared = Arc::new(Shared {
      pending: Mutex::new(VecDeque::new()),
      errors: Mutex::new(VecDeque::new()),
      glob_set,
      only_kinds,
      write_fd,
    });

    let callback_shared = Arc::clone(&shared);

    let mut debouncer = match new_debouncer(Duration::from_secs_f64(debounce), None, move |result: DebounceEventResult| {
      match result {
        Ok(events) => {
          let mut added = false;

          for debounced_event in events {
            if callback_shared.enqueue(debounced_event.event) {
              added = true;
            }
          }

          if added {
            callback_shared.wake();
          }
        }

        Err(errors) => {
          for error in &errors {
            callback_shared.enqueue_error(describe_error(error));
          }

          if !errors.is_empty() {
            callback_shared.wake();
          }
        }
      }
    }) {
      Ok(debouncer) => debouncer,

      Err(error) => {
        unsafe { libc::close(read_fd) };
        return Err(format!("Failed to create watcher: {error}"));
      }
    };

    for path in paths {
      let watch_path = PathBuf::from(path);

      if !watch_path.exists() {
        unsafe { libc::close(read_fd) };
        return Err(format!("Path does not exist: {}", watch_path.display()));
      }

      if let Err(error) = debouncer.watch(&watch_path, RecursiveMode::Recursive) {
        unsafe { libc::close(read_fd) };
        return Err(format!("Failed to watch path: {error}"));
      }
    }

    Ok((
      Watcher {
        _debouncer: Box::new(debouncer),
        shared,
      },
      read_fd,
    ))
  }

  pub fn poll(&self) -> Option<(String, String)> {
    self.shared.pending.lock().unwrap().pop_front()
  }

  pub fn poll_error(&self) -> Option<String> {
    self.shared.errors.lock().unwrap().pop_front()
  }
}

fn describe_error(error: &notify::Error) -> String {
  match error.kind {
    ErrorKind::MaxFilesWatch => format!(
      "Reached the limit of watched files, so parts of the watched tree are no longer monitored. \
       On Linux raise it with `sysctl fs.inotify.max_user_watches`. ({error})"
    ),

    _ => error.to_string(),
  }
}

fn make_pipe() -> Result<(RawFd, RawFd), String> {
  let mut fds = [0 as c_int; 2];

  if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
    return Err("Failed to create event pipe".to_string());
  }

  set_nonblock_cloexec(fds[0]);
  set_nonblock_cloexec(fds[1]);

  Ok((fds[0], fds[1]))
}

fn set_nonblock_cloexec(fd: c_int) {
  unsafe {
    let flags = libc::fcntl(fd, libc::F_GETFL);
    libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);

    let fd_flags = libc::fcntl(fd, libc::F_GETFD);
    libc::fcntl(fd, libc::F_SETFD, fd_flags | libc::FD_CLOEXEC);
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
