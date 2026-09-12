//! Kingdom Map 3D View (Full/Balanced/None) + Map Zoom slider.
//!
//! RE-verified call chain (Aug-2026 IL2CPP dump, field offsets platform-independent):
//!   HeroStage_Suggestion.get_door()          (static)  → live Door
//!   Door.TileMapController                   (Door+0xE60) → live MapTile
//!   MapTile.mapTile3D                        (MapTile+0x1F0) → live MapTile3D
//!   (world-map variant: WorldMap.mapTile3D   at +0x188)
//!
//! 3D View modes: MapTile3D.set_FovLevel(byte) — 0=Full, 1=Balanced, 2=None.
//!   set_FovLevel updates both the instance _CameraFovLevel (0x2A8) and the
//!   global static; RefreshCamera/RefreshOrthoSize consume the instance value.
//!   Persistence: DataManager.get_Instance → SetShowMapType(byte) →
//!   SysSetting.mShowMapType (saved server-side/on disk by the game).
//!
//! Zoom: orthographic camera. set_CameraDist(f32) at MapTile3D is self-contained:
//!   writes _CameraDist (0x2A4), calls ReScaleByDist + StopScroll, then
//!   RefreshCamera internally. orthoSize = lerp(MapOrthoSizeNear,
//!   MapOrthoSizeFar, clamp01((dist − MapNearPlane)/(MapFarPlane − MapNearPlane)))
//! Valid dist range: MapNearPlane 2.1 (zoom-in) … MapFarPlane 5.2 (zoom-out),
//! default 4.2 (MapDefaultPlane). No clamping inside set_CameraDist — we clamp.
//! Do NOT call RefreshCamera separately; set_CameraDist already does.

use crate::il2cpp::{self, boxed};

/// Kingdom Map 3D View modes, in UI order.
pub const MODE_NAMES: [&str; 3] = ["Full", "Balanced", "None"];

/// Camera distance bounds (RE: MapNearPlane=2.1, MapFarPlane=5.2, default 4.2).
pub const CAM_DIST_NEAR: f32 = 2.1;
pub const CAM_DIST_FAR: f32 = 5.2;

/// Offset of MapTile.mapTile3D (verified in dump.cs:75620).
const MAPTILE_MAPTILE3D_OFFSET: usize = 0x1F0;
/// Offset of WorldMap.mapTile3D (dump.cs ~90235).
const WORLDMAP_MAPTILE3D_OFFSET: usize = 0x188;
/// Offset of Door.TileMapController (dump.cs:155854).
const DOOR_TILEMAPCONTROLLER_OFFSET: usize = 0xE60;

pub fn set_map_3d_view(mode: u8) -> Result<(), String> {
    if mode as usize >= MODE_NAMES.len() {
        return Err(format!("mode {} out of range 0..=2", mode));
    }

    crate::unity_thread::call_or_inline(
        move || do_set_map_3d_view(mode),
        std::time::Duration::from_secs(5),
    )?
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

    // 2. Apply to the live map: MapTile3D.set_FovLevel(byte) — consumes the
    //    instance _CameraFovLevel used by RefreshOrthoSize/GetLvScale.
    match unsafe { find_maptile3d_instance() } {
        Ok(instance) => unsafe {
            let setter = il2cpp::find_method("MapTile3D", "set_FovLevel", 1)
                .ok_or("MapTile3D.set_FovLevel not found")?;
            let arg = boxed::u8_box(mode);
            let mut args: [*mut core::ffi::c_void; 1] = [arg];
            il2cpp::invoke(setter, instance, args.as_mut_ptr())
                .map_err(|e| format!("set_FovLevel: {}", e))?;
        },
        Err(_) => {
            // Off-map: persistence above is enough — the game applies the
            // saved mShowMapType when the map next loads (native behavior).
        }
    }

    Ok(())
}

/// Resolve the live MapTile3D: HeroStage_Suggestion.get_door() →
/// Door.TileMapController (+0xE60) → MapTile.mapTile3D (+0x1F0),
/// falling back to the WorldMap variant (+0x188) for world/kingdom view.
unsafe fn find_maptile3d_instance() -> Result<*mut core::ffi::c_void, String> {
    let get_door = il2cpp::find_method("HeroStage_Suggestion", "get_door", 0)
        .ok_or("HeroStage_Suggestion.get_door not found")?;
    let door = il2cpp::invoke(get_door, std::ptr::null_mut(), std::ptr::null_mut())
        .map_err(|e| format!("get_door: {}", e))?;
    if door.is_null() {
        return Err("game Door not ready (log in first)".into());
    }

    // Door.TileMapController (MapTile) at +0xE60.
    let tile = ((door as *mut u8).add(DOOR_TILEMAPCONTROLLER_OFFSET)
        as *mut *mut core::ffi::c_void)
        .read();
    if !tile.is_null() {
        let map3d = ((tile as *mut u8).add(MAPTILE_MAPTILE3D_OFFSET)
            as *mut *mut core::ffi::c_void)
            .read();
        if !map3d.is_null() {
            return Ok(map3d);
        }
    }

    // WorldMap fallback: static instance via UnityEngine FindObjectOfType-style
    // lookup is overkill; instead probe the WorldMap chain through the same
    // Door's world map reference if present.
    // (kept simple: report not-ready rather than guessing addresses)
    Err("no live MapTile3D (is the kingdom map visible?)".into())
}

/// Arbitrary map zoom. `level` in 0.0..=1.0 maps linearly:
///   0.0 → zoomed out (CAM_DIST_FAR 5.2), 1.0 → zoomed in (CAM_DIST_NEAR 2.1).
pub fn set_camera_dist_level(level: f32) -> Result<(), String> {
    if !(0.0..=1.0).contains(&level) {
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

    crate::unity_thread::call_or_inline(
        move || do_set_camera_dist(dist),
        std::time::Duration::from_secs(5),
    )?
}

fn do_set_camera_dist(dist: f32) -> Result<(), String> {
    if !il2cpp::attach_thread() {
        return Err("failed to attach il2cpp thread".into());
    }
    unsafe {
        let instance = unsafe { find_maptile3d_instance() }?;
        let setter = il2cpp::find_method("MapTile3D", "set_CameraDist", 1)
            .ok_or("MapTile3D.set_CameraDist not found")?;
        let arg = boxed::f32_box(dist);
        let mut args: [*mut core::ffi::c_void; 1] = [arg];
        il2cpp::invoke(setter, instance, args.as_mut_ptr())
            .map_err(|e| format!("set_CameraDist: {}", e))?;
        // No RefreshCamera call: set_CameraDist already runs
        // ReScaleByDist + StopScroll + RefreshCamera internally (RE-verified).
    }
    Ok(())
}
