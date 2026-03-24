use std::path::PathBuf;

use globset::{Glob, GlobSet, GlobSetBuilder};
use magnus::{method, prelude::*, value::Lazy, Error, IntoValue, RClass, RModule, Ruby, Value};

use notify::event::{CreateKind, ModifyKind, RemoveKind};
use notify::{EventKind, RecursiveMode};
use notify_debouncer_full::{new_debouncer, DebounceEventResult};

static MODULE: Lazy<RModule> = Lazy::new(|ruby| ruby.define_module("Cruise").expect("failed to define Cruise module"));

static EVENT_CLASS: Lazy<RClass> = Lazy::new(|ruby| {
  let module = ruby.get_inner(&MODULE);
  let class = module.define_class("Event", ruby.class_object()).expect("failed to define Cruise::Event class");

  class
    .define_method("path", method!(CruiseEvent::path, 0))
    .expect("failed to define path method");

  class
    .define_method("kind", method!(CruiseEvent::kind, 0))
    .expect("failed to define kind method");

  class
    .define_method("inspect", method!(CruiseEvent::inspect, 0))
    .expect("failed to define inspect method");

  class
    .define_method("to_s", method!(CruiseEvent::to_s, 0))
    .expect("failed to define to_s method");

  class
});

#[magnus::wrap(class = "Cruise::Event", free_immediately)]
struct CruiseEvent {
  path: String,
  kind: String,
}

impl CruiseEvent {
  fn path(&self) -> &str {
    &self.path
  }

  fn kind(&self) -> &str {
    &self.kind
  }

  fn inspect(&self) -> String {
    format!("#<Cruise::Event kind={:?} path={:?}>", self.kind, self.path)
  }

  fn to_s(&self) -> String {
    format!("{}: {}", self.kind, self.path)
  }
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

fn build_glob_set(patterns: Vec<String>) -> Result<Option<GlobSet>, Error> {
  if patterns.is_empty() {
    return Ok(None);
  }

  let mut builder = GlobSetBuilder::new();

  for pattern in &patterns {
    let glob = Glob::new(pattern).map_err(|error| {
      Error::new(
        magnus::Ruby::get().unwrap().exception_arg_error(),
        format!("Invalid glob pattern '{}': {}", pattern, error),
      )
    })?;
    builder.add(glob);
  }

  let set = builder.build().map_err(|error| {
    Error::new(
      magnus::Ruby::get().unwrap().exception_runtime_error(),
      format!("Failed to build glob set: {}", error),
    )
  })?;

  Ok(Some(set))
}

enum WaitResult {
  Event(notify::Event),
  Timeout,
  Disconnected,
}

unsafe extern "C" fn wait_for_event(data: *mut std::ffi::c_void) -> *mut std::ffi::c_void {
  let receiver = unsafe { &*(data as *const std::sync::mpsc::Receiver<notify::Event>) };

  let result = match receiver.recv_timeout(std::time::Duration::from_millis(200)) {
    Ok(event) => Box::new(WaitResult::Event(event)),
    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Box::new(WaitResult::Timeout),
    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Box::new(WaitResult::Disconnected),
  };

  Box::into_raw(result) as *mut std::ffi::c_void
}

unsafe extern "C" fn unblock_wait(_data: *mut std::ffi::c_void) {}

fn watch(
  ruby: &Ruby,
  paths: Vec<String>,
  callback: magnus::block::Proc,
  debounce: f64,
  glob_patterns: Vec<String>,
  only_kinds: Vec<String>,
) -> Result<magnus::Value, Error> {
  let debounce_duration = std::time::Duration::from_secs_f64(debounce);
  let glob_set = build_glob_set(glob_patterns)?;
  let filter_kinds = if only_kinds.is_empty() { None } else { Some(only_kinds) };

  let (sender, receiver) = std::sync::mpsc::channel::<notify::Event>();

  let mut debouncer = new_debouncer(debounce_duration, None, move |result: DebounceEventResult| {
    if let Ok(events) = result {
      for debounced_event in events {
        let _ = sender.send(debounced_event.event);
      }
    }
  })
  .map_err(|error| Error::new(ruby.exception_runtime_error(), format!("Failed to create watcher: {error}")))?;

  for path in &paths {
    let watch_path = PathBuf::from(path);

    if !watch_path.exists() {
      return Err(Error::new(ruby.exception_arg_error(), format!("Path does not exist: {}", watch_path.display())));
    }

    debouncer
      .watch(&watch_path, RecursiveMode::Recursive)
      .map_err(|error| Error::new(ruby.exception_runtime_error(), format!("Failed to watch path: {error}")))?;
  }

  loop {
    let result = unsafe {
      rb_sys::rb_thread_call_without_gvl(
        Some(wait_for_event),
        &receiver as *const _ as *mut std::ffi::c_void,
        Some(unblock_wait),
        std::ptr::null_mut(),
      )
    };

    let wait_result = unsafe { *Box::from_raw(result as *mut WaitResult) };

    match &wait_result {
      WaitResult::Event(event) => {
        let kind_string = event_kind_to_string(&event.kind);

        if let Some(ref allowed) = filter_kinds {
          if !allowed.iter().any(|kind| kind == kind_string) {
            continue;
          }
        }

        for path in &event.paths {
          if let Some(ref globs) = glob_set {
            if !globs.is_match(path) {
              continue;
            }
          }

          let cruise_event = CruiseEvent {
            path: path.to_string_lossy().to_string(),
            kind: kind_string.to_string(),
          };

          let value: Value = cruise_event.into_value_with(ruby);
          callback.call::<_, Value>((value,))?;
        }
      }
      WaitResult::Timeout => {
        continue;
      }
      WaitResult::Disconnected => {
        return Err(Error::new(ruby.exception_runtime_error(), "Watcher channel disconnected unexpectedly"));
      }
    }
  }
}

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
  let module = ruby.get_inner(&MODULE);
  let _ = ruby.get_inner(&EVENT_CLASS);

  module.define_module_function("_watch", magnus::function!(watch, 5))?;

  Ok(())
}
