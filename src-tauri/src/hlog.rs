//! File logging for the LMPlus host.
//!
//! Writes to `<exe_dir>/logs/lmplus_host_<YYYYMMDD_HHMMSS>.log`.
//! Always on (diagnostic phase); cheap appends, no tracing deps.

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::time::Instant;

static LOG_FILE: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);
static START: once_cell::sync::Lazy<std::time::Instant> =
    once_cell::sync::Lazy::new(std::time::Instant::now);

pub fn init() {
    let Ok(path) = log_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // Fresh log per LMPlus launch.
    let _ = std::fs::write(&path, b"");
    *LOG_FILE.lock().unwrap() = Some(path);
    log("info", "host logger initialized");
}

fn log_path() -> Result<std::path::PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().ok_or("no exe dir")?;
    let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    Ok(dir.join(format!("logs/lmplus_host_{stamp}.log")))
}

pub fn log(level: &str, message: &str) {
    let line = format!(
        "[{:03}.{:03}] [{}] [host] {}\n",
        START.elapsed().as_secs(),
        START.elapsed().subsec_millis(),
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
    // Also visible in dev console.
    eprint!("{}", line);
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
