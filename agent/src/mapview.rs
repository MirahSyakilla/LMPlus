//! Kingdom Map 3D View (Full/Balanced/None) + Map Zoom slider — bridge edition.
//!
//! RE-verified chain (Aug-2026 IL2CPP dump; field offsets platform-independent):
//!   HeroStage_Suggestion.get_door()   (static)  → live Door
//!   Door.TileMapController            (+0xE60)  → live MapTile
//!   MapTile.mapTile3D                 (+0x1F0)  → live MapTile3D
//!
//! 3D View: MapTile3D.set_FovLevel(byte) — 0=Full, 1=Balanced, 2=None
//!   (updates instance _CameraFovLevel 0x2A8 consumed by RefreshOrthoSize),
//!   plus DataManager.get_Instance → SetShowMapType(byte) for persistence.
//!
//! Zoom: orthographic camera; set_CameraDist(f32) is self-contained
//!   (ReScaleByDist + StopScroll + RefreshCamera internally, no clamp —
//!   we clamp). Valid range: 2.1 (near) … 5.2 (far), default 4.2.

use crate::il2cpp;
use crate::unity_thread;

/// Kingdom Map 3D View modes, in UI order.
pub const MODE_NAMES: [&str; 3] = ["Full", "Balanced", "None"];

/// Camera distance bounds (RE: MapNearPlane=2.1, MapFarPlane=5.2, default 4.2).
pub const CAM_DIST_NEAR: f32 = 2.1;
pub const CAM_DIST_FAR: f32 = 5.2;

const MAPTILE_MAPTILE3D_OFFSET: usize = 0x1F0;
const DOOR_TILEMAPCONTROLLER_OFFSET: usize = 0xE60;

pub fn set_map_3d_view(mode: u8) -> Result<(), String> {
    if mode as usize >= MODE_NAMES.len() {
        return Err(format!("mode {} out of range 0..=2", mode));
    }
    unity_thread::call_or_inline(
        move || do_set_map_3d_view(mode),
        std::time::Duration::from_secs(5),
    )?
}

fn do_set_map_3d_view(mode: u8) -> Result<(), String> {
    il2cpp::ensure_bridge()?;

    let asm = il2cpp::bridge_csharp();

    // 1. Persist: DataManager.Instance.SetShowMapType(mode).
    let dm_class = asm
        .class("DataManager")
        .ok_or("DataManager class not found")?;
    let get_dm = dm_class
        .method("get_Instance")
        .ok_or("DataManager.get_Instance not found")?;
    let mut set_type = dm_class
        .method(("SetShowMapType", 1))
        .ok_or("DataManager.SetShowMapType not found")?;

    let dm_ptr: *mut std::ffi::c_void = unsafe { get_dm.call(&[])? };
    if dm_ptr.is_null() {
        return Err("DataManager instance is null".into());
    }
    set_type.instance = Some(dm_ptr);
    unsafe {
        set_type
            .call::<()>(&[&mode as *const u8 as *mut std::ffi::c_void])
            .map_err(|e| format!("SetShowMapType: {}", e))?;
    }

    // 2. Apply live: MapTile3D.set_FovLevel(mode) — skip silently when
    //    off-map; the persisted value applies on next map load (native parity).
    if let Ok(map3d) = find_maptile3d(&asm) {
        let setter = map3d
            .method(("set_FovLevel", 1))
            .ok_or("MapTile3D.set_FovLevel not found")?;
        unsafe {
            setter
                .call::<()>(&[&mode as *const u8 as *mut std::ffi::c_void])
                .map_err(|e| format!("set_FovLevel: {}", e))?;
        }
    }

    Ok(())
}

/// Resolve the live MapTile3D via the Door chain (bridge-wrapped raw reads).
fn find_maptile3d(
    asm: &std::sync::Arc<il2cpp_bridge_rs::structs::Assembly>,
) -> Result<il2cpp_bridge_rs::structs::Object, String> {
    let hs_class = asm
        .class("HeroStage_Suggestion")
        .ok_or("HeroStage_Suggestion class not found")?;
    let get_door = hs_class
        .method("get_door")
        .ok_or("HeroStage_Suggestion.get_door not found")?;

    let door_ptr: *mut std::ffi::c_void = unsafe { get_door.call(&[])? };
    if door_ptr.is_null() {
        return Err("game Door not ready (log in first)".into());
    }
    let door = unsafe { il2cpp_bridge_rs::structs::Object::from_ptr(door_ptr) };

    // Door.TileMapController (+0xE60) — raw pointer read of a reference field.
    let tile_ptr = unsafe {
        (door.as_ptr() as *mut u8)
            .add(DOOR_TILEMAPCONTROLLER_OFFSET)
            .cast::<*mut std::ffi::c_void>()
            .read()
    };
    if tile_ptr.is_null() {
        return Err("no live MapTile (is the kingdom map visible?)".into());
    }

    // MapTile.mapTile3D (+0x1F0).
    let map3d_ptr = unsafe {
        (tile_ptr as *mut u8)
            .add(MAPTILE_MAPTILE3D_OFFSET)
            .cast::<*mut std::ffi::c_void>()
            .read()
    };
    if map3d_ptr.is_null() {
        return Err("no live MapTile3D (is the kingdom map visible?)".into());
    }
    Ok(unsafe { il2cpp_bridge_rs::structs::Object::from_ptr(map3d_ptr) })
}

