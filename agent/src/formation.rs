//! Formation switching via the game's own UI handler — frame-split native flow.
//!
//! Verified live (frida + user visual confirmation): the switch works when the
//! steps are spaced across frames:
//!   frame 0:  close existing window + QLink.DoorOpenMenu(UI_FormationSelect=129)
//!   frame 60: OnButtonClick(real CoordBtn[idx])  — selects slot
//!   frame 120: OnButtonClick(real ConfirmBtn) if its status == Setup(0)
//!   frame 180: OnCloseBtnClick if still open
//! All clicks use the game's REAL button objects (no fakes). The per-frame
//! stepping is driven by the unity_thread frame runner; ~1.2s gaps mimic a
//! human and avoid the server's "too fast" rejection.

use crate::il2cpp_compat as raw;
use crate::unity_thread;

#[allow(dead_code)]
pub const FORMATION_NAMES: [&str; 6] = [
    "Infantry Phalanx",
    "Ranged Phalanx",
    "Cavalry Phalanx",
    "Infantry Wedge",
    "Ranged Wedge",
    "Cavalry Wedge",
];

const GAP_FRAMES: i32 = 90; // ~1.2s at 75fps

/// Deliberate pre-switch latency (ms) — server-side fast-switch detection.
const SWITCH_LATENCY_MS: u64 = 1500;

pub fn set_formation(index: u8) -> Result<(), String> {
    if index as usize >= FORMATION_NAMES.len() {
        return Err(format!(
            "formation index {} out of range 0..={}",
            index,
            FORMATION_NAMES.len() - 1
        ));
    }
    std::thread::sleep(std::time::Duration::from_millis(SWITCH_LATENCY_MS));

    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let result = unity_thread::call_or_inline(
        move || {
            // Start the frame-stepped machine; completion reported via `tx`.
            crate::unity_thread::set_frame_runner(Box::new(move || {
                static STEP: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
                static FRAMES_LEFT: std::sync::atomic::AtomicI32 =
                    std::sync::atomic::AtomicI32::new(0);
                static IDX: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
                let step = STEP.load(std::sync::atomic::Ordering::SeqCst);
                if step == 0 {
                    IDX.store(index, std::sync::atomic::Ordering::SeqCst);
                    STEP.store(1, std::sync::atomic::Ordering::SeqCst);
                    FRAMES_LEFT.store(1, std::sync::atomic::Ordering::SeqCst);
                    let _ = tx.send(Ok(())); // machine started; final result via pump
                    return true;
                }
                let idx = IDX.load(std::sync::atomic::Ordering::SeqCst);
                if FRAMES_LEFT.load(std::sync::atomic::Ordering::SeqCst) > 0 {
                    FRAMES_LEFT.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                    return true;
                }
                match step {
                    1 => match step_close_and_open() {
                        Ok(()) => {
                            STEP.store(2, std::sync::atomic::Ordering::SeqCst);
                            FRAMES_LEFT.store(GAP_FRAMES, std::sync::atomic::Ordering::SeqCst);
                        }
                        Err(e) => {
                            let _ = tx.send(Err(format!("open: {}", e)));
                            return false;
                        }
                    },
                    2 => match step_coord_click(idx) {
                        Ok(()) => {
                            STEP.store(3, std::sync::atomic::Ordering::SeqCst);
                            FRAMES_LEFT.store(GAP_FRAMES, std::sync::atomic::Ordering::SeqCst);
                        }
                        Err(e) => {
                            let _ = tx.send(Err(format!("coord: {}", e)));
                            return false;
                        }
                    },
                    3 => match step_confirm_click() {
                        Ok(()) => {
                            STEP.store(4, std::sync::atomic::Ordering::SeqCst);
                            FRAMES_LEFT.store(GAP_FRAMES, std::sync::atomic::Ordering::SeqCst);
                        }
                        Err(e) => {
                            let _ = tx.send(Err(format!("confirm: {}", e)));
                            return false;
                        }
                    },
                    4 => {
                        step_close_if_open();
                        let _ = tx.send(Ok(()));
                        crate::alog::info("formation: flow complete");
                        return false;
                    }
                    _ => return false,
                }
                true
            }));
            ()
        },
        std::time::Duration::from_secs(5),
    );
    // The started-ack is meaningless; wait for the real result.
    let final_result = rx
        .recv_timeout(std::time::Duration::from_secs(90))
        .map_err(|_| "formation flow timed out".to_string());
    let _ = result;
    final_result?
}

fn step_close_and_open() -> Result<(), String> {
    raw::attach_thread()?;
    if let Some(inst) = find_ui_instance() {
        if let Some(close) = raw::find_method(
            raw::find_class("UIFormationSelect").ok_or("no UIS class")?,
            "OnCloseBtnClick",
            0,
        ) {
            unsafe { raw::call_instance0_ignore(close, inst) };
        }
        crate::alog::info("formation: existing window closed");
    }
    let qlink = raw::find_class("QLink").ok_or("QLink class not found")?;
    let menu = raw::find_method(qlink, "DoorOpenMenu", 5).ok_or("DoorOpenMenu not found")?;
    unsafe { raw::call_static5_void(menu, 129, 0, 0, 0, 0) };
    crate::alog::info("formation: window opened");
    Ok(())
}

