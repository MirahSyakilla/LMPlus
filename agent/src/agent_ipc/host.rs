//! Non-Windows stub: only used so `cargo test` runs on the Linux dev host.
//! The real pipe server lives in `win.rs` and is compiled only on Windows.

pub use super::types::*;

pub const PIPE_NAME: &str = "\\\\.\\pipe\\lmp-agent";

pub struct AgentClient;

impl AgentClient {
    pub fn send(_line: &str) -> Result<String, String> {
        Err("agent IPC is Windows-only".into())
    }
}

pub struct PipeStream;

pub fn ipc_server_main(
    _handler: fn(ActionRequest) -> ActionResponse,
    _shutdown: &std::sync::atomic::AtomicBool,
) {
    // Never runs on the host; the agent DLL is Windows-only.
}
