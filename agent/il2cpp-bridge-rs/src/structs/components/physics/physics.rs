//! Unity Physics helper wrapper
use crate::structs::components::physics::collider::Collider;
use crate::structs::components::physics::layer_mask::LayerMask;
use crate::structs::math::{Ray, RaycastHit, Vector3};
use std::ffi::c_void;

#[repr(transparent)]
pub struct Physics;

impl Physics {
    /// Gets the Physics class definition
    ///
    /// # Returns
    /// * `Option<Class>` - The UnityEngine.Physics class
    pub fn get_class() -> Option<crate::structs::core::Class> {
        crate::api::cache::coremodule().class("UnityEngine.Physics")
    }

    /// Casts a ray against all colliders
    ///
    /// # Arguments
    /// * `origin` - The starting point of the ray
    /// * `direction` - The direction of the ray
    /// * `hit_info` - Output structure containing hit information
    /// * `max_distance` - The maximum distance the ray should check
    /// * `layer_mask` - A LayerMask that filters which colliders to check
    ///
    /// # Returns
    /// * `bool` - True if the ray hits a collider
    pub fn raycast(
        origin: Vector3,
        direction: Vector3,
        hit_info: &mut RaycastHit,
        max_distance: f32,
        layer_mask: LayerMask,
    ) -> bool {
        if let Some(class) = Self::get_class() {
            if let Some(method) = class.method((
                "Raycast",
                [
                    "UnityEngine.Vector3",
                    "UnityEngine.Vector3",
                    "UnityEngine.RaycastHit&",
                    "System.Single",
                    "System.Int32",
                ],
            )) {
                let mut origin_cp = origin;
                let mut direction_cp = direction;
                let mut max_distance_cp = max_distance;
                let mut layer_mask_cp = layer_mask.value;

                let params = &mut [
                    &mut origin_cp as *mut Vector3 as *mut c_void,
                    &mut direction_cp as *mut Vector3 as *mut c_void,
                    hit_info as *mut RaycastHit as *mut c_void,
                    &mut max_distance_cp as *mut f32 as *mut c_void,
                    &mut layer_mask_cp as *mut i32 as *mut c_void,
                ];

                let res = unsafe { method.call::<bool>(params) };
                return res.unwrap_or(false);
            }
        }
        false
    }

    /// Casts a ray using a Ray struct
    ///
    /// # Arguments
    /// * `ray` - The Ray to cast
    /// * `hit_info` - Output structure containing hit information
    /// * `max_distance` - The maximum distance the ray should check
    /// * `layer_mask` - A LayerMask that filters which colliders to check
    ///
    /// # Returns
    /// * `bool` - True if the ray hits a collider
    pub fn raycast_ray(
        ray: Ray,
        hit_info: &mut RaycastHit,
        max_distance: f32,
        layer_mask: LayerMask,
    ) -> bool {
        if let Some(class) = Self::get_class() {
            if let Some(method) = class.method((
                "Raycast",
                [
                    "UnityEngine.Ray",
                    "UnityEngine.RaycastHit&",
                    "System.Single",
                    "System.Int32",
                ],
            )) {
                let mut ray_cp = ray;
                let mut max_distance_cp = max_distance;
                let mut layer_mask_cp = layer_mask.value;

                let params = &mut [
                    &mut ray_cp as *mut Ray as *mut c_void,
                    hit_info as *mut RaycastHit as *mut c_void,
                    &mut max_distance_cp as *mut f32 as *mut c_void,
                    &mut layer_mask_cp as *mut i32 as *mut c_void,
                ];

                let res = unsafe { method.call::<bool>(params) };
                return res.unwrap_or(false);
            }
        }
        false
    }

