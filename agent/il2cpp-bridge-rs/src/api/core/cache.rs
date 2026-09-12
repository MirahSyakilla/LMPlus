//! Global cache for hydrated IL2CPP metadata.
//!
//! The cache is populated during [`crate::init`] and is the normal starting
//! point for metadata-driven workflows. Most users should use the helper
//! functions in this module rather than traversing IL2CPP metadata manually.
use super::api;
#[cfg(debug_assertions)]
use crate::logger;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;
use std::ffi::{c_char, c_void, CStr};
use std::ptr;
use std::sync::Arc;

use crate::memory::image;
use crate::structs::{Arg, Assembly, Class, Field, Image, Method, Property, Type};

/// Caches IL2CPP assemblies, classes, and methods
pub struct Il2CppCache {
    /// Helper map of assembly names to Assembly structs (Arc for cheap cloning)
    pub assemblies: DashMap<String, Arc<Assembly>>,
    /// Helper map of class names to Class structs (boxed)
    pub classes: DashMap<String, Box<Class>>,
    /// Pointer-to-class-name index used to avoid O(n) scans and name-collision bugs.
    pub classes_by_ptr: DashMap<usize, String>,
    /// Helper map of method keys to Method structs
    pub methods: DashMap<String, Method>,
}

unsafe impl Send for Il2CppCache {}
unsafe impl Sync for Il2CppCache {}

/// Global cache instance
pub static CACHE: Lazy<Il2CppCache> = Lazy::new(|| Il2CppCache {
    assemblies: DashMap::new(),
    classes: DashMap::new(),
    classes_by_ptr: DashMap::new(),
    methods: DashMap::new(),
});

/// Retrieves an assembly by name.
///
/// The name may be provided with or without the `.dll` suffix.
///
/// Returns a cheap [`Arc`] clone if the assembly exists in the hydrated cache.
pub fn assembly(name: &str) -> Option<Arc<Assembly>> {
    if let Some(asm) = CACHE.assemblies.get(name) {
        return Some(Arc::clone(&asm));
    }

    if !name.ends_with(".dll") {
        let name_with_ext = format!("{}.dll", name);
        if let Some(asm) = CACHE.assemblies.get(&name_with_ext) {
            return Some(Arc::clone(&asm));
        }
    }

    None
}

/// Returns the cached `mscorlib.dll` assembly.
///
/// # Panics
///
/// Panics if the cache is not ready or if `mscorlib.dll` is unavailable in the
/// current runtime.
pub fn mscorlib() -> Arc<Assembly> {
    const KEY: &str = "mscorlib.dll";

    if let Some(asm) = CACHE.assemblies.get(KEY) {
        return Arc::clone(&asm);
    }

    assembly(KEY).expect("mscorlib not found")
}

/// Returns the cached `Assembly-CSharp.dll` assembly.
///
/// # Panics
///
/// Panics if the cache is not ready or if `Assembly-CSharp.dll` is unavailable
/// in the current runtime.
pub fn csharp() -> Arc<Assembly> {
    const KEY: &str = "Assembly-CSharp.dll";

    if let Some(asm) = CACHE.assemblies.get(KEY) {
        return Arc::clone(&asm);
    }

    assembly(KEY).expect("Assembly-CSharp not found")
}

/// Returns the cached `UnityEngine.CoreModule.dll` assembly.
///
/// # Panics
///
/// Panics if the cache is not ready or if `UnityEngine.CoreModule.dll` is
/// unavailable in the current runtime.
pub fn coremodule() -> Arc<Assembly> {
    const KEY: &str = "UnityEngine.CoreModule.dll";

    if let Some(asm) = CACHE.assemblies.get(KEY) {
        return Arc::clone(&asm);
    }

    assembly(KEY).expect("UnityEngine.CoreModule not found")
}

/// Hydrates a [`Class`] from a raw `Il2CppClass` pointer.
///
/// This is mainly useful when lower-level APIs hand you raw class pointers and
/// you want to re-enter the safe-ish metadata layer.
pub fn class_from_ptr(ptr: *mut c_void) -> Option<Class> {
    if ptr.is_null() {
        return None;
    }

    if let Some(class) = cached_class_by_ptr(ptr) {
        return Some(class);
    }

    unsafe { hydrate_class(ptr).ok() }
}

