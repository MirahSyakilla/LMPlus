//! DLL injection: loads lmp_agent.dll into the game process and talks to it
//! over the agent's named pipe.

use std::ffi::CString;
use std::os::windows::ffi::OsStrExt;
use std::time::Duration;
use winapi::shared::minwindef::{DWORD, FALSE, MAX_PATH};
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::memoryapi::{VirtualAllocEx, VirtualFreeEx, WriteProcessMemory};
use winapi::um::processthreadsapi::{CreateRemoteThread, GetExitCodeThread, OpenProcess, TerminateProcess};
use winapi::um::libloaderapi::{GetModuleHandleW, GetProcAddress};
use winapi::um::synchapi::WaitForSingleObject;
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use winapi::um::winbase::WAIT_OBJECT_0;
use winapi::um::winnt::{
    MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, PROCESS_CREATE_THREAD,
    PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
};

pub const AGENT_DLL_NAME: &str = "lmp_agent.dll";

fn to_wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

/// Find the game process by its FULL exe path (e.g.
/// `D:\...\Game\Lords Mobile PC.exe`). Only a process whose on-disk image
/// matches exactly (case-insensitive) counts — this prevents grabbing a game
/// instance launched from a different install folder.
pub fn find_game_pid(exe_path: &str) -> Option<DWORD> {
    if exe_path.trim().is_empty() {
        return None;
    }
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return None;
    }
    let mut pe: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    pe.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as DWORD;
    let mut result = None;
    unsafe {
        if Process32FirstW(snapshot, &mut pe) != 0 {
            loop {
                // Cheap pre-filter on the process name before opening handles.
                let name = String::from_utf16_lossy(
                    &pe.szExeFile[..pe
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(pe.szExeFile.len())],
                );
                let expected_name = exe_path
                    .rsplit(['\\', '/'])
                    .next()
                    .unwrap_or("");
                if !expected_name.is_empty() && name.eq_ignore_ascii_case(expected_name) {
                    if process_image_path_matches(pe.th32ProcessID, exe_path) {
                        result = Some(pe.th32ProcessID);
                        break;
                    }
                }
                if Process32NextW(snapshot, &mut pe) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
    }
    result
}

/// Read the on-disk image path of a PID and compare (case-insensitive) with
/// the expected exe path. Returns false on any query failure.
fn process_image_path_matches(pid: DWORD, expected_path: &str) -> bool {
    use winapi::um::psapi::GetModuleFileNameExW;
    use winapi::um::processthreadsapi::OpenProcess as OpenProcessQuery;
    use winapi::um::winnt::PROCESS_QUERY_INFORMATION;

    let normalize = |s: &str| s.replace('/', "\\").to_ascii_lowercase();
    let expected = normalize(expected_path);

    unsafe {
        let handle = OpenProcessQuery(PROCESS_QUERY_INFORMATION, FALSE, pid);
        if handle.is_null() {
            return false;
        }
        let mut path: [u16; 1024] = [0; 1024];
        let len = GetModuleFileNameExW(handle, std::ptr::null_mut(), path.as_mut_ptr(), 1024);
        CloseHandle(handle);
        if len == 0 {
            return false;
        }
        let actual = String::from_utf16_lossy(&path[..len as usize]);
        normalize(&actual) == expected
    }
}

/// Inject the agent DLL into the target process via the classic
/// LoadLibraryW + CreateRemoteThread technique.
pub fn inject(pid: DWORD, dll_path: &str) -> Result<(), String> {
    unsafe {
        let process = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_OPERATION | PROCESS_VM_WRITE | PROCESS_VM_READ,
            FALSE,
            pid,
        );
        if process.is_null() {
            return Err(format!("OpenProcess({}) failed", pid));
        }

        let path_wide = to_wide(dll_path);
        let path_bytes = path_wide.len() * 2;

        let remote_mem = VirtualAllocEx(
            process,
            std::ptr::null_mut(),
            path_bytes,
            MEM_RESERVE | MEM_COMMIT,
            PAGE_READWRITE,
        );
        if remote_mem.is_null() {
            CloseHandle(process);
            return Err("VirtualAllocEx failed".into());
        }

        if WriteProcessMemory(
            process,
            remote_mem,
            path_wide.as_ptr() as *const _,
            path_bytes,
            std::ptr::null_mut(),
        ) == 0
        {
            VirtualFreeEx(process, remote_mem, 0, MEM_RELEASE);
            CloseHandle(process);
            return Err("WriteProcessMemory failed".into());
        }

        let kernel32 = GetModuleHandleW(to_wide("kernel32.dll").as_ptr());
        let load_library = GetProcAddress(kernel32 as _, CString::new("LoadLibraryW").unwrap().as_ptr());
        if load_library.is_null() {
            VirtualFreeEx(process, remote_mem, 0, MEM_RELEASE);
            CloseHandle(process);
            return Err("LoadLibraryW not found in kernel32".into());
        }
        let load_library = load_library as *mut core::ffi::c_void;

        let thread = CreateRemoteThread(
            process,
            std::ptr::null_mut(),
            0,
            Some(std::mem::transmute::<
                *mut core::ffi::c_void,
                winapi::um::minwinbase::LPTHREAD_START_ROUTINE,
            >(load_library).unwrap()),
            remote_mem,
            0, // dwStackSize
            std::ptr::null_mut(),
        );
        if thread.is_null() {
            VirtualFreeEx(process, remote_mem, 0, MEM_RELEASE);
            CloseHandle(process);
            return Err("CreateRemoteThread failed".into());
        }

        // Wait for the load to finish; then clean up the remote memory.
        let wait = WaitForSingleObject(thread, 15_000);
        if wait != WAIT_OBJECT_0 {
            TerminateProcess(thread, 1);
            VirtualFreeEx(process, remote_mem, 0, MEM_RELEASE);
            CloseHandle(thread);
            CloseHandle(process);
            return Err("LoadLibrary thread timed out".into());
        }

        let mut exit_code: DWORD = 0;
        GetExitCodeThread(thread, &mut exit_code);
        CloseHandle(thread);
        VirtualFreeEx(process, remote_mem, 0, MEM_RELEASE);
        CloseHandle(process);

        if exit_code == 0 {
            return Err("LoadLibraryW returned NULL (DLL rejected by the game process)".into());
        }
        Ok(())
    }
}

