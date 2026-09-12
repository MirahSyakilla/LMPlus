//! Type metadata wrapper.
use std::ffi::c_void;

/// Represents a hydrated IL2CPP type.
///
/// `Type` values usually come from fields, properties, method arguments, and
/// return metadata. They are useful for comparison, reflection-style queries,
/// and producing readable dump output.
#[derive(Debug, Clone)]
pub struct Type {
    /// Pointer to the internal IL2CPP type structure
    pub address: *mut c_void,
    /// Name of the type
    pub name: String,
    /// Size of the type in bytes (if applicable)
    pub size: i32,
}

impl Default for Type {
    fn default() -> Self {
        Self {
            address: std::ptr::null_mut(),
            name: String::new(),
            size: 0,
        }
    }
}

unsafe impl Send for Type {}
unsafe impl Sync for Type {}

impl Type {
    /// Returns the managed `System.Type` object for this type.
    pub fn get_object(&self) -> *mut c_void {
        unsafe { crate::api::type_get_object(self.address) }
    }

    /// Returns the raw `Il2CppClass*` for this type.
    pub fn get_class(&self) -> *mut c_void {
        unsafe { crate::api::class_from_type(self.address) }
    }

    /// Returns `true` when the two IL2CPP types are equal.
    pub fn equals(&self, other: &Type) -> bool {
        unsafe { crate::api::type_equals(self.address, other.address) }
    }

    /// Returns `true` if this type is passed by reference.
    pub fn is_byref(&self) -> bool {
        unsafe { crate::api::type_is_byref(self.address) }
    }

    /// Returns the underlying IL2CPP type enum value.
    pub fn get_type_enum(&self) -> i32 {
        unsafe { crate::api::type_get_type(self.address) }
    }

    /// Returns true when `expected` matches the raw IL2CPP name, C# alias,
    /// simplified class name, or the fully formatted display name.
    pub fn matches_name(&self, expected: &str) -> bool {
        if super::type_fmt::type_name_matches(&self.name, expected) {
            return true;
        }

        if !self.name.is_empty() {
            let formatted = super::type_fmt::format_type_name_str(&self.name);
            if formatted == expected {
                return true;
            }
        }

        !self.address.is_null() && self.cpp_name() == expected
    }

    /// Returns a readable C#-style display name for this type.
    pub fn cpp_name(&self) -> String {
        // For generics and other compound types, prefer the runtime-qualified
        // name when available because `type_get_name` can collapse inflated
        // collection types back to their generic definition.
        if !self.address.is_null()
            && (self.name.contains('`') || self.name.contains('<') || self.name.contains('['))
        {
            if let Some(name) = self.try_resolve_inflated() {
                return name;
            }
        }

        // Try formatting from stored type name
        if !self.name.is_empty() {
            let formatted = super::type_fmt::format_type_name_str(&self.name);
            if formatted.contains('<') {
                return formatted;
            }
        }

        // Fall back to class name from type pointer
        if !self.address.is_null() {
            unsafe {
                let class_ptr = crate::api::class_from_type(self.address);
                if !class_ptr.is_null() {
                    let name_ptr = crate::api::class_get_name(class_ptr);
                    if !name_ptr.is_null() {
                        let raw = std::ffi::CStr::from_ptr(name_ptr).to_string_lossy();
                        return super::type_fmt::format_class_name(&raw);
                    }
                }
            }
        }

        super::type_fmt::format_type_name_str(&self.name)
    }

    /// Resolves a more specific runtime name using
    /// `type_get_assembly_qualified_name`.
    fn try_resolve_inflated(&self) -> Option<String> {
        unsafe {
            let aqn_ptr = crate::api::type_get_assembly_qualified_name(self.address);
            if aqn_ptr.is_null() {
                return None;
            }
            let aqn = std::ffi::CStr::from_ptr(aqn_ptr)
                .to_string_lossy()
                .into_owned();
            crate::api::free(aqn_ptr as *mut c_void);

            let cleaned = super::type_fmt::strip_assembly_qualifiers(&aqn);
            let formatted = super::type_fmt::format_type_name_str(&cleaned);
            (!formatted.is_empty()).then_some(formatted)
        }
    }
}
