//! Unity MonoBehaviour wrapper
//!
//! This module provides a wrapper for Unity's MonoBehaviour class,
//! with support for starting and stopping coroutines.

use super::component::{Component, ComponentTrait};
use crate::structs::core::runtime::coroutine::Coroutine;
use crate::structs::Il2cppString;
use std::ffi::c_void;
use std::ops::Deref;

/// Wrapper around Unity's MonoBehaviour component
///
/// MonoBehaviour is the base class from which every Unity script derives.
/// This wrapper provides access to coroutine functionality.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MonoBehaviour {
    /// Base Component structure
    pub component: Component,
}

impl ComponentTrait for MonoBehaviour {
    fn from_ptr(ptr: *mut c_void) -> Self {
        Self {
            component: Component::from_ptr(ptr),
        }
    }
}

impl MonoBehaviour {
    /// Creates a MonoBehaviour from a raw pointer
    ///
    /// # Arguments
    /// * `ptr` - The raw pointer to the MonoBehaviour
    ///
    /// # Returns
    /// * `Self` - The created MonoBehaviour wrapper
    pub fn from_ptr(ptr: *mut c_void) -> Self {
        <Self as ComponentTrait>::from_ptr(ptr)
    }

    /// Returns the raw pointer to the MonoBehaviour
    ///
    /// # Returns
    /// * `*mut c_void` - The raw pointer
    pub fn as_ptr(&self) -> *mut c_void {
        self.component.as_ptr()
    }

    /// Starts a coroutine with the given IEnumerator
    ///
    /// This calls `MonoBehaviour.StartCoroutine(IEnumerator routine)` on the
    /// Unity side. The IEnumerator should be created from a method that
    /// returns IEnumerator (e.g., a generator method).
    ///
    /// # Arguments
    /// * `enumerator` - Pointer to the IEnumerator object
    ///
    /// # Returns
    /// * `Result<Coroutine, String>` - The started Coroutine, or an error
    pub fn start_coroutine(&self, enumerator: *mut c_void) -> Result<Coroutine, String> {
        if enumerator.is_null() {
            return Err("Enumerator is null".to_string());
        }

        unsafe {
            let method = self
                .method(("StartCoroutine", ["System.Collections.IEnumerator"]))
                .ok_or("Method 'StartCoroutine(IEnumerator)' not found")?;

            let result = method.call::<*mut c_void>(&[enumerator])?;

            if result.is_null() {
                return Err("StartCoroutine returned null".to_string());
            }

            Ok(Coroutine::from_ptr(result))
        }
    }

    /// Starts a coroutine by method name
    ///
    /// This calls `MonoBehaviour.StartCoroutine(string methodName)` on the
    /// Unity side.
    ///
    /// # Arguments
    /// * `method_name` - The name of the coroutine method to start
    ///
    /// # Returns
    /// * `Result<Coroutine, String>` - The started Coroutine, or an error
    pub fn start_coroutine_by_name(&self, method_name: &str) -> Result<Coroutine, String> {
        unsafe {
            let name_str = Il2cppString::new(method_name);

            let method = self
                .method(("StartCoroutine", ["System.String"]))
                .ok_or("Method 'StartCoroutine(String)' not found")?;

            let result = method.call::<*mut c_void>(&[name_str as *mut c_void])?;

            if result.is_null() {
                return Err("StartCoroutine returned null".to_string());
            }

            Ok(Coroutine::from_ptr(result))
        }
    }

    /// Stops a specific coroutine
    ///
    /// # Arguments
    /// * `coroutine` - The Coroutine to stop
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if successful, or an error
    pub fn stop_coroutine(&self, coroutine: Coroutine) -> Result<(), String> {
        if coroutine.is_null() {
            return Err("Coroutine is null".to_string());
        }

        unsafe {
            let method = self
                .method(("StopCoroutine", ["UnityEngine.Coroutine"]))
                .ok_or("Method 'StopCoroutine(Coroutine)' not found")?;

            method.call::<()>(&[coroutine.as_ptr()])?;

            Ok(())
        }
    }

    /// Stops a coroutine by method name
    ///
    /// # Arguments
    /// * `method_name` - The name of the coroutine method to stop
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if successful, or an error
    pub fn stop_coroutine_by_name(&self, method_name: &str) -> Result<(), String> {
        unsafe {
            let name_str = Il2cppString::new(method_name);

            let method = self
                .method(("StopCoroutine", ["System.String"]))
                .ok_or("Method 'StopCoroutine(String)' not found")?;

            method.call::<()>(&[name_str as *mut c_void])?;

            Ok(())
        }
    }

    /// Stops all coroutines running on this MonoBehaviour
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if successful, or an error
    pub fn stop_all_coroutines(&self) -> Result<(), String> {
        unsafe {
            let method = self
                .method("StopAllCoroutines")
                .ok_or("Method 'StopAllCoroutines' not found")?;

            method.call::<()>(&[])?;

            Ok(())
        }
    }

    /// Checks if the MonoBehaviour is enabled
    ///
    /// # Returns
    /// * `Result<bool, String>` - True if enabled
    pub fn get_enabled(&self) -> Result<bool, String> {
        unsafe {
            self.method("get_enabled")
                .ok_or("Method 'get_enabled' not found")?
                .call::<bool>(&[])
        }
    }

    /// Sets the enabled state of the MonoBehaviour
    ///
    /// # Arguments
    /// * `enabled` - Whether the MonoBehaviour should be enabled
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if successful
    pub fn set_enabled(&self, enabled: bool) -> Result<(), String> {
        unsafe {
            let mut arg = enabled;
            self.method("set_enabled")
                .ok_or("Method 'set_enabled' not found")?
                .call::<()>(&[&mut arg as *mut bool as *mut c_void])?;

            Ok(())
        }
    }
}

impl Deref for MonoBehaviour {
    type Target = Component;

    fn deref(&self) -> &Self::Target {
        &self.component
    }
}
