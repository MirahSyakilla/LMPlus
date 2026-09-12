//! Raw IL2CPP invoker — no bridge, no metadata hydration.
//!
//! Verified live via Frida against the running game:
//! - il2cpp_runtime_invoke CRASHES on some ctors (MessagePacket..ctor → 0x400 AV),
//!   so ALL calls go through raw method pointers with the IL2CPP x64 ABI:
//!     static:   ret fn(MethodInfo*)
//!     instance: ret fn(this, args..., MethodInfo*)
//! - MessagePacket layout (PC): Offset 0x10, SaveOffset 0x14, length 0x18,
//!   Channel 0x1C, Delimiter 0x20, MaxSize 0x24, Data 0x28, Protocol 0x30 (u16 enum).
//! - .ctor(ushort Max = 1024), Add(byte/ushort/...), Send(bool Force = false) → bool.

use std::ffi::{c_void, CString};
use std::sync::OnceLock;

type ClassPtr = *mut c_void;
type MethodPtr = *mut c_void;
type ImagePtr = *mut c_void;

struct Api {
    domain_get: extern "system" fn() -> ClassPtr,
    thread_attach: extern "system" fn(ClassPtr) -> ClassPtr,
    domain_assemblies: extern "system" fn(ClassPtr, *mut usize) -> *mut ClassPtr,
    assembly_image: extern "system" fn(ClassPtr) -> ImagePtr,
    class_from_name: extern "system" fn(ImagePtr, *const i8, *const i8) -> ClassPtr,
    get_method_from_name: extern "system" fn(ClassPtr, *const i8, i32) -> MethodPtr,
    object_new: extern "system" fn(ClassPtr) -> *mut c_void,
    class_get_type: extern "system" fn(ClassPtr) -> *mut c_void,
    type_get_object: extern "system" fn(*mut c_void) -> *mut c_void,
    class_get_field_from_name: extern "system" fn(ClassPtr, *const i8) -> *mut c_void,
    field_get_offset: extern "system" fn(*mut c_void) -> i32,
    static_field_data: extern "system" fn(ClassPtr) -> *mut c_void,
}

static API: OnceLock<Api> = OnceLock::new();
static DOMAIN: OnceLock<usize> = OnceLock::new();

fn api() -> &'static Api {
    API.get_or_init(|| unsafe {
        let ga = lib_handle();
        let sym = |name: &str| -> *mut c_void {
            let c = CString::new(name).unwrap();
            get_proc(ga, &c)
        };
        Api {
            domain_get: std::mem::transmute(sym("il2cpp_domain_get")),
            thread_attach: std::mem::transmute(sym("il2cpp_thread_attach")),
            domain_assemblies: std::mem::transmute(sym("il2cpp_domain_get_assemblies")),
            assembly_image: std::mem::transmute(sym("il2cpp_assembly_get_image")),
            class_from_name: std::mem::transmute(sym("il2cpp_class_from_name")),
            get_method_from_name: std::mem::transmute(sym("il2cpp_class_get_method_from_name")),
            object_new: std::mem::transmute(sym("il2cpp_object_new")),
            class_get_type: std::mem::transmute(sym("il2cpp_class_get_type")),
            type_get_object: std::mem::transmute(sym("il2cpp_type_get_object")),
            class_get_field_from_name: std::mem::transmute(sym("il2cpp_class_get_field_from_name")),
            field_get_offset: std::mem::transmute(sym("il2cpp_field_get_offset")),
            static_field_data: std::mem::transmute(sym("il2cpp_class_get_static_field_data")),
        }
    })
}

#[cfg(windows)]
fn lib_handle() -> *mut c_void {
    unsafe {
        let name = CString::new("GameAssembly.dll").unwrap();
        GetModuleHandleW(name.as_ptr() as *const u16) as *mut c_void
    }
}

#[cfg(windows)]
unsafe fn get_proc(module: *mut c_void, name: &CString) -> *mut c_void {
    let addr = GetProcAddress(module as winapi::shared::minwindef::HMODULE, name.as_ptr());
    if addr.is_null() {
        std::ptr::null_mut()
    } else {
        addr as *mut c_void
    }
}

#[cfg(windows)]
use winapi::um::libloaderapi::{GetModuleHandleW, GetProcAddress};

/// Attach current thread to the il2cpp domain (idempotent).
pub fn attach_thread() -> Result<(), String> {
    let api = api();
    let domain_ptr = *DOMAIN.get_or_init(|| unsafe { (api.domain_get)() as usize });
    if domain_ptr == 0 {
        return Err("il2cpp domain null (runtime not ready?)".into());
    }
    let domain = domain_ptr as ClassPtr;
    let t = unsafe { (api.thread_attach)(domain) };
    if t.is_null() {
        return Err("il2cpp_thread_attach failed".into());
    }
    Ok(())
}

