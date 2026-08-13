use std::ffi::OsStr;
use std::mem;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use winapi::shared::minwindef::{DWORD, FALSE, HMODULE, MAX_PATH};
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::handleapi::CloseHandle;
use winapi::um::processthreadsapi::{CreateProcessW, OpenProcess, TerminateProcess, PROCESS_INFORMATION, STARTUPINFOW};
use winapi::um::psapi::{EnumProcessModules, GetModuleFileNameExW};
use winapi::um::winbase::CREATE_NO_WINDOW;
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

fn find_process_by_exe_path(exe_path: &str, process_name: &str) -> Result<Option<DWORD>, String> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() || snapshot == winapi::um::handleapi::INVALID_HANDLE_VALUE {
        return Err(format!("CreateToolhelp32Snapshot failed: {}", unsafe { GetLastError() }));
    }

    let mut pe: PROCESSENTRY32W = unsafe { mem::zeroed() };
    pe.dwSize = mem::size_of::<PROCESSENTRY32W>() as DWORD;

    let mut result = None;

    if unsafe { Process32FirstW(snapshot, &mut pe) } != 0 {
        loop {
            let name = String::from_utf16_lossy(
                &pe.szExeFile[..pe.szExeFile.iter().position(|&c| c == 0).unwrap_or(pe.szExeFile.len())],
            );

            if name.eq_ignore_ascii_case(process_name) {
                let pid = pe.th32ProcessID;
                if let Ok(Some(found)) = verify_process_path(pid, exe_path) {
                    if found {
                        result = Some(pid);
                        break;
                    }
                }
            }

            if unsafe { Process32NextW(snapshot, &mut pe) } == 0 {
                break;
            }
        }
    }

    unsafe { CloseHandle(snapshot) };
    Ok(result)
}

fn verify_process_path(pid: DWORD, expected_path: &str) -> Result<Option<bool>, String> {
    let handle = unsafe {
        OpenProcess(
            winapi::um::winnt::PROCESS_QUERY_INFORMATION | winapi::um::winnt::PROCESS_VM_READ,
            FALSE,
            pid,
        )
    };
    if handle.is_null() {
        return Ok(None);
    }

    let mut h_mod: HMODULE = ptr::null_mut();
    let mut cb_needed: DWORD = 0;
    if unsafe { EnumProcessModules(handle, &mut h_mod, mem::size_of::<HMODULE>() as DWORD, &mut cb_needed) } == 0 {
        unsafe { CloseHandle(handle) };
        return Ok(None);
    }

    let mut path_buf: [u16; MAX_PATH] = [0; MAX_PATH];
    let len = unsafe { GetModuleFileNameExW(handle, h_mod, path_buf.as_mut_ptr(), MAX_PATH as DWORD) };
    unsafe { CloseHandle(handle) };

    if len == 0 {
        return Ok(None);
    }

    let actual_path = String::from_utf16_lossy(&path_buf[..len as usize]);
    Ok(Some(actual_path.eq_ignore_ascii_case(expected_path)))
}

fn count_instances(process_name: &str) -> Result<u32, String> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() || snapshot == winapi::um::handleapi::INVALID_HANDLE_VALUE {
        return Err(format!("CreateToolhelp32Snapshot failed: {}", unsafe { GetLastError() }));
    }

    let mut pe: PROCESSENTRY32W = unsafe { mem::zeroed() };
    pe.dwSize = mem::size_of::<PROCESSENTRY32W>() as DWORD;

    let mut count = 0u32;

    if unsafe { Process32FirstW(snapshot, &mut pe) } != 0 {
        loop {
            let name = String::from_utf16_lossy(
                &pe.szExeFile[..pe.szExeFile.iter().position(|&c| c == 0).unwrap_or(pe.szExeFile.len())],
            );
            if name.eq_ignore_ascii_case(process_name) {
                count += 1;
            }
            if unsafe { Process32NextW(snapshot, &mut pe) } == 0 {
                break;
            }
        }
    }

    unsafe { CloseHandle(snapshot) };
    Ok(count)
}

#[tauri::command]
pub fn launch_game(exe_path: String, process_name: String) -> Result<(), String> {
    if is_game_running(exe_path.clone(), process_name)? {
        return Ok(());
    }

    let args = format!("-AD=0-0 -PATH=\"{}\"", exe_path);
    let app_wide = to_wide(&exe_path);
    let mut cmd_wide = to_wide(&args);

    let mut si: STARTUPINFOW = unsafe { mem::zeroed() };
    si.cb = mem::size_of::<STARTUPINFOW>() as DWORD;

    let mut pi: PROCESS_INFORMATION = unsafe { mem::zeroed() };

    let result = unsafe {
        CreateProcessW(
            app_wide.as_ptr(),
            cmd_wide.as_mut_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
            FALSE,
            CREATE_NO_WINDOW,
            ptr::null_mut(),
            ptr::null(),
            &mut si,
            &mut pi,
        )
    };

    if result == 0 {
        return Err(format!("CreateProcessW failed: {}", unsafe { GetLastError() }));
    }

    unsafe {
        CloseHandle(pi.hProcess);
        CloseHandle(pi.hThread);
    }

    Ok(())
}

#[tauri::command]
pub fn kill_game(exe_path: String, process_name: String) -> Result<bool, String> {
    let pid = find_process_by_exe_path(&exe_path, &process_name)?;
    match pid {
        Some(pid) => {
            let handle = unsafe { OpenProcess(winapi::um::winnt::PROCESS_TERMINATE, FALSE, pid) };
            if handle.is_null() {
                return Err(format!("OpenProcess failed: {}", unsafe { GetLastError() }));
            }
            let result = unsafe { TerminateProcess(handle, 0) };
            unsafe { CloseHandle(handle) };
            if result == 0 {
                Err(format!("TerminateProcess failed: {}", unsafe { GetLastError() }))
            } else {
                Ok(true)
            }
        }
        None => Ok(false),
    }
}

#[tauri::command]
pub fn restart_game(exe_path: String, process_name: String, relaunch: bool) -> Result<(), String> {
    kill_game(exe_path.clone(), process_name.clone())?;

    if relaunch {
        std::thread::sleep(std::time::Duration::from_secs(1));
        launch_game(exe_path, process_name)?;
    }

    Ok(())
}

#[tauri::command]
pub fn is_game_running(exe_path: String, process_name: String) -> Result<bool, String> {
    let pid = find_process_by_exe_path(&exe_path, &process_name)?;
    Ok(pid.is_some())
}

#[tauri::command]
pub fn is_another_instance_running() -> Result<bool, String> {
    let count = count_instances("LMPlus.exe")?;
    Ok(count > 1)
}

#[tauri::command]
pub fn launch_lm_updater() -> Result<(), String> {
    let appdata = std::env::var("APPDATA").map_err(|_| "APPDATA is not set".to_string())?;
    let updater = std::path::PathBuf::from(appdata)
        .join("IGG")
        .join("Lords Mobile PC")
        .join("Lords Mobile Updater.exe");
    if !updater.exists() {
        return Err(format!("Updater not found: {}", updater.display()));
    }
    std::process::Command::new(updater)
        .spawn()
        .map_err(|e| format!("Failed to launch updater: {}", e))?;
    Ok(())
}
