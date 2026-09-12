//! Unity Collider component wrapper
use crate::structs::components::{Component, ComponentTrait};
use crate::structs::math::Bounds;
use crate::structs::Vector3;
use std::ffi::c_void;
use std::ops::Deref;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Collider {
    /// Base Component structure
    pub component: Component,
}

impl ComponentTrait for Collider {
    fn from_ptr(ptr: *mut c_void) -> Self {
        Self {
            component: Component::from_ptr(ptr),
        }
    }
}

impl Deref for Collider {
    type Target = Component;
    fn deref(&self) -> &Self::Target {
        &self.component
    }
}

impl Collider {
    /// Creates a Collider from a raw pointer
    ///
    /// # Arguments
    /// * `ptr` - The raw pointer to the Collider
    ///
    /// # Returns
    /// * `Self` - The created Collider wrapper
    pub unsafe fn from_ptr(ptr: *mut c_void) -> Self {
        <Self as ComponentTrait>::from_ptr(ptr)
    }

    /// Gets the bounding box of the collider
    ///
    /// # Returns
    /// * `Result<Bounds, String>` - The world-space bounding box
    pub fn get_bounds(&self) -> Result<Bounds, String> {
        let mut bounds = Bounds::default();
        unsafe {
            self.method("get_bounds_Injected")
                .ok_or("Method 'get_bounds_Injected' not found")?
                .call::<()>(&[&mut bounds as *mut Bounds as *mut c_void])?;
        }
        Ok(bounds)
    }
    /// Gets enabled state
    ///
    /// # Returns
    /// * `Result<bool, String>` - True if the collider is enabled
    pub fn get_enabled(&self) -> Result<bool, String> {
        unsafe {
            self.method("get_enabled")
                .ok_or("Method 'get_enabled' not found")?
                .call::<bool>(&[])
        }
    }

    /// Sets enabled state
    ///
    /// # Arguments
    /// * `value` - The new enabled state
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_enabled(&self, value: bool) -> Result<(), String> {
        unsafe {
            let mut value_cp = value;
            self.method("set_enabled")
                .ok_or("Method 'set_enabled' not found")?
                .call::<()>(&[&mut value_cp as *mut bool as *mut c_void])?;
            Ok(())
        }
    }

    /// Gets isTrigger state
    ///
    /// # Returns
    /// * `Result<bool, String>` - True if the collider is a trigger
    pub fn get_is_trigger(&self) -> Result<bool, String> {
        unsafe {
            self.method("get_isTrigger")
                .ok_or("Method 'get_isTrigger' not found")?
                .call::<bool>(&[])
        }
    }

    /// Sets isTrigger state
    ///
    /// # Arguments
    /// * `value` - The new isTrigger state
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_is_trigger(&self, value: bool) -> Result<(), String> {
        unsafe {
            let mut value_cp = value;
            self.method("set_isTrigger")
                .ok_or("Method 'set_isTrigger' not found")?
                .call::<()>(&[&mut value_cp as *mut bool as *mut c_void])?;
            Ok(())
        }
    }

    /// Gets attached Rigidbody
    ///
    /// # Returns
    /// * `Result<Rigidbody, String>` - The rigidbody attached to this collider (or the one it belongs to)
    pub fn get_attached_rigidbody(&self) -> Result<super::rigidbody::Rigidbody, String> {
        unsafe {
            let ptr = self
                .method("get_attachedRigidbody")
                .ok_or("Method 'get_attachedRigidbody' not found")?
                .call::<*mut c_void>(&[])?;

            if ptr.is_null() {
                return Err("No attached Rigidbody".to_string());
            }

            Ok(super::rigidbody::Rigidbody::from_ptr(ptr))
        }
    }

    /// Gets the closest point on the collider to a given position
    ///
    /// # Arguments
    /// * `position` - The position to find the closest point to
    ///
    /// # Returns
    /// * `Result<Vector3, String>` - The closest point on the collider surface
    pub fn closest_point(&self, position: Vector3) -> Result<Vector3, String> {
        unsafe {
            let mut pos_cp = position;
            let result = self
                .method("ClosestPoint")
                .ok_or("Method 'ClosestPoint' not found")?
                .call::<Vector3>(&[&mut pos_cp as *mut Vector3 as *mut c_void])?;
            Ok(result)
        }
    }
}
