//! il2cpp runtime FFI bindings + name-based resolver.
//!
//! All game interaction goes through the game's exported il2cpp_* API, so the
//! agent survives game updates as long as class/method names stay stable
//! (no hardcoded RVAs). RVAs from the Aug-2026 dump are only used as fallback
//! documentation, never as addresses.

use std::ffi::CString;

pub type DomainPtr = *mut core::ffi::c_void;
pub type ThreadPtr = *mut core::ffi::c_void;
pub type AssemblyPtr = *mut core::ffi::c_void;
pub type ImagePtr = *mut core::ffi::c_void;
pub type ClassPtr = *mut core::ffi::c_void;
pub type MethodPtr = *mut core::ffi::c_void;
pub type ObjectPtr = *mut core::ffi::c_void;
pub type VoidPtr = *mut core::ffi::c_void;

extern "C" {
    fn il2cpp_domain_get() -> DomainPtr;
    fn il2cpp_thread_attach(domain: DomainPtr) -> ThreadPtr;
    fn il2cpp_domain_get_assemblies(domain: DomainPtr, size: *mut usize) -> *mut AssemblyPtr;
    fn il2cpp_assembly_get_image(assembly: AssemblyPtr) -> ImagePtr;
    #[allow(dead_code)]
    #[allow(dead_code)]
    fn il2cpp_image_get_class_count(image: ImagePtr) -> usize;
    #[allow(dead_code)]
    fn il2cpp_image_get_class(image: ImagePtr, index: usize) -> ClassPtr;
    fn il2cpp_class_from_name(image: ImagePtr, namespace: *const i8, name: *const i8) -> ClassPtr;
    #[allow(dead_code)]
    fn il2cpp_class_get_name(klass: ClassPtr) -> *const i8;
    #[allow(dead_code)]
    #[allow(dead_code)]
    fn il2cpp_class_get_namespace(klass: ClassPtr) -> *const i8;
    fn il2cpp_class_get_method_from_name(
        klass: ClassPtr,
        name: *const i8,
        args: i32,
    ) -> MethodPtr;
    fn il2cpp_runtime_invoke(
        method: MethodPtr,
        obj: VoidPtr,
        params: *mut *mut core::ffi::c_void,
        exc: *mut *mut core::ffi::c_void,
    ) -> ObjectPtr;
    #[allow(dead_code)]
    fn il2cpp_static_field_get_address(klass: ClassPtr) -> VoidPtr;
}

/// Handle to a resolved static method ready for runtime_invoke.
#[derive(Clone, Copy)]
pub struct Il2CppMethod(pub MethodPtr);

/// Resolve a method by walking all assembly images looking for
/// `class_name::method_name` with `arg_count` parameters.
/// Mirrors the resolver used by generic IL2CPP injectors.
pub fn find_method(class_name: &str, method_name: &str, arg_count: i32) -> Option<Il2CppMethod> {
    unsafe {
        let domain = il2cpp_domain_get();
        if domain.is_null() {
            return None;
        }
        let mut size: usize = 0;
        let assemblies = il2cpp_domain_get_assemblies(domain, &mut size);
        if assemblies.is_null() {
            return None;
        }
        let cname = CString::new(class_name).ok()?;
        let mname = CString::new(method_name).ok()?;
        for i in 0..size {
            let assembly = *assemblies.add(i);
            if assembly.is_null() {
                continue;
            }
            let image = il2cpp_assembly_get_image(assembly);
            if image.is_null() {
                continue;
            }
            // Fast path: known namespace-less game classes.
            let klass = il2cpp_class_from_name(image, std::ptr::null(), cname.as_ptr());
            if !klass.is_null() {
                if let Some(m) = try_method(klass, &mname, arg_count) {
                    return Some(m);
                }
            }
        }
        None
    }
}

unsafe fn try_method(klass: ClassPtr, mname: &CString, arg_count: i32) -> Option<Il2CppMethod> {
    let m = il2cpp_class_get_method_from_name(klass, mname.as_ptr(), arg_count);
    if m.is_null() {
        None
    } else {
        Some(Il2CppMethod(m))
    }
}

/// Safety-checked invoke wrapper.
pub unsafe fn invoke(
    method: Il2CppMethod,
    obj: VoidPtr,
    params: *mut *mut core::ffi::c_void,
) -> Result<ObjectPtr, String> {
    let mut exc: *mut core::ffi::c_void = std::ptr::null_mut();
    let result = il2cpp_runtime_invoke(method.0, obj, params, &mut exc);
    if !exc.is_null() {
        return Err("il2cpp exception thrown during invoke".into());
    }
    Ok(result)
}

pub fn attach_thread() -> bool {
    unsafe {
        let domain = il2cpp_domain_get();
        if domain.is_null() {
            return false;
        }
        !il2cpp_thread_attach(domain).is_null()
    }
}

/// Wait until the il2cpp domain is initialized (game finished booting).
pub unsafe fn wait_for_il2cpp_ready(timeout_ms: u64) -> bool {
    let start = std::time::Instant::now();
    loop {
        let domain = il2cpp_domain_get();
        if !domain.is_null() {
            // Domain exists; also require at least one assembly to be loaded.
            let mut size: usize = 0;
            let asm = il2cpp_domain_get_assemblies(domain, &mut size);
            if !asm.is_null() && size > 0 {
                return true;
            }
        }
        if start.elapsed().as_millis() as u64 > timeout_ms {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

/// Value-boxing helpers for runtime_invoke arguments.
pub mod boxed {
    pub fn u8_box(value: u8) -> *mut core::ffi::c_void {
        let b = Box::new(value as u8);
        Box::into_raw(b) as *mut _
    }

    pub fn f32_box(value: f32) -> *mut core::ffi::c_void {
        let b = Box::new(value);
        Box::into_raw(b) as *mut _
    }

    pub fn i32_box(value: i32) -> *mut core::ffi::c_void {
        let b = Box::new(value);
        Box::into_raw(b) as *mut _
    }
}

// Host-side (non-injected) resolution tests use these type aliases only.
#[allow(dead_code)]
pub type Il2CppDomain = DomainPtr;
#[allow(dead_code)]
pub type Il2CppImage = ImagePtr;
