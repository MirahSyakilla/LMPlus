//! Unity LayerMask wrapper
use crate::api::cache;
use std::ffi::c_void;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerMask {
    pub value: i32,
}

impl From<i32> for LayerMask {
    fn from(value: i32) -> Self {
        Self { value }
    }
}

impl From<LayerMask> for i32 {
    fn from(mask: LayerMask) -> Self {
        mask.value
    }
}

impl LayerMask {
    /// Gets the LayerMask class definition
    ///
    /// # Returns
    /// * `Option<Class>` - The UnityEngine.LayerMask class
    pub fn get_class() -> Option<crate::structs::core::Class> {
        cache::coremodule().class("LayerMask")
    }

    /// Converts a layer name to a layer index
    ///
    /// # Arguments
    /// * `name` - The name of the layer
    ///
    /// # Returns
    /// * `i32` - The layer index, or -1 if not found
    pub fn name_to_layer(name: &str) -> i32 {
        if let Some(class) = Self::get_class() {
            if let Some(method) = class.method("NameToLayer") {
                let name_str = crate::structs::Il2cppString::new(name);
                let res = unsafe { method.call::<i32>(&[name_str as *mut c_void]) };
                return res.unwrap_or(-1);
            }
        }
        -1
    }

    /// Converts a layer index to a layer name
    ///
    /// # Arguments
    /// * `layer` - The layer index
    ///
    /// # Returns
    /// * `String` - The name of the layer
    pub fn layer_to_name(layer: i32) -> String {
        if let Some(class) = Self::get_class() {
            if let Some(method) = class.method("LayerToName") {
                let mut layer_val = layer;
                unsafe {
                    let ptr = method.call::<*mut crate::structs::Il2cppString>(&[&mut layer_val
                        as *mut i32
                        as *mut c_void]);
                    if let Ok(ptr) = ptr {
                        if !ptr.is_null() {
                            return (*ptr).to_string().unwrap_or_default();
                        }
                    }
                }
            }
        }
        String::new()
    }

    /// Gets a mask for the specified layer names
    ///
    /// # Arguments
    /// * `layer_names` - A list of layer names to include in the mask
    ///
    /// # Returns
    /// * `LayerMask` - The resulting bitmask
    pub fn get_mask(layer_names: &[&str]) -> LayerMask {
        if let Some(class) = Self::get_class() {
            if let Some(method) = class.method("GetMask") {
                let array_len = layer_names.len() as u32;
                let string_class = crate::api::cache::mscorlib()
                    .class("System.String")
                    .unwrap();
                let array = crate::structs::collections::Il2cppArray::<
                    *mut crate::structs::Il2cppString,
                >::new(&string_class, array_len as usize);

                if !array.is_null() {
                    let array_obj = unsafe { &mut *array };
                    for (i, name) in layer_names.iter().enumerate() {
                        let il2cpp_string = crate::structs::Il2cppString::new(name);
                        array_obj.set(i, il2cpp_string);
                    }

                    let res = unsafe { method.call::<i32>(&[array as *mut c_void]) };
                    if let Ok(val) = res {
                        return LayerMask { value: val };
                    }
                }
            }
        }
        LayerMask { value: 0 }
    }
}