/// Arbitrary map zoom. `level` in 0.0..=1.0 maps linearly:
///   0.0 → zoomed out (CAM_DIST_FAR 5.2), 1.0 → zoomed in (CAM_DIST_NEAR 2.1).
pub fn set_camera_dist_level(level: f32) -> Result<(), String> {
    if !(0.0..=1.0).contains(&level) || !level.is_finite() {
        return Err("zoom level out of range 0.0..=1.0".into());
    }
    let dist = CAM_DIST_FAR - (CAM_DIST_FAR - CAM_DIST_NEAR) * level;
    set_camera_dist(dist)
}

pub fn set_camera_dist(dist: f32) -> Result<(), String> {
    if !dist.is_finite() || dist < CAM_DIST_NEAR || dist > CAM_DIST_FAR {
        return Err(format!(
            "camera distance {} out of range {}..={}",
            dist, CAM_DIST_NEAR, CAM_DIST_FAR
        ));
    }
    unity_thread::call_or_inline(
        move || do_set_camera_dist(dist),
        std::time::Duration::from_secs(5),
    )?
}

fn do_set_camera_dist(dist: f32) -> Result<(), String> {
    il2cpp::ensure_bridge()?;
    let asm = il2cpp::bridge_csharp();
    let map3d = find_maptile3d(&asm)?;

    let setter = map3d
        .method(("set_CameraDist", 1))
        .ok_or("MapTile3D.set_CameraDist not found")?;
    unsafe {
        setter
            .call::<()>(&[&dist as *const f32 as *mut std::ffi::c_void])
            .map_err(|e| format!("set_CameraDist: {}", e))?;
    }
    // No RefreshCamera call: set_CameraDist handles it internally (RE-verified).
    Ok(())
}

/// True when the game has a text input focused (chat / mail compose / search).
/// EventSystem.current.currentSelectedGameObject -> GetComponent(InputField|TMP_InputField).
pub fn is_typing() -> Result<bool, String> {
    il2cpp::ensure_bridge()?;

    let es = il2cpp_bridge_rs::api::cache::csharp()
        .class("UnityEngine.EventSystems.EventSystem")
        .or_else(|| il2cpp_bridge_rs::api::cache::coremodule()
            .class("UnityEngine.EventSystems.EventSystem"))
        .or_else(|| find_class_in_cache("UnityEngine.EventSystems.EventSystem"))
        .ok_or("EventSystem class not found")?;

    let current = es
        .method("get_current")
        .ok_or("EventSystem.get_current not found")?;
    let es_ptr: *mut std::ffi::c_void = unsafe { current.call(&[])? };
    if es_ptr.is_null() {
        return Ok(false);
    }
    let es_obj = unsafe { il2cpp_bridge_rs::structs::Object::from_ptr(es_ptr) };
    let get_sel = es_obj
        .method("get_currentSelectedGameObject")
        .ok_or("get_currentSelectedGameObject not found")?;
    let sel_ptr: *mut std::ffi::c_void = unsafe { get_sel.call(&[])? };
    if sel_ptr.is_null() {
        return Ok(false); // nothing focused -> not typing
    }
    let sel = unsafe { il2cpp_bridge_rs::structs::Object::from_ptr(sel_ptr) };

    let get_comp = match sel.method(("GetComponent", 1)) {
        Some(m) => m,
        None => return Ok(false),
    };

    for type_name in ["UnityEngine.UI.InputField", "TMPro.TMP_InputField"] {
        if let Some(field_class) = find_class_in_cache(type_name) {
            let type_obj = field_class.object;
            if type_obj.is_null() {
                continue;
            }
            let comp: *mut std::ffi::c_void = unsafe {
                get_comp.call(&[type_obj])?
            };
            if !comp.is_null() {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Look up a class in the global cache by full name across all assemblies.
fn find_class_in_cache(full_name: &str) -> Option<il2cpp_bridge_rs::structs::Class> {
    il2cpp_bridge_rs::api::cache::CACHE
        .classes
        .get(full_name)
        .map(|c| (**c).clone())
}
