//! Initialization of the IL2CPP runtime and metadata cache.
//!
//! [`init`] is the front door to the crate. It resolves IL2CPP exports, loads
//! the function table, attaches the worker thread to the runtime, initializes
//! the cache, hydrates metadata, and then runs queued callbacks.
use crate::api::{self, cache, Thread};
use crate::logger;
use crate::memory::symbol::{promote_library_to_global, resolve_symbol};
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

/// The target image name, set once during [`init()`](super::init).
pub(crate) static TARGET_IMAGE_NAME: OnceLock<String> = OnceLock::new();

type Callback = Box<dyn FnOnce() + Send + 'static>;

enum State {
    /// No initialization has started yet.
    Idle,
    /// Background thread is running; callbacks queued here fire when it finishes.
    Running(Vec<Callback>),
    /// Initialization completed successfully; new callers fire immediately.
    Done,
}

static STATE: Mutex<State> = Mutex::new(State::Idle);
const CACHE_INIT_MAX_ATTEMPTS: u8 = 5;
const CACHE_INIT_RETRY_DELAY: Duration = Duration::from_secs(3);

/// Initializes IL2CPP symbol loading and cache hydration.
///
/// This function must be called before using cache-backed helpers such as
/// [`crate::api::cache::csharp`] or class/method lookups that depend on the
/// hydrated metadata cache.
///
/// Behavior:
///
/// - First call: spawns the initialization worker and queues `on_complete`.
/// - Calls while initialization is running: queue additional callbacks.
/// - Calls after successful initialization: execute `on_complete` immediately
///   on a newly spawned thread.
/// - On failure: the internal state resets to idle so initialization can be
///   attempted again.
///
/// The `target_image` should be the loaded module used to compute image base
/// addresses and method RVA/VA information. Common values include
/// `UnityFramework` on iOS and `GameAssembly` on many desktop Unity builds.
///
/// # Example
///
/// ```no_run
/// use il2cpp_bridge_rs::{api, init};
///
/// init("GameAssembly", || {
///     let asm = api::cache::csharp();
///     println!("Ready: {}", asm.name);
/// });
/// ```
///
/// # Parameters
///
/// - `target_image`: name of the loaded image backing RVA/VA calculations
/// - `on_complete`: callback executed after a successful initialization pass
pub fn init<F>(target_image: &str, on_complete: F)
where
    F: FnOnce() + Send + 'static,
{
    TARGET_IMAGE_NAME
        .set(target_image.to_owned())
        .unwrap_or_else(|_| {
            logger::info("TARGET_IMAGE_NAME already set, ignoring new value.");
        });
    let mut guard = STATE.lock().unwrap();

    match &mut *guard {
        State::Done => {
            drop(guard);
            std::thread::spawn(on_complete);
        }
        State::Running(callbacks) => {
            callbacks.push(Box::new(on_complete));
        }
        State::Idle => {
            *guard = State::Running(vec![Box::new(on_complete)]);
            drop(guard);

            std::thread::spawn(move || {
                if let Some(target) = TARGET_IMAGE_NAME.get() {
                    promote_library_to_global(target);
                }

                if let Err(missing) = api::load(|symbol| match resolve_symbol(symbol) {
                    Ok(addr) => addr as *mut c_void,
                    Err(e) => {
                        logger::error(&format!("{}", e));
                        std::ptr::null_mut()
                    }
                }) {
                    logger::error(&format!(
                        "Failed to load IL2CPP API symbols: {}",
                        missing.join(", ")
                    ));

                    let mut guard = STATE.lock().unwrap();
                    *guard = State::Idle;
                    return;
                }

                logger::info("IL2CPP API loaded, waiting for cache initialization...");

                let _thread = Thread::attach(true);

                let mut cache_ready = false;
                let mut asm_count = 0usize;
                let mut cls_count = 0usize;

                for attempt in 1..=CACHE_INIT_MAX_ATTEMPTS {
                    cache::clear();

                    match unsafe { cache::load_all_assemblies() } {
                        Ok(a) => {
                            asm_count = a;
                            match cache::hydrate_all_classes() {
                                Ok(c) => {
                                    cls_count = c;
                                    logger::info(&format!(
                                        "Cache initialized: {} assemblies loaded, {} classes hydrated",
                                        a, c
                                    ));
                                    cache_ready = true;
                                    break;
                                }
                                Err(e) => {
                                    logger::error(&format!(
                                        "Cache init failed during class hydration: {}",
                                        e
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            logger::error(&format!("Cache init failed: {}", e));
                        }
                    }

                    if attempt < CACHE_INIT_MAX_ATTEMPTS {
                        logger::info(&format!(
                            "Cache initialization attempt {}/{} failed. Retrying in {}s...",
                            attempt,
                            CACHE_INIT_MAX_ATTEMPTS,
                            CACHE_INIT_RETRY_DELAY.as_secs()
                        ));
                        thread::sleep(CACHE_INIT_RETRY_DELAY);
                    }
                }

                let _ = (asm_count, cls_count);

                if cache_ready {
                    logger::info("Cache ready, starting callback...");

                    let callbacks = {
                        let mut guard = STATE.lock().unwrap();
                        let old = std::mem::replace(&mut *guard, State::Done);
                        match old {
                            State::Running(cbs) => cbs,
                            _ => vec![],
                        }
                    };

                    std::thread::spawn(move || {
                        for cb in callbacks {
                            cb();
                        }
                    });
                } else {
                    logger::error("Cache initialization failed after all retry attempts.");

                    let mut guard = STATE.lock().unwrap();
                    *guard = State::Idle;
                }
            });
        }
    }
}
