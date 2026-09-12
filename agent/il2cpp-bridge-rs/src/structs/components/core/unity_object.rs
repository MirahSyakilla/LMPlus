//! Base wrapper for managed `UnityEngine.Object` values.
use crate::api::cache;
use crate::structs::components::Transform;
use crate::structs::core::{Class, Il2cppObject, Object};
use crate::structs::math::{Quaternion, Vector3};
use crate::structs::Il2cppString;
use std::ffi::c_void;
use std::ops::Deref;

/// Wrapper for a managed `UnityEngine.Object`.
///
/// This sits between the generic [`Object`] wrapper and more specific Unity
/// types such as `GameObject`, `Component`, and `Transform`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UnityObject {
    /// Base Object structure
    pub object: Object,
    /// Cached pointer to the object
    pub m_cached_ptr: *mut c_void,
}

impl UnityObject {
    /// Creates a `UnityObject` from a raw managed pointer.
    pub fn from_ptr(ptr: *mut c_void) -> Self {
        let object = unsafe { Object::from_ptr(ptr) };
        let m_cached_ptr = unsafe {
            let offset = std::mem::size_of::<Il2cppObject>() as isize;
            *(ptr.offset(offset) as *mut *mut c_void)
        };
        Self {
            object,
            m_cached_ptr,
        }
    }

    /// Returns the raw managed pointer.
    pub fn as_ptr(&self) -> *mut c_void {
        self.object.as_ptr()
    }

    /// Returns the Unity object name.
    pub fn get_name(&self) -> Result<String, String> {
        unsafe {
            let obj = Object::from_ptr(self.as_ptr());
            let ptr = obj
                .method("get_name")
                .ok_or("Method 'get_name' not found")?
                .call::<*mut Il2cppString>(&[])?;

            if ptr.is_null() {
                return Err("Name is null".to_string());
            }

            (*ptr)
                .to_string()
                .ok_or_else(|| "Failed to convert name to String".to_string())
        }
    }

    /// Returns the managed `ToString()` representation.
    pub fn to_string(&self) -> Result<String, String> {
        unsafe {
            let obj = Object::from_ptr(self.as_ptr());
            let ptr = obj
                .method("ToString")
                .ok_or("Method 'ToString' not found")?
                .call::<*mut Il2cppString>(&[])?;

            if ptr.is_null() {
                return Err("String is null".to_string());
            }

            (*ptr)
                .to_string()
                .ok_or_else(|| "Failed to convert String to String".to_string())
        }
    }
    /// Returns the cached `UnityEngine.Object` class definition.
    pub fn get_class() -> Option<Class> {
        cache::coremodule().class("Object")
    }

    /// Clones the given Unity object using `Object.Instantiate`.
    pub fn instantiate(original: &UnityObject) -> Result<UnityObject, String> {
        unsafe {
            let object_class = Self::get_class()
                .ok_or_else(|| "Could not find UnityEngine.Object class".to_string())?;
            let method = object_class
                .method(("Instantiate", 1))
                .ok_or("Method 'Instantiate' not found")?;

            let ptr = method.call::<*mut c_void>(&[original.as_ptr()])?;

            if ptr.is_null() {
                return Err("Failed to instantiate object".to_string());
            }

            Ok(UnityObject::from_ptr(ptr))
        }
    }

    /// Clones the object at the given position and rotation.
    pub fn instantiate_at(
        original: &UnityObject,
        position: Vector3,
        rotation: Quaternion,
    ) -> Result<UnityObject, String> {
        unsafe {
            let object_class = Self::get_class()
                .ok_or_else(|| "Could not find UnityEngine.Object class".to_string())?;
            let method = object_class
                .method(("Instantiate", 3))
                .ok_or("Method 'Instantiate' not found")?;

            let ptr = method.call::<*mut c_void>(&[
                original.as_ptr(),
                &position as *const _ as *mut c_void,
                &rotation as *const _ as *mut c_void,
            ])?;

            if ptr.is_null() {
                return Err("Failed to instantiate object".to_string());
            }

            Ok(UnityObject::from_ptr(ptr))
        }
    }

    /// Clones the object and parents it under `parent`.
    pub fn instantiate_with_parent(
        original: &UnityObject,
        parent: &Transform,
    ) -> Result<UnityObject, String> {
        unsafe {
            let object_class = Self::get_class()
                .ok_or_else(|| "Could not find UnityEngine.Object class".to_string())?;
            let method = object_class
                .method(("Instantiate", 2))
                .ok_or("Method 'Instantiate' not found")?;

            let ptr = method.call::<*mut c_void>(&[original.as_ptr(), parent.as_ptr()])?;

            if ptr.is_null() {
                return Err("Failed to instantiate object".to_string());
            }

            Ok(UnityObject::from_ptr(ptr))
        }
    }

    /// Schedules this object for destruction after `time_delay` seconds.
    pub fn destroy(&self, time_delay: f32) -> Result<(), String> {
        unsafe {
            let object_class = Self::get_class()
                .ok_or_else(|| "Could not find UnityEngine.Object class".to_string())?;
            let method = object_class
                .method("Destroy")
                .ok_or("Method 'Destroy' not found")?;

            method.call::<()>(&[self.as_ptr(), &time_delay as *const f32 as *mut c_void])?;
            Ok(())
        }
    }

    /// Destroys the object immediately.
    pub fn destroy_immediate(
        obj: &UnityObject,
        allow_destroying_assets: bool,
    ) -> Result<(), String> {
        unsafe {
            let object_class = Self::get_class()
                .ok_or_else(|| "Could not find UnityEngine.Object class".to_string())?;
            let method = object_class
                .method("DestroyImmediate")
                .ok_or("Method 'DestroyImmediate' not found")?;

            method.call::<()>(&[
                obj.as_ptr(),
                &allow_destroying_assets as *const bool as *mut c_void,
            ])?;
            Ok(())
        }
    }

    /// Marks the object to survive scene loads.
    pub fn dont_destroy_on_load(obj: &UnityObject) -> Result<(), String> {
        unsafe {
            let object_class = Self::get_class()
                .ok_or_else(|| "Could not find UnityEngine.Object class".to_string())?;
            let method = object_class
                .method("DontDestroyOnLoad")
                .ok_or("Method 'DontDestroyOnLoad' not found")?;

            method.call::<()>(&[obj.as_ptr()])?;
            Ok(())
        }
    }
}

impl Deref for UnityObject {
    type Target = Object;

    fn deref(&self) -> &Self::Target {
        &self.object
    }
}
