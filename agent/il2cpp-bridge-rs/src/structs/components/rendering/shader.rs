use crate::api::cache;
use crate::structs::collections::Il2cppString;
use crate::structs::core::Object;
use std::ffi::c_void;
use std::ops::Deref;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Shader {
    pub object: Object,
}

impl Shader {
    pub fn from_ptr(ptr: *mut c_void) -> Self {
        Self {
            object: unsafe { Object::from_ptr(ptr) },
        }
    }
}

impl Deref for Shader {
    type Target = Object;
    fn deref(&self) -> &Self::Target {
        &self.object
    }
}

impl Shader {
    /// Gets the Shader class definition
    ///
    /// # Returns
    /// * `Option<Class>` - The UnityEngine.Shader class
    pub fn get_class() -> Option<crate::structs::core::Class> {
        cache::coremodule().class("UnityEngine.Shader")
    }

    /// Finds a shader with the given name.
    ///
    /// # Arguments
    /// * `name` - The name of the shader to find
    ///
    /// # Returns
    /// * `Option<Shader>` - The shader if found, None otherwise
    pub fn find(name: &str) -> Option<Self> {
        unsafe {
            let string_new_ptr = Il2cppString::new(name);
            Self::get_class()
                .and_then(|cls| cls.method(("Find", 1)))
                .and_then(|method| {
                    method
                        .call::<*mut c_void>(&[string_new_ptr as *mut c_void])
                        .ok()
                })
                .filter(|ptr| !ptr.is_null())
                .map(Self::from_ptr)
        }
    }

    /// Gets the unique identifier for a shader property name.
    ///
    /// # Arguments
    /// * `name` - The property name
    ///
    /// # Returns
    /// * `i32` - The ID of the property
    pub fn property_to_id(name: &str) -> i32 {
        unsafe {
            let string_new_ptr = Il2cppString::new(name);
            Self::get_class()
                .and_then(|cls| cls.method(("PropertyToID", 1)))
                .and_then(|method| method.call::<i32>(&[string_new_ptr as *mut c_void]).ok())
                .unwrap_or(0)
        }
    }
}
