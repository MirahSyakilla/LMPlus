//! Raw IL2CPP method invocation with managed exception handling.

use super::super::api;
use crate::structs::core::hierarchy::object::Object;
use std::ffi::c_void;

/// Invokes an IL2CPP method using raw pointers.
///
/// This is the low-level escape hatch behind
/// [`crate::structs::Method::call`](crate::structs::Method::call). Prefer that
/// higher-level API unless you already have raw `MethodInfo`, object, and
/// parameter pointers.
///
/// Managed exceptions are intercepted and returned as `Err(String)`.
pub fn invoke_method(
    method: *mut c_void,
    obj: *mut c_void,
    params: *const *mut c_void,
) -> Result<*mut c_void, String> {
    unsafe {
        let mut exc: *mut c_void = std::ptr::null_mut();
        let result = api::runtime_invoke(method, obj, params, &mut exc);

        if !exc.is_null() {
            let name_ptr = api::method_get_name(method);
            let name = if !name_ptr.is_null() {
                std::ffi::CStr::from_ptr(name_ptr).to_string_lossy()
            } else {
                std::borrow::Cow::Borrowed("unknown")
            };

            if name == "get_Message" {
                return Err("Exception occurred while getting exception message".to_string());
            }

            let exc_class = api::object_get_class(exc);
            let get_message_name = std::ffi::CString::new("get_Message").unwrap();
            let get_message_method =
                api::class_get_method_from_name(exc_class, get_message_name.as_ptr(), 0);

            let message = if !get_message_method.is_null() {
                let mut inner_exc: *mut c_void = std::ptr::null_mut();
                let message_obj = api::runtime_invoke(
                    get_message_method,
                    exc,
                    std::ptr::null_mut(),
                    &mut inner_exc,
                );

                if !inner_exc.is_null() {
                    "Exception thrown while getting exception message".to_string()
                } else if !message_obj.is_null() {
                    let il2cpp_string = message_obj as *mut crate::structs::Il2cppString;
                    (*il2cpp_string).to_string().unwrap_or_default()
                } else {
                    "Exception message was null".to_string()
                }
            } else {
                let exc_object = Object::from_ptr(exc);
                exc_object.il2cpp_to_string()
            };

            return Err(format!(
                "IL2CPP exception during invocation of '{}': {}",
                name, message
            ));
        }

        Ok(result)
    }
}