/// Find a class by "Namespace.Name" or "Name" across all assemblies.
pub fn find_class(full_name: &str) -> Option<ClassPtr> {
    let api = api();
    let domain_ptr = *DOMAIN.get_or_init(|| unsafe { (api.domain_get)() as usize });
    if domain_ptr == 0 {
        return None;
    }
    let domain = domain_ptr as ClassPtr;
    unsafe {
        let mut size: usize = 0;
        let arr = (api.domain_assemblies)(domain, &mut size);
        if arr.is_null() {
            return None;
        }
        let (ns, cn) = match full_name.rfind('.') {
            Some(i) => (&full_name[..i], &full_name[i + 1..]),
            None => ("", full_name),
        };
        let ns_c = CString::new(ns).ok()?;
        let cn_c = CString::new(cn).ok()?;
        for i in 0..size {
            let asm = *arr.add(i);
            if asm.is_null() {
                continue;
            }
            let image = (api.assembly_image)(asm);
            if image.is_null() {
                continue;
            }
            let k = (api.class_from_name)(image, ns_c.as_ptr(), cn_c.as_ptr());
            if !k.is_null() {
                return Some(k);
            }
        }
    }
    None
}

/// Resolve a MethodInfo* by name + arg count.
pub fn find_method(class: ClassPtr, name: &str, arg_count: i32) -> Option<MethodPtr> {
    let api = api();
    let c = CString::new(name).ok()?;
    let m = unsafe { (api.get_method_from_name)(class, c.as_ptr(), arg_count) };
    if m.is_null() {
        None
    } else {
        Some(m)
    }
}

/// Allocate a managed object of a class.
pub fn new_object(class: ClassPtr) -> *mut c_void {
    let api = api();
    unsafe { (api.object_new)(class) }
}

/// Read the raw function pointer from MethodInfo* (first field).
pub fn method_fnptr(method: MethodPtr) -> *mut c_void {
    unsafe { (method as *mut *mut c_void).read() }
}

/// Call an instance method with NO arguments: R fn(this, MethodInfo*).
///
/// # Safety
/// The method must genuinely have zero params and belong to `this`'s class.
pub unsafe fn call_instance0<R>(method: MethodPtr, this: *mut c_void) -> R {
    let f: extern "system" fn(*mut c_void, MethodPtr) -> R =
        std::mem::transmute(method_fnptr(method));
    f(this, method)
}

/// Call a static method with NO arguments: R fn(MethodPtr).
pub unsafe fn call_static0<R>(method: MethodPtr) -> R {
    let f: extern "system" fn(MethodPtr) -> R = std::mem::transmute(method_fnptr(method));
    f(method)
}

/// Call an instance method with one u8 arg.
pub unsafe fn call_instance1_u8(method: MethodPtr, this: *mut c_void, a: u8) {
    let f: extern "system" fn(*mut c_void, u8, MethodPtr) =
        std::mem::transmute(method_fnptr(method));
    f(this, a, method)
}

/// Call an instance method with one u16 arg.
pub unsafe fn call_instance1_u16(method: MethodPtr, this: *mut c_void, a: u16) {
    let f: extern "system" fn(*mut c_void, u16, MethodPtr) =
        std::mem::transmute(method_fnptr(method));
    f(this, a, method)
}

/// Call an instance method with one i32 arg.
pub unsafe fn call_instance1_i32(method: MethodPtr, this: *mut c_void, a: i32) {
    let f: extern "system" fn(*mut c_void, i32, MethodPtr) =
        std::mem::transmute(method_fnptr(method));
    f(this, a, method)
}

/// Call an instance method with one f32 arg.
pub unsafe fn call_instance1_f32(method: MethodPtr, this: *mut c_void, a: f32) {
    let f: extern "system" fn(*mut c_void, f32, MethodPtr) =
        std::mem::transmute(method_fnptr(method));
    f(this, a, method)
}

/// Call an instance method with one bool arg returning bool.
pub unsafe fn call_instance1_bool_bool(method: MethodPtr, this: *mut c_void, a: bool) -> bool {
    let f: extern "system" fn(*mut c_void, i32, MethodPtr) -> i32 =
        std::mem::transmute(method_fnptr(method));
    f(this, a as i32, method) != 0
}

/// Get the System.Type managed object for a class.
pub fn type_object(class: ClassPtr) -> *mut c_void {
    let api = api();
    unsafe {
        let t = (api.class_get_type)(class);
        if t.is_null() {
            return std::ptr::null_mut();
        }
        (api.type_get_object)(t)
    }
}

