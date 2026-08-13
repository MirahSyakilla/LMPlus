use std::mem;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use winapi::shared::minwindef::{DWORD, FALSE, MAX_PATH};
use winapi::um::handleapi::CloseHandle;
use winapi::um::memoryapi::{ReadProcessMemory, VirtualQueryEx, WriteProcessMemory};
use winapi::um::processthreadsapi::{GetExitCodeProcess, OpenProcess};
use winapi::um::psapi::GetModuleFileNameExW;
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use winapi::um::winnt::{
    MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_EXECUTE_READWRITE, PAGE_READWRITE,
    PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
    STILL_ACTIVE,
};

static PERSISTENT_ACTIVE: Lazy<Arc<AtomicBool>> = Lazy::new(|| Arc::new(AtomicBool::new(false)));
static PERSISTENT_HANDLE: Lazy<Mutex<Option<winapi::shared::ntdef::HANDLE>>> = Lazy::new(|| Mutex::new(None));
static PERSISTENT_PID: Lazy<Mutex<DWORD>> = Lazy::new(|| Mutex::new(0));
static PERSISTENT_ADDR: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(0));

const TARGET_VALUE: DWORD = 1091052860;
const ZOOM_VALUE: DWORD = 1099999999;
const ADDR_SUFFIX: usize = 0x89C;
const MAX_CHUNK_SIZE: usize = 1048576;

fn find_process_pid(exe_path: &str) -> Option<DWORD> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() || snapshot == winapi::um::handleapi::INVALID_HANDLE_VALUE {
        return None;
    }

    let mut pe: PROCESSENTRY32W = unsafe { mem::zeroed() };
    pe.dwSize = mem::size_of::<PROCESSENTRY32W>() as DWORD;

    let mut pid = None;

    if unsafe { Process32FirstW(snapshot, &mut pe) } != 0 {
        loop {
            let h_proc = unsafe {
                OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, pe.th32ProcessID)
            };
            if !h_proc.is_null() {
                let mut path: [u16; MAX_PATH] = [0; MAX_PATH];
                let len = unsafe {
                    GetModuleFileNameExW(h_proc, ptr::null_mut(), path.as_mut_ptr(), MAX_PATH as DWORD)
                };
                let actual = String::from_utf16_lossy(&path[..len as usize]);
                if actual.eq_ignore_ascii_case(exe_path) {
                    pid = Some(pe.th32ProcessID);
                }
                unsafe { CloseHandle(h_proc) };
                if pid.is_some() {
                    break;
                }
            }
            if unsafe { Process32NextW(snapshot, &mut pe) } == 0 {
                break;
            }
        }
    }
    unsafe { CloseHandle(snapshot) };
    pid
}

fn search_memory(handle: winapi::shared::ntdef::HANDLE) -> Option<usize> {
    let mut address: usize = 0;
    loop {
        let mut mbi: MEMORY_BASIC_INFORMATION = unsafe { mem::zeroed() };
        let result = unsafe {
            VirtualQueryEx(handle, address as *const _, &mut mbi, mem::size_of::<MEMORY_BASIC_INFORMATION>())
        };
        if result == 0 {
            break;
        }

        if mbi.State == MEM_COMMIT
            && (mbi.Protect == PAGE_READWRITE || mbi.Protect == PAGE_EXECUTE_READWRITE)
            && mbi.RegionSize >= 4096
        {
            let base = mbi.BaseAddress as usize;
            let size = mbi.RegionSize;
            let mut current = base;

            if (current & 0xFFF) != ADDR_SUFFIX {
                current += (ADDR_SUFFIX - (current & 0xFFF)) & 0xFFF;
            }

            while current < base + size {
                let chunk = (MAX_CHUNK_SIZE).min(base + size - current);
                let mut buffer = vec![0u8; chunk];
                let mut bytes_read: usize = 0;

                if unsafe {
                    ReadProcessMemory(
                        handle,
                        current as *const _,
                        buffer.as_mut_ptr() as *mut _,
                        chunk,
                        &mut bytes_read,
                    )
                } != 0
                {
                    let mut i = 0;
                    while i + 3 < bytes_read {
                        let addr = current + i;
                        if (addr & 0xFFF) == ADDR_SUFFIX {
                            let value: DWORD = unsafe { ptr::read_unaligned(buffer.as_ptr().add(i) as *const DWORD) };
                            if value == TARGET_VALUE {
                                return Some(addr);
                            }
                        }
                        i += 4;
                    }
                }
                current += MAX_CHUNK_SIZE;
            }
        }
        address += mbi.RegionSize;
    }
    None
}

#[tauri::command]
pub fn perform_map_zoom(exe_path: String, persistent: bool) -> Result<(), String> {
    stop_persistent_zoom()?;

    let pid = find_process_pid(&exe_path).ok_or("Process not found")?;

    let handle = unsafe {
        OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION,
            FALSE,
            pid,
        )
    };
    if handle.is_null() {
        return Err("Failed to open process".to_string());
    }

    let addr = search_memory(handle);

    let Some(addr) = addr else {
        unsafe { CloseHandle(handle) };
        return Err("Target value not found in memory".to_string());
    };

    let mut bytes_written: usize = 0;
    let result = unsafe {
        WriteProcessMemory(
            handle,
            addr as *mut _,
            &ZOOM_VALUE as *const _ as *const _,
            mem::size_of::<DWORD>(),
            &mut bytes_written,
        )
    };

    if result == 0 {
        unsafe { CloseHandle(handle) };
        return Err("Failed to write memory".to_string());
    }

    if persistent {
        PERSISTENT_ACTIVE.store(true, Ordering::SeqCst);
        *PERSISTENT_HANDLE.lock().unwrap() = Some(handle);
        *PERSISTENT_PID.lock().unwrap() = pid;
        *PERSISTENT_ADDR.lock().unwrap() = addr;

        let active = PERSISTENT_ACTIVE.clone();
        let exe = exe_path;

        thread::spawn(move || {
            while active.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(100));

                let mut guard = PERSISTENT_HANDLE.lock().unwrap();
                let h = match *guard {
                    Some(h) => h,
                    None => break,
                };

                let mut exit_code: DWORD = 0;
                if unsafe { GetExitCodeProcess(h, &mut exit_code) } == 0
                    || exit_code != STILL_ACTIVE
                {
                    *guard = None;
                    drop(guard);
                    *PERSISTENT_PID.lock().unwrap() = 0;
                    *PERSISTENT_ADDR.lock().unwrap() = 0;
                    break;
                }

                let addr = *PERSISTENT_ADDR.lock().unwrap();
                let mut bytes_written: usize = 0;
                unsafe {
                    WriteProcessMemory(
                        h,
                        addr as *mut _,
                        &ZOOM_VALUE as *const _ as *const _,
                        mem::size_of::<DWORD>(),
                        &mut bytes_written,
                    );
                }
            }
        });
    } else {
        unsafe { CloseHandle(handle) };
    }

    Ok(())
}

#[tauri::command]
pub fn stop_persistent_zoom() -> Result<(), String> {
    PERSISTENT_ACTIVE.store(false, Ordering::SeqCst);
    let mut guard = PERSISTENT_HANDLE.lock().unwrap();
    if let Some(h) = *guard {
        unsafe { CloseHandle(h) };
        *guard = None;
    }
    *PERSISTENT_PID.lock().unwrap() = 0;
    *PERSISTENT_ADDR.lock().unwrap() = 0;
    Ok(())
}