//! lmp_agent — injected companion DLL for Lords Mobile (IL2CPP).
//!
//! Loaded by LMPlus into the game process. Provides:
//! 1. Named-pipe IPC server (`\\.\pipe\lmp-agent`) that receives LMPlusAction
//!    requests and executes them on the Unity main thread.
//! 2. il2cpp runtime invoker: resolves classes/methods by name through the
//!    game's exported il2cpp_* API (no hardcoded RVAs required).
//! 3. Direct implementations:
//!    - Formation switch: replicates UIFormationSelect.ExeConfirmButtonEvent
//!      status==0 path (MessagePacket protocol 6801 + formation byte).
//!    - Kingdom Map 3D View: MapTile3D::set_GlobalMapFovLevel (0=Full,1=Balanced,2=None)
//!      + DataManager::SetShowMapType for persistence.
//!    - Map zoom: MapTile3D::set_CameraDist + RefreshCamera on a live instance
//!      (fallback: legacy native-camera poke handled by LMPlus itself).
//!    - Account switch: UISwitchAccount button path → AccountManager::SwitchAccountRestart.

use agent_ipc::{ipc_server_main, ActionRequest, ActionResponse};
use std::sync::atomic::{AtomicBool, Ordering};

mod agent_ipc;
#[cfg(windows)]
mod alog;

#[cfg(windows)]
mod formation;
#[cfg(windows)]
mod mapview;
#[cfg(windows)]
#[cfg(windows)]
pub(crate) use raw as il2cpp_compat;
#[cfg(windows)]
pub(crate) mod raw;
#[cfg(windows)]
mod unity_thread;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);
static UNITY_READY: AtomicBool = AtomicBool::new(false);
// il2cpp_* symbols are provided by the host process (GameAssembly.dll) and are
// resolved at load time through native/libgameassembly.a (generated from the
// game's export table).
#[cfg(windows)]
#[link(name = "gameassembly", kind = "static")]
extern "C" {}

#[cfg(windows)]
#[no_mangle]
extern "system" fn DllMain(_hinst: *mut core::ffi::c_void, reason: u32, _reserved: *mut core::ffi::c_void) -> i32 {
    const DLL_PROCESS_ATTACH: u32 = 1;
    const DLL_PROCESS_DETACH: u32 = 0;
    match reason {
        DLL_PROCESS_ATTACH => {
            // Start the IPC server immediately (ping works right away).
            std::thread::spawn(|| {
                alog::info("agent thread up, starting pipe server");
                ipc_server_main(handle_request, &SHUTDOWN);
            });
            // Warmup worker: install the main-thread pump with retries (game
            // window may not exist yet right after injection). Raw il2cpp
            // calls need no metadata hydration.
            std::thread::spawn(|| {
                for attempt in 0..480 {
                    if unity_thread::install() {
                        alog::info(&format!("warmup: unity hook installed (attempt {})", attempt));
                        UNITY_READY.store(true, Ordering::SeqCst);
                        // Keep supervising: if the game recreates its window
                        // (resolution change, relog), reinstall on the new thread.
                        loop {
                            std::thread::sleep(std::time::Duration::from_secs(30));
                            if !unity_thread::hook_alive() {
                                alog::warn("pump: hook died, reinstalling");
                                UNITY_READY.store(false, Ordering::SeqCst);
                                break;
                            }
                        }
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
                alog::error("warmup: unity hook never installed (no visible window)");
            });
            1
        }
        DLL_PROCESS_DETACH => {
            SHUTDOWN.store(true, Ordering::SeqCst);
            1
        }
        _ => 1,
    }
}

#[cfg(windows)]
fn handle_request(req: ActionRequest) -> ActionResponse {
    alog::info(&format!("request: {:?}", req));
    // Real actions wait for the warmup worker (pump hook), bounded.
    if !matches!(req, ActionRequest::Ping | ActionRequest::LogDir(_)) {
        if !UNITY_READY.load(Ordering::SeqCst) && unity_thread::install() {
            alog::info("pump: unity hook installed on demand");
            UNITY_READY.store(true, Ordering::SeqCst);
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(45);
        while !UNITY_READY.load(Ordering::SeqCst) {
            if std::time::Instant::now() > deadline {
                alog::error("action aborted: warmup not finished in time");
                return ActionResponse::Err("agent warming up (bridge/hook not ready); retry".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    let response = run_action(req);
    alog::info(&format!("response: {:?}", response));
    response
}

#[cfg(windows)]
fn run_action(req: ActionRequest) -> ActionResponse {
    match req {
        ActionRequest::Ping => ActionResponse::Ok("pong".into()),
        ActionRequest::LogDir(dir) => {
            alog::set_base_dir(&dir);
            ActionResponse::Ok("logdir set".into())
        }
        ActionRequest::Formation { index } => match formation::set_formation(index) {
            Ok(()) => ActionResponse::Ok(format!("formation {}", index)),
            Err(e) => ActionResponse::Err(e),
        },
        ActionRequest::Map3DView { mode } => match mapview::set_map_3d_view(mode) {
            Ok(()) => ActionResponse::Ok(format!("map3dview {}", mode)),
            Err(e) => ActionResponse::Err(e),
        },
        ActionRequest::MapZoom { value } => match mapview::set_camera_dist_level(value) {
            Ok(()) => ActionResponse::Ok(format!("zoom level {}", value)),
            Err(e) => ActionResponse::Err(e),
        },
        ActionRequest::SwitchAccount => match formation::switch_account_restart() {
            Ok(()) => ActionResponse::Ok("switch_account".into()),
            Err(e) => ActionResponse::Err(e),
        },
    }
}

/// Host calls this right after injection so agent logs land next to lmplus.exe.
#[cfg(windows)]
#[no_mangle]
pub extern "system" fn lmp_set_log_dir(dir: *const i8) {
    if dir.is_null() {
        return;
    }
    let cstr = unsafe { std::ffi::CStr::from_ptr(dir) };
    if let Ok(dir) = cstr.to_str() {
        alog::set_base_dir(dir);
    }
}

// Re-exports used by unit tests (host-side logic only).
pub use agent_ipc::{parse_request, serialize_response};
