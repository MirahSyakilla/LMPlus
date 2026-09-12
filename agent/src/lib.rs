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
mod formation;
#[cfg(windows)]
mod mapview;
#[cfg(windows)]
mod il2cpp;
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
            // Start the IPC server immediately so the host can verify injection
            // with a ping right away. il2cpp readiness + main-thread hook are
            // established lazily on the first real action (see handle_request),
            // and each action also retries the hook install.
            std::thread::spawn(|| {
                ipc_server_main(handle_request, &SHUTDOWN);
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
    // Lazily bring up the bridge + the main-thread pump on first contact.
    if !matches!(req, ActionRequest::Ping) {
        if !UNITY_READY.load(Ordering::SeqCst) {
            if unity_thread::install() {
                UNITY_READY.store(true, Ordering::SeqCst);
            }
        }
    }
    match req {
        ActionRequest::Ping => ActionResponse::Ok("pong".into()),
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

// Re-exports used by unit tests (host-side logic only).
pub use agent_ipc::{parse_request, serialize_response};
