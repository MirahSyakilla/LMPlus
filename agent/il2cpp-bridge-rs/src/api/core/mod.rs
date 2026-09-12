//! Core utilities for IL2CPP interaction
//!
//! This module contains essential utilities for interacting with the IL2CPP runtime,
//! including API definitions, caching mechanisms, method invocation helpers,
//! and thread management.
//!
//! # Modules
//!
//! * `api` - raw FFI bindings to the IL2CPP API
//! * `cache` - caching mechanisms for IL2CPP types and metadata
//! * `caller` - helper traits and functions for invoking IL2CPP methods
//! * `internals` - internal initialization and state management
//! * `thread` - thread attachment and detachment utilities
pub mod api;
pub mod cache;
pub mod internals;
pub mod runtime;

pub use runtime::caller;
pub use runtime::thread;
