//! Safe wrapper for valid mutable pointers
//!
//! This module provides the `MutPtr` wrapper which guarantees non-null pointer access
//! with safe ergonomic wrappers while maintaining the raw pointer representation.

use std::ops::{Deref, DerefMut};

/// A transparent wrapper around a mutable pointer that ensures safety checks on access
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct MutPtr<T>(pub *mut T);

impl<T> MutPtr<T> {
    /// Creates a new `MutPtr` from a raw pointer
    #[inline(always)]
    pub fn new(ptr: *mut T) -> Self {
        Self(ptr)
    }

    /// Checks if the underlying pointer is null
    #[inline(always)]
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    /// Returns the underlying raw pointer
    #[inline(always)]
    pub fn as_ptr(&self) -> *mut T {
        self.0
    }
}

impl<T> Deref for MutPtr<T> {
    type Target = T;

    /// Dereferences the pointer. Panics if the pointer is null.
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        if self.0.is_null() {
            panic!("Null pointer dereference: {}", std::any::type_name::<T>());
        }
        unsafe { &*self.0 }
    }
}

impl<T> DerefMut for MutPtr<T> {
    /// Mutably dereferences the pointer. Panics if the pointer is null.
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        if self.0.is_null() {
            panic!("Null pointer dereference: {}", std::any::type_name::<T>());
        }
        unsafe { &mut *self.0 }
    }
}
