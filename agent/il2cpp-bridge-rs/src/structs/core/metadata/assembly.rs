//! Assembly metadata wrapper.

use crate::api::{self, cache, dump_assembly};
use crate::logger;
use crate::structs::core::Class;
use std::ffi::c_void;

use super::image::Image;

/// Represents a hydrated IL2CPP assembly.
///
/// In normal usage, values of this type come from [`crate::api::cache`] helper
/// functions such as [`crate::api::cache::csharp`].
#[derive(Debug, Clone)]
pub struct Assembly {
    /// Wrapper for the image associated with this assembly
    pub image: Image,
    /// Pointer to the internal IL2CPP assembly structure
    pub address: *mut c_void,
    /// Filename of the assembly (e.g. "Assembly-CSharp.dll")
    pub file: String,
    /// Name of the assembly (e.g. "Assembly-CSharp")
    pub name: String,
    /// List of classes defined in this assembly
    pub classes: Vec<Class>,
}

unsafe impl Send for Assembly {}
unsafe impl Sync for Assembly {}

impl std::fmt::Display for Assembly {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_assembly())
    }
}

impl Assembly {
    /// Generates a string representation of the assembly
    ///
    /// This includes the assembly name, file path, address, and a dump of all classes if any are loaded.
    ///
    /// # Returns
    /// * `String` - The formatted string representation
    fn fmt_assembly(&self) -> String {
        let mut s = format!(
            "// Assembly: {} ({}) @ {:?}\n",
            self.name, self.file, self.address
        );

        if !self.classes.is_empty() {
            for class in &self.classes {
                s.push('\n');
                s.push_str(&class.to_string());
            }
        }
        s
    }

    /// Finds a class by name within the assembly.
    ///
    /// Accepts either a fully qualified type name such as
    /// `UnityEngine.GameObject` or an unqualified name for the global
    /// namespace.
    pub fn class(&self, name: &str) -> Option<Class> {
        let (namespace, class_name) = if let Some(last_dot) = name.rfind('.') {
            (&name[..last_dot], &name[last_dot + 1..])
        } else {
            ("", name)
        };

        let namespace_cstr = std::ffi::CString::new(namespace).ok()?;
        let name_cstr = std::ffi::CString::new(class_name).ok()?;

        unsafe {
            let class_ptr = api::class_from_name(
                self.image.address,
                namespace_cstr.as_ptr(),
                name_cstr.as_ptr(),
            );

            if !class_ptr.is_null() {
                return cache::class_from_ptr(class_ptr);
            }
        }

        if let Some(class) = cache::CACHE.classes.get(name) {
            return Some((**class).clone());
        }

        None
    }

    /// Dumps this assembly into a C#-like pseudo-code file and returns `self`.
    ///
    /// This is mainly intended for debugging and offline inspection.
    pub fn dump(&self) -> &Self {
        if dump_assembly(Some(&self.name)).is_none() {
            logger::error("Failed to dump assembly");
        }
        self
    }
}