fn step_coord_click(idx: u8) -> Result<(), String> {
    let inst = find_ui_instance().ok_or("formation window not found")?;
    let uis = raw::find_class("UIFormationSelect").ok_or("no UIS class")?;
    let ob = raw::find_method(uis, "OnButtonClick", 1).ok_or("OnButtonClick not found")?;
    // CoordBtn array at +0x58 (il2cpp array: items at +0x20)
    let arr = unsafe { (inst as *mut u8).add(0x58).cast::<*mut c_void>().read() };
    if arr.is_null() {
        return Err("CoordBtn array null".into());
    }
    let btn = unsafe { (arr as *mut u8).add(0x20 + (idx as usize) * 8).cast::<*mut c_void>().read() };
    if btn.is_null() {
        return Err("CoordBtn null".into());
    }
    unsafe { raw::call_instance1_ptr(ob, inst, btn) };
    crate::alog::info(&format!("formation: coord {} clicked", idx));
    Ok(())
}

fn step_confirm_click() -> Result<(), String> {
    let inst = find_ui_instance().ok_or("formation window not found")?;
    let uis = raw::find_class("UIFormationSelect").ok_or("no UIS class")?;
    let ob = raw::find_method(uis, "OnButtonClick", 1).ok_or("OnButtonClick not found")?;
    let confirm_btn =
        unsafe { (inst as *mut u8).add(0x98).cast::<*mut c_void>().read() };
    if confirm_btn.is_null() {
        return Err("ConfirmBtn null".into());
    }
    let status = unsafe { (confirm_btn as *mut u8).add(0x10C).cast::<i32>().read() };
    crate::alog::info(&format!("formation: confirm status={}", status));
    if status != 0 {
        // Not in "Setup" state — either already in use or game refreshed it away.
        return Err(format!("confirm status={} (not applyable)", status));
    }
    unsafe { raw::call_instance1_ptr(ob, inst, confirm_btn) };
    crate::alog::info("formation: apply clicked");
    Ok(())
}

fn step_close_if_open() {
    if let Some(inst) = find_ui_instance() {
        if let Some(uis) = raw::find_class("UIFormationSelect") {
            if let Some(close) = raw::find_method(uis, "OnCloseBtnClick", 0) {
                unsafe { raw::call_instance0_ignore(close, inst) };
            }
        }
    }
}

fn find_ui_instance() -> Option<*mut c_void> {
    let uis = raw::find_class("UIFormationSelect")?;
    let gm = raw::find_class("GUIManager")?;
    let get_inst = raw::find_method(gm, "get_Instance", 0)?;
    let gm_inst: *mut c_void = unsafe { raw::call_static0(get_inst) };
    if gm_inst.is_null() {
        return None;
    }
    for off in [0x3A8usize, 0x3B0, 0x3B8, 0x3C0, 0x3E0] {
        let w = unsafe { (gm_inst as *mut u8).add(off).cast::<*mut c_void>().read() };
        if !w.is_null() {
            // Compare klass name: *(void**)w = klass; il2cpp_class_get_name(klass)
            let wk = unsafe { (w as *mut *mut c_void).read() };
            if wk.is_null() { continue; }
            if let Some(name) = raw::class_name_of(wk) {
                if name == "UIFormationSelect" {
                    return Some(w);
                }
            }
        }
    }
    None
}

use std::ffi::c_void;

/// Account switch: triggers the native UISwitchAccount path (in-process).
pub fn switch_account_restart() -> Result<(), String> {
    std::thread::sleep(std::time::Duration::from_millis(SWITCH_LATENCY_MS));
    unity_thread::call_or_inline(
        do_switch_account,
        std::time::Duration::from_secs(10),
    )?
}

fn do_switch_account() -> Result<(), String> {
    raw::attach_thread()?;
    crate::alog::info("switch_account: begin");

    let asm = raw::find_class("AccountManager").ok_or("AccountManager class not found")?;
    let get_instance = raw::find_method(asm, "get_Instance", 0)
        .ok_or("AccountManager.get_Instance not found")?;
    let mut switch = raw::find_method(asm, "SwitchAccountRestart", 0)
        .ok_or("AccountManager.SwitchAccountRestart not found")?;

    let instance: *mut c_void = unsafe { raw::call_static0(get_instance) };
    if instance.is_null() {
        return Err("AccountManager instance is null (log in first?)".into());
    }
    unsafe {
        raw::call_instance0_ignore(switch, instance);
    }
    crate::alog::info("switch_account: SwitchAccountRestart called");
    Ok(())
}
