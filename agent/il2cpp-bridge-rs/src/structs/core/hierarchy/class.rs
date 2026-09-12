//! Class metadata wrapper and lookup helpers.

use crate::api::{self, cache};
use crate::structs::collections::Il2cppArray;
use crate::structs::core::{Field, Method, Property};
use std::ffi::c_void;
use std::sync::Arc;

use super::object::Object;

/// Hydrated IL2CPP class metadata.
///
/// A `Class` is the main entry point for metadata-driven workflows after
/// fetching an [`crate::structs::Assembly`] from the cache. From here you can
/// locate methods, fields, properties, create objects, or search for live
/// instances in the scene.
#[derive(Debug, Clone)]
pub struct Class {
    /// Pointer to the internal IL2CPP class structure
    pub address: *mut c_void,
    /// Pointer to the image defining this class
    pub image: *mut c_void,
    /// Metadata token for this class
    pub token: u32,
    /// Name of the class
    pub name: String,
    /// Name of the parent class (if any)
    pub parent: Option<String>,
    /// Namespace of the class
    pub namespace: String,
    /// Whether the class is an enum
    pub is_enum: bool,
    /// Whether the class is generic
    pub is_generic: bool,
    /// Whether the class is an inflated generic type
    pub is_inflated: bool,
    /// Whether the class is an interface
    pub is_interface: bool,
    /// Whether the class is abstract
    pub is_abstract: bool,
    /// Whether the class is blittable
    pub is_blittable: bool,
    /// Whether the class is a value type
    pub is_valuetype: bool,
    /// Flags associated with the class
    pub flags: i32,
    /// Rank of the class (if array)
    pub rank: i32,
    /// Size of an instance of this class
    pub instance_size: i32,
    /// Size of array elements (if this is an array class)
    pub array_element_size: i32,
    /// Number of fields in this class
    pub num_fields_count: usize,
    /// Enum base type pointer (if this is an enum)
    pub enum_basetype: *mut c_void,
    /// Pointer to static field data
    pub static_field_data: *mut c_void,
    /// Name of the assembly defining this class
    pub assembly_name: String,
    /// Assembly that this class belongs to
    pub assembly: Option<std::sync::Arc<crate::structs::Assembly>>,
    /// List of fields in this class
    pub fields: Vec<Field>,

    /// List of methods in this class
    pub methods: Vec<Method>,
    /// List of properties in this class
    pub properties: Vec<Property>,
    /// List of interfaces implemented by this class
    pub interfaces: Vec<*mut c_void>,
    /// List of nested types within this class
    pub nested_types: Vec<*mut c_void>,
    /// Element class (if this is an array class)
    pub element_class: *mut c_void, // For arrays
    /// Declaring type (if this is a nested class)
    pub declaring_type: *mut c_void, // For nested classes
    /// Pointer to the Type object representing this class
    pub ty: *mut c_void,
    /// Pointer to the System.Type object
    pub object: *mut c_void,
}

unsafe impl Send for Class {}
unsafe impl Sync for Class {}

/// Selector trait used by [`Class::method`] and [`crate::structs::Object::method`].
pub trait MethodSelector {
    /// Returns true if the method matches the selector criteria
    fn matches(&self, method: &Method) -> bool;
}

/// Selects a method by name.
impl MethodSelector for &str {
    fn matches(&self, method: &Method) -> bool {
        method.name == *self
    }
}

/// Selects a method by name and parameter type names.
impl MethodSelector for (&str, &[&str]) {
    fn matches(&self, method: &Method) -> bool {
        if method.name != self.0 {
            return false;
        }

        if method.args.len() != self.1.len() {
            return false;
        }

        for (i, param_name) in self.1.iter().enumerate() {
            let arg_type = &method.args[i].type_info;
            if !arg_type.matches_name(*param_name) {
                return false;
            }
        }

        true
    }
}

/// Selects a method by name and parameter type names using a fixed-size array.
impl<const N: usize> MethodSelector for (&str, [&str; N]) {
    fn matches(&self, method: &Method) -> bool {
        (self.0, self.1.as_slice()).matches(method)
    }
}

