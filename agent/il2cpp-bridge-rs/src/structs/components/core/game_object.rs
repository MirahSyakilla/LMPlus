//! Unity `GameObject` wrapper.
use super::component::ComponentTrait;
use super::transform::Transform;
use super::unity_object::UnityObject;
use crate::api::cache;
use crate::structs::components::scene::scene_management::Scene;
use crate::structs::core::{Class, Il2cppObject};
use crate::structs::Il2cppString;
use std::ffi::c_void;
use std::ops::Deref;

/// Wrapper for a managed `UnityEngine.GameObject`.
///
/// This builds on top of [`UnityObject`] and is useful once you already have a
/// live Unity object or want to find a scene object by name.
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct GameObject {
    /// Base UnityObject structure
    pub object: UnityObject,
}

impl Deref for GameObject {
    type Target = UnityObject;

    fn deref(&self) -> &Self::Target {
        &self.object
    }
}

impl GameObject {
    /// Creates a GameObject from a raw pointer
    ///
    /// # Arguments
    /// * `ptr` - The raw pointer to the GameObject
    ///
    /// # Returns
    /// * `Self` - The created GameObject wrapper
    pub fn from_ptr(ptr: *mut c_void) -> Self {
        Self {
            object: UnityObject::from_ptr(ptr),
        }
    }

    /// Internally creates a new GameObject
    ///
    /// # Arguments
    /// * `obj` - The GameObject instance to initialize
    /// * `name` - The name of the new GameObject
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if creation succeeds, failure otherwise
    pub fn internal_create(obj: &mut GameObject, name: &str) -> Result<(), String> {
        let core_module = cache::coremodule();
        let game_object_class = core_module
            .class("GameObject")
            .ok_or_else(|| "Could not find GameObject class".to_string())?;

        let create_method = game_object_class
            .method("Internal_CreateGameObject")
            .ok_or_else(|| {
                "Could not find GameObject.Internal_CreateGameObject method".to_string()
            })?;

        unsafe {
            let name_str = Il2cppString::new(name);
            create_method.call::<()>(&[obj.as_ptr(), name_str as *mut c_void])?;
            Ok(())
        }
    }

    /// Finds a scene `GameObject` by name.
    pub fn find(name: &str) -> Result<GameObject, String> {
        let core_module = cache::coremodule();
        let game_object_class = core_module
            .class("GameObject")
            .ok_or_else(|| "Could not find GameObject class".to_string())?;

        let find_method = game_object_class
            .method("Find")
            .ok_or_else(|| "Could not find GameObject.Find method".to_string())?;

        unsafe {
            let name_str = Il2cppString::new(name);
            let ptr = find_method.call::<*mut Il2cppObject>(&[name_str as *mut c_void])?;

            if ptr.is_null() {
                return Err("GameObject not found".to_string());
            }

            Ok(GameObject::from_ptr(ptr as *mut c_void))
        }
    }

    /// Gets a component of the specified class attached to this GameObject
    ///
    /// # Type Parameters
    /// * `T` - The type of component to retrieve (must implement ComponentTrait)
    ///
    /// # Arguments
    /// * `class` - The IL2CPP class of the component
    ///
    /// # Returns
    /// * `Result<T, String>` - The requested component, or error if not found
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

    /// Gets the Transform attached to this GameObject
    ///
    /// # Returns
    /// * `Result<Transform, String>` - The Transform attached to this GameObject
    pub fn get_transform(&self) -> Result<Transform, String> {
        unsafe {
            let ptr = self
                .method("get_transform")
                .ok_or("Method 'get_transform' not found")?
                .call::<*mut Il2cppObject>(&[])?;

            if ptr.is_null() {
                return Err("Transform is null".to_string());
            }

            Ok(Transform::from_ptr(ptr as *mut c_void))
        }
    }

    /// Checks if the GameObject is active in the scene
    ///
    /// # Returns
    /// * `Result<bool, String>` - True if active self
    pub fn get_active_self(&self) -> Result<bool, String> {
        unsafe {
            self.method("get_activeSelf")
                .ok_or("Method 'get_activeSelf' not found")?
                .call::<bool>(&[])
        }
    }

    /// Checks if the GameObject is active in the hierarchy
    ///
    /// # Returns
    /// * `Result<bool, String>` - True if active in hierarchy
    pub fn get_active_in_hierarchy(&self) -> Result<bool, String> {
        unsafe {
            self.method("get_activeInHierarchy")
                .ok_or("Method 'get_activeInHierarchy' not found")?
                .call::<bool>(&[])
        }
    }

    /// Sets the active state of the GameObject
    ///
    /// # Arguments
    /// * `active` - Whether the GameObject should be active
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_active(&self, active: bool) -> Result<(), String> {
        unsafe {
            let mut arg = active;
            self.method("SetActive")
                .ok_or("Method 'SetActive' not found")?
                .call::<()>(&[&mut arg as *mut bool as *mut c_void])?;
            Ok(())
        }
    }

    /// Gets the layer of the GameObject
    ///
    /// # Returns
    /// * `Result<i32, String>` - The layer index
    pub fn get_layer(&self) -> Result<i32, String> {
        unsafe {
            self.method("get_layer")
                .ok_or("Method 'get_layer' not found")?
                .call::<i32>(&[])
        }
    }

    /// Sets the layer of the GameObject
    ///
    /// # Arguments
    /// * `layer` - The layer index to set
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_layer(&self, layer: i32) -> Result<(), String> {
        unsafe {
            let mut arg = layer;
            self.method("set_layer")
                .ok_or("Method 'set_layer' not found")?
                .call::<()>(&[&mut arg as *mut i32 as *mut c_void])?;
            Ok(())
        }
    }

    /// Checks if the GameObject is static
    ///
    /// # Returns
    /// * `Result<bool, String>` - True if the object is static
    pub fn get_is_static(&self) -> Result<bool, String> {
        unsafe {
            self.method("get_isStatic")
                .ok_or("Method 'get_isStatic' not found")?
                .call::<bool>(&[])
        }
    }

    /// Gets the scene that the GameObject belongs to
    ///
    /// # Returns
    /// * `Result<Scene, String>` - The scene this GameObject belongs to
    pub fn get_scene(&self) -> Result<Scene, String> {
        unsafe {
            self.method("get_scene")
                .ok_or("Method 'get_scene' not found")?
                .call::<Scene>(&[])
        }
    }
}
