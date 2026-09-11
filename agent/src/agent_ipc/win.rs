//! Windows named-pipe server + client for the agent IPC protocol.

pub use super::types::*;
use std::io::{BufRead, BufReader, Write};
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicBool, Ordering};
use winapi::um::fileapi::CreateFileW;
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::shared::ntdef::HANDLE;
use winapi::um::namedpipeapi::CreateNamedPipeW;
use winapi::um::namedpipeapi::ConnectNamedPipe;
use winapi::um::winbase::{
    FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_WAIT,
};
use winapi::um::fileapi::OPEN_EXISTING;
use winapi::um::winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_READ, GENERIC_WRITE};

pub const PIPE_NAME: &str = "\\\\.\\pipe\\lmp-agent";

pub(crate) fn to_wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

type RequestHandler = fn(ActionRequest) -> ActionResponse;

/// Blocking pipe server loop; call from a dedicated thread.
pub fn ipc_server_main(handler: RequestHandler, shutdown: &AtomicBool) {
    loop {
        if shutdown.load(Ordering::SeqCst) {
            return;
        }
        unsafe {
            let wide = to_wide(PIPE_NAME);
            let pipe = CreateNamedPipeW(
                wide.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
                1,
                4096,
                4096,
                0,
                std::ptr::null_mut(),
            );
            if pipe == INVALID_HANDLE_VALUE {
                // Pipe already exists (agent restarted inside a game instance
                // that already has one) — idle and retry later.
                std::thread::sleep(std::time::Duration::from_secs(5));
                continue;
            }
            let connected = ConnectNamedPipe(pipe, std::ptr::null_mut()) != 0;
            if !connected {
                CloseHandle(pipe);
                continue;
            }
            serve_connection(pipe, handler);
            CloseHandle(pipe);
        }
    }
}

unsafe fn serve_connection(pipe: HANDLE, handler: RequestHandler) {
    let mut reader = BufReader::new(PipeStream(pipe));
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
        let response = match parse_request(line.trim()) {
            Ok(req) => handler(req),
            Err(e) => ActionResponse::Err(e),
        };
        let mut out = serialize_response(&response).into_bytes();
        out.push(b'\n');
        if PipeStream(pipe).write_all(&out).is_err() {
            return;
        }
    }
}

pub struct PipeStream(pub HANDLE);

impl std::io::Read for PipeStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        unsafe {
            let mut read: u32 = 0;
            if winapi::um::fileapi::ReadFile(
                self.0,
                buf.as_mut_ptr() as _,
                buf.len() as u32,
                &mut read,
                std::ptr::null_mut(),
            ) == 0
            {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(read as usize)
            }
        }
    }
}

impl std::io::Write for PipeStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        unsafe {
            let mut written: u32 = 0;
            if winapi::um::fileapi::WriteFile(
                self.0,
                buf.as_ptr() as _,
                buf.len() as u32,
                &mut written,
                std::ptr::null_mut(),
            ) == 0
            {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(written as usize)
            }
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Host-side client used by tests to talk to an already-injected agent.
#[allow(dead_code)]
pub struct AgentClient;

impl AgentClient {
    #[allow(dead_code)]
    pub fn send(line: &str) -> Result<String, String> {
        unsafe {
            let wide = to_wide(PIPE_NAME);
            let handle = CreateFileW(
                wide.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            );
            if handle == INVALID_HANDLE_VALUE {
                return Err("agent pipe not available (is the agent injected?)".into());
            }
            let mut req = line.as_bytes().to_vec();
            req.push(b'\n');
            PipeStream(handle).write_all(&req).map_err(|e| e.to_string())?;
            let mut reader = BufReader::new(PipeStream(handle));
            let mut resp = String::new();
            reader.read_line(&mut resp).map_err(|e| e.to_string())?;
            CloseHandle(handle);
            Ok(resp.trim_end().to_string())
        }
    }
}
