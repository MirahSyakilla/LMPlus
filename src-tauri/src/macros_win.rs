use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::mem;
use std::ptr;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use winapi::shared::minwindef::{DWORD, FALSE, MAX_PATH};
use winapi::um::handleapi::CloseHandle;
use winapi::um::processthreadsapi::OpenProcess;
use winapi::um::psapi::GetModuleFileNameExW;
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use winapi::um::winuser::{
    ClientToScreen, FindWindowExW, GetClientRect, GetCursorPos, GetSystemMetrics,
    GetWindowThreadProcessId, IsWindowVisible, SendInput, SetCursorPos, INPUT, INPUT_MOUSE,
    MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE, SM_CXSCREEN,
    SM_CYSCREEN,
};

static LAST_MACRO_TIME: Lazy<Mutex<Instant>> = Lazy::new(|| Mutex::new(Instant::now()));
const DEBOUNCE_MS: u64 = 200;

fn find_game_window(exe_path: &str, process_name: &str) -> Option<winapi::shared::windef::HWND> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() || snapshot == winapi::um::handleapi::INVALID_HANDLE_VALUE {
        return None;
    }

    let mut pe: PROCESSENTRY32W = unsafe { mem::zeroed() };
    pe.dwSize = mem::size_of::<PROCESSENTRY32W>() as DWORD;

    let mut hwnd = None;

    if unsafe { Process32FirstW(snapshot, &mut pe) } != 0 {
        loop {
            let name = String::from_utf16_lossy(
                &pe.szExeFile[..pe
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(pe.szExeFile.len())],
            );
            if name.eq_ignore_ascii_case(process_name) {
                let pid = pe.th32ProcessID;
                let h_proc = unsafe {
                    OpenProcess(
                        winapi::um::winnt::PROCESS_QUERY_INFORMATION
                            | winapi::um::winnt::PROCESS_VM_READ,
                        FALSE,
                        pid,
                    )
                };
                if !h_proc.is_null() {
                    let mut path: [u16; MAX_PATH] = [0; MAX_PATH];
                    let len = unsafe {
                        GetModuleFileNameExW(
                            h_proc,
                            ptr::null_mut(),
                            path.as_mut_ptr(),
                            MAX_PATH as DWORD,
                        )
                    };
                    let actual = String::from_utf16_lossy(&path[..len as usize]);
                    if actual.eq_ignore_ascii_case(exe_path) {
                        let mut w = unsafe {
                            FindWindowExW(
                                ptr::null_mut(),
                                ptr::null_mut(),
                                ptr::null(),
                                ptr::null(),
                            )
                        };
                        while !w.is_null() {
                            let mut w_pid: DWORD = 0;
                            unsafe { GetWindowThreadProcessId(w, &mut w_pid) };
                            if w_pid == pid && unsafe { IsWindowVisible(w) } != 0 {
                                hwnd = Some(w);
                                break;
                            }
                            w = unsafe {
                                FindWindowExW(ptr::null_mut(), w, ptr::null(), ptr::null())
                            };
                        }
                    }
                    unsafe { CloseHandle(h_proc) };
                }
            }
            if unsafe { Process32NextW(snapshot, &mut pe) } == 0 {
                break;
            }
        }
    }
    unsafe { CloseHandle(snapshot) };
    hwnd
}

fn send_click_at(x: i32, y: i32, is_speed: bool) {
    let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };

    let dx = ((x as i64 * 65535) / screen_w as i64) as i32;
    let dy = ((y as i64 * 65535) / screen_h as i64) as i32;

    let offset = if is_speed { 5 } else { 2 };
    let offset_dx = (((x + offset) as i64 * 65535) / screen_w as i64) as i32;

    let mut inputs: [INPUT; 4] = unsafe { mem::zeroed() };
    inputs[0].type_ = INPUT_MOUSE;
    unsafe {
        inputs[0].u.mi_mut().dx = dx;
        inputs[0].u.mi_mut().dy = dy;
        inputs[0].u.mi_mut().dwFlags = MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE;
    }

    inputs[1].type_ = INPUT_MOUSE;
    unsafe {
        inputs[1].u.mi_mut().dx = offset_dx;
        inputs[1].u.mi_mut().dy = dy;
        inputs[1].u.mi_mut().dwFlags = MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE;
    }

    inputs[2].type_ = INPUT_MOUSE;
    unsafe {
        inputs[2].u.mi_mut().dx = dx;
        inputs[2].u.mi_mut().dy = dy;
        inputs[2].u.mi_mut().dwFlags = MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_LEFTDOWN;
    }

    inputs[3].type_ = INPUT_MOUSE;
    unsafe {
        inputs[3].u.mi_mut().dx = dx;
        inputs[3].u.mi_mut().dy = dy;
        inputs[3].u.mi_mut().dwFlags = MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_LEFTUP;
    }

    unsafe { SendInput(1, &mut inputs[0], mem::size_of::<INPUT>() as i32) };
    unsafe { SendInput(1, &mut inputs[1], mem::size_of::<INPUT>() as i32) };
    unsafe { SendInput(1, &mut inputs[2], mem::size_of::<INPUT>() as i32) };
    unsafe { SendInput(1, &mut inputs[3], mem::size_of::<INPUT>() as i32) };
    unsafe { SendInput(1, &mut inputs[3], mem::size_of::<INPUT>() as i32) };
}

