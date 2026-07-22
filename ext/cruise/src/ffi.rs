//! C ABI for Cruise.
//!
//! # Safety
//!
//! Pointer arguments must be valid (or null where documented). String arrays
//! are `(pointer, count)` pairs of NUL-terminated UTF-8 C strings. Any C string
//! returned through an out-parameter is heap-allocated by Rust and must be
//! released with [`cruise_string_free`]; the watcher handle with
//! [`cruise_watcher_free`].

#![allow(clippy::missing_safety_doc)]

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;
use std::time::Duration;

use crate::{WaitStatus, Watcher};

/// A single filesystem event handed to the C side. Both strings are owned by
/// the caller once populated and must be freed with [`cruise_string_free`].
#[repr(C)]
pub struct CruiseEvent {
  pub path: *mut c_char,
  pub kind: *mut c_char,
}

/// Result of [`cruise_watcher_next_event`].
#[repr(C)]
pub enum CruiseWaitStatus {
  Event,
  Timeout,
  Disconnected,
}

unsafe fn c_string_array(ptr: *const *const c_char, count: usize) -> Vec<String> {
  if ptr.is_null() || count == 0 {
    return Vec::new();
  }

  std::slice::from_raw_parts(ptr, count)
    .iter()
    .filter_map(|&entry| {
      if entry.is_null() {
        None
      } else {
        CStr::from_ptr(entry).to_str().ok().map(|value| value.to_string())
      }
    })
    .collect()
}

/// Create a watcher over `paths`, filtered by `globs` and `only_kinds`.
///
/// Returns a heap-allocated handle, or null on error (in which case a message
/// is written to `out_error` when non-null, to be freed with
/// [`cruise_string_free`]).
#[no_mangle]
pub unsafe extern "C" fn cruise_watcher_new(
  paths: *const *const c_char,
  path_count: usize,
  debounce: f64,
  globs: *const *const c_char,
  glob_count: usize,
  only_kinds: *const *const c_char,
  only_count: usize,
  out_error: *mut *mut c_char,
) -> *mut Watcher {
  let paths = c_string_array(paths, path_count);
  let globs = c_string_array(globs, glob_count);
  let only = c_string_array(only_kinds, only_count);

  match Watcher::new(&paths, debounce, &globs, only) {
    Ok(watcher) => Box::into_raw(Box::new(watcher)),

    Err(message) => {
      if !out_error.is_null() {
        *out_error = CString::new(message).unwrap_or_default().into_raw();
      }

      ptr::null_mut()
    }
  }
}

/// Block up to `timeout_ms` for the next matching event.
///
/// On [`CruiseWaitStatus::Event`], `out_event` is populated with freshly
/// allocated `path` and `kind` strings. This function never touches Ruby, so
/// the C wrapper can call it with the GVL released.
#[no_mangle]
pub unsafe extern "C" fn cruise_watcher_next_event(watcher: *mut Watcher, timeout_ms: u64, out_event: *mut CruiseEvent) -> CruiseWaitStatus {
  if watcher.is_null() {
    return CruiseWaitStatus::Disconnected;
  }

  let watcher = &mut *watcher;

  match watcher.next_event(Duration::from_millis(timeout_ms)) {
    WaitStatus::Event(path, kind) => {
      if !out_event.is_null() {
        (*out_event).path = CString::new(path).unwrap_or_default().into_raw();
        (*out_event).kind = CString::new(kind).unwrap_or_default().into_raw();
      }

      CruiseWaitStatus::Event
    }

    WaitStatus::Timeout => CruiseWaitStatus::Timeout,
    WaitStatus::Disconnected => CruiseWaitStatus::Disconnected,
  }
}

/// Free a watcher handle. Stops watching and joins the background thread.
#[no_mangle]
pub unsafe extern "C" fn cruise_watcher_free(watcher: *mut Watcher) {
  if !watcher.is_null() {
    drop(Box::from_raw(watcher));
  }
}

/// Free a string previously returned by this library.
#[no_mangle]
pub unsafe extern "C" fn cruise_string_free(s: *mut c_char) {
  if !s.is_null() {
    drop(CString::from_raw(s));
  }
}
