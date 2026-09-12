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

pub fn info(msg: &str) {
    ensure_initialized();
    log::info!("{}", msg);
}

pub fn warning(msg: &str) {
    ensure_initialized();
    log::warn!("{}", msg);
}

pub fn error(msg: &str) {
    ensure_initialized();
    log::error!("{}", msg);
}
