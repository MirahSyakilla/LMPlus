//! Kingdom Map 3D View (Full/Balanced/None) + Map Zoom — raw invoker edition.
//!
//! RE (PC, verified live):
//!   DataManager.get_Instance() static → instance; SetShowMapType(byte) persists the
//!   3D-view mode (SysSetting.mShowMapType).
//!   MapTile3D.set_FovLevel(byte): 0=Full, 1=Balanced, 2=None (applies live when the
//!   map is open). set_CameraDist(f32): ortho camera distance, self-contained refresh;
//!   valid 2.1 (near) … 5.2 (far), default 4.2. No RefreshCamera call needed.

use crate::raw;
use crate::unity_thread;

pub const MODE_NAMES: [&str; 3] = ["Full", "Balanced", "None"];
pub const CAM_DIST_NEAR: f32 = 2.1;
pub const CAM_DIST_FAR: f32 = 5.2;

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
    raw::attach_thread()?;
    crate::alog::info(&format!("map3dview: begin mode={}", mode));

    let dm = raw::find_class("DataManager").ok_or("DataManager class not found")?;
    let get_instance = raw::find_method(dm, "get_Instance", 0)
        .ok_or("DataManager.get_Instance not found")?;
    let inst: *mut std::ffi::c_void = unsafe { raw::call_static0(get_instance) };
    if inst.is_null() {
        return Err("DataManager instance null".into());
    }
    let setter = raw::find_method(dm, "SetShowMapType", 1)
        .ok_or("DataManager.SetShowMapType not found")?;
    unsafe { raw::call_instance1_u8(setter, inst, mode) };
    crate::alog::info("map3dview: persisted via SetShowMapType");

    // Live apply when the map is open (setter consumes instance _CameraFovLevel).
    let map3d_cls = raw::find_class("MapTile3D");
    if let Some(cls) = map3d_cls {
        if let Some(live) = raw::find_object_of_type(cls) {
            if let Some(set_fov) = raw::find_method(cls, "set_FovLevel", 1) {
                unsafe { raw::call_instance1_u8(set_fov, live, mode) };
                crate::alog::info("map3dview: applied to live MapTile3D");
            }
        } else {
            crate::alog::info("map3dview: no live MapTile3D (off-map) — applied on next map load");
        }
    }
    Ok(())
}

/// `level` 0.0..=1.0: 0 = zoomed out (5.2), 1 = zoomed in (2.1).
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
    raw::attach_thread()?;
    let cls = raw::find_class("MapTile3D").ok_or("MapTile3D class not found")?;
    let live = raw::find_object_of_type(cls)
        .ok_or("no live MapTile3D (is the kingdom map visible?)")?;
    let setter = raw::find_method(cls, "set_CameraDist", 1)
        .ok_or("MapTile3D.set_CameraDist not found")?;
    unsafe { raw::call_instance1_f32(setter, live, dist) };
    crate::alog::info(&format!("zoom: set_CameraDist({})", dist));
    Ok(())
}