/// Selects a method by name and parameter count.
impl MethodSelector for (&str, usize) {
    fn matches(&self, method: &Method) -> bool {
        method.name == self.0 && method.args.len() == self.1
    }
}

impl std::fmt::Display for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.dump_string())
    }
}

impl Class {
    /// Generates a string representation of the class, including fields and methods
    ///
    /// # Returns
    /// * `String` - The formatted string dump of the class
    pub fn dump_string(&self) -> String {
        let mut s = String::new();

        // Header comment: // Namespace: <ns> — <dll>
        let ns_part = if !self.namespace.is_empty() {
            self.namespace.clone()
        } else {
            String::new()
        };

        let dll_part = if !self.assembly_name.is_empty() {
            if self.assembly_name.ends_with(".dll") {
                self.assembly_name.clone()
            } else {
                format!("{}.dll", self.assembly_name)
            }
        } else {
            String::new()
        };

        match (!ns_part.is_empty(), !dll_part.is_empty()) {
            (true, true) => s.push_str(&format!(
                "// Namespace: {} \u{2014} {}\n",
                ns_part, dll_part
            )),
            (true, false) => s.push_str(&format!("// Namespace: {}\n", ns_part)),
            (false, true) => s.push_str(&format!("// Image: {}\n", dll_part)),
            (false, false) => {}
        }

        let abstract_kw = if self.is_abstract && !self.is_interface {
            "abstract "
        } else {
            ""
        };

        let type_kw = if self.is_enum {
            "enum"
        } else if self.is_interface {
            "interface"
        } else if self.is_valuetype {
            "struct"
        } else {
            "class"
        };

        // TypeDefIndex: lower 24 bits of the metadata token
        let typedef_index = self.token & 0x00FF_FFFF;

        let inheritance = if let Some(parent) = &self.parent {
            format!(" : {}", parent)
        } else {
            String::new()
        };

        if self.is_enum {
            let underlying_type = self.enum_underlying_type();
            s.push_str(&format!(
                "public {} {} : {} // TypeDefIndex: {}\n{{\n",
                type_kw, self.name, underlying_type, typedef_index
            ));

            let mut has_variants = false;
            for field in self
                .fields
                .iter()
                .filter(|field| field.is_static && field.name != "value__")
            {
                has_variants = true;
                if let Some(value) = Self::enum_value_string(field, underlying_type) {
                    s.push_str(&format!("    {} = {},\n", field.name, value));
                } else {
                    s.push_str(&format!("    {},\n", field.name));
                }
            }

            if !has_variants {
                s.push_str("    // Empty enum\n");
            }

            s.push_str("}\n");
            return s;
        }

        s.push_str(&format!(
            "public {}{} {}{} // TypeDefIndex: {}\n{{\n",
            abstract_kw, type_kw, self.name, inheritance, typedef_index
        ));

        // Fields section
        if !self.fields.is_empty() {
            s.push_str("    // Fields\n");
            for field in &self.fields {
                s.push_str(&format!("    {}\n", field));
            }
        }

        // Properties section
        if !self.properties.is_empty() {
            if !self.fields.is_empty() {
                s.push('\n');
            }
            s.push_str("    // Properties\n");
            for prop in &self.properties {
                s.push_str(&format!("    {}\n", prop));
            }
        }

        // Split methods into constructors vs regular methods
        let constructors: Vec<_> = self
            .methods
            .iter()
            .filter(|m| m.name == ".ctor" || m.name == ".cctor")
            .collect();

        let regular_methods: Vec<_> = self
            .methods
            .iter()
            .filter(|m| m.name != ".ctor" && m.name != ".cctor")
            .collect();

        // Constructors section
        if !constructors.is_empty() {
            if !self.fields.is_empty() || !self.properties.is_empty() {
                s.push('\n');
            }
            s.push_str("    // Constructors\n");
            for method in &constructors {
                for line in method.to_string().lines() {
                    s.push_str(&format!("    {}\n", line));
                }
                s.push('\n');
            }
        }

        // Methods section
        if !regular_methods.is_empty() {
            if constructors.is_empty() && (!self.fields.is_empty() || !self.properties.is_empty()) {
                s.push('\n');
            }
            s.push_str("    // Methods\n");
            for method in &regular_methods {
                for line in method.to_string().lines() {
                    s.push_str(&format!("    {}\n", line));
                }
                s.push('\n');
            }
        }

        s.push_str("}\n");
        s
    }

