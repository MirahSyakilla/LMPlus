//! Unity Coroutine wrapper
use std::ffi::c_void;

/// Wrapper around Unity's UnityEngine.Coroutine object
///
/// A Coroutine represents a running coroutine instance that can be
/// used to stop the coroutine later.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Coroutine {
    /// Pointer to the IL2CPP Coroutine object
    ptr: *mut c_void,
}

unsafe impl Send for Coroutine {}
unsafe impl Sync for Coroutine {}

impl Coroutine {
    /// Creates a Coroutine from a raw pointer
    ///
    /// # Arguments
    /// * `ptr` - The raw pointer to the IL2CPP Coroutine object
    ///
    /// # Returns
    /// * `Self` - The created Coroutine wrapper
    pub unsafe fn from_ptr(ptr: *mut c_void) -> Self {
        Self { ptr }
    }

    /// Returns the raw pointer to the Coroutine object
    ///
    /// # Returns
    /// * `*mut c_void` - The raw pointer
    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr
    }

    /// Checks if the Coroutine pointer is null
    ///
    /// # Returns
    /// * `bool` - True if the pointer is null
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }
}
