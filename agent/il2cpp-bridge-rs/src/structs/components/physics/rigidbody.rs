//! Unity Rigidbody component wrapper
use crate::structs::components::{Component, ComponentTrait};
use crate::structs::Vector3;
use std::ffi::c_void;
use std::ops::Deref;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Rigidbody {
    /// Base Component structure
    pub component: Component,
}

impl ComponentTrait for Rigidbody {
    fn from_ptr(ptr: *mut c_void) -> Self {
        Self {
            component: Component::from_ptr(ptr),
        }
    }
}

impl Deref for Rigidbody {
    type Target = Component;
    fn deref(&self) -> &Self::Target {
        &self.component
    }
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForceMode {
    /// Applies a continuous force to the rigidbody, using its mass
    Force = 0,
    /// Applies a continuous acceleration to the rigidbody, ignoring its mass
    Acceleration = 5,
    /// Applies an instant force impulse to the rigidbody, using its mass
    Impulse = 1,
    /// Applies an instant velocity change to the rigidbody, ignoring its mass
    VelocityChange = 2,
}

impl Rigidbody {
    /// Checks if collision detection is enabled
    ///
    /// # Returns
    /// * `Result<bool, String>` - True if collision detection is enabled
    pub fn get_detect_collisions(&self) -> Result<bool, String> {
        unsafe {
            let result = self
                .method("get_detectCollisions")
                .ok_or("Method 'get_detectCollisions' not found")?
                .call::<bool>(&[])?;
            Ok(result)
        }
    }

    /// Sets whether collision detection is enabled
    ///
    /// # Arguments
    /// * `value` - True to enable collision detection
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_detect_collisions(&self, value: bool) -> Result<(), String> {
        unsafe {
            self.method("set_detectCollisions")
                .ok_or("Method 'set_detectCollisions' not found")?
                .call::<()>(&[&value as *const bool as *mut c_void])?;
            Ok(())
        }
    }

    /// Gets the velocity of the rigidbody
    ///
    /// # Returns
    /// * `Result<Vector3, String>` - The velocity vector
    pub fn get_velocity(&self) -> Result<Vector3, String> {
        unsafe {
            let result = self
                .method("get_velocity")
                .ok_or("Method 'get_velocity' not found")?
                .call::<Vector3>(&[])?;
            Ok(result)
        }
    }

    /// Sets the velocity of the rigidbody
    ///
    /// # Arguments
    /// * `value` - The new velocity vector
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_velocity(&self, value: Vector3) -> Result<(), String> {
        unsafe {
            self.method("set_velocity")
                .ok_or("Method 'set_velocity' not found")?
                .call::<()>(&[&value as *const Vector3 as *mut c_void])?;
            Ok(())
        }
    }
    /// Adds a force to the rigidbody
    ///
    /// # Arguments
    /// * `force` - The force vector to apply
    /// * `mode` - The ForceMode to use
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn add_force(&self, force: Vector3, mode: ForceMode) -> Result<(), String> {
        unsafe {
            let mut force_cp = force;
            let mut mode_cp = mode as i32;
            let params = &[
                &mut force_cp as *mut Vector3 as *mut c_void,
                &mut mode_cp as *mut i32 as *mut c_void,
            ];
            self.method(("AddForce", 2))
                .ok_or("Method 'AddForce' not found")?
                .call::<()>(params)?;
            Ok(())
        }
    }

    /// Gets mass
    ///
    /// # Returns
    /// * `Result<f32, String>` - The mass of the rigidbody
    pub fn get_mass(&self) -> Result<f32, String> {
        unsafe {
            self.method("get_mass")
                .ok_or("Method 'get_mass' not found")?
                .call::<f32>(&[])
        }
    }

    /// Sets mass
    ///
    /// # Arguments
    /// * `value` - The new mass
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_mass(&self, value: f32) -> Result<(), String> {
        unsafe {
            let mut value_cp = value;
            self.method("set_mass")
                .ok_or("Method 'set_mass' not found")?
                .call::<()>(&[&mut value_cp as *mut f32 as *mut c_void])?;
            Ok(())
        }
    }

    /// Gets drag
    ///
    /// # Returns
    /// * `Result<f32, String>` - The linear drag coefficient
    pub fn get_drag(&self) -> Result<f32, String> {
        unsafe {
            self.method("get_drag")
                .ok_or("Method 'get_drag' not found")?
                .call::<f32>(&[])
        }
    }

    /// Sets drag
    ///
    /// # Arguments
    /// * `value` - The new linear drag coefficient
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_drag(&self, value: f32) -> Result<(), String> {
        unsafe {
            let mut value_cp = value;
            self.method("set_drag")
                .ok_or("Method 'set_drag' not found")?
                .call::<()>(&[&mut value_cp as *mut f32 as *mut c_void])?;
            Ok(())
        }
    }

    /// Gets angular drag
    ///
    /// # Returns
    /// * `Result<f32, String>` - The angular drag coefficient
    pub fn get_angular_drag(&self) -> Result<f32, String> {
        unsafe {
            self.method("get_angularDrag")
                .ok_or("Method 'get_angularDrag' not found")?
                .call::<f32>(&[])
        }
    }

    /// Sets angular drag
    ///
    /// # Arguments
    /// * `value` - The new angular drag coefficient
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_angular_drag(&self, value: f32) -> Result<(), String> {
        unsafe {
            let mut value_cp = value;
            self.method("set_angularDrag")
                .ok_or("Method 'set_angularDrag' not found")?
                .call::<()>(&[&mut value_cp as *mut f32 as *mut c_void])?;
            Ok(())
        }
    }

    /// Checks if kinematic
    ///
    /// # Returns
    /// * `Result<bool, String>` - True if the rigidbody is kinematic
    pub fn get_is_kinematic(&self) -> Result<bool, String> {
        unsafe {
            self.method("get_isKinematic")
                .ok_or("Method 'get_isKinematic' not found")?
                .call::<bool>(&[])
        }
    }

    /// Sets kinematic
    ///
    /// # Arguments
    /// * `value` - True to set the rigidbody as kinematic
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_is_kinematic(&self, value: bool) -> Result<(), String> {
        unsafe {
            let mut value_cp = value;
            self.method("set_isKinematic")
                .ok_or("Method 'set_isKinematic' not found")?
                .call::<()>(&[&mut value_cp as *mut bool as *mut c_void])?;
            Ok(())
        }
    }

    /// Checks if gravity is used
    ///
    /// # Returns
    /// * `Result<bool, String>` - True if gravity is applied
    pub fn get_use_gravity(&self) -> Result<bool, String> {
        unsafe {
            self.method("get_useGravity")
                .ok_or("Method 'get_useGravity' not found")?
                .call::<bool>(&[])
        }
    }

    /// Sets use gravity
    ///
    /// # Arguments
    /// * `value` - True to apply gravity
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_use_gravity(&self, value: bool) -> Result<(), String> {
        unsafe {
            let mut value_cp = value;
            self.method("set_useGravity")
                .ok_or("Method 'set_useGravity' not found")?
                .call::<()>(&[&mut value_cp as *mut bool as *mut c_void])?;
            Ok(())
        }
    }
}