    /// Casts a sphere along a ray
    ///
    /// # Arguments
    /// * `origin` - The center of the sphere at the start of the sweep
    /// * `radius` - The radius of the sphere
    /// * `direction` - The direction into which to sweep the sphere
    /// * `hit_info` - Output structure containing hit information
    /// * `max_distance` - The maximum distance the sphere should check
    /// * `layer_mask` - A LayerMask that filters which colliders to check
    ///
    /// # Returns
    /// * `bool` - True if the sphere hits a collider
    pub fn sphere_cast(
        origin: Vector3,
        radius: f32,
        direction: Vector3,
        hit_info: &mut RaycastHit,
        max_distance: f32,
        layer_mask: LayerMask,
    ) -> bool {
        if let Some(class) = Self::get_class() {
            if let Some(method) = class.method((
                "SphereCast",
                [
                    "UnityEngine.Vector3",
                    "System.Single",
                    "UnityEngine.Vector3",
                    "UnityEngine.RaycastHit&",
                    "System.Single",
                    "System.Int32",
                ],
            )) {
                let mut origin_cp = origin;
                let mut radius_cp = radius;
                let mut direction_cp = direction;
                let mut max_distance_cp = max_distance;
                let mut layer_mask_cp = layer_mask.value;

                let params = &mut [
                    &mut origin_cp as *mut Vector3 as *mut c_void,
                    &mut radius_cp as *mut f32 as *mut c_void,
                    &mut direction_cp as *mut Vector3 as *mut c_void,
                    hit_info as *mut RaycastHit as *mut c_void,
                    &mut max_distance_cp as *mut f32 as *mut c_void,
                    &mut layer_mask_cp as *mut i32 as *mut c_void,
                ];

                let res = unsafe { method.call::<bool>(params) };
                return res.unwrap_or(false);
            }
        }
        false
    }

    /// Computes and stores colliders touching or inside the sphere
    ///
    /// # Arguments
    /// * `position` - Center of the sphere
    /// * `radius` - Radius of the sphere
    /// * `layer_mask` - A LayerMask that filters which colliders to check
    ///
    /// # Returns
    /// * `Vec<Collider>` - A list of colliders that overlap the sphere
    pub fn overlap_sphere(position: Vector3, radius: f32, layer_mask: LayerMask) -> Vec<Collider> {
        if let Some(class) = Self::get_class() {
            if let Some(method) = class.method((
                "OverlapSphere",
                ["UnityEngine.Vector3", "System.Single", "System.Int32"],
            )) {
                let mut position_cp = position;
                let mut radius_cp = radius;
                let mut layer_mask_cp = layer_mask.value;

                let params = &mut [
                    &mut position_cp as *mut Vector3 as *mut c_void,
                    &mut radius_cp as *mut f32 as *mut c_void,
                    &mut layer_mask_cp as *mut i32 as *mut c_void,
                ];

                let res = unsafe { method.call::<*mut c_void>(params) };
                if let Ok(ptr) = res {
                    if !ptr.is_null() {
                        let array_ptr =
                            ptr as *mut crate::structs::collections::Il2cppArray<*mut c_void>;
                        let mut colliders = Vec::new();
                        let len = unsafe { (*array_ptr).max_length };
                        for i in 0..len {
                            let item_ptr = unsafe { (*array_ptr).at(i) };
                            if !item_ptr.is_null() {
                                unsafe {
                                    colliders.push(Collider::from_ptr(item_ptr));
                                }
                            }
                        }
                        return colliders;
                    }
                }
            }
        }
        Vec::new()
    }

    /// Gets global gravity
    ///
    /// # Returns
    /// * `Vector3` - The gravity vector
    pub fn get_gravity() -> Vector3 {
        if let Some(class) = Self::get_class() {
            if let Some(method) = class.method("get_gravity") {
                let res = unsafe { method.call::<Vector3>(&[]) };
                return res.unwrap_or(Vector3::ZERO);
            }
        }
        Vector3::ZERO
    }

    /// Sets global gravity
    ///
    /// # Arguments
    /// * `value` - The new gravity vector
    pub fn set_gravity(value: Vector3) {
        if let Some(class) = Self::get_class() {
            if let Some(method) = class.method("set_gravity") {
                let mut value_cp = value;
                let params = &mut [&mut value_cp as *mut Vector3 as *mut c_void];
                let _ = unsafe { method.call::<()>(params) };
            }
        }
    }
}
