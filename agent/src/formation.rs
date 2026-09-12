//! Formation switching — replicates the native UI's exact network call.
//!
//! RE reference (Aug-2026 IL2CPP dump, ARM64 libil2cpp.so RVAs; logic identical
//! on PC, resolved here by name instead of address):
//!   UIFormationSelect.OnButtonClick      — stores CoordBtn index (0..5) into
//!                                          static UIFormationSelect.NowArmyCoordIndex
//!   UIFormationSelect.ExeConfirmButtonEvent(status==0) —
//!       MessagePacket..ctor(1024) → MP.Protocol = 0x1A91 (6801
//!       _MSG_REQUEST_COORD_CHANGE) → MP.Add(byte NowArmyCoordIndex) →
//!       MP.Send(false)
//!   Server ack _MSG_RESP_COORD_CHANGE (6802) → UIFormationSelect.RecvFormation
//!       writes DataManager.Instance.RoleAttr.NowArmyCoordIndex (+0x61C).
//!
//! We reproduce (b) directly: build the same MessagePacket with the chosen
//! formation index and send it on the connected socket. No UI window required.

use crate::il2cpp::{self, boxed};
use crate::agent_ipc::FORMATION_COUNT;

/// Formation indices as shown by the six CoordBtn slots:
/// 0 Inf Phal, 1 Range Phal, 2 Cav Phal, 3 Inf Wedge, 4 Range Wedge, 5 Cav Wedge.
#[allow(dead_code)]
pub const FORMATION_NAMES: [&str; FORMATION_COUNT] = [
    "Infantry Phalanx",
    "Ranged Phalanx",
    "Cavalry Phalanx",
    "Infantry Wedge",
    "Ranged Wedge",
    "Cavalry Wedge",
];

pub fn set_formation(index: u8) -> Result<(), String> {
    if index as usize >= FORMATION_COUNT {
        return Err(format!(
            "formation index {} out of range 0..={}",
            index,
            FORMATION_COUNT - 1
        ));
    }

    // Marshal to Unity main thread when the hook is up; MessagePacket.Send
    // touches the socket state.
    crate::unity_thread::call_or_inline(
        move || do_set_formation(index),
        std::time::Duration::from_secs(5),
    )?
}

fn do_set_formation(index: u8) -> Result<(), String> {
    if !il2cpp::attach_thread() {
        return Err("failed to attach il2cpp thread".into());
    }

    // 1. Set UIFormationSelect.NowArmyCoordIndex static (matches native flow).
    let set_now = il2cpp::find_method("UIFormationSelect", "set_NowArmyCoordIndex", 0)
        .or_else(|| il2cpp::find_method("App", "set_NowArmyCoordIndex", 0));
    let _ = set_now; // static byte is written via the MessagePacket path below.

    // 2. Find MessagePacket class methods.
    let mp_ctor = il2cpp::find_method("MessagePacket", ".ctor", 1)
        .ok_or("MessagePacket..ctor not found")?;
    let mp_add = il2cpp::find_method("MessagePacket", "Add", 1)
        .ok_or("MessagePacket.Add not found")?;
    let mp_send = il2cpp::find_method("MessagePacket", "Send", 1)
        .ok_or("MessagePacket.Send not found")?;

    unsafe {
        // new MessagePacket(maxSize=1024)
        let max_size = boxed::i32_box(1024);
        let mut args: [*mut core::ffi::c_void; 1] = [max_size];
        // Allocate object: il2cpp_object_new equivalent via ctor invoke with null obj
        // is not possible; instead use il2cpp runtime's object allocation through
        // the class. We resolve the class pointer first.
        let obj = new_message_packet()?;
        if obj.is_null() {
            return Err("failed to allocate MessagePacket".into());
        }
        il2cpp::invoke(mp_ctor, obj, args.as_mut_ptr())
            .map_err(|e| format!("MessagePacket ctor: {}", e))?;

        // MP.Protocol = 6801 — the game writes the ushort at MP+0x30; we use the
        // setter if present, else direct field write.
        if let Some(set_proto) = il2cpp::find_method("MessagePacket", "set_Protocol", 1) {
            let proto = boxed::i32_box(6801);
            let mut pargs: [*mut core::ffi::c_void; 1] = [proto];
            il2cpp::invoke(set_proto, obj, pargs.as_mut_ptr())
                .map_err(|e| format!("set_Protocol: {}", e))?;
        } else {
            // Fallback: direct field write at the documented offset 0x30.
            let proto_ptr = (obj as *mut u8).add(0x30) as *mut u16;
            std::ptr::write_volatile(proto_ptr, 6801u16);
        }

        // MP.Add((byte)formationIndex)
        let value = boxed::u8_box(index);
        let mut addargs: [*mut core::ffi::c_void; 1] = [value];
        il2cpp::invoke(mp_add, obj, addargs.as_mut_ptr())
            .map_err(|e| format!("MessagePacket.Add: {}", e))?;

        // MP.Send(false)
        let flag = boxed::u8_box(0);
        let mut sendargs: [*mut core::ffi::c_void; 1] = [flag];
        il2cpp::invoke(mp_send, obj, sendargs.as_mut_ptr())
            .map_err(|e| format!("MessagePacket.Send: {}", e))?;
    }

    Ok(())
}