fn get_formation_coords(name: &str) -> Vec<(i32, i32)> {
    match name {
        "Inf Phal" => vec![(50, 100), (50, 480), (180, 440), (740, 480)],
        "Range Phal" => vec![(50, 100), (50, 480), (330, 440), (740, 480)],
        "Cav Phal" => vec![(50, 100), (50, 480), (540, 440), (740, 480)],
        "Inf Wedge" => vec![(50, 100), (50, 480), (180, 500), (740, 480)],
        "Range Wedge" => vec![(50, 100), (50, 480), (330, 500), (740, 480)],
        "Cav Wedge" => vec![(50, 100), (50, 480), (540, 500), (740, 480)],
        _ => vec![],
    }
}

fn load_misc_macros() -> HashMap<String, Vec<(i32, i32)>> {
    use std::fs;
    let exe = std::env::current_exe().unwrap_or_default();
    let dir = exe.parent().unwrap_or(std::path::Path::new("."));
    let path = dir.join("misc.cfg");
    let content = fs::read_to_string(&path).unwrap_or_default();
    let mut map = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split(';').collect();
        if parts.len() < 3 {
            continue;
        }
        let name = parts[0].to_string();
        let mut points = Vec::new();
        for i in 2..parts.len().min(6) {
            let xy: Vec<&str> = parts[i].split(',').collect();
            if xy.len() == 2 {
                if let (Ok(x), Ok(y)) = (xy[0].trim().parse::<i32>(), xy[1].trim().parse::<i32>()) {
                    points.push((x, y));
                }
            }
        }
        map.insert(name, points);
    }
    map
}

#[tauri::command]
pub fn execute_macro(
    macro_name: String,
    exe_path: String,
    process_name: String,
) -> Result<(), String> {
    let mut last = LAST_MACRO_TIME.lock().map_err(|e| e.to_string())?;
    if last.elapsed().as_millis() < DEBOUNCE_MS as u128 {
        return Ok(());
    }
    *last = Instant::now();
    drop(last);

    let coords = get_formation_coords(&macro_name);
    let points = if coords.is_empty() {
        load_misc_macros()
            .get(&macro_name)
            .cloned()
            .unwrap_or_default()
    } else {
        coords
    };

    if points.is_empty() {
        return Ok(());
    }

    let hwnd = find_game_window(&exe_path, &process_name);
    let Some(hwnd) = hwnd else {
        return Ok(());
    };

    let mut rect = unsafe { mem::zeroed() };
    unsafe { GetClientRect(hwnd, &mut rect) };
    let client_w = rect.right - rect.left;
    let client_h = rect.bottom - rect.top;

    if client_w <= 0 || client_h <= 0 {
        return Ok(());
    }

    let scale_x = client_w as f64 / 1024.0;
    let scale_y = client_h as f64 / 576.0;

    let is_speed = macro_name.to_lowercase().contains("speed");

    let mut original_pos = unsafe { mem::zeroed() };
    unsafe { GetCursorPos(&mut original_pos) };

    for (x, y) in &points {
        let sx = (*x as f64 * scale_x + 0.5) as i32;
        let sy = (*y as f64 * scale_y + 0.5) as i32;

        let mut p = winapi::shared::windef::POINT { x: sx, y: sy };
        unsafe { ClientToScreen(hwnd, &mut p) };

        send_click_at(p.x, p.y, is_speed);

        if is_speed {
            std::thread::sleep(Duration::from_millis(135));
        } else {
            std::thread::sleep(Duration::from_millis(60));
        }
    }

    if !is_speed {
        let delay = match points.len() {
            1 => 150,
            2 => 240,
            3 => 280,
            4 => 450,
            _ => 450,
        };
        std::thread::sleep(Duration::from_millis(delay));

        let fx = (980.0 * scale_x + 0.5) as i32;
        let fy = (30.0 * scale_y + 0.5) as i32;
        let mut fp = winapi::shared::windef::POINT { x: fx, y: fy };
        unsafe { ClientToScreen(hwnd, &mut fp) };
        send_click_at(fp.x, fp.y, false);
        std::thread::sleep(Duration::from_millis(60));
    }

    unsafe { SetCursorPos(original_pos.x, original_pos.y) };

    Ok(())
}

#[tauri::command]
pub fn parse_misc_cfg() -> Result<serde_json::Value, String> {
    let macros = load_misc_macros();
    let result: Vec<serde_json::Value> = macros
        .into_iter()
        .map(|(name, points)| {
            serde_json::json!({
                "name": name,
                "points": points.iter().map(|(x, y)| serde_json::json!({"x": x, "y": y})).collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(serde_json::json!(result))
}
