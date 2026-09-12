//! Unity Renderer component wrapper
use super::material::Material;
use crate::structs::collections::Il2cppArray;
use crate::structs::components::{Component, ComponentTrait};
use crate::structs::math::Bounds;
use std::ffi::c_void;
use std::ops::Deref;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Renderer {
    /// Base Component structure
    pub component: Component,
}

impl ComponentTrait for Renderer {
    fn from_ptr(ptr: *mut c_void) -> Self {
        Self {
            component: Component::from_ptr(ptr),
        }
    }
}

impl Deref for Renderer {
    type Target = Component;
    fn deref(&self) -> &Self::Target {
        &self.component
    }
}

impl Renderer {
    /// Gets the bounding box of the renderer
    ///
    /// # Returns
    /// * `Result<Bounds, String>` - The axis-aligned bounding box of the renderer
    pub fn get_bounds(&self) -> Result<Bounds, String> {
        let mut bounds = Bounds::default();
        unsafe {
            self.method("get_bounds_Injected")
                .ok_or("Method 'get_bounds_Injected' not found")?
                .call::<()>(&[&mut bounds as *mut Bounds as *mut c_void])?;
        }
        Ok(bounds)
    }

    /// Returns the first instantiated Material assigned to the renderer.
    ///
    /// # Returns
    /// * `Result<Material, String>` - The first material assigned to this renderer
    pub fn get_material(&self) -> Result<Material, String> {
        unsafe {
            let ptr = self
                .method(("get_material", 0))
                .ok_or("Method 'get_material' not found")?
                .call::<*mut c_void>(&[])?;
            if ptr.is_null() {
                return Err("Material is null".to_string());
            }
            Ok(Material::from_ptr(ptr))
        }
    }

    /// Assigns a Material to the renderer.
    ///
    /// # Arguments
    /// * `material` - The material to assign
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_material(&self, material: Material) -> Result<(), String> {
        unsafe {
            self.method(("set_material", 1))
                .ok_or("Method 'set_material' not found")?
                .call::<()>(&[material.object.ptr as *mut c_void])?;
        }
        Ok(())
    }

    /// Returns all the instantiated materials of this object.
    ///
    /// # Returns
    /// * `Result<Vec<Material>, String>` - A vector containing all materials
    pub fn get_materials(&self) -> Result<Vec<Material>, String> {
        unsafe {
            let ptr = self
                .method(("get_materials", 0))
                .ok_or("Method 'get_materials' not found")?
                .call::<*mut Il2cppArray<*mut c_void>>(&[])?;

            if ptr.is_null() {
                return Ok(Vec::new());
            }

            Ok((*ptr)
                .to_vector()
                .into_iter()
                .filter(|&p| !p.is_null())
                .map(Material::from_ptr)
                .collect())
        }
    }
}