/// Initializes the cache by loading all assemblies and resetting prior state.
///
/// Also performs an eager full class hydration pass.
///
/// This is primarily used internally by [`crate::init`].
pub fn init() -> bool {
    CACHE.assemblies.clear();
    CACHE.classes.clear();
    CACHE.classes_by_ptr.clear();
    CACHE.methods.clear();

    unsafe {
        match load_all_assemblies() {
            Ok(_assembly_count) => match hydrate_all_classes() {
                Ok(_class_count) => {
                    #[cfg(debug_assertions)]
                    logger::info(&format!(
                        "Cache initialized: {} assemblies loaded, {} classes hydrated",
                        _assembly_count, _class_count
                    ));
                    true
                }
                Err(_e) => {
                    #[cfg(debug_assertions)]
                    logger::error(&format!("Cache init failed during class hydration: {}", _e));
                    false
                }
            },
            Err(_e) => {
                #[cfg(debug_assertions)]
                logger::error(&format!("Cache init failed: {}", _e));
                false
            }
        }
    }
}

/// Clears the cache so it can be re-populated.
pub(crate) fn clear() {
    CACHE.assemblies.clear();
    CACHE.classes.clear();
    CACHE.classes_by_ptr.clear();
    CACHE.methods.clear();
}

/// Hydrates all classes in all cached assemblies.
pub(crate) fn hydrate_all_classes() -> Result<usize, String> {
    let assembly_names: Vec<String> = CACHE
        .assemblies
        .iter()
        .map(|entry| entry.key().clone())
        .collect();
    let mut hydrated_class_count = 0usize;

    for name in assembly_names {
        let assembly_opt = CACHE
            .assemblies
            .get(&name)
            .map(|entry| Arc::clone(entry.value()));

        if let Some(assembly) = assembly_opt {
            let class_count = unsafe { api::image_get_class_count(assembly.image.address) };
            let mut classes = Vec::with_capacity(class_count as usize);

            for i in 0..class_count {
                let class_ptr = unsafe { api::image_get_class(assembly.image.address, i) };
                if !class_ptr.is_null() {
                    match unsafe { hydrate_class(class_ptr) } {
                        Ok(class) => {
                            classes.push(class);
                            hydrated_class_count += 1;
                        }
                        Err(e) => {
                            return Err(format!(
                                "Failed to hydrate class {} in assembly '{}': {}",
                                i, name, e
                            ));
                        }
                    }
                }
            }

            let mut new_assembly = (*assembly).clone();
            new_assembly.classes = classes;

            CACHE.assemblies.insert(name, Arc::new(new_assembly));
        }
    }
    #[cfg(debug_assertions)]
    logger::info(&format!("Hydrated {} classes", hydrated_class_count));
    Ok(hydrated_class_count)
}

/// Loads all assemblies at initialization
///
/// # Returns
/// * `Result<usize, String>` - The number of loaded assemblies or an error message
pub(crate) unsafe fn load_all_assemblies() -> Result<usize, String> {
    let domain = api::domain_get();
    if domain.is_null() {
        return Err("Failed to get domain".to_string());
    }

    let mut size = 0;
    let assemblies_ptr = api::domain_get_assemblies(domain, &mut size);
    if assemblies_ptr.is_null() {
        return Err("Failed to get assemblies".to_string());
    }

    let assemblies_slice = std::slice::from_raw_parts(assemblies_ptr, size);
    let mut count = 0;

    for &assembly_ptr in assemblies_slice {
        if assembly_ptr.is_null() {
            continue;
        }

        let image = api::assembly_get_image(assembly_ptr);
        if image.is_null() {
            continue;
        }

        let name_ptr = api::image_get_name(image);
        if name_ptr.is_null() {
            continue;
        }

        let name = CStr::from_ptr(name_ptr).to_string_lossy();

        if let Some(asm) = hydrate_assembly(assembly_ptr) {
            CACHE.assemblies.insert(name.to_string(), asm);
            count += 1;
        }
    }

    Ok(count)
}

