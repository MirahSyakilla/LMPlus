//! IL2CPP Property definition
use super::method::Method;
use crate::structs::core::Type;
use std::ffi::c_void;

/// Property metadata synthesized from getter and setter methods.
#[derive(Debug, Clone)]
pub struct Property {
    /// Name of the property (without get_/set_ prefix)
    pub name: String,
    /// Type of the property
    pub type_info: Type,
    /// Getter method (if exists)
    pub getter: Option<Method>,
    /// Setter method (if exists)
    pub setter: Option<Method>,
    /// Whether the property is static
    pub is_static: bool,
    /// Access modifier
    pub access: &'static str,
    /// Instance pointer for instance properties
    pub instance: Option<*mut c_void>,
}

unsafe impl Send for Property {}
unsafe impl Sync for Property {}

impl std::fmt::Display for Property {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fmt_property())
    }
}

impl Property {
    /// Creates a new Property from getter and/or setter methods
    pub fn from_methods(getter: Option<Method>, setter: Option<Method>) -> Option<Self> {
        if getter.is_none() && setter.is_none() {
            return None;
        }

        let name = if let Some(ref g) = getter {
            g.name[4..].to_string() // Strip "get_"
        } else if let Some(ref s) = setter {
            s.name[4..].to_string() // Strip "set_"
        } else {
            return None;
        };

        let type_info = if let Some(ref g) = getter {
            g.return_type.clone()
        } else if let Some(ref s) = setter {
            s.args
                .first()
                .map(|a| a.type_info.clone())
                .unwrap_or_default()
        } else {
            Type::default()
        };

        let is_static = getter
            .as_ref()
            .map(|g| g.is_static)
            .unwrap_or_else(|| setter.as_ref().map(|s| s.is_static).unwrap_or(false));

        let access = getter
            .as_ref()
            .map(|g| g.get_attribute())
            .unwrap_or_else(|| {
                setter
                    .as_ref()
                    .map(|s| s.get_attribute())
                    .unwrap_or("public")
            });

        Some(Property {
            name,
            type_info,
            getter,
            setter,
            is_static,
            access,
            instance: None,
        })
    }

    /// Returns a copy of this property with the instance pointer set
    pub fn with_instance(&self, instance: *mut c_void) -> Self {
        let mut prop = self.clone();
        prop.instance = Some(instance);
        prop
    }

    /// Returns true if this property has a getter
    pub fn has_getter(&self) -> bool {
        self.getter.is_some()
    }

    /// Returns true if this property has a setter
    pub fn has_setter(&self) -> bool {
        self.setter.is_some()
    }

    /// Reads the property value through its getter.
    ///
    /// For instance properties, the property must have a bound instance
    /// pointer. Use [`crate::structs::Object::property`] to obtain one.
    pub unsafe fn get_value<T: Copy>(&self) -> Result<T, String> {
        let getter = self
            .getter
            .as_ref()
            .ok_or_else(|| format!("Property '{}' does not have a getter", self.name))?;

        let mut method = getter.clone();

        if !self.is_static {
            let inst = self.instance.ok_or_else(|| {
                format!(
                    "Property '{}' is not static but no instance was provided. Use Object::property or set the instance manually.",
                    self.name
                )
            })?;
            method.instance = Some(inst);
        }

        method.call::<T>(&[])
    }

    /// Writes a property value through its setter.
    ///
    /// For instance properties, the property must have a bound instance
    /// pointer. Use [`crate::structs::Object::property`] to obtain one.
    pub unsafe fn set_value<T>(&self, value: T) -> Result<(), String> {
        let setter = self
            .setter
            .as_ref()
            .ok_or_else(|| format!("Property '{}' does not have a setter", self.name))?;

        let mut method = setter.clone();

        if !self.is_static {
            let inst = self.instance.ok_or_else(|| {
                format!(
                    "Property '{}' is not static but no instance was provided. Use Object::property or set the instance manually.",
                    self.name
                )
            })?;
            method.instance = Some(inst);
        }

        let value_ptr = &value as *const T as *mut c_void;
        method.call::<()>(&[value_ptr])?;
        Ok(())
    }

    /// Generates a string representation of the property
    fn fmt_property(&self) -> String {
        let static_prefix = if self.is_static { "static " } else { "" };

        let accessors = match (self.has_getter(), self.has_setter()) {
            (true, true) => "get; set;",
            (true, false) => "get;",
            (false, true) => "set;",
            _ => "",
        };

        format!(
            "{} {}{} {} {{ {} }}",
            self.access,
            static_prefix,
            self.type_info.cpp_name(),
            self.name,
            accessors,
        )
    }
}
