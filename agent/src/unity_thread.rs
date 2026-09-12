//! Unity main-thread marshalling.
//!
//! il2cpp calls that touch Unity objects must run on the Unity main thread.
//! We install a Windows hook on the game's main thread and pump queued
//! actions from it via a timer, which is safe, thread-agnostic and survives
//! Unity version changes.

use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

static QUEUE: Lazy<Mutex<VecDeque<Box<dyn FnOnce() + Send>>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

static HOOK_HANDLE: AtomicUsize = AtomicUsize::new(0);

const WM_TIMER_HOOK_ID: usize = 0x4C4D50; // 'LMP'

#[allow(non_snake_case)]
unsafe extern "system" fn hook_proc(code: i32, wparam: usize, lparam: isize) -> isize {
    if code >= 0 {
        pump_once();
    }
    winapi::um::winuser::CallNextHookEx(
        std::ptr::null_mut(),
        code,
        wparam as _,
        lparam as _,
    )
}

/// Execute at most one queued closure. Called from the main-thread hook.
pub fn pump_once() {
    tick_frame_runner();
    let next = QUEUE
        .lock()
        .ok()
        .and_then(|mut q| q.pop_front());
    if let Some(job) = next {
        job();
    }
}

pub fn enqueue(job: Box<dyn FnOnce() + Send>) {
    if let Ok(mut q) = QUEUE.lock() {
        q.push_back(job);
    }
}

/// Install a WH_GETMESSAGE hook on the thread that owns the game window.
/// Returns false if the game window cannot be found yet (caller retries).
pub fn install() -> bool {
    unsafe {
        let hwnd = find_game_window();
        if hwnd.is_null() {
            return false;
        }
        let mut pid: u32 = 0;
        winapi::um::winuser::GetWindowThreadProcessId(hwnd, &mut pid);
        let thread_id = winapi::um::winuser::GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
        if thread_id == 0 {
            return false;
        }
        if HOOK_HANDLE.load(Ordering::SeqCst) != 0 {
            return true; // already installed
        }
        let hook = winapi::um::winuser::SetWindowsHookExW(
            winapi::um::winuser::WH_GETMESSAGE,
            Some(hook_proc),
            std::ptr::null_mut(),
            thread_id,
        );
        if hook.is_null() {
            return false;
        }
        HOOK_HANDLE.store(hook as usize, Ordering::SeqCst);
        // Kick the message loop so the hook fires and starts pumping.
        winapi::um::winuser::PostMessageW(hwnd, WM_TIMER_HOOK_ID as u32, 0, 0);
        true
    }
}

unsafe fn find_game_window() -> winapi::shared::windef::HWND {
    let mut found: winapi::shared::windef::HWND = std::ptr::null_mut();
    let our_pid = winapi::um::processthreadsapi::GetCurrentProcessId();
    unsafe extern "system" fn enum_proc(
        hwnd: winapi::shared::windef::HWND,
        lparam: isize,
    ) -> i32 {
        let out = lparam as *mut winapi::shared::windef::HWND;
        let mut pid: u32 = 0;
        winapi::um::winuser::GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == winapi::um::processthreadsapi::GetCurrentProcessId() {
            // Prefer visible top-level windows (the Unity player window).
            if winapi::um::winuser::IsWindowVisible(hwnd) != 0 {
                *out = hwnd;
                return 0;
            }
        }
        1
    }
    winapi::um::winuser::EnumWindows(Some(enum_proc), &mut found as *mut _ as isize);
    let _ = our_pid;
    found
}

/// Flush remaining queued jobs (used in tests / shutdown).
#[allow(dead_code)]
pub fn drain() {
    loop {
        let next = QUEUE
            .lock()
            .ok()
            .and_then(|mut q| q.pop_front());
        match next {
            Some(job) => job(),
            None => return,
        }
    }
}

/// Enqueue a job and wait for it. Falls back to running inline when the
/// main-thread hook is not installed (e.g. game window not found yet).
pub fn call_or_inline<F, T>(job: F, timeout: std::time::Duration) -> Result<T, String>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    if HOOK_HANDLE.load(Ordering::SeqCst) == 0 {
        // No pump available; run inline on this thread (caller must have
        // attached the thread to il2cpp).
        return Ok(job());
    }
    let (tx, rx) = std::sync::mpsc::channel();
    enqueue(Box::new(move || {
        tx.send(job()).ok();
    }));
    rx.recv_timeout(timeout)
        .map_err(|_| "action timed out on unity thread".to_string())
}

use std::sync::Arc as StdArc;

static FRAME_RUNNER: Lazy<Mutex<Option<StdArc<Mutex<Box<dyn FnMut() -> bool + Send>>>>>> =
    Lazy::new(|| Mutex::new(None));

/// Register a per-tick runner; it stays installed while the closure returns true.
pub fn set_frame_runner(f: Box<dyn FnMut() -> bool + Send>) {
    if let Ok(mut g) = FRAME_RUNNER.lock() {
        *g = Some(StdArc::new(Mutex::new(f)));
    }
}

/// Pump the registered runner (call from the Update hook every tick).
pub fn tick_frame_runner() {
    let next = FRAME_RUNNER.lock().ok().and_then(|mut g| g.take());
    if let Some(runner) = next {
        let cont = runner.lock().map(|mut f| f()).unwrap_or(false);
        if cont {
            if let Ok(mut g) = FRAME_RUNNER.lock() {
                *g = Some(runner);
            }
        }
    }
}
