//! Thin wrapper around `UnityEngine.Time`.
//!
//! These helpers are intentionally small and are mainly useful as examples of
//! cache-backed static property access.

use crate::api::cache;
use crate::structs::core::Class;
use std::ffi::c_void;

/// Accessors for selected `UnityEngine.Time` properties.
pub struct Time;

impl Time {
    /// Gets the `UnityEngine.Time` class
    pub fn get_class() -> Option<Class> {
        cache::coremodule().class("UnityEngine.Time")
    }

    /// The time at the beginning of this frame (Read Only)
    pub fn get_time() -> f32 {
        unsafe {
            Self::get_class()
                .and_then(|cls| cls.method(("get_time", 0)))
                .map(|method| method.call::<f32>(&[] as &[*mut c_void]).unwrap_or(0.0))
                .unwrap_or(0.0)
        }
    }

    /// The time in seconds it took to complete the last frame (Read Only)
    pub fn get_delta_time() -> f32 {
        unsafe {
            Self::get_class()
                .and_then(|cls| cls.method(("get_deltaTime", 0)))
                .map(|method| method.call::<f32>(&[] as &[*mut c_void]).unwrap_or(0.0))
                .unwrap_or(0.0)
        }
    }

    /// The interval in seconds at which physics and other fixed frame rate updates (like MonoBehaviour's FixedUpdate) are performed.
    pub fn get_fixed_delta_time() -> f32 {
        unsafe {
            Self::get_class()
                .and_then(|cls| cls.method(("get_fixedDeltaTime", 0)))
                .map(|method| method.call::<f32>(&[] as &[*mut c_void]).unwrap_or(0.0))
                .unwrap_or(0.0)
        }
    }

    /// The scale at which time passes. This can be used for slow motion effects.
    pub fn get_time_scale() -> f32 {
        unsafe {
            Self::get_class()
                .and_then(|cls| cls.method(("get_timeScale", 0)))
                .map(|method| method.call::<f32>(&[] as &[*mut c_void]).unwrap_or(1.0))
                .unwrap_or(1.0)
        }
    }

    /// Sets the scale at which time passes.
    pub fn set_time_scale(value: f32) {
        unsafe {
            if let Some(method) = Self::get_class().and_then(|cls| cls.method(("set_timeScale", 1)))
            {
                let _ = method.call::<()>(&[&mut { value } as *mut f32 as *mut c_void]);
            }
        }
    }
}
