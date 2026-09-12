//! Unity SceneManager and Scene component wrapper
use crate::logger;
use crate::{
    api::cache,
    structs::{
        collections::{Il2cppArray, Il2cppString},
        components::GameObject,
        core::Class,
    },
};
use std::ffi::c_void;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scene {
    /// Handle to the underlying scene
    pub handle: i32,
}

impl Scene {
    /// Gets the Scene class definition
    fn get_class() -> Option<Class> {
        cache::coremodule().class("UnityEngine.SceneManagement.Scene")
    }

    /// Checks if the scene is valid
    ///
    /// # Returns
    /// * `bool` - True if the scene is valid
    pub fn is_valid(&self) -> bool {
        unsafe {
            if let Some(class) = Self::get_class() {
                if let Some(method) = class.method("IsValidInternal") {
                    return method
                        .call::<bool>(&[&self.handle as *const i32 as *mut c_void])
                        .unwrap_or(false);
                }
            }
            false
        }
    }

    /// Gets the build index of the scene
    ///
    /// # Returns
    /// * `i32` - The index in Build Settings, or -1 if not found
    pub fn build_index(&self) -> i32 {
        unsafe {
            if let Some(class) = Self::get_class() {
                if let Some(method) = class.method("GetBuildIndexInternal") {
                    return method
                        .call::<i32>(&[&self.handle as *const i32 as *mut c_void])
                        .unwrap_or(-1);
                }
            }
            -1
        }
    }

    /// Checks if the scene is loaded
    ///
    /// # Returns
    /// * `bool` - True if the scene is currently loaded
    pub fn is_loaded(&self) -> bool {
        unsafe {
            if let Some(class) = Self::get_class() {
                if let Some(method) = class.method("GetIsLoadedInternal") {
                    return method
                        .call::<bool>(&[&self.handle as *const i32 as *mut c_void])
                        .unwrap_or(false);
                }
            }
            false
        }
    }

    /// Gets the name of the scene
    ///
    /// # Returns
    /// * `String` - The name of the scene
    pub fn name(&self) -> String {
        unsafe {
            if let Some(class) = Self::get_class() {
                if let Some(method) = class.method("GetNameInternal") {
                    if let Ok(ptr) = method
                        .call::<*mut Il2cppString>(&[&self.handle as *const i32 as *mut c_void])
                    {
                        if !ptr.is_null() {
                            return (*ptr).to_string().unwrap_or_default();
                        }
                    }
                }
            }
            String::new()
        }
    }

    /// Gets the path of the scene asset
    ///
    /// # Returns
    /// * `String` - The asset path of the scene
    pub fn path(&self) -> String {
        unsafe {
            if let Some(class) = Self::get_class() {
                if let Some(method) = class.method("GetPathInternal") {
                    if let Ok(ptr) = method
                        .call::<*mut Il2cppString>(&[&self.handle as *const i32 as *mut c_void])
                    {
                        if !ptr.is_null() {
                            return (*ptr).to_string().unwrap_or_default();
                        }
                    }
                }
            }
            String::new()
        }
    }

    /// Gets the count of root game objects in the scene
    ///
    /// # Returns
    /// * `i32` - The number of root GameObjects
    pub fn root_count(&self) -> i32 {
        unsafe {
            if let Some(class) = Self::get_class() {
                if let Some(method) = class.method("GetRootCountInternal") {
                    return method
                        .call::<i32>(&[&self.handle as *const i32 as *mut c_void])
                        .unwrap_or(0);
                }
            }
            0
        }
    }

    /// Gets all root game objects in the scene
    ///
    /// # Returns
    /// * `Vec<GameObject>` - A list of all root GameObjects
    pub fn root_game_objects(&self) -> Vec<GameObject> {
        unsafe {
            let class = match Self::get_class() {
                Some(c) => c,
                None => return Vec::new(),
            };

            let mut method = match class.method(("GetRootGameObjects", 0)) {
                Some(m) => m,
                None => {
                    logger::error("Method 'GetRootGameObjects' with 0 args not found");
                    return Vec::new();
                }
            };

            method.instance = Some(&self.handle as *const i32 as *mut c_void);

            let ptr = method
                .call::<*mut Il2cppArray<*mut c_void>>(&[])
                .unwrap_or(std::ptr::null_mut());

            if ptr.is_null() {
                return Vec::new();
            }

            let array_ptr = ptr;
            let mut objects = Vec::new();
            let array = &*array_ptr;

            for i in 0..array.max_length {
                let obj_ptr = array.at(i);
                if !obj_ptr.is_null() {
                    objects.push(GameObject::from_ptr(obj_ptr));
                }
            }
            objects
        }
    }
}

