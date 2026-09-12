//! Unity `Component` wrapper and conversion trait.
use super::game_object::GameObject;
use super::transform::Transform;
use super::unity_object::UnityObject;
use crate::structs::core::{Class, Il2cppObject};
use std::ffi::c_void;
use std::ops::Deref;

/// Wrapper for a managed `UnityEngine.Component`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Component {
    /// Base UnityObject structure
    pub object: UnityObject,
    /// Cached pointer to the component
    pub m_cached_ptr: *mut c_void,
}

/// Trait implemented by Unity component wrappers that can be constructed from a raw pointer.
pub trait ComponentTrait {
    /// Creates the wrapper from a raw managed object pointer.
    fn from_ptr(ptr: *mut c_void) -> Self;
}

impl ComponentTrait for Component {
    fn from_ptr(ptr: *mut c_void) -> Self {
        let object = UnityObject::from_ptr(ptr);
        let m_cached_ptr = unsafe {
            let offset = std::mem::size_of::<Il2cppObject>() as isize;
            *(ptr.offset(offset) as *mut *mut c_void)
        };
        Self {
            object,
            m_cached_ptr,
        }
    }
}

impl Component {
    /// Creates a `Component` wrapper from a raw pointer.
    pub fn from_ptr(ptr: *mut c_void) -> Self {
        <Self as ComponentTrait>::from_ptr(ptr)
    }

    /// Returns the raw managed pointer.
    pub fn as_ptr(&self) -> *mut c_void {
        self.object.as_ptr()
    }

    /// Returns the `GameObject` attached to this component.
    pub fn get_game_object(&self) -> Result<GameObject, String> {
        unsafe {
            let ptr = self
                .method("get_gameObject")
                .ok_or("Method 'get_gameObject' not found")?
                .call::<*mut c_void>(&[])?;

            if ptr.is_null() {
                return Err("Component.gameObject is null".to_string());
            }

            Ok(GameObject::from_ptr(ptr))
        }
    }

    /// Returns the `Transform` attached to this component.
    pub fn get_transform(&self) -> Result<Transform, String> {
        unsafe {
            let ptr = self
                .method("get_transform")
                .ok_or("Method 'get_transform' not found")?
                .call::<*mut c_void>(&[])?;

            if ptr.is_null() {
                return Err("Component.transform is null".to_string());
            }

            Ok(Transform::from_ptr(ptr))
        }
    }

    /// Resolves another component of the specified class on the same `GameObject`.
    pub fn get_component<T: ComponentTrait>(&self, class: &Class) -> Result<T, String> {
        if class.object.is_null() {
            return Err(format!(
                "Class '{}' does not have a valid System.Type object",
                class.name
            ));
        }

        unsafe {
            let ptr = self
                .method(("GetComponent", ["System.Type"]))
                .ok_or("Method 'GetComponent' not found")?
                .call::<*mut c_void>(&[class.object])?;

            if ptr.is_null() {
                return Err("Component not found".to_string());
            }

            Ok(T::from_ptr(ptr))
        }
    }
}

impl Deref for Component {
    type Target = UnityObject;

    fn deref(&self) -> &Self::Target {
        &self.object
    }
}
