//! Formation switching — replicates the native UI's exact network call.
//!
//! RE reference (Aug-2026 IL2CPP dump):
//!   UIFormationSelect.ExeConfirmButtonEvent(status==0):
//!     MessagePacket..ctor(1024) → MP.Protocol = 0x1A91 (6801
//!     _MSG_REQUEST_COORD_CHANGE) → MP.Add(byte NowArmyCoordIndex) →
//!     MP.Send(false)
//!   Server ack 6802 → RecvFormation → DataManager.Instance.RoleAttr.
//!     NowArmyCoordIndex (+0x61C) updated.
//!
//! We build and send the same MessagePacket with the chosen formation index.
//! No UI window required. All calls run through il2cpp-bridge-rs with proper
//! thread attachment and managed-exception conversion.

use crate::il2cpp;
use crate::unity_thread;

/// Formation indices as shown by the six CoordBtn slots:
/// 0 Inf Phal, 1 Range Phal, 2 Cav Phal, 3 Inf Wedge, 4 Range Wedge, 5 Cav Wedge.
#[allow(dead_code)]
pub const FORMATION_NAMES: [&str; 6] = [
    "Infantry Phalanx",
    "Ranged Phalanx",
    "Cavalry Phalanx",
    "Infantry Wedge",
    "Ranged Wedge",
    "Cavalry Wedge",
];

/// Deliberate pre-switch latency (ms) — see switch_account_restart.
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
        std::time::Duration::from_secs(5),
    )?
}

fn do_set_formation(index: u8) -> Result<(), String> {
    il2cpp::ensure_bridge()?;

    let asm = il2cpp::bridge_csharp();

    let mp_class = asm
        .class("MessagePacket")
        .ok_or("MessagePacket class not found")?;
    let obj = mp_class.new_object().map_err(|e| format!("alloc MessagePacket: {}", e))?;
    if obj.ptr.is_null() {
        return Err("failed to allocate MessagePacket".into());
    }

    let max_size: i32 = 1024;
    let ctor_bound = obj.method((".ctor", 1)).ok_or("bound .ctor")?;
    unsafe {
        ctor_bound
            .call::<()>(&[&max_size as *const i32 as *mut c_void])
            .map_err(|e| format!("MessagePacket ctor: {}", e))?;
    }

    // MP.Protocol = 6801 — prefer the setter; fallback to direct field write.
    let proto: i32 = 6801;
    if let Some(set_proto) = obj.method(("set_Protocol", 1)) {
        unsafe {
            set_proto
                .call::<()>(&[&proto as *const i32 as *mut c_void])
                .map_err(|e| format!("set_Protocol: {}", e))?;
        }
    } else if let Some(field) = obj.field("Protocol") {
        unsafe {
            field.set_value(6801u16).map_err(|e| format!("Protocol field: {}", e))?;
        }
    } else {
        return Err("no way to set MessagePacket.Protocol".into());
    }

    // MP.Add((byte)index)
    let value: u8 = index;
    let add_bound = obj.method(("Add", 1)).ok_or("bound Add")?;
    unsafe {
        add_bound
            .call::<()>(&[&value as *const u8 as *mut c_void])
            .map_err(|e| format!("MessagePacket.Add: {}", e))?;
    }

    // MP.Send(false)
    let flag: bool = false;
    let send_bound = obj.method(("Send", 1)).ok_or("bound Send")?;
    unsafe {
        send_bound
            .call::<()>(&[&flag as *const bool as *mut c_void])
            .map_err(|e| format!("MessagePacket.Send: {}", e))?;
    }

    Ok(())
}

use std::ffi::c_void;

/// Account switch: triggers the native UISwitchAccount path.
/// RE: UISwitchAccount.OnButtonClick → AccountManager.SwitchAccountRestart →
/// SteamIGGSDKPlugin.SDK_SwitchLogin → in-process LoginPhase re-entry.
/// The OS process does NOT restart.
pub fn switch_account_restart() -> Result<(), String> {
    // Human-like latency before the switch (server-side detection watches for
    // machine-speed switching; a deliberate pause keeps us under that pattern).
    std::thread::sleep(std::time::Duration::from_millis(SWITCH_LATENCY_MS));
    unity_thread::call_or_inline(
        do_switch_account,
        std::time::Duration::from_secs(10),
    )?
}

fn do_switch_account() -> Result<(), String> {
    il2cpp::ensure_bridge()?;

    let asm = il2cpp::bridge_csharp();
    let am_class = asm
        .class("AccountManager")
        .ok_or("AccountManager class not found")?;
    let get_instance = am_class
        .method("get_Instance")
        .ok_or("AccountManager.get_Instance not found")?;
    let mut switch = am_class
        .method("SwitchAccountRestart")
        .ok_or("AccountManager.SwitchAccountRestart not found")?;

    let instance_ptr: *mut c_void = unsafe { get_instance.call(&[])? };
    if instance_ptr.is_null() {
        return Err("AccountManager instance is null (log in first?)".into());
    }
    switch.instance = Some(instance_ptr);
    unsafe {
        switch.call::<()>(&[]).map_err(|e| format!("SwitchAccountRestart: {}", e))?;
    }
    Ok(())
}
