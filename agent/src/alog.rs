//! File logging inside the injected agent.
//!
//! Writes to `<lmplus_exe_dir>/logs/lmp_agent_<YYYYMMDD_HHMMSS>.log` —
//! resolved from the LMPlus.exe path the HOST passes right after injection
//! (module 0 doesn't know where LMPlus lives, so the host tells us).
//! Falls back to <game_dir>/logs if the host didn't tell us yet.

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

static LOG_FILE: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);
static START_MS: AtomicU64 = AtomicU64::new(0);

static mut LOG_BASE_DIR: Option<String> = None;

/// Called on DllMain-attach thread with the LMPlus exe dir from the host
/// (via SetLogDir export or the first ping payload). Safe before file use.
pub fn set_base_dir(dir: &str) {
    unsafe {
        LOG_BASE_DIR = Some(dir.to_string());
    }
    init_file();
    log("info", &format!("log dir set to {}", dir));
}

fn init_file() {
    let dir = unsafe { LOG_BASE_DIR.clone() }.unwrap_or_default();
    let _ = std::fs::create_dir_all(format!("{}/logs", dir));
    let stamp = now_stamp();
    let path = std::path::PathBuf::from(format!("{}/logs/lmp_agent_{}.log", dir, stamp));
    let _ = std::fs::write(&path, b"");
    *LOG_FILE.lock().unwrap() = Some(path);
    START_MS.store(now_millis(), Ordering::SeqCst);
    log("info", "agent logger initialized");
}

fn now_stamp() -> String {
    // UTC-based compact stamp; no chrono in the agent to keep deps small.
    let ms = now_millis();
    let secs = (ms / 1000) as i64;
    let days = secs / 86400;
    let rem = secs % 86400;
    let (h, m, s) = ((rem / 3600) as u32, ((rem % 3600) / 60) as u32, (rem % 60) as u32);
    // civil_from_days (Howard Hinnant algorithm)
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = if month <= 2 { y + 1 } else { y };
    format!("{:04}{:02}{:02}_{:02}{:02}{:02}", year, month, d, h, m, s)
}

fn now_millis() -> u64 {
    use winapi::um::sysinfoapi::GetTickCount64;
    unsafe { GetTickCount64() }
}

pub fn log(level: &str, message: &str) {
    let total = START_MS.load(Ordering::SeqCst);
    let line = format!(
        "[{:03}.{:03}] [{}] [agent] {}\n",
        total / 1000 % 100_000,
        total % 1000,
        level,
        message
    );
    if let Ok(guard) = LOG_FILE.lock() {
        if let Some(path) = guard.as_ref() {
            if let Ok(mut f) = OpenOptions::new().append(true).create(true).open(path) {
                let _ = f.write_all(line.as_bytes());
            }
        }
    }
}

pub fn info(message: &str) {
    log("info", message);
}

pub fn warn(message: &str) {
    log("warn", message);
}

pub fn error(message: &str) {
    log("error", message);
}
