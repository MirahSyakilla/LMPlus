//! Logging via platform-appropriate logging backend.
//!
//! On Apple platforms (macOS/iOS) the Apple Unified Logging System is used via
//! `oslog`. On all other platforms `env_logger` is used, which writes to
//! stderr and respects the `RUST_LOG` environment variable.

use std::sync::Once;

static INIT: Once = Once::new();

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn ensure_initialized() {
    INIT.call_once(|| {
        use log::LevelFilter;
        use oslog::OsLogger;
        OsLogger::new("com.batch.il2cpp-bridge")
            .level_filter(LevelFilter::Debug)
            .init()
            .ok();
    });
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
fn ensure_initialized() {
    INIT.call_once(|| {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    });
}

static SINK: std::sync::Mutex<Option<Box<dyn Fn(&str, &str) + Send + Sync>>> =
    std::sync::Mutex::new(None);

/// Install a sink that receives (level, message) for every bridge log line.
/// Used by embedders whose host has no stderr (e.g. injected into a GUI game).
pub fn set_sink(f: Box<dyn Fn(&str, &str) + Send + Sync>) {
    *SINK.lock().unwrap() = Some(f);
}

fn emit(level: &str, msg: &str) {
    ensure_initialized();
    if let Ok(guard) = SINK.lock() {
        if let Some(f) = guard.as_ref() {
            f(level, msg);
        }
    }
    match level {
        "warn" => log::warn!("{}", msg),
        "error" => log::error!("{}", msg),
        _ => log::info!("{}", msg),
    }
}

pub fn info(msg: &str) {
    emit("info", msg);
}

pub fn warning(msg: &str) {
    emit("warn", msg);
}

pub fn error(msg: &str) {
    emit("error", msg);
}
