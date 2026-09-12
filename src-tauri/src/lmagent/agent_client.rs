//! Host-side agent client (named-pipe JSON line protocol).
//! Mirrors agent/src/agent_ipc/win.rs without pulling winapi into a second copy.

use std::io::{BufRead, BufReader, Write};
use std::os::windows::ffi::OsStrExt;
use winapi::um::fileapi::{CreateFileW, ReadFile, WriteFile, OPEN_EXISTING};
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::winnt::{
    FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_READ, GENERIC_WRITE,
};

pub const PIPE_NAME: &str = "\\\\.\\pipe\\lmp-agent";

fn to_wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

struct PipeStream(winapi::shared::ntdef::HANDLE);

impl std::io::Read for PipeStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        unsafe {
            let mut read: u32 = 0;
            if ReadFile(
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
            if WriteFile(
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

/// Send one JSON line to the agent, return its JSON response line.
pub fn send_line(line: &str) -> Result<String, String> {
    let handle = open_pipe(true)?;
    unsafe {
        let wide = to_wide(PIPE_NAME);
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

/// Single-attempt send for fast probes (focus/typing guards).
pub fn send_line_fast(line: &str) -> Result<String, String> {
    let handle = open_pipe(false)?;
    unsafe {
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

/// Open the agent pipe. retry=true waits up to 2s (for injection settle),
/// retry=false is a single attempt (fast probes).
fn open_pipe(retry: bool) -> Result<winapi::shared::ntdef::HANDLE, String> {
    unsafe {
        let wide = to_wide(PIPE_NAME);
        let attempts = if retry { 20 } else { 1 };
        let mut handle = INVALID_HANDLE_VALUE;
        for _ in 0..attempts {
            handle = CreateFileW(
                wide.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            );
            if handle != INVALID_HANDLE_VALUE {
                return Ok(handle);
            }
            if retry {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
        Err(
            "agent pipe not available (agent not injected or game closed)".into(),
        )
    }
}