/// Hydrates an assembly from a raw pointer
///
/// # Arguments
/// * `assembly_ptr` - Pointer to the Il2CppAssembly
///
/// # Returns
/// * `Option<Arc<Assembly>>` - The hydrated Assembly struct
unsafe fn hydrate_assembly(assembly_ptr: *mut c_void) -> Option<Arc<Assembly>> {
    let image = api::assembly_get_image(assembly_ptr);
    if image.is_null() {
        return None;
    }

    let file_ptr = api::image_get_filename(image);
    let name_ptr = api::image_get_name(image);

    let file = if !file_ptr.is_null() {
        CStr::from_ptr(file_ptr).to_string_lossy().into_owned()
    } else {
        String::new()
    };

    let name = if !name_ptr.is_null() {
        CStr::from_ptr(name_ptr).to_string_lossy().into_owned()
    } else {
        String::new()
    };

    let image_wrapper = Image {
        address: image,
        name: name.clone(),
        filename: file.clone(),
        assembly: assembly_ptr,
        entry_point: api::image_get_entry_point(image),
    };

    let assembly = Assembly {
        image: image_wrapper,
        address: assembly_ptr,
        file: file.clone(),
        name: name.clone(),
        classes: Vec::new(),
    };

    Some(Arc::new(assembly))
}

