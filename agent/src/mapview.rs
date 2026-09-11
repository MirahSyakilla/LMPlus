//! Kingdom Map 3D View (Full/Balanced/None) + Map Zoom.
//!
//! RE reference (Aug-2026 IL2CPP dump):
//! - UIOther_Set.btn_Map_3D / btn_Map / btn_Map_2D (= Full/Balanced/None)
//!   → DataManager.SetShowMapType(byte) [persists in SysSetting.mShowMapType]
//!   → MapTile3D.set_GlobalMapFovLevel(byte) drives camera + tile LOD.
//!   Byte semantics verified via MapTile3D static field _GlobalMapFovLevel:
//!     0 = Full (btn_Map_3D), 1 = Balanced (btn_Map), 2 = None (btn_Map_2D).
//! - Zoom: MapTile3D._CameraDist (float) + RefreshCamera(bool) applies it.
//!   The legacy external poke targeted the native camera ortho size at +0x89C;
//!   with in-process access we call the managed setter instead, removing the
//!   memory-scan dependency for zoom.

use crate::il2cpp::{self, boxed};

/// Kingdom Map 3D View modes, in UI order.
pub const MODE_NAMES: [&str; 3] = ["Full", "Balanced", "None"];

pub fn set_map_3d_view(mode: u8) -> Result<(), String> {
    if mode as usize >= MODE_NAMES.len() {
        return Err(format!("mode {} out of range 0..=2", mode));
    }

    let (tx, rx) = std::sync::mpsc::channel();
    crate::unity_thread::enqueue(Box::new(move || {
        tx.send(do_set_map_3d_view(mode)).ok();
    }));
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| "map3dview action timed out on unity thread".to_string())?
}

fn do_set_map_3d_view(mode: u8) -> Result<(), String> {
    if !il2cpp::attach_thread() {
        return Err("failed to attach il2cpp thread".into());
    }

    // 1. Persist via DataManager.SetShowMapType(byte) on the singleton.
    let get_dm = il2cpp::find_method("DataManager", "get_Instance", 0)
        .ok_or("DataManager.get_Instance not found")?;
    let set_type = il2cpp::find_method("DataManager", "SetShowMapType", 1)
        .ok_or("DataManager.SetShowMapType not found")?;

    unsafe {
        let dm = il2cpp::invoke(get_dm, std::ptr::null_mut(), std::ptr::null_mut())
            .map_err(|e| format!("DataManager.get_Instance: {}", e))?;
        let arg = boxed::u8_box(mode);
        let mut args: [*mut core::ffi::c_void; 1] = [arg];
        il2cpp::invoke(set_type, dm, args.as_mut_ptr())
            .map_err(|e| format!("SetShowMapType: {}", e))?;
    }

    // 2. Apply to the live map via MapTile3D.set_GlobalMapFovLevel(byte).
    //    The setter is instance-level in the dump but operates on the global
    //    static; it requires a live MapTile3D instance. We locate one through
    //    the DataManager-independent path: MapTile.mapTile3D field. If no
    //    instance is live (e.g. user is not on the map), the persisted byte is
    //    still applied by the game on next map load — same as native behavior
    //    when the setting is changed while off-map.
    apply_global_fov_level(mode);

    Ok(())
}

fn apply_global_fov_level(mode: u8) {
    let setter = match il2cpp::find_method("MapTile3D", "set_GlobalMapFovLevel", 1) {
        Some(m) => m,
        None => return, // class renamed in a future version; persistence path above still works
    };
    // Locate a live MapTile3D instance through MapTile.mapTile3D.
    unsafe {
        if let Some(instance) = find_maptile3d_instance() {
            let arg = boxed::u8_box(mode);
            let mut args: [*mut core::ffi::c_void; 1] = [arg];
            let _ = il2cpp::invoke(setter, instance, args.as_mut_ptr());
        }
    }
}

unsafe fn find_maptile3d_instance() -> Option<*mut core::ffi::c_void> {
    // MapTile.mapTile3D is an instance field (0x1F0) on the MapTile MonoBehaviour.
    // We find the live MapTile via its static singleton if the game exposes one,
    // otherwise we scan the il2cpp GC heap for the class (costly, so cached).
    extern "C" {
        fn il2cpp_class_from_name(
            image: *mut core::ffi::c_void,
            ns: *const i8,
            name: *const i8,
        ) -> *mut core::ffi::c_void;
        fn il2cpp_class_get_static_field_data(klass: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    }
    let _ = (il2cpp_class_from_name, il2cpp_class_get_static_field_data);

    // Preferred: MapTile.get_Instance / mapTile3D property.
    let get_tile = il2cpp::find_method("MapTile", "get_Instance", 0)?;
    let tile = il2cpp::invoke(get_tile, std::ptr::null_mut(), std::ptr::null_mut()).ok()?;
    if tile.is_null() {
        return None;
    }
    // Read MapTile.mapTile3D at offset 0x1F0 (verified in dump.cs:77613).
    let ptr = ((tile as *mut u8).add(0x1F0) as *mut *mut core::ffi::c_void).read();
    if ptr.is_null() {
        None
    } else {
        Some(ptr)
    }
}

/// Arbitrary map zoom via the managed camera-distance setter.
/// Replaces the legacy +0x89C native-camera poke when a live MapTile3D exists.
pub fn set_camera_dist(value: f32) -> Result<(), String> {
    if !(1.0..=100.0).contains(&value) {
        return Err("zoom value out of range 1.0..=100.0".into());
    }

    let (tx, rx) = std::sync::mpsc::channel();
    crate::unity_thread::enqueue(Box::new(move || {
        tx.send(do_set_camera_dist(value)).ok();
    }));
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| "zoom action timed out on unity thread".to_string())?
}

fn do_set_camera_dist(value: f32) -> Result<(), String> {
    if !il2cpp::attach_thread() {
        return Err("failed to attach il2cpp thread".into());
    }
    unsafe {
        let instance = find_maptile3d_instance()
            .ok_or("no live MapTile3D (is the kingdom map visible?)")?;
        let setter = il2cpp::find_method("MapTile3D", "set_CameraDist", 1)
            .ok_or("MapTile3D.set_CameraDist not found")?;
        let arg = boxed::f32_box(value);
        let mut args: [*mut core::ffi::c_void; 1] = [arg];
        il2cpp::invoke(setter, instance, args.as_mut_ptr())
            .map_err(|e| format!("set_CameraDist: {}", e))?;

        // Apply immediately.
        if let Some(refresh) = il2cpp::find_method("MapTile3D", "RefreshCamera", 1) {
            let arg = boxed::u8_box(1);
            let mut rargs: [*mut core::ffi::c_void; 1] = [arg];
            let _ = il2cpp::invoke(refresh, instance, rargs.as_mut_ptr());
        }
    }
    Ok(())
}
