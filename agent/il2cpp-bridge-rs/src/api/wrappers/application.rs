//! Thin wrapper around `UnityEngine.Application`.
//!
//! These helpers are useful for reading common runtime metadata such as version,
//! bundle identifier, and persistent data path without repeating method lookup
//! boilerplate.

use crate::api::{cache, internals::Internals};
use crate::structs::Il2cppString;
use std::ffi::c_void;

/// Accessors for selected `UnityEngine.Application` properties.
pub struct Application;

impl Application {
    /// Gets the Unity engine version
    ///
    /// Returns "unknown" if the version string is empty.
    pub fn unity_version() -> Result<String, String> {
        match Self::get_string("get_unityVersion") {
            Ok(ver) if ver.is_empty() => Ok("unknown".to_string()),
            Ok(ver) => Ok(ver),
            Err(e) => Err(e),
        }
    }

    /// Gets the application version
    pub fn version() -> Result<String, String> {
        Self::get_string("get_version")
    }

    /// Gets the application identifier (bundle ID)
    ///
    /// Tries both `get_identifier` and `get_bundleIdentifier`.
    pub fn identifier() -> Result<String, String> {
        let id = Self::get_string("get_identifier");
        if let Ok(val) = &id {
            if !val.is_empty() {
                return id;
            }
        }
        let id = Self::get_string("get_bundleIdentifier");
        if let Ok(val) = &id {
            if !val.is_empty() {
                return id;
            }
        }
        Ok("unknown.application".to_string())
    }

    /// Gets the persistent data path
    ///
    /// Returns the path to a persistent data directory.
    pub fn data_path() -> Result<String, String> {
        Self::get_string("get_persistentDataPath")
    }

    fn get_string(method_name: &str) -> Result<String, String> {
        let full_name = format!("UnityEngine.Application::{}", method_name);

        let func = Internals::resolve(&full_name);
        if !func.is_null() {
            unsafe {
                let func: extern "C" fn() -> *mut c_void = std::mem::transmute(func);
                let result = func();
                if result.is_null() {
                    return Ok(String::new());
                }
                let s = result as *mut Il2cppString;
                return Ok((*s).to_string().unwrap_or_default());
            }
        }

        let klass = cache::coremodule()
            .class("UnityEngine.Application")
            .ok_or("Class 'UnityEngine.Application' not found")?;

        let method = klass
            .method(method_name)
            .ok_or(format!("Method '{}' not found", method_name))?;

        unsafe {
            let result: Result<*mut Il2cppString, _> = method.call(&[]);
            match result {
                Ok(ptr) => {
                    if ptr.is_null() {
                        Ok(String::new())
                    } else {
                        Ok((*ptr).to_string().unwrap_or_default())
                    }
                }
                Err(e) => Err(e),
            }
        }
    }
}
