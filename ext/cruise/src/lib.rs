//! Cruise - a fast, OS-native file watcher.
//!
//! This crate contains the platform-agnostic watching logic ([`watcher`]) and a
//! C ABI ([`ffi`]) that a thin Ruby C extension links against. It deliberately
//! knows nothing about Ruby: it hands filtered `(path, kind)` pairs across the
//! FFI boundary and lets the C wrapper turn them into Ruby objects.

mod ffi;
mod watcher;

pub use watcher::{WaitStatus, Watcher};
