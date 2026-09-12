//! il2cpp bridge layer — wraps il2cpp-bridge-rs (vendored) for the agent.
//!
//! Bridge handles symbol resolution (patched to look in GameAssembly.dll),
//! thread attachment, class/method caching and invocation with exception
//! handling. We orchestrate:
//!   1. `ensure_bridge()` — run bridge init once, block until cache is ready
//!   2. `with_class_method` — resolve + call helpers used by formation/mapview

use il2cpp_bridge_rs::{api, init};
use il2cpp_bridge_rs::structs::Method;
use once_cell::sync::OnceCell;
use std::ffi::c_void;
use std::sync::mpsc;
use std::time::Duration;

static BRIDGE_READY: OnceCell<()> = OnceCell::new();

/// Initialize the bridge exactly once and wait for the metadata cache.
/// Safe to call from any thread; subsequent calls return immediately.
pub fn ensure_bridge() -> Result<(), String> {
    if BRIDGE_READY.get().is_some() {
        return Ok(());
    }
    let (tx, rx) = mpsc::channel::<()>();
    init("GameAssembly", move || {
        tx.send(()).ok();
    });
    // A failed init resets bridge state and never calls the callback; retry
    // init() periodically until the deadline.
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    let mut last_retry = std::time::Instant::now();
    loop {
        if rx.try_recv().is_ok() {
            let _ = BRIDGE_READY.set(());
            return Ok(());
        }
        if last_retry.elapsed() >= Duration::from_secs(5) {
            last_retry = std::time::Instant::now();
            let (tx2, rx2) = mpsc::channel::<()>();
            init("GameAssembly", move || {
                tx2.send(()).ok();
            });
            if rx2.try_recv().is_ok() {
                let _ = BRIDGE_READY.set(());
                return Ok(());
            }
        }
        if std::time::Instant::now() > deadline {
            return Err("il2cpp bridge initialization timed out".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Access the hydrated Assembly-CSharp (game logic) cache.
pub fn bridge_csharp() -> std::sync::Arc<il2cpp_bridge_rs::structs::Assembly> {
    api::cache::csharp()
}

/// Resolve a static method by class + method name from the cached assemblies.
#[allow(dead_code)]
pub fn find_method(class_name: &str, method_name: &str) -> Option<Method> {
    let asm = api::cache::csharp();
    let class = asm.class(class_name)?;
    class.method(method_name)
}

/// Bind an instance pointer to a method for invocation.
#[allow(dead_code)]
pub fn bind_method(method: &Method, instance: *mut c_void) -> Method {
    let mut m = method.clone();
    m.instance = Some(instance);
    m
}
