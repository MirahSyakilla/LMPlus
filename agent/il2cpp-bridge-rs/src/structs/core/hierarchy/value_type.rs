//! IL2CPP ValueType wrapper and operations
use super::class::MethodSelector;
use crate::api::cache;
use crate::structs::core::Method;
use std::ffi::c_void;

/// Represents an IL2CPP ValueType (struct) instance
///
/// ValueTypes are stack-allocated (or part of another object/array) and differ from `Object` which is heap-allocated.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ValueType {
    /// Pointer to the value type data
    pub ptr: *mut c_void,
    /// Pointer to the class definition
    pub class: *mut c_void,
}

impl ValueType {
    /// Creates a ValueType from a raw pointer
    ///
    /// # Arguments
    /// * `ptr` - Pointer to the raw value type data
    pub unsafe fn from_ptr(ptr: *mut c_void) -> Self {
        Self {
            ptr,
            class: std::ptr::null_mut(),
        }
    }

    /// Creates a ValueType from a raw pointer and class
    ///
    /// # Arguments
    /// * `ptr` - Pointer to the raw value type data
    /// * `class` - Pointer to the Class definition
    pub unsafe fn from_ptr_with_class(ptr: *mut c_void, class: *mut c_void) -> Self {
        Self { ptr, class }
    }

    /// Returns the raw pointer to the value type
    ///
    /// # Returns
    /// * `*mut c_void` - The raw pointer
    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr
    }

    /// Unboxes the value type into a native type
    ///
    /// # Type Parameters
    /// * `T` - The native type to unbox into (e.g., `i32`, `f32`)
    ///
    /// # Returns
    /// * `T` - The unboxed value
    pub unsafe fn unbox<T: Copy>(&self) -> T {
        if self.ptr.is_null() {
            return std::mem::zeroed();
        }
        std::ptr::read(self.ptr as *const T)
    }

    /// Gets a field specific to this value type instance
    ///
    /// # Arguments
    /// * `name` - The name of the field to retrieve
    ///
    /// # Returns
    /// * `Option<crate::structs::core::field::Field>` - The field if found, or None
    pub fn field(&self, name: &str) -> Option<crate::structs::core::members::field::Field> {
        let klass = self.class;

        if klass.is_null() {
            return None;
        }

        match cache::class_from_ptr(klass) {
            Some(class) => match class.field(name) {
                Some(mut field) => {
                    field.instance = Some(self.as_ptr());
                    Some(field)
                }
                None => None,
            },
            None => None,
        }
    }

    /// Gets a method specific to this value type instance
    ///
    /// # Type Parameters
    /// * `S` - A type that implements `MethodSelector`
    ///
    /// # Arguments
    /// * `selector` - The selector to use for finding the method
    ///
    /// # Returns
    /// * `Option<Method>` - The found method, or None
    pub fn method<S: MethodSelector>(&self, selector: S) -> Option<Method> {
        let klass = self.class;

        if klass.is_null() {
            return None;
        }

        cache::class_from_ptr(klass).and_then(|class| {
            class.method(selector).map(|mut method| {
                method.instance = Some(self.as_ptr());
                method
            })
        })
    }

    /// Gets a property specific to this value type instance
    ///
    /// # Arguments
    /// * `name` - The name of the property to retrieve
    ///
    /// # Returns
    /// * `Option<crate::structs::core::property::Property>` - The property if found, or None
    pub fn property(
        &self,
        name: &str,
    ) -> Option<crate::structs::core::members::property::Property> {
        let klass = self.class;

        if klass.is_null() {
            return None;
        }

        cache::class_from_ptr(klass).and_then(|class| {
            class
                .property(name)
                .map(|prop| prop.with_instance(self.as_ptr()))
        })
    }
}