#[inline]
unsafe fn cstr_lossy(ptr: *const c_char, default: &str) -> String {
    if ptr.is_null() {
        default.to_owned()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

#[inline]
unsafe fn type_name_or(type_ptr: *mut c_void, default: &str) -> String {
    if type_ptr.is_null() {
        return default.to_owned();
    }
    cstr_lossy(api::type_get_name(type_ptr), default)
}

#[inline]
fn make_class_key(namespace: &str, name: &str) -> String {
    if namespace.is_empty() {
        name.to_owned()
    } else {
        format!("{}.{}", namespace, name)
    }
}

fn make_unique_class_key(preferred: &str, class_ptr: *mut c_void) -> String {
    match CACHE.classes.get(preferred) {
        Some(existing) if existing.address != class_ptr => {
            format!("{}@0x{:X}", preferred, class_ptr as usize)
        }
        _ => preferred.to_owned(),
    }
}

fn cached_class_by_ptr(ptr: *mut c_void) -> Option<Class> {
    let key = CACHE.classes_by_ptr.get(&(ptr as usize))?;
    let class = CACHE.classes.get(key.value())?;
    Some((**class.value()).clone())
}

fn cached_class_ref_by_ptr(ptr: *mut c_void) -> Option<*const Class> {
    let key = CACHE.classes_by_ptr.get(&(ptr as usize))?;
    let class = CACHE.classes.get(key.value())?;
    Some(&**class.value() as *const Class)
}

fn method_cache_key(full_class_name: &str, method: &Method) -> String {
    let mut key = String::with_capacity(
        full_class_name.len() + method.name.len() + method.args.len().saturating_mul(16) + 4,
    );
    key.push_str(full_class_name);
    key.push_str("::");
    key.push_str(&method.name);
    key.push('(');
    for (i, arg) in method.args.iter().enumerate() {
        if i > 0 {
            key.push(',');
        }
        key.push_str(&arg.type_info.name);
    }
    key.push(')');
    key
}

/// Hydrates a class from a pointer, populating fields and methods
///
/// # Arguments
/// * `class_ptr` - Pointer to the Il2CppClass
///
/// # Returns
/// * `Result<Class, String>` - The hydrated Class struct or an error
unsafe fn hydrate_class(class_ptr: *mut c_void) -> Result<Class, String> {
    if let Some(existing) = cached_class_by_ptr(class_ptr) {
        return Ok(existing);
    }

    let name_ptr = api::class_get_name(class_ptr);
    if name_ptr.is_null() {
        return Err("class_get_name returned null".to_string());
    }
    let namespace_ptr = api::class_get_namespace(class_ptr);

    let name = cstr_lossy(name_ptr, "");
    let namespace = cstr_lossy(namespace_ptr, "");
    let key = make_class_key(&namespace, &name);

    if let Some(existing) = CACHE.classes.get(&key) {
        if existing.address == class_ptr {
            CACHE.classes_by_ptr.insert(class_ptr as usize, key.clone());
            return Ok((**existing).clone());
        }
    }

    let cache_key = make_unique_class_key(&key, class_ptr);

    let parent_ptr = api::class_get_parent(class_ptr);
    let parent = if !parent_ptr.is_null() {
        let p_name = api::class_get_name(parent_ptr);
        if !p_name.is_null() {
            let name = cstr_lossy(p_name, "");
            let namespace = cstr_lossy(api::class_get_namespace(parent_ptr), "");
            Some(make_class_key(&namespace, &name))
        } else {
            None
        }
    } else {
        None
    };

    let mut interfaces = Vec::new();
    let mut iter = ptr::null_mut();
    loop {
        let interface_ptr = api::class_get_interfaces(class_ptr, &mut iter);
        if interface_ptr.is_null() {
            break;
        }
        interfaces.push(interface_ptr);
    }

    let mut nested_types = Vec::new();
    let mut iter = ptr::null_mut();
    loop {
        let nested_ptr = api::class_get_nested_types(class_ptr, &mut iter);
        if nested_ptr.is_null() {
            break;
        }
        nested_types.push(nested_ptr);
    }

    let assembly_name = cstr_lossy(api::class_get_assemblyname(class_ptr), "");

    // Use Arc clone for cheap assembly reference
    let assembly = CACHE
        .assemblies
        .get(&assembly_name)
        .map(|a| Arc::clone(a.value()));

    let ty = api::class_get_type(class_ptr);
    let object = if ty.is_null() {
        ptr::null_mut()
    } else {
        api::type_get_object(ty)
    };

    let mut class_box = Box::new(Class {
        address: class_ptr,
        image: api::class_get_image(class_ptr),
        token: api::class_get_type_token(class_ptr),
        name: name.clone(),
        parent,
        namespace: namespace.clone(),
        is_enum: api::class_is_enum(class_ptr),
        is_generic: api::class_is_generic(class_ptr),
        is_inflated: api::class_is_inflated(class_ptr),
        is_interface: api::class_is_interface(class_ptr),
        is_abstract: api::class_is_abstract(class_ptr),
        is_blittable: api::class_is_blittable(class_ptr),
        is_valuetype: api::class_is_valuetype(class_ptr),
        flags: api::class_get_flags(class_ptr) as i32,
        rank: api::class_get_rank(class_ptr) as i32,
        instance_size: api::class_instance_size(class_ptr),
        array_element_size: api::class_array_element_size(class_ptr),
        num_fields_count: api::class_num_fields(class_ptr),
        enum_basetype: api::class_enum_basetype(class_ptr),
        static_field_data: api::class_get_static_field_data(class_ptr),
        assembly_name,
        assembly,
        fields: Vec::new(),
        methods: Vec::new(),
        properties: Vec::new(),
        interfaces,
        nested_types,
        element_class: api::class_get_element_class(class_ptr),
        declaring_type: api::class_get_declaring_type(class_ptr),
        ty,
        object,
    });

    let full_class_name = make_class_key(&namespace, &name);

    archive_fields(&mut class_box, class_ptr)?;
    archive_methods(&mut class_box, class_ptr, &full_class_name)?;
    archive_properties(&mut class_box);

    let class_clone = (*class_box).clone();

    CACHE.classes.insert(cache_key.clone(), class_box);
    CACHE.classes_by_ptr.insert(class_ptr as usize, cache_key);

    Ok(class_clone)
}

/// Archives fields for a class
///
/// # Arguments
/// * `class` - Mutuable reference to the Class struct
/// * `class_ptr` - Pointer to the Il2CppClass
///
/// # Returns
/// * `Result<(), String>` - Result indicating success
unsafe fn archive_fields(class: &mut Class, class_ptr: *mut c_void) -> Result<(), String> {
    class.fields.reserve(class.num_fields_count);

    let mut iter = ptr::null_mut();
    loop {
        let field_ptr = api::class_get_fields(class_ptr, &mut iter);
        if field_ptr.is_null() {
            break;
        }

        let name = cstr_lossy(api::field_get_name(field_ptr), "");
        let offset = api::field_get_offset(field_ptr);
        let type_ptr = api::field_get_type(field_ptr);
        let type_name = type_name_or(type_ptr, "System.Object");
        let flags = api::field_get_flags(field_ptr);

        let field = Field {
            address: field_ptr,
            name,
            type_info: Type {
                address: type_ptr,
                name: type_name,
                size: get_type_size(type_ptr),
            },
            class: Some(class as *const Class),
            offset,
            flags,
            is_static: (flags & api::FIELD_ATTRIBUTE_STATIC) != 0
                || (flags & api::FIELD_ATTRIBUTE_LITERAL) != 0,
            is_literal: (flags & api::FIELD_ATTRIBUTE_LITERAL) != 0,
            is_readonly: (flags & api::FIELD_ATTRIBUTE_INIT_ONLY) != 0,
            is_not_serialized: (flags & api::FIELD_ATTRIBUTE_NOT_SERIALIZED) != 0,
            is_special_name: (flags & api::FIELD_ATTRIBUTE_SPECIAL_NAME) != 0,
            is_pinvoke_impl: (flags & api::FIELD_ATTRIBUTE_PINVOKE_IMPL) != 0,
            instance: None,
        };

        class.fields.push(field);
    }
    Ok(())
}

/// Archives methods for a class
///
/// # Arguments
/// * `class` - Mutable reference to the Class struct
/// * `class_ptr` - Pointer to the Il2CppClass
/// * `full_class_name` - The fully qualified name of the class
///
/// # Returns
/// * `Result<(), String>` - Result indicating success
unsafe fn archive_methods(
    class: &mut Class,
    class_ptr: *mut c_void,
    full_class_name: &str,
) -> Result<(), String> {
    let image_base = image::get_image_base(
        crate::init::TARGET_IMAGE_NAME
            .get()
            .map(|s| s.as_str())
            .unwrap_or(""),
    )
    .unwrap_or(0);

    let mut iter = ptr::null_mut();
    loop {
        let method_ptr = api::class_get_methods(class_ptr, &mut iter);
        if method_ptr.is_null() {
            break;
        }

        let name = cstr_lossy(api::method_get_name(method_ptr), "");
        let flags = api::method_get_flags(method_ptr, ptr::null_mut());
        let return_type_ptr = api::method_get_return_type(method_ptr);
        let return_type_name = type_name_or(return_type_ptr, "");
        let function_ptr = *(method_ptr as *const *mut c_void);

        let rva = if image_base > 0 && !function_ptr.is_null() {
            (function_ptr as usize).wrapping_sub(image_base) as u64
        } else {
            0
        };

        let param_count = api::method_get_param_count(method_ptr);
        let mut method = Method {
            address: method_ptr,
            token: api::method_get_token(method_ptr),
            name: name.clone(),
            class: Some(class as *const Class),
            return_type: Type {
                address: return_type_ptr,
                name: return_type_name,
                size: get_type_size(return_type_ptr),
            },
            flags: flags as i32,
            is_static: (flags & api::METHOD_ATTRIBUTE_STATIC as u32) != 0,
            function: function_ptr,
            rva,
            va: function_ptr as u64,
            args: Vec::with_capacity(param_count as usize),
            is_generic: api::method_is_generic(method_ptr),
            is_inflated: api::method_is_inflated(method_ptr),
            is_instance: api::method_is_instance(method_ptr),
            param_count,
            declaring_type: api::method_get_declaring_type(method_ptr),
            instance: None,
        };

        for i in 0..param_count {
            let param_name = cstr_lossy(api::method_get_param_name(method_ptr, i as u32), "");
            let param_type_ptr = api::method_get_param(method_ptr, i as u32);
            let param_type_name = type_name_or(param_type_ptr, "");

            method.args.push(Arg {
                name: param_name,
                type_info: Type {
                    address: param_type_ptr,
                    name: param_type_name,
                    size: get_type_size(param_type_ptr),
                },
            });
        }

        let method_key = method_cache_key(full_class_name, &method);
        CACHE.methods.insert(method_key, method.clone());

        class.methods.push(method);
    }

    Ok(())
}

/// Archives properties for a class by combining get_/set_ method pairs
///
/// # Arguments
/// * `class` - Mutable reference to the Class struct
fn archive_properties(class: &mut Class) {
    let mut property_map: FxHashMap<String, (Option<Method>, Option<Method>)> =
        FxHashMap::default();

    for method in &class.methods {
        let is_getter = method.name.starts_with("get_") && method.args.is_empty();
        let is_setter = method.name.starts_with("set_") && method.args.len() == 1;

        if is_getter || is_setter {
            let prop_name = method.name[4..].to_string();
            let entry = property_map.entry(prop_name).or_insert((None, None));

            if is_getter {
                entry.0 = Some(method.clone());
            } else {
                entry.1 = Some(method.clone());
            }
        }
    }

    let mut props: Vec<_> = property_map.into_iter().collect();
    props.sort_by(|a, b| a.0.cmp(&b.0));

    for (_name, (getter, setter)) in props {
        if let Some(prop) = Property::from_methods(getter, setter) {
            class.properties.push(prop);
        }
    }
}

/// Creates a Method struct from a raw pointer
///
/// # Arguments
/// * `method_ptr` - Pointer to the MethodInfo
///
/// # Returns
/// * `Option<Method>` - The hydrated Method struct
pub fn method_from_ptr(method_ptr: *mut c_void) -> Option<Method> {
    if method_ptr.is_null() {
        return None;
    }

    unsafe {
        let name = cstr_lossy(api::method_get_name(method_ptr), "");
        let flags = api::method_get_flags(method_ptr, ptr::null_mut());
        let return_type_ptr = api::method_get_return_type(method_ptr);
        let return_type_name = type_name_or(return_type_ptr, "");
        let function_ptr = *(method_ptr as *const *mut c_void);

        let image_base = image::get_image_base(
            crate::init::TARGET_IMAGE_NAME
                .get()
                .map(|s| s.as_str())
                .unwrap_or(""),
        )
        .unwrap_or(0);
        let mut rva = 0;

        if image_base > 0 && !function_ptr.is_null() {
            rva = (function_ptr as usize).wrapping_sub(image_base) as u64;
        }

        let class_ptr = api::method_get_class(method_ptr);
        let class_ref = if class_ptr.is_null() {
            None
        } else {
            if cached_class_ref_by_ptr(class_ptr).is_none() {
                let _ = hydrate_class(class_ptr);
            }
            cached_class_ref_by_ptr(class_ptr)
        };

        let param_count = api::method_get_param_count(method_ptr);
        let mut method = Method {
            address: method_ptr,
            token: api::method_get_token(method_ptr),
            name: name.clone(),
            class: class_ref,
            return_type: Type {
                address: return_type_ptr,
                name: return_type_name,
                size: get_type_size(return_type_ptr),
            },
            flags: flags as i32,
            is_static: (flags & api::METHOD_ATTRIBUTE_STATIC as u32) != 0,
            function: function_ptr,
            rva,
            va: function_ptr as u64,
            args: Vec::with_capacity(param_count as usize),
            is_generic: api::method_is_generic(method_ptr),
            is_inflated: api::method_is_inflated(method_ptr),
            is_instance: api::method_is_instance(method_ptr),
            param_count,
            declaring_type: api::method_get_declaring_type(method_ptr),
            instance: None,
        };

        for i in 0..param_count {
            let param_name = cstr_lossy(api::method_get_param_name(method_ptr, i as u32), "");
            let param_type_ptr = api::method_get_param(method_ptr, i as u32);
            let param_type_name = type_name_or(param_type_ptr, "");

            method.args.push(Arg {
                name: param_name,
                type_info: Type {
                    address: param_type_ptr,
                    name: param_type_name,
                    size: get_type_size(param_type_ptr),
                },
            });
        }

        Some(method)
    }
}

/// Helper to get the size of a type
///
/// # Arguments
/// * `type_ptr` - Pointer to the Il2CppType
///
/// # Returns
/// * `i32` - The size of the type in bytes
unsafe fn get_type_size(type_ptr: *mut c_void) -> i32 {
    if type_ptr.is_null() {
        return 0;
    }

    let class_ptr = api::class_from_type(type_ptr);
    if class_ptr.is_null() {
        return 0;
    }

    if api::class_is_valuetype(class_ptr) {
        let mut align: usize = 0;
        api::class_value_size(class_ptr, &mut align)
    } else {
        std::mem::size_of::<*mut c_void>() as i32
    }
}
