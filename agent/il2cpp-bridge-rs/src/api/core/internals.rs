//! Helpers for resolving and registering IL2CPP internal calls.
//!
//! These functions are lower-level than the usual cache/object workflow and are
//! mainly useful when Unity exposes the functionality you need as an icall.
use super::api;
use std::ffi::{c_void, CString};

/// Resolver and registration helper for IL2CPP internal calls.
pub struct Internals;

impl Internals {
    /// Resolves an internal call by its fully qualified name.
    ///
    /// Returns a raw function pointer or null if the icall is unavailable in
    /// the current runtime.
    pub fn resolve(name: &str) -> *mut c_void {
        let name_c = CString::new(name).unwrap();
        unsafe { api::resolve_icall(name_c.as_ptr()) }
    }

    /// Registers a new internal call implementation.
    ///
    /// This is mainly relevant when embedding or extending a runtime that
    /// expects a native implementation to be available through the icall table.
    pub fn add(name: &str, method: *mut c_void) {
        let name_c = CString::new(name).unwrap();
        unsafe { api::add_internal_call(name_c.as_ptr(), method) }
    }
}