/// Ensure the agent is present: if the pipe responds to ping we're done,
/// otherwise inject into the game process.
pub fn ensure_agent(exe_path: &str, dll_path: &str) -> Result<(), String> {
    if agent_alive() {
        return Ok(());
    }
    let pid = find_game_pid(exe_path).ok_or_else(|| {
        format!(
            "game process not found for this app path (no running process with image {})",
            exe_path
        )
    })?;
    inject(pid, dll_path)?;
    // Give the agent a moment to boot its pipe server.
    for _ in 0..40 {
        if agent_alive() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err("agent injected but pipe never came up".into())
}

pub fn agent_alive() -> bool {
    matches!(
        super::agent_client::send_line("{\"action\":\"ping\"}"),
        Ok(resp) if resp.contains("\"ok\":true")
    )
}

/// Single-attempt ping — no 2s retry stall. For focus/typing guards.
pub fn agent_alive_fast() -> bool {
    matches!(
        super::agent_client::send_line_fast("{\"action\":\"ping\"}"),
        Ok(resp) if resp.contains("\"ok\":true")
    )
}

/// Eject the agent DLL (uses FreeLibraryAndExitThread inside the target).
pub fn eject(pid: DWORD) -> Result<(), String> {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_CREATE_THREAD | PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE, FALSE, pid);
        if process.is_null() {
            return Err("OpenProcess failed for eject".into());
        }
        // Find module base by enumerating remote modules.
        let base = remote_module_base(pid, AGENT_DLL_NAME);
        CloseHandle(process);
        let Some(base) = base else {
            return Ok(()); // not loaded — nothing to do
        };
        let process = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_CREATE_THREAD | PROCESS_VM_OPERATION | PROCESS_VM_WRITE,
            FALSE,
            pid,
        );
        if process.is_null() {
            return Err("OpenProcess failed for eject".into());
        }
        let kernel32 = GetModuleHandleW(to_wide("kernel32.dll").as_ptr());
        let free_lib = GetProcAddress(kernel32 as _, CString::new("FreeLibraryAndExitThread").unwrap().as_ptr());
        if free_lib.is_null() {
            CloseHandle(process);
            return Err("FreeLibraryAndExitThread not found".into());
        }
        let thread = CreateRemoteThread(
            process,
            std::ptr::null_mut(),
            0,
            std::mem::transmute::<
                *mut core::ffi::c_void,
                winapi::um::minwinbase::LPTHREAD_START_ROUTINE,
            >(free_lib as *mut core::ffi::c_void),
            base as *mut _,
            0, // dwStackSize
            std::ptr::null_mut(),
        );
        if thread.is_null() {
            CloseHandle(process);
            return Err("CreateRemoteThread(eject) failed".into());
        }
        WaitForSingleObject(thread, 5_000);
        CloseHandle(thread);
        CloseHandle(process);
        Ok(())
    }
}

