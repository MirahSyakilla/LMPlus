//! Unity Camera component wrapper
use super::screen::Screen;
use crate::api::cache;
use crate::structs::collections::Il2cppArray;
use crate::structs::components::{Component, ComponentTrait};
use crate::structs::math::{Matrix4x4, Ray, Vector2, Vector3};
use std::ffi::c_void;
use std::ops::Deref;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraEye {
    Left = 0,
    Right = 1,
    Mono = 2,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// Base Component structure
    pub component: Component,
}

impl ComponentTrait for Camera {
    fn from_ptr(ptr: *mut c_void) -> Self {
        Self {
            component: Component::from_ptr(ptr),
        }
    }
}

impl Deref for Camera {
    type Target = Component;
    fn deref(&self) -> &Self::Target {
        &self.component
    }
}

impl Camera {
    /// Gets the Camera class definition
    ///
    /// # Returns
    /// * `Option<Class>` - The UnityEngine.Camera class
    pub fn get_class() -> Option<crate::structs::core::Class> {
        cache::coremodule().class("UnityEngine.Camera")
    }

    /// Gets the main camera in the scene (Camera.main)
    ///
    /// # Returns
    /// * `Result<Option<Camera>, String>` - The main camera if it exists
    pub fn get_main() -> Result<Option<Camera>, String> {
        let class = Self::get_class().ok_or("Class 'UnityEngine.Camera' not found")?;
        let method = class
            .method("get_main")
            .ok_or("Method 'get_main' not found")?;
        unsafe {
            let ptr = method.call::<*mut c_void>(&[])?;
            if ptr.is_null() {
                Ok(None)
            } else {
                Ok(Some(Camera::from_ptr(ptr)))
            }
        }
    }

    /// Gets the current camera (for rendering events)
    ///
    /// # Returns
    /// * `Result<Option<Camera>, String>` - The current camera if it exists
    pub fn get_current() -> Result<Option<Camera>, String> {
        let class = Self::get_class().ok_or("Class 'UnityEngine.Camera' not found")?;
        let method = class
            .method("get_current")
            .ok_or("Method 'get_current' not found")?;
        unsafe {
            let ptr = method.call::<*mut c_void>(&[])?;
            if ptr.is_null() {
                Ok(None)
            } else {
                Ok(Some(Camera::from_ptr(ptr)))
            }
        }
    }

    /// Gets the count of all active cameras
    ///
    /// # Returns
    /// * `Result<i32, String>` - The number of all cameras enabled in the scene
    pub fn get_all_count() -> Result<i32, String> {
        let class = Self::get_class().ok_or("Class 'UnityEngine.Camera' not found")?;
        let method = class
            .method("get_allCamerasCount")
            .ok_or("Method 'get_allCamerasCount' not found")?;
        unsafe { method.call::<i32>(&[]) }
    }

    /// Gets a list of all active cameras
    ///
    /// # Returns
    /// * `Result<Vec<Camera>, String>` - A vector containing all active cameras
    pub fn get_all_cameras() -> Result<Vec<Camera>, String> {
        let class = Self::get_class().ok_or("Class 'UnityEngine.Camera' not found")?;
        let method = class
            .method("GetAllCameras")
            .ok_or("Method 'GetAllCameras' not found")?;

        let count = Self::get_all_count()?;
        if count == 0 {
            return Ok(Vec::new());
        }

        let array_ptr = Il2cppArray::<*mut c_void>::new(&class, count as usize);
        if array_ptr.is_null() {
            return Err("Failed to create array".to_string());
        }

        unsafe {
            method.call::<i32>(&[array_ptr as *mut c_void])?;

            let array = &*array_ptr;
            let mut cameras = Vec::with_capacity(count as usize);
            for i in 0..count as usize {
                let ptr = array.at(i);
                if !ptr.is_null() {
                    cameras.push(Camera::from_ptr(ptr));
                }
            }
            Ok(cameras)
        }
    }

    /// Gets the depth of the camera
    ///
    /// # Returns
    /// * `Result<f32, String>` - The camera's depth in the rendering order
    pub fn get_depth(&self) -> Result<f32, String> {
        unsafe {
            self.method("get_depth")
                .ok_or("Method 'get_depth' not found")?
                .call::<f32>(&[])
        }
    }

    /// Sets the depth of the camera
    ///
    /// # Arguments
    /// * `depth` - The new depth value
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_depth(&self, depth: f32) -> Result<(), String> {
        unsafe {
            self.method("set_depth")
                .ok_or("Method 'set_depth' not found")?
                .call::<()>(&[&depth as *const f32 as *mut c_void])?;
        }
        Ok(())
    }

