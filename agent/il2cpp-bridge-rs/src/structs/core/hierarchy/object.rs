//! IL2CPP Object wrapper and operations
use super::class::MethodSelector;
use crate::api::{self, cache};
use crate::structs::components::GameObject;
use crate::structs::core::{Class, Field, Method, Property};
use crate::structs::Il2cppString;
use std::ffi::c_void;

/// Low-level IL2CPP Object structure (matches C layout)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Il2cppObject {
    /// Pointer to the class definition
    pub klass: *mut c_void,
    /// Monitor for synchronization
    pub monitor: *mut c_void,
}

/// Safe-ish wrapper around a managed IL2CPP object pointer.
///
/// Use this type when you already have a live managed object and want
/// instance-bound access to methods, fields, or properties.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Object {
    /// Pointer to the internal IL2CPP object structure
    pub ptr: *mut Il2cppObject,
}

impl Object {
    /// Creates an Object from a raw pointer
    ///
    /// # Arguments
    /// * `ptr` - The raw pointer to the IL2CPP object
    pub unsafe fn from_ptr(ptr: *mut c_void) -> Self {
        Self {
            ptr: ptr as *mut Il2cppObject,
        }
    }

    /// Returns the raw pointer to the object
    ///
    /// # Returns
    /// * `*mut c_void` - The raw pointer
    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr as *mut c_void
    }

    /// Returns an instance-bound field lookup.
    ///
    /// The returned [`Field`] carries this object's instance pointer so
    /// [`Field::get_value`](crate::structs::Field::get_value) and
    /// [`Field::set_value`](crate::structs::Field::set_value) can operate on it.
    pub fn field(&self, name: &str) -> Option<Field> {
        let class_ptr = unsafe { api::object_get_class(self.as_ptr()) };

        if class_ptr.is_null() {
            return None;
        }

        match cache::class_from_ptr(class_ptr) {
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

    /// Returns an instance-bound method lookup.
    ///
    /// This is the preferred way to prepare instance method calls because the
    /// returned [`Method`] already carries the correct `this` pointer.
    pub fn method<S: MethodSelector>(&self, selector: S) -> Option<Method> {
        unsafe {
            let class_ptr = api::object_get_class(self.as_ptr());
            if class_ptr.is_null() {
                return None;
            }
            cache::class_from_ptr(class_ptr).and_then(|class| {
                class.method(selector).map(|mut method| {
                    method.instance = Some(self.as_ptr());
                    method
                })
            })
        }
    }

    /// Returns an instance-bound property lookup.
    pub fn property(&self, name: &str) -> Option<Property> {
        let class_ptr = unsafe { api::object_get_class(self.as_ptr()) };

        if class_ptr.is_null() {
            return None;
        }

        cache::class_from_ptr(class_ptr).and_then(|class| {
            class
                .property(name)
                .map(|prop| prop.with_instance(self.as_ptr()))
        })
    }

    /// Calls ToString on the object
    ///
    /// # Returns
    /// * `String` - The string representation, or "null" if failed
    pub fn il2cpp_to_string(&self) -> String {
        unsafe {
            if let Some(method) = self.method("ToString") {
                if let Ok(result) = method.call::<*mut Il2cppString>(&[]) {
                    if !result.is_null() {
                        return (*result).to_string().unwrap_or_else(|| "null".to_string());
                    }
                }
            }
            "null".to_string()
        }
    }

    /// Gets the GameObject associated with this object (if Is a Component)
    ///
    /// # Returns
    /// * `Result<GameObject, String>` - The GameObject, or an error if null/not found
    pub fn get_game_object(&self) -> Result<GameObject, String> {
        unsafe {
            let method = self
                .method("get_gameObject")
                .ok_or("Method 'get_gameObject' not found")?;
            let result = method.call::<*mut c_void>(&[])?;

            if result.is_null() {
                return Err("GameObject is null".to_string());
            }

            Ok(GameObject::from_ptr(result))
        }
    }

    /// Gets the size of the object header
    ///
    /// This is cached for performance.
    ///
    /// # Returns
    /// * `usize` - The size of the header in bytes
    pub fn get_header_size() -> usize {
        use std::sync::OnceLock;
        static HEADER_SIZE: OnceLock<usize> = OnceLock::new();

        *HEADER_SIZE.get_or_init(|| unsafe {
            let system_object_class = cache::mscorlib().class("System.Object");

            if let Some(class) = system_object_class {
                let size = api::class_instance_size(class.address);
                if size > 0 {
                    return size as usize;
                }
            }

            std::mem::size_of::<Il2cppObject>()
        })
    }

    /// Gets the class of this object
    ///
    /// # Returns
    /// * `Option<Class>` - The class definition, or None if failed
    pub fn get_class(&self) -> Option<Class> {
        let class_ptr = unsafe { api::object_get_class(self.as_ptr()) };
        cache::class_from_ptr(class_ptr)
    }

    /// Gets the virtual method implementation for this object
    ///
    /// # Arguments
    /// * `method` - The method definition to resolve
    ///
    /// # Returns
    /// * `*mut c_void` - Pointer to the implementation
    pub fn get_virtual_method(&self, method: &Method) -> *mut c_void {
        unsafe { api::object_get_virtual_method(self.as_ptr(), method.address) }
    }

    /// Initializes an exception object
    ///
    /// # Arguments
    /// * `exc` - Pointer to the exception object
    pub fn init_exception(&self, exc: &mut c_void) {
        unsafe { api::runtime_object_init_exception(self.as_ptr(), exc) }
    }

    /// Gets the size of the object
    ///
    /// # Returns
    /// * `u32` - Size in bytes
    pub fn get_size(&self) -> u32 {
        unsafe { api::object_get_size(self.as_ptr()) }
    }

    /// Unboxes a value type object
    ///
    /// # Returns
    /// * `*mut c_void` - Pointer to the unboxed value
    pub fn unbox(&self) -> *mut c_void {
        unsafe { api::object_unbox(self.as_ptr()) }
    }
}