pub struct SceneManager;

impl SceneManager {
    /// Gets the SceneManager class definition
    fn get_class() -> Option<Class> {
        cache::coremodule().class("UnityEngine.SceneManagement.SceneManager")
    }

    /// Gets the total number of scenes
    ///
    /// # Returns
    /// * `i32` - The number of scenes currently loaded
    pub fn scene_count() -> i32 {
        unsafe {
            if let Some(class) = Self::get_class() {
                if let Some(method) = class.method("get_sceneCount") {
                    return method.call::<i32>(&[]).unwrap_or(0);
                }
            }
            0
        }
    }

    /// Gets the currently active scene
    ///
    /// # Returns
    /// * `Result<Scene, String>` - The active scene
    pub fn get_active_scene() -> Result<Scene, String> {
        let class = Self::get_class()
            .ok_or("Class 'UnityEngine.SceneManagement.SceneManager' not found")?;

        let method = class
            .method("GetActiveScene")
            .ok_or("Method 'GetActiveScene' not found")?;

        unsafe { method.call::<Scene>(&[]) }
    }

    /// Gets the scene at the specified index
    ///
    /// # Arguments
    /// * `index` - The index of the scene to retrieve
    ///
    /// # Returns
    /// * `Result<Scene, String>` - The requested scene
    pub fn get_scene_at(index: i32) -> Result<Scene, String> {
        let class = Self::get_class()
            .ok_or("Class 'UnityEngine.SceneManagement.SceneManager' not found")?;

        let method = class
            .method("GetSceneAt")
            .ok_or("Method 'GetSceneAt' not found")?;

        unsafe { method.call::<Scene>(&[&index as *const i32 as *mut c_void]) }
    }

    /// Gets a scene by name
    ///
    /// # Arguments
    /// * `name` - The name of the scene to find
    ///
    /// # Returns
    /// * `Result<Scene, String>` - The requested scene
    pub fn get_scene_by_name(name: &str) -> Result<Scene, String> {
        let class = Self::get_class()
            .ok_or("Class 'UnityEngine.SceneManagement.SceneManager' not found")?;

        let method = class
            .method("GetSceneByName")
            .ok_or("Method 'GetSceneByName' not found")?;

        let name_str = Il2cppString::new(name);
        if name_str.is_null() {
            return Err("Failed to create Il2cppString".to_string());
        }

        unsafe { method.call::<Scene>(&[name_str as *mut c_void]) }
    }

    /// Loads the scene at the specified index
    ///
    /// # Arguments
    /// * `index` - The build index of the scene to load
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn load_scene_at(index: i32) -> Result<(), String> {
        let class = Self::get_class()
            .ok_or("Class 'UnityEngine.SceneManagement.SceneManager' not found")?;

        let method = class
            .method(("LoadScene", ["System.Int32"]))
            .ok_or("Method 'LoadScene(int)' not found")?;

        unsafe {
            method.call::<()>(&[&index as *const i32 as *mut c_void])?;
        }
        Ok(())
    }

    /// Loads the scene by name
    ///
    /// # Arguments
    /// * `name` - The name or path of the scene to load
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn load_scene_name(name: &str) -> Result<(), String> {
        let class = Self::get_class()
            .ok_or("Class 'UnityEngine.SceneManagement.SceneManager' not found")?;

        let method = class
            .method(("LoadScene", ["System.String"]))
            .ok_or("Method 'LoadScene(string)' not found")?;

        let name_str = Il2cppString::new(name);
        if name_str.is_null() {
            return Err("Failed to create Il2cppString".to_string());
        }

        unsafe {
            method.call::<()>(&[name_str as *mut c_void])?;
        }
        Ok(())
    }
}