    /// Creates a managed instance using `System.Activator.CreateInstance`.
    ///
    /// Use this when you want constructor semantics rather than raw allocation.
    pub fn create_instance(&self) -> Result<Object, String> {
        unsafe {
            if self.ty.is_null() {
                return Err(format!("Could not get Type for class '{}'", self.name));
            }

            let corlib = cache::mscorlib();
            let activator_class = corlib
                .class("System.Activator")
                .ok_or_else(|| "Could not find System.Activator class".to_string())?;

            let method = activator_class
                .method(("CreateInstance", ["System.Type"]))
                .ok_or_else(|| "Could not find CreateInstance(Type) method".to_string())?;

            if self.object.is_null() {
                return Err("Could not get Type object".to_string());
            }

            match method.call::<Object>(&[self.object]) {
                Ok(result_handle) => {
                    if result_handle.ptr.is_null() {
                        return Err("CreateInstance returned null".to_string());
                    }
                    Ok(result_handle)
                }
                Err(e) => Err(format!("CreateInstance invocation failed: {}", e)),
            }
        }
    }

    /// Allocates a new object of this class via `il2cpp_object_new`.
    ///
    /// This is the low-level allocation path and does not imply constructor
    /// execution.
    pub fn new_object(&self) -> Result<Object, String> {
        unsafe {
            let obj = Object::from_ptr(api::object_new(self.address));
            if obj.ptr.is_null() {
                return Err("Could not create object".to_string());
            }
            Ok(obj)
        }
    }

    /// Creates a `ScriptableObject` instance of this class.
    pub fn create_scriptable_instance(&self) -> Result<Object, String> {
        unsafe {
            if self.object.is_null() {
                return Err(format!(
                    "Could not get Type object for class '{}'",
                    self.name
                ));
            }

            let core_module = cache::coremodule();
            let so_class = core_module
                .class("UnityEngine.ScriptableObject")
                .ok_or("Class 'UnityEngine.ScriptableObject' not found")?;

            let method = so_class
                .method(("CreateInstance", ["System.Type"]))
                .ok_or("Method 'CreateInstance' not found in ScriptableObject")?;

            let result = method.call::<Object>(&[self.object])?;

            if result.ptr.is_null() {
                return Err("ScriptableObject.CreateInstance returned null".to_string());
            }

            Ok(result)
        }
    }

    /// Finds live objects of this type in the scene.
    ///
    /// This wraps `UnityEngine.Object.FindObjectsOfType` and returns bound
    /// [`Object`] wrappers for each match.
    pub fn find_objects_of_type(&self, include_inactive: bool) -> Vec<Object> {
        if self.object.is_null() {
            return Vec::new();
        }

        let type_object = self.object;

        let unity_engine_core = cache::coremodule();
        let unity_object = match unity_engine_core.class("UnityEngine.Object") {
            Some(c) => c,
            None => return Vec::new(),
        };

        let method = unity_object
            .method(("FindObjectsOfType", ["System.Type", "System.Boolean"]))
            .or_else(|| unity_object.method(("FindObjectsOfType", ["System.Type"])));

        match method {
            Some(method) => unsafe {
                let result = if method.args.len() == 2 {
                    let params = [type_object, &include_inactive as *const bool as *mut c_void];
                    method.call::<*mut Il2cppArray<Object>>(&params)
                } else {
                    let params = [type_object];
                    method.call::<*mut Il2cppArray<Object>>(&params)
                };

                match result {
                    Ok(array) => {
                        if array.is_null() {
                            return Vec::new();
                        }
                        (*array).to_vector()
                    }
                    Err(_) => Vec::new(),
                }
            },
            None => Vec::new(),
        }
    }

