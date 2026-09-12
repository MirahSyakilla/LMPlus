//! Formation switching via the game's OWN UI handler — zero packet building.
//!
//! Verified live against the PC client (frida autopilot):
//!   1. QLink.DoorOpenMenu(EGUIWindow.UI_FormationSelect=129, 0, 0, false, false)
//!      opens the formation dialog without changing scene.
//!   2. UnityEngine.Object.FindObjectOfType(UIFormationSelect type) → live instance.
//!   3. instance.OnButtonClick(fakeButton{ id = N+1 })   → static NowArmyCoordIndex = N
//!      (fake button: +0x108 = e_UIFSButtonID, +0x10C = confirm status)
//!   4. instance.OnButtonClick(fakeButton{ id = 7 (ConfirmBtn), status = 0 })
//!      → native ExeConfirmButtonEvent builds & sends the REAL packet
//!        (protocol 6801, correct payload incl. sequence) — server-accepted,
//!        connection stays up (stage stayed LP_InGame).
//!   5. instance.OnCloseBtnClick() closes the dialog.
//! All steps run on the Unity main thread via the agent's hook pump.

use crate::il2cpp_compat as raw;
use crate::unity_thread;

/// Formation slots (CoordBtn1..6): 0 Inf Phal, 1 Range Phal, 2 Cav Phal,
/// 3 Inf Wedge, 4 Range Wedge, 5 Cav Wedge.
#[allow(dead_code)]
pub const FORMATION_NAMES: [&str; 6] = [
    "Infantry Phalanx",
    "Ranged Phalanx",
    "Cavalry Phalanx",
    "Infantry Wedge",
    "Ranged Wedge",
    "Cavalry Wedge",
];

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
    unity_thread::call_or_inline(
        move || do_set_formation(index),
        std::time::Duration::from_secs(15),
    )?
}

fn do_set_formation(index: u8) -> Result<(), String> {
    raw::attach_thread()?;
    crate::alog::info(&format!("formation: begin index={}", index));

    // No window, no scene change — replicate the native confirm path directly:
    //   1. static UIFormationSelect.NowArmyCoordIndex = N
    //   2. ExeConfirmButtonEvent(fakeThis, 0) — status 0 builds & sends the real
    //      packet (proto 6801) exactly as the Apply button does. The status==0
    //      branch only reads this+0x110 (OpenUICase) which must be 0 (Normal);
    //      a zeroed fake `this` satisfies it (verified live: server accepts,
    //      connection stays in LP_InGame).
    let uis = raw::find_class("UIFormationSelect")
        .ok_or("UIFormationSelect class not found")?;

    raw::static_write_u8(uis, "NowArmyCoordIndex", index)
        .map_err(|e| format!("static write failed: {}", e))?;
    crate::alog::info("formation: NowArmyCoordIndex set");

    let exe = raw::find_method(uis, "ExeConfirmButtonEvent", 1)
        .ok_or("ExeConfirmButtonEvent not found")?;
    let fake_this = raw::alloc_zeroed(0x400);
    if fake_this.is_null() {
        return Err("alloc fake this failed".into());
    }
    unsafe { raw::call_instance1_i32(exe, fake_this, 0) };
    crate::alog::info("formation: confirm sent via ExeConfirmButtonEvent");

    Ok(())
}

/// Account switch: triggers the native UISwitchAccount path (in-process).
/// RE: UISwitchAccount.OnButtonClick → AccountManager.SwitchAccountRestart →
/// SDK_SwitchLogin → in-process LoginPhase re-entry. No process restart.
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

use std::ffi::c_void;
