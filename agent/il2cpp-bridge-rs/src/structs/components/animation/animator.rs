//! Unity Animator component wrapper
use crate::structs::components::Transform;
use crate::structs::components::{Component, ComponentTrait};
use std::ffi::c_void;
use std::ops::Deref;

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanBodyBones {
    Hips = 0,
    LeftUpperLeg = 1,
    RightUpperLeg = 2,
    LeftLowerLeg = 3,
    RightLowerLeg = 4,
    LeftFoot = 5,
    RightFoot = 6,
    Spine = 7,
    Chest = 8,
    UpperChest = 54,
    Neck = 9,
    Head = 10,
    LeftShoulder = 11,
    RightShoulder = 12,
    LeftUpperArm = 13,
    RightUpperArm = 14,
    LeftLowerArm = 15,
    RightLowerArm = 16,
    LeftHand = 17,
    RightHand = 18,
    LeftToes = 19,
    RightToes = 20,
    LeftEye = 21,
    RightEye = 22,
    Jaw = 23,
    LeftThumbProximal = 24,
    LeftThumbIntermediate = 25,
    LeftThumbDistal = 26,
    LeftIndexProximal = 27,
    LeftIndexIntermediate = 28,
    LeftIndexDistal = 29,
    LeftMiddleProximal = 30,
    LeftMiddleIntermediate = 31,
    LeftMiddleDistal = 32,
    LeftRingProximal = 33,
    LeftRingIntermediate = 34,
    LeftRingDistal = 35,
    LeftLittleProximal = 36,
    LeftLittleIntermediate = 37,
    LeftLittleDistal = 38,
    RightThumbProximal = 39,
    RightThumbIntermediate = 40,
    RightThumbDistal = 41,
    RightIndexProximal = 42,
    RightIndexIntermediate = 43,
    RightIndexDistal = 44,
    RightMiddleProximal = 45,
    RightMiddleIntermediate = 46,
    RightMiddleDistal = 47,
    RightRingProximal = 48,
    RightRingIntermediate = 49,
    RightRingDistal = 50,
    RightLittleProximal = 51,
    RightLittleIntermediate = 52,
    RightLittleDistal = 53,
    LastBone = 55,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Animator {
    /// Base Component structure
    pub component: Component,
}

impl ComponentTrait for Animator {
    fn from_ptr(ptr: *mut c_void) -> Self {
        Self {
            component: Component::from_ptr(ptr),
        }
    }
}

impl Deref for Animator {
    type Target = Component;
    fn deref(&self) -> &Self::Target {
        &self.component
    }
}

impl Animator {
    /// Gets the transform of a specific bone (if the animator has a humanoid avatar)
    ///
    /// # Arguments
    /// * `bone` - The humanoid bone to retrieve
    ///
    /// # Returns
    /// * `Result<Transform, String>` - The Transform of the requested bone
    pub fn get_bone_transform(&self, bone: HumanBodyBones) -> Result<Transform, String> {
        let bone_id = bone as i32;
        unsafe {
            let ptr = self
                .method("GetBoneTransform")
                .ok_or("Method 'GetBoneTransform' not found")?
                .call::<*mut c_void>(&[&bone_id as *const i32 as *mut c_void])?;

            if ptr.is_null() {
                return Err("Bone transform is null".to_string());
            }

            Ok(Transform::from_ptr(ptr))
        }
    }
}
