//! Unity `Transform` wrapper.
use crate::structs::core::{Il2cppObject, Object};
use crate::structs::math::{Quaternion, Vector3};
use std::ffi::c_void;

use super::component::ComponentTrait;
use super::game_object::GameObject;

/// Wrapper for a managed `UnityEngine.Transform`.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    /// Pointer to the internal IL2CPP object
    pub ptr: *mut Il2cppObject,
}

impl ComponentTrait for Transform {
    fn from_ptr(ptr: *mut c_void) -> Self {
        Self {
            ptr: ptr as *mut Il2cppObject,
        }
    }
}

impl Transform {
    /// Creates a `Transform` from a raw managed pointer.
    pub unsafe fn from_ptr(ptr: *mut c_void) -> Self {
        <Self as ComponentTrait>::from_ptr(ptr)
    }

    /// Returns the raw managed pointer.
    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr as *mut c_void
    }

    /// Returns the `GameObject` attached to this transform.
    pub fn get_game_object(&self) -> Result<GameObject, String> {
        unsafe {
            let obj = Object::from_ptr(self.as_ptr());
            let ptr = obj
                .method("get_gameObject")
                .ok_or("Method 'get_gameObject' not found")?
                .call::<*mut c_void>(&[])?;

            if ptr.is_null() {
                return Err("Transform.gameObject is null".to_string());
            }

            Ok(GameObject::from_ptr(ptr))
        }
    }

    /// Returns the world position.
    pub fn get_position(&self) -> Result<Vector3, String> {
        unsafe {
            let obj = Object::from_ptr(self.as_ptr());
            obj.method("get_position")
                .ok_or("Method 'get_position' not found")?
                .call::<Vector3>(&[])
        }
    }

    /// Sets the world position.
    pub fn set_position(&self, value: Vector3) -> Result<(), String> {
        unsafe {
            let obj = Object::from_ptr(self.as_ptr());
            let params = [&value as *const Vector3 as *mut c_void];
            obj.method("set_position")
                .ok_or("Method 'set_position' not found")?
                .call::<()>(&params)
        }
    }

    /// Returns the local position relative to the parent transform.
    pub fn local_position(&self) -> Result<Vector3, String> {
        unsafe {
            let obj = Object::from_ptr(self.as_ptr());
            obj.method("get_localPosition")
                .ok_or("Method 'get_localPosition' not found")?
                .call::<Vector3>(&[])
        }
    }

    /// Sets the local position relative to the parent transform.
    pub fn set_local_position(&self, value: Vector3) -> Result<(), String> {
        unsafe {
            let obj = Object::from_ptr(self.as_ptr());
            let params = [&value as *const Vector3 as *mut c_void];
            obj.method("set_localPosition")
                .ok_or("Method 'set_localPosition' not found")?
                .call::<()>(&params)
        }
    }

    /// Returns the world rotation.
    pub fn get_rotation(&self) -> Result<Quaternion, String> {
        unsafe {
            let obj = Object::from_ptr(self.as_ptr());
            obj.method("get_rotation")
                .ok_or("Method 'get_rotation' not found")?
                .call::<Quaternion>(&[])
        }
    }

    /// Sets the world rotation.
    pub fn set_rotation(&self, value: Quaternion) -> Result<(), String> {
        unsafe {
            let obj = Object::from_ptr(self.as_ptr());
            let params = [&value as *const Quaternion as *mut c_void];
            obj.method("set_rotation")
                .ok_or("Method 'set_rotation' not found")?
                .call::<()>(&params)
        }
    }

    /// Returns the local rotation relative to the parent transform.
    pub fn local_rotation(&self) -> Result<Quaternion, String> {
        unsafe {
            let obj = Object::from_ptr(self.as_ptr());
            obj.method("get_localRotation")
                .ok_or("Method 'get_localRotation' not found")?
                .call::<Quaternion>(&[])
        }
    }

    /// Sets the local rotation relative to the parent transform.
    pub fn set_local_rotation(&self, value: Quaternion) -> Result<(), String> {
        unsafe {
            let obj = Object::from_ptr(self.as_ptr());
            let params = [&value as *const Quaternion as *mut c_void];
            obj.method("set_localRotation")
                .ok_or("Method 'set_localRotation' not found")?
                .call::<()>(&params)
        }
    }

    /// Returns the local scale.
    pub fn get_local_scale(&self) -> Result<Vector3, String> {
        unsafe {
            let obj = Object::from_ptr(self.as_ptr());
            obj.method("get_localScale")
                .ok_or("Method 'get_localScale' not found")?
                .call::<Vector3>(&[])
        }
    }

    /// Sets the local scale.
    pub fn set_local_scale(&self, value: Vector3) -> Result<(), String> {
        unsafe {
            let obj = Object::from_ptr(self.as_ptr());
            let params = [&value as *const Vector3 as *mut c_void];
            obj.method("set_localScale")
                .ok_or("Method 'set_localScale' not found")?
                .call::<()>(&params)
        }
    }

    /// Returns the lossy world scale approximation.
    pub fn get_lossy_scale(&self) -> Result<Vector3, String> {
        unsafe {
            let obj = Object::from_ptr(self.as_ptr());
            obj.method("get_lossyScale")
                .ok_or("Method 'get_lossyScale' not found")?
                .call::<Vector3>(&[])
        }
    }

    /// Returns the number of direct child transforms.
    pub fn get_child_count(&self) -> i32 {
        unsafe {
            let obj = Object::from_ptr(self.as_ptr());
            if let Some(method) = obj.method("get_childCount") {
                return method.call::<i32>(&[]).unwrap_or(0);
            }
            0
        }
    }

    /// Returns the child transform at `index`.
    pub fn get_child(&self, index: i32) -> Result<Transform, String> {
        unsafe {
            let obj = Object::from_ptr(self.as_ptr());
            let params = [&index as *const i32 as *mut c_void];
            let ptr = obj
                .method(("GetChild", ["System.Int32"]))
                .ok_or("Method 'GetChild' not found")?
                .call::<*mut c_void>(&params)?;

            if ptr.is_null() {
                return Err("Child is null".to_string());
            }

            Ok(Transform::from_ptr(ptr))
        }
    }
}