    /// Finds a method in this class or its parent chain.
    ///
    /// Selectors may match by name, by name plus parameter type names, or by
    /// name plus parameter count.
    pub fn method<S: MethodSelector>(&self, selector: S) -> Option<Method> {
        if let Some(method) = self.methods.iter().find(|m| selector.matches(m)).cloned() {
            return Some(method);
        }

        if self.parent.is_some() {
            return self
                .get_parent_class()
                .and_then(|parent| parent.method(selector));
        }

        None
    }

    /// Finds a field in this class or its parent chain.
    pub fn field(&self, name: &str) -> Option<Field> {
        if let Some(field) = self.fields.iter().find(|f| f.name == name).cloned() {
            return Some(field);
        }

        if self.parent.is_some() {
            return self
                .get_parent_class()
                .and_then(|parent| parent.field(name));
        }

        None
    }

    /// Finds a property in this class or its parent chain.
    pub fn property(&self, name: &str) -> Option<Property> {
        if let Some(prop) = self.properties.iter().find(|p| p.name == name).cloned() {
            return Some(prop);
        }

        if self.parent.is_some() {
            return self
                .get_parent_class()
                .and_then(|parent| parent.property(name));
        }

        None
    }

    /// Retrieves the parent class by hydrating it from the parent pointer
    fn get_parent_class(&self) -> Option<Class> {
        unsafe {
            let parent_ptr = crate::api::class_get_parent(self.address);
            if parent_ptr.is_null() {
                return None;
            }
            cache::class_from_ptr(parent_ptr)
        }
    }

    /// Initializes the class if it hasn't been initialized yet
    ///
    /// Calls `il2cpp_runtime_class_init`.
    pub fn init(&self) {
        unsafe { api::runtime_class_init(self.address) }
    }

    /// Checks if the class has references
    ///
    /// # Returns
    /// * `bool` - True if class has references
    pub fn has_references(&self) -> bool {
        unsafe { api::class_has_references(self.address) }
    }

    /// Checks if this class is assignable from another class
    ///
    /// # Arguments
    /// * `other` - The other class to check against
    ///
    /// # Returns
    /// * `bool` - True if assignable
    pub fn is_assignable_from(&self, other: &Class) -> bool {
        unsafe { api::class_is_assignable_from(self.address, other.address) }
    }

    /// Checks if this class is a subclass of another class
    ///
    /// # Arguments
    /// * `other` - The prospective parent class
    /// * `check_interfaces` - Whether to check interfaces as well
    ///
    /// # Returns
    /// * `bool` - True if it is a subclass
    pub fn is_subclass_of(&self, other: &Class, check_interfaces: bool) -> bool {
        unsafe { api::class_is_subclass_of(self.address, other.address, check_interfaces) }
    }

    /// Gets the size of the value type
    ///
    /// # Returns
    /// * `i32` - The size of the value type in bytes
    pub fn get_value_size(&self) -> i32 {
        unsafe { api::class_value_size(self.address, std::ptr::null_mut()) }
    }

    /// Checks if this class has the specified parent class (wrapper for il2cpp_class_has_parent)
    ///
    /// # Arguments
    /// * `parent` - The parent class to check for
    ///
    /// # Returns
    /// * `bool` - True if parent exists in the inheritance hierarchy
    pub fn has_parent(&self, parent: &Class) -> bool {
        unsafe { api::class_has_parent(self.address, parent.address) }
    }

    /// Checks if this class has the specified attribute (wrapper for il2cpp_class_has_attribute)
    ///
    /// # Arguments
    /// * `attr_class` - The attribute class to check for
    ///
    /// # Returns
    /// * `bool` - True if the class has the attribute
    pub fn has_attribute(&self, attr_class: &Class) -> bool {
        unsafe { api::class_has_attribute(self.address, attr_class.address) }
    }