fn remote_module_base(pid: DWORD, module_name: &str) -> Option<*mut core::ffi::c_void> {
    use winapi::um::psapi::{EnumProcessModules, GetModuleFileNameExW};
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, FALSE, pid);
        if process.is_null() {
            return None;
        }
        let mut modules: [winapi::shared::minwindef::HMODULE; 512] =
            [std::ptr::null_mut(); 512];
        let mut needed: DWORD = 0;
        if EnumProcessModules(
            process,
            modules.as_mut_ptr(),
            (modules.len() * std::mem::size_of::<winapi::shared::minwindef::HMODULE>()) as DWORD,
            &mut needed,
        ) == 0
        {
            CloseHandle(process);
            return None;
        }
        let count = std::cmp::min(
            needed as usize / std::mem::size_of::<winapi::shared::minwindef::HMODULE>(),
            modules.len(),
        );
        let mut result = None;
        for module in modules.iter().take(count) {
            let mut path: [u16; MAX_PATH] = [0; MAX_PATH];
            let len = GetModuleFileNameExW(process, *module, path.as_mut_ptr(), MAX_PATH as DWORD);
            if len == 0 {
                continue;
            }
            let name = String::from_utf16_lossy(&path[..len as usize]);
            if name.ends_with(module_name) || name.ends_with(&module_name.to_uppercase()) {
                result = Some(*module as *mut core::ffi::c_void);
                break;
            }
        }
        CloseHandle(process);
        result
    }
}

/// Launch the game with the agent preloaded: CreateProcessW(CREATE_SUSPENDED)
/// → inject → ResumeThread. AV-friendlier than remote-threading into a
/// running process, and guarantees the agent is present from frame zero.
pub fn launch_game_with_agent(
    exe_path: &str,
    args: &str,
    dll_path: &str,
) -> Result<(), String> {
    use winapi::um::processthreadsapi::{CreateProcessW, ResumeThread, TerminateProcess};
    use winapi::um::winbase::CREATE_SUSPENDED;
    use winapi::um::processthreadsapi::STARTUPINFOW;

    let wide_app = to_wide(exe_path);
    let cmd = if args.is_empty() {
        format!("\"{}\"", exe_path)
    } else {
        format!("\"{}\" {}", exe_path, args)
    };
    let mut cmd_wide = to_wide(&cmd);

    unsafe {
        let mut si: STARTUPINFOW = std::mem::zeroed();
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        let mut pi: winapi::um::processthreadsapi::PROCESS_INFORMATION = std::mem::zeroed();

        let ok = CreateProcessW(
            wide_app.as_ptr(),
            cmd_wide.as_mut_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            CREATE_SUSPENDED,
            std::ptr::null_mut(),
            std::ptr::null(),
            &mut si,
            &mut pi,
        );
        if ok == 0 {
            return Err(format!(
                "CreateProcessW(suspended) failed: {}",
                winapi::um::errhandlingapi::GetLastError()
            ));
        }

        // Inject while suspended.
        if let Err(e) = inject(pi.dwProcessId, dll_path) {
            TerminateProcess(pi.hProcess, 1);
            CloseHandle(pi.hProcess);
            CloseHandle(pi.hThread);
            return Err(format!("pre-launch inject failed: {}", e));
        }

        ResumeThread(pi.hThread);
        CloseHandle(pi.hThread);
        CloseHandle(pi.hProcess);
    }
    Ok(())
}