    /// Gets the field of view of the camera
    ///
    /// # Returns
    /// * `Result<f32, String>` - The field of view in degrees
    pub fn get_field_of_view(&self) -> Result<f32, String> {
        unsafe {
            self.method("get_fieldOfView")
                .ok_or("Method 'get_fieldOfView' not found")?
                .call::<f32>(&[])
        }
    }

    /// Sets the field of view of the camera
    ///
    /// # Arguments
    /// * `fov` - The new field of view in degrees
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_field_of_view(&self, fov: f32) -> Result<(), String> {
        unsafe {
            self.method("set_fieldOfView")
                .ok_or("Method 'set_fieldOfView' not found")?
                .call::<()>(&[&fov as *const f32 as *mut c_void])?;
        }
        Ok(())
    }

    /// Converts a point from world space to screen space
    ///
    /// # Arguments
    /// * `position` - The world position to convert
    /// * `eye` - The camera eye to use for conversion
    ///
    /// # Returns
    /// * `Result<Vector3, String>` - The screen point
    pub fn world_to_screen_point(
        &self,
        position: Vector3,
        eye: CameraEye,
    ) -> Result<Vector3, String> {
        unsafe {
            self.method("WorldToScreenPoint")
                .ok_or("Method 'WorldToScreenPoint' not found")?
                .call::<Vector3>(&[
                    &position as *const Vector3 as *mut c_void,
                    &eye as *const CameraEye as *mut c_void,
                ])
        }
    }

    /// Converts a point from screen space to world space
    ///
    /// # Arguments
    /// * `position` - The screen position to convert
    /// * `eye` - The camera eye to use for conversion
    ///
    /// # Returns
    /// * `Result<Vector3, String>` - The world point
    pub fn screen_to_world_point(
        &self,
        position: Vector3,
        eye: CameraEye,
    ) -> Result<Vector3, String> {
        unsafe {
            self.method("ScreenToWorldPoint")
                .ok_or("Method 'ScreenToWorldPoint' not found")?
                .call::<Vector3>(&[
                    &position as *const Vector3 as *mut c_void,
                    &eye as *const CameraEye as *mut c_void,
                ])
        }
    }

    /// Gets the camera to world matrix
    ///
    /// # Returns
    /// * `Result<Matrix4x4, String>` - The matrix that transforms from camera space to world space
    pub fn camera_to_world_matrix(&self) -> Result<Matrix4x4, String> {
        unsafe {
            self.method("get_cameraToWorldMatrix")
                .ok_or("Method 'get_cameraToWorldMatrix' not found")?
                .call::<Matrix4x4>(&[])
        }
    }

    /// Converts world position to screen coordinates and checks if it's on screen
    ///
    /// # Arguments
    /// * `position` - The world position
    /// * `eye` - The camera eye
    ///
    /// # Returns
    /// * `Result<(Vector2, bool), String>` - A tuple of (screen position, is on screen)
    pub fn world_to_screen(
        &self,
        position: Vector3,
        eye: CameraEye,
    ) -> Result<(Vector2, bool), String> {
        let mut screen_point = self.world_to_screen_point(position, eye)?;

        if screen_point.z < 0.01 {
            return Ok((Vector2 { x: 0.0, y: 0.0 }, false));
        }

        let screen_width = Screen::get_width()? as f32;
        let screen_height = Screen::get_height()? as f32;

        screen_point.y = screen_height - screen_point.y;

        let on_screen = screen_point.x > 0.0
            && screen_point.x < screen_width
            && screen_point.y > 0.0
            && screen_point.y < screen_height;

        Ok((
            Vector2 {
                x: screen_point.x,
                y: screen_point.y,
            },
            on_screen,
        ))
    }

    /// Returns a ray from the camera through a screen point
    ///
    /// # Arguments
    /// * `position` - The screen position
    /// * `eye` - The camera eye
    ///
    /// # Returns
    /// * `Result<Ray, String>` - The ray passing through the screen point
    pub fn screen_point_to_ray(&self, position: Vector2, eye: CameraEye) -> Result<Ray, String> {
        unsafe {
            self.method("ScreenPointToRay")
                .ok_or("Method 'ScreenPointToRay' not found")?
                .call::<Ray>(&[
                    &position as *const Vector2 as *mut c_void,
                    &eye as *const CameraEye as *mut c_void,
                ])
        }
    }
}