unsafe fn new_message_packet() -> Result<*mut core::ffi::c_void, String> {
    // il2cpp_object_new is exported by GameAssembly but not redeclared above;
    // declare it here to keep the binding surface minimal.
    extern "C" {
        fn il2cpp_object_new(klass: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    }
    // Resolve the class pointer by finding a method owner: use
    // il2cpp_class_from_name on the same image that has MessagePacket.
    // Simplification: find_method already walks images; find a method to get
    // its class is not exposed, so instead resolve the class directly per image.
    let domain = il2cpp_domain_get_pub();
    if domain.is_null() {
        return Err("il2cpp domain null".into());
    }
    let mut size: usize = 0;
    let assemblies = il2cpp_domain_get_assemblies_pub(domain, &mut size);
    if assemblies.is_null() {
        return Err("no assemblies".into());
    }
    let cname = std::ffi::CString::new("MessagePacket").map_err(|_| "cname")?;
    for i in 0..size {
        let assembly = *assemblies.add(i);
        if assembly.is_null() {
            continue;
        }
        let image = il2cpp_assembly_get_image_pub(assembly);
        if image.is_null() {
            continue;
        }
        let klass = il2cpp_class_from_name_pub(image, std::ptr::null(), cname.as_ptr());
        if !klass.is_null() {
            let obj = il2cpp_object_new(klass);
            if !obj.is_null() {
                return Ok(obj);
            }
        }
    }
    Err("MessagePacket class not found in any image".into())
}

// Thin re-exports of the raw FFI to avoid circular visibility issues.
fn il2cpp_domain_get_pub() -> crate::il2cpp::DomainPtr {
    unsafe { il2cpp_domain_get_raw() }
}
fn il2cpp_domain_get_assemblies_pub(
    domain: crate::il2cpp::DomainPtr,
    size: *mut usize,
) -> *mut crate::il2cpp::AssemblyPtr {
    unsafe { il2cpp_domain_get_assemblies_raw(domain, size) }
}
fn il2cpp_assembly_get_image_pub(
    assembly: crate::il2cpp::AssemblyPtr,
) -> crate::il2cpp::ImagePtr {
    unsafe { il2cpp_assembly_get_image_raw(assembly) }
}
fn il2cpp_class_from_name_pub(
    image: crate::il2cpp::ImagePtr,
    ns: *const i8,
    name: *const i8,
) -> crate::il2cpp::ClassPtr {
    unsafe { il2cpp_class_from_name_raw(image, ns, name) }
}

extern "C" {
    #[link_name = "il2cpp_domain_get"]
    fn il2cpp_domain_get_raw() -> crate::il2cpp::DomainPtr;
    #[link_name = "il2cpp_domain_get_assemblies"]
    fn il2cpp_domain_get_assemblies_raw(
        domain: crate::il2cpp::DomainPtr,
        size: *mut usize,
    ) -> *mut crate::il2cpp::AssemblyPtr;
    #[link_name = "il2cpp_assembly_get_image"]
    fn il2cpp_assembly_get_image_raw(assembly: crate::il2cpp::AssemblyPtr) -> crate::il2cpp::ImagePtr;
    #[link_name = "il2cpp_class_from_name"]
    fn il2cpp_class_from_name_raw(
        image: crate::il2cpp::ImagePtr,
        ns: *const i8,
        name: *const i8,
    ) -> crate::il2cpp::ClassPtr;
}

/// Deliberate pre-switch latency (ms) — see switch_account_restart.
const SWITCH_LATENCY_MS: u64 = 1500;

/// Account switch: triggers the native UISwitchAccount path.
/// RE reference: UISwitchAccount.OnButtonClick → AccountManager.SwitchAccountRestart
/// → ContinuousConfirmation.SwitchAccountRestart → SteamIGGSDKPlugin.SDK_SwitchLogin.
/// The game tears the network session down and re-runs the LoginPhase state
/// machine in-process; the OS process does NOT restart.
pub fn switch_account_restart() -> Result<(), String> {
    // Human-like latency before the switch (server-side detection watches for
    // machine-speed switching; a deliberate pause keeps us under that pattern).
    std::thread::sleep(std::time::Duration::from_millis(SWITCH_LATENCY_MS));
    crate::unity_thread::call_or_inline(
        do_switch_account,
        std::time::Duration::from_secs(10),
    )?
}

fn do_switch_account() -> Result<(), String> {
    if !il2cpp::attach_thread() {
        return Err("failed to attach il2cpp thread".into());
    }
    // Preferred: invoke AccountManager.SwitchAccountRestart() on the singleton.
    let get_instance = il2cpp::find_method("AccountManager", "get_Instance", 0)
        .ok_or("AccountManager.get_Instance not found")?;
    let switch = il2cpp::find_method("AccountManager", "SwitchAccountRestart", 0)
        .ok_or("AccountManager.SwitchAccountRestart not found")?;

    unsafe {
        let instance = il2cpp::invoke(get_instance, std::ptr::null_mut(), std::ptr::null_mut())
            .map_err(|e| format!("AccountManager.get_Instance: {}", e))?;
        il2cpp::invoke(switch, instance, std::ptr::null_mut())
            .map_err(|e| format!("SwitchAccountRestart: {}", e))?;
    }
    Ok(())
}