    /// Inflates a generic class with the specified type arguments to create a concrete generic type.
    ///
    /// # Arguments
    /// * `classes` - Slice of classes to use as type arguments
    ///
    /// # Returns
    /// * `Result<Arc<Class>, String>` - The inflated concrete class
    pub fn inflate(&self, classes: &[&Class]) -> Result<Arc<Class>, String> {
        unsafe {
            if !self.is_generic {
                return Err(format!(
                    "Class '{}' is not a generic type definition and cannot be inflated",
                    self.name
                ));
            }

            if self.object.is_null() {
                return Err(format!(
                    "Could not get Type object for class '{}'",
                    self.name
                ));
            }

            if classes.is_empty() {
                return Err("No type arguments provided".to_string());
            }

            let mut type_args = Vec::with_capacity(classes.len());
            for (i, cls) in classes.iter().enumerate() {
                if cls.object.is_null() {
                    return Err(format!(
                        "Class '{}' (arg {}) has no Type object",
                        cls.name, i
                    ));
                }
                type_args.push(cls.object);
            }

            let corlib = cache::mscorlib();
            let type_class = corlib
                .class("System.Type")
                .ok_or_else(|| "Could not find System.Type class".to_string())?;

            let type_array = Il2cppArray::<*mut c_void>::new(&type_class, type_args.len());
            if type_array.is_null() {
                return Err("Could not create Type[] array".to_string());
            }

            let array_ref = &mut *type_array;
            for (i, &type_arg) in type_args.iter().enumerate() {
                array_ref.set(i, type_arg);
            }

            let type_obj = Object::from_ptr(self.object);

            let make_generic_type_method = type_obj
                .method(("MakeGenericType", ["System.Type[]"]))
                .ok_or_else(|| "Could not find MakeGenericType(Type[]) method".to_string())?;

            let inflated_type_obj =
                make_generic_type_method.call::<*mut c_void>(&[type_array as *mut c_void])?;

            if inflated_type_obj.is_null() {
                return Err("MakeGenericType returned null".to_string());
            }

            let inflated_class_ptr = api::class_from_system_type(inflated_type_obj);
            if inflated_class_ptr.is_null() {
                return Err("Could not get Il2CppClass from inflated Type".to_string());
            }

            api::runtime_class_init(inflated_class_ptr);

            cache::class_from_ptr(inflated_class_ptr)
                .map(Arc::new)
                .ok_or_else(|| {
                    "Could not convert inflated class pointer to Class struct".to_string()
                })
        }
    }

    /// Gets the underlying type of an enum
    fn enum_underlying_type(&self) -> &'static str {
        let type_name = self
            .fields
            .iter()
            .find(|field| field.name == "value__")
            .map(|field| field.type_info.name.as_str())
            .unwrap_or("System.Int32");

        match type_name {
            "System.Byte" | "Byte" | "byte" => "byte",
            "System.SByte" | "SByte" | "sbyte" => "sbyte",
            "System.Int16" | "Int16" | "short" => "short",
            "System.UInt16" | "UInt16" | "ushort" => "ushort",
            "System.Int32" | "Int32" | "int" => "int",
            "System.UInt32" | "UInt32" | "uint" => "uint",
            "System.Int64" | "Int64" | "long" => "long",
            "System.UInt64" | "UInt64" | "ulong" => "ulong",
            _ => "int",
        }
    }

    /// Gets the value of an enum field as a string
    fn enum_value_string(field: &Field, underlying_type: &str) -> Option<String> {
        unsafe {
            match underlying_type {
                "byte" => field.get_value::<u8>().ok().map(|value| value.to_string()),
                "sbyte" => field.get_value::<i8>().ok().map(|value| value.to_string()),
                "short" => field.get_value::<i16>().ok().map(|value| value.to_string()),
                "ushort" => field.get_value::<u16>().ok().map(|value| value.to_string()),
                "int" => field.get_value::<i32>().ok().map(|value| value.to_string()),
                "uint" => field.get_value::<u32>().ok().map(|value| value.to_string()),
                "long" => field.get_value::<i64>().ok().map(|value| value.to_string()),
                "ulong" => field.get_value::<u64>().ok().map(|value| value.to_string()),
                _ => None,
            }
        }
    }
}
