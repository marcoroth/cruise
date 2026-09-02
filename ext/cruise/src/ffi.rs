//! C ABI for Cruise.
//!
//! # Safety
//!
//! Pointer arguments must be valid (or null where documented). String arrays
//! are `(pointer, count)` pairs of NUL-terminated UTF-8 C strings. Any C string
//! returned through an out-parameter is heap-allocated by Rust and must be
//! released with [`cruise_string_free`]; the watcher handle with
//! [`cruise_watcher_free`]. The read fd returned by [`cruise_watcher_new`] is
//! transferred to the caller, who is responsible for closing it.

#![allow(clippy::missing_safety_doc)]

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

use crate::Watcher;

/// A single filesystem event handed to the C side. Both strings are owned by
/// the caller once populated and must be freed with [`cruise_string_free`].
#[repr(C)]
pub struct CruiseEvent {
  pub path: *mut c_char,
  pub kind: *mut c_char,
}

unsafe fn c_string_array(pointer: *const *const c_char, count: usize) -> Vec<String> {
  if pointer.is_null() || count == 0 {
    return Vec::new();
  }

  std::slice::from_raw_parts(pointer, count)
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
/// On success returns a heap-allocated handle and writes the read end of the
/// event pipe to `out_read_fd` (ownership transfers to the caller). On error
/// returns null and, when `out_error` is non-null, writes a message to be freed
/// with [`cruise_string_free`].
#[no_mangle]
pub unsafe extern "C" fn cruise_watcher_new(
  paths: *const *const c_char,
  path_count: usize,
  debounce: f64,
  globs: *const *const c_char,
  glob_count: usize,
  only_kinds: *const *const c_char,
  only_count: usize,
  out_read_fd: *mut c_int,
  out_error: *mut *mut c_char,
) -> *mut Watcher {
  let paths = c_string_array(paths, path_count);
  let globs = c_string_array(globs, glob_count);
  let only = c_string_array(only_kinds, only_count);

  match Watcher::new(&paths, debounce, &globs, only) {
    Ok((watcher, read_fd)) => {
      if !out_read_fd.is_null() {
        *out_read_fd = read_fd;
      }

      Box::into_raw(Box::new(watcher))
    }
    Err(message) => {
      if !out_error.is_null() {
        *out_error = CString::new(message).unwrap_or_default().into_raw();
      }

      ptr::null_mut()
    }
  }
}

/// Non-blocking: pop the next queued event into `out_event` and return true, or
/// return false if the queue is empty. Never touches Ruby and never blocks — the
/// C wrapper waits on the pipe fd instead.
#[no_mangle]
pub unsafe extern "C" fn cruise_watcher_poll(watcher: *mut Watcher, out_event: *mut CruiseEvent) -> bool {
  if watcher.is_null() {
    return false;
  }

  let watcher = &*watcher;

  match watcher.poll() {
    Some((path, kind)) => {
      if !out_event.is_null() {
        (*out_event).path = CString::new(path).unwrap_or_default().into_raw();
        (*out_event).kind = CString::new(kind).unwrap_or_default().into_raw();
      }

      true
    }
    None => false,
  }
}

/// Non-blocking: pop the next queued watcher error into `out_message` and
/// return true, or return false if there are none. The message is heap
/// allocated by Rust and must be freed with [`cruise_string_free`].
#[no_mangle]
pub unsafe extern "C" fn cruise_watcher_poll_error(watcher: *mut Watcher, out_message: *mut *mut c_char) -> bool {
  if watcher.is_null() {
    return false;
  }

  let watcher = &*watcher;

  match watcher.poll_error() {
    Some(message) => {
      if !out_message.is_null() {
        *out_message = CString::new(message).unwrap_or_default().into_raw();
      }

      true
    }
    None => false,
  }
}

/// Free a watcher handle. Stops watching, joins the background thread, and
/// closes the write end of the pipe (the reader then observes EOF).
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
