use crate::structs::collections::Il2cppString;
use crate::structs::components::rendering::shader::Shader;
use crate::structs::core::Object;
use crate::structs::math::Color;
use std::ffi::c_void;
use std::ops::Deref;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Material {
    pub object: Object,
}

impl Material {
    /// Creates a Material from a raw pointer
    ///
    /// # Arguments
    /// * `ptr` - The raw pointer to the material
    ///
    /// # Returns
    /// * `Self` - The created Material wrapper
    pub fn from_ptr(ptr: *mut c_void) -> Self {
        Self {
            object: unsafe { Object::from_ptr(ptr) },
        }
    }
}

impl Deref for Material {
    type Target = Object;
    fn deref(&self) -> &Self::Target {
        &self.object
    }
}

impl Material {
    /// Gets the main color of the material.
    ///
    /// # Returns
    /// * `Color` - The main color (returns Black if retrieval fails)
    pub fn get_color(&self) -> Color {
        unsafe {
            self.method(("get_color", 0))
                .and_then(|method| method.call::<Color>(&[]).ok())
                .unwrap_or(Color::BLACK) // if the method fails, return black
        }
    }

    /// Sets the main color of the material.
    ///
    /// # Arguments
    /// * `color` - The new color to set
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_color(&self, color: Color) -> Result<(), String> {
        unsafe {
            self.method(("set_color", 1))
                .ok_or("Method 'set_color' not found")?
                .call::<()>(&[&mut (color.clone()) as *mut Color as *mut c_void])?;
        }
        Ok(())
    }

    /// Sets a named color value.
    ///
    /// # Arguments
    /// * `name` - The property name of the color
    /// * `color` - The color value to set
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_color_named(&self, name: &str, color: Color) -> Result<(), String> {
        unsafe {
            let string_new_ptr = Il2cppString::new(name);
            self.method(("SetColor", 2))
                .ok_or("Method 'SetColor' not found")?
                .call::<()>(&[
                    string_new_ptr as *mut c_void,
                    &mut (color.clone()) as *mut Color as *mut c_void,
                ])?;
        }
        Ok(())
    }

    /// Sets a float value for a shader property.
    ///
    /// # Arguments
    /// * `name` - The property name
    /// * `value` - The float value
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_float(&self, name: &str, value: f32) -> Result<(), String> {
        unsafe {
            let string_new_ptr = Il2cppString::new(name);
            self.method(("SetFloat", 2))
                .ok_or("Method 'SetFloat' not found")?
                .call::<()>(&[string_new_ptr as *mut c_void, &mut { value } as *mut f32
                    as *mut c_void])?;
        }
        Ok(())
    }

    /// Sets an integer value for a shader property.
    ///
    /// # Arguments
    /// * `name` - The property name
    /// * `value` - The integer value
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_int(&self, name: &str, value: i32) -> Result<(), String> {
        unsafe {
            let string_new_ptr = Il2cppString::new(name);
            self.method(("SetInt", 2))
                .ok_or("Method 'SetInt' not found")?
                .call::<()>(&[string_new_ptr as *mut c_void, &mut { value } as *mut i32
                    as *mut c_void])?;
        }
        Ok(())
    }

    /// Gets the shader used by the material.
    ///
    /// # Returns
    /// * `Result<Shader, String>` - The shader assigned to this material
    pub fn get_shader(&self) -> Result<Shader, String> {
        unsafe {
            let ptr = self
                .method(("get_shader", 0))
                .ok_or("Method 'get_shader' not found")?
                .call::<*mut c_void>(&[])?;

            if ptr.is_null() {
                return Err("Shader is null".to_string());
            }
            Ok(Shader::from_ptr(ptr))
        }
    }

    /// Sets the shader used by the material.
    ///
    /// # Arguments
    /// * `shader` - The new shader to assign
    ///
    /// # Returns
    /// * `Result<(), String>` - Ok if success
    pub fn set_shader(&self, shader: Shader) -> Result<(), String> {
        unsafe {
            self.method(("set_shader", 1))
                .ok_or("Method 'set_shader' not found")?
                .call::<()>(&[shader.object.ptr as *mut c_void])?;
        }
        Ok(())
    }
}