/// Write a static u8 field of a class.
pub fn static_write_u8(class: ClassPtr, field: &str, value: u8) -> Result<(), String> {
    let api = api();
    let Ok(c) = CString::new(field) else { return Err("bad field name".into()); };
    unsafe {
        let f = (api.class_get_field_from_name)(class, c.as_ptr());
        if f.is_null() {
            return Err(format!("field {} not found", field));
        }
        let off = (api.field_get_offset)(f);
        let data = (api.static_field_data)(class);
        if data.is_null() {
            return Err("static field data null".into());
        }
        std::ptr::write_bytes(data.add(off as usize), value, 1);
    }
    Ok(())
}

/// Allocate a zeroed buffer (for minimal fake `this` objects).
pub fn alloc_zeroed(size: usize) -> *mut c_void {
    let mem = unsafe { std::alloc::alloc(std::alloc::Layout::from_size_align(size, 16).unwrap()) };
    if mem.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { std::ptr::write_bytes(mem, 0, size) };
    mem as *mut c_void
}

/// Read a static u8 field of a class.
pub fn static_read_u8(class: ClassPtr, field: &str) -> Option<u8> {
    let api = api();
    let c = CString::new(field).ok()?;
    unsafe {
        let f = (api.class_get_field_from_name)(class, c.as_ptr());
        if f.is_null() {
            return None;
        }
        let off = (api.field_get_offset)(f);
        let data = (api.static_field_data)(class);
        if data.is_null() {
            return None;
        }
        Some(*(data.add(off as usize) as *const u8))
    }
}

/// UnityEngine.Object.FindObjectOfType(Type, includeInactive=false) via raw call.
/// Static signature: R fn(Type, bool, MethodInfo*).
pub fn find_object_of_type(class: ClassPtr) -> Option<*mut c_void> {
    let uo = find_class("UnityEngine.Object")?;
    let m = find_method(uo, "FindObjectOfType", 2)?;
    let ty = type_object(class);
    if ty.is_null() {
        return None;
    }
    unsafe {
        let f: extern "system" fn(*mut c_void, i32, MethodPtr) -> *mut c_void =
            std::mem::transmute(method_fnptr(m));
        let r = f(ty, 0, m);
        if r.is_null() {
            None
        } else {
            Some(r)
        }
    }
}

/// Call a static void method with five i32 args: fn(int,int,int,int,int, MethodInfo*).
pub unsafe fn call_static5_void(method: MethodPtr, a: i32, b: i32, c: i32, d: i32, e: i32) {
    let f: extern "system" fn(i32, i32, i32, i32, i32, MethodPtr) =
        std::mem::transmute(method_fnptr(method));
    f(a, b, c, d, e, method)
}

/// Call an instance void method with one pointer arg: fn(this, ptr, MethodInfo*).
pub unsafe fn call_instance1_ptr(method: MethodPtr, this: *mut c_void, arg: *mut c_void) {
    let f: extern "system" fn(*mut c_void, *mut c_void, MethodPtr) =
        std::mem::transmute(method_fnptr(method));
    f(this, arg, method)
}

/// Call an instance bool method with no args: fn(this, MethodInfo*).
pub unsafe fn call_instance0_bool(method: MethodPtr, this: *mut c_void) -> bool {
    let f: extern "system" fn(*mut c_void, MethodPtr) -> i32 =
        std::mem::transmute(method_fnptr(method));
    f(this, method) != 0
}

/// Call an instance method with no args, ignore any return.
pub unsafe fn call_instance0_ignore(method: MethodPtr, this: *mut c_void) {
    let f: extern "system" fn(*mut c_void, MethodPtr) = std::mem::transmute(method_fnptr(method));
    f(this, method)
}

/// Allocate a zeroed fake "UIButton"-like object big enough for the id fields
/// (id at +0x108, status at +0x10C — read by UIFormationSelect.OnButtonClick).
pub fn alloc_fake_button(id: i32, status: i32) -> *mut c_void {
    let mem = unsafe { std::alloc::alloc(std::alloc::Layout::from_size_align(0x200, 16).unwrap()) };
    if mem.is_null() {
        return std::ptr::null_mut();
    }
    unsafe {
        std::ptr::write_bytes(mem, 0, 0x200);
        (mem.add(0x108) as *mut i32).write(id);
        (mem.add(0x10C) as *mut i32).write(status);
    }
    mem as *mut c_void
}

/// Get the class name of an object's class.
pub fn class_name_of(obj: *mut c_void) -> Option<String> {
    extern "C" {
        fn il2cpp_class_get_name(klass: ClassPtr) -> *const i8;
    }
    let klass = unsafe { (obj as *mut *mut c_void).read() };
    if klass.is_null() {
        return None;
    }
    unsafe {
        let n = il2cpp_class_get_name(klass);
        if n.is_null() {
            return None;
        }
        Some(std::ffi::CStr::from_ptr(n).to_string_lossy().into_owned())
    }
}
