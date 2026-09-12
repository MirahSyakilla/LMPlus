//! Symbol resolution and caching utilities

use dashmap::DashMap;
use once_cell::sync::Lazy;
use thiserror::Error;

#[derive(Error, Debug)]
/// Errors that can occur during symbol resolution
pub enum SymbolError {
    /// The specified symbol was not found
    #[error("Symbol not found: {0}")]
    NotFound(String),
    /// Failed to convert the symbol name to a CString
    #[error("CString error")]
    StringError,
}

static CACHE: Lazy<DashMap<String, usize>> = Lazy::new(DashMap::new);

/// Resolves a symbol to its address using platform-specific lookup.
///
/// # Arguments
/// * `symbol` - The name of the symbol to resolve (e.g., "MGCopyAnswer")
///
/// # Returns
/// * `Result<usize, SymbolError>` - The address of the symbol or an error
pub fn resolve_symbol(symbol: &str) -> Result<usize, SymbolError> {
    if let Some(entry) = CACHE.get(symbol) {
        return Ok(*entry);
    }

    let addr = platform::raw_resolve(symbol)?;
    CACHE.insert(symbol.into(), addr);
    Ok(addr)
}

/// Manually caches a symbol address
///
/// Use this if you have resolved a symbol via other means and want to store it for future lookups.
///
/// # Arguments
/// * `s` - The symbol name
/// * `a` - The symbol address
pub fn cache_symbol(s: &str, a: usize) {
    CACHE.insert(s.into(), a);
}

/// Clears the symbol cache
pub fn clear_cache() {
    CACHE.clear();
}

/// Promotes the named library's symbols to global visibility.
///
/// On Linux/Android, Unity may load `GameAssembly.so` with `RTLD_LOCAL`, making
/// its symbols invisible to `dlsym(RTLD_DEFAULT, ...)`. This function re-opens
/// the library with `RTLD_GLOBAL` to fix that.
///
/// Called automatically during initialization when a target image name is
/// available, but can also be called manually before [`crate::init`] if needed.
///
/// On non-Linux platforms this is a no-op.
pub fn promote_library_to_global(library_name: &str) {
    platform::ensure_global_visibility(library_name);
}

// macOS / iOS: use dlsym with RTLD_DEFAULT to search all loaded dylibs.
#[cfg(any(target_os = "macos", target_os = "ios"))]
mod platform {
    use super::SymbolError;
    use std::ffi::CString;

    pub fn raw_resolve(symbol: &str) -> Result<usize, SymbolError> {
        let c_str = CString::new(symbol).map_err(|_| SymbolError::StringError)?;
        unsafe {
            let addr_ptr = libc::dlsym(libc::RTLD_DEFAULT, c_str.as_ptr());
            if addr_ptr.is_null() {
                Err(SymbolError::NotFound(symbol.into()))
            } else {
                Ok(addr_ptr as usize)
            }
        }
    }

    pub fn ensure_global_visibility(_library_name: &str) {}
}

// Linux / Android: use dlsym with RTLD_DEFAULT to search all loaded shared objects.
#[cfg(any(target_os = "linux", target_os = "android"))]
mod platform {
    use super::SymbolError;
    use std::ffi::{CStr, CString};
    use std::sync::Once;

    static PROMOTE_ONCE: Once = Once::new();

    /// Promotes the library's symbols to global visibility so that
    /// `dlsym(RTLD_DEFAULT, ...)` can find them.
    ///
    /// Unity on Linux may load `GameAssembly.so` with `RTLD_LOCAL`, confining
    /// its symbols to a private namespace. Re-opening with `RTLD_GLOBAL`
    /// makes them discoverable via `RTLD_DEFAULT`.
    pub fn ensure_global_visibility(library_name: &str) {
        PROMOTE_ONCE.call_once(|| {
            let path = match crate::memory::image::get_image_path(library_name) {
                Some(p) => p,
                None => {
                    crate::logger::warning(&format!(
                        "Could not find loaded library matching '{}'; symbol promotion skipped",
                        library_name
                    ));
                    return;
                }
            };

            let c_path = match CString::new(path.as_str()) {
                Ok(c) => c,
                Err(_) => return,
            };

            unsafe {
                let handle = libc::dlopen(
                    c_path.as_ptr(),
                    libc::RTLD_NOW | libc::RTLD_GLOBAL | libc::RTLD_NOLOAD,
                );
                if handle.is_null() {
                    let err = libc::dlerror();
                    let msg = if err.is_null() {
                        "unknown error".to_string()
                    } else {
                        CStr::from_ptr(err).to_string_lossy().into_owned()
                    };
                    crate::logger::warning(&format!(
                        "dlopen promotion failed for '{}': {}",
                        path, msg
                    ));
                } else {
                    crate::logger::info(&format!(
                        "Promoted '{}' symbols to global visibility",
                        path
                    ));
                }
            }
        });
    }

    pub fn raw_resolve(symbol: &str) -> Result<usize, SymbolError> {
        let c_str = CString::new(symbol).map_err(|_| SymbolError::StringError)?;
        unsafe {
            // Try RTLD_DEFAULT first (works when the library was loaded with GLOBAL)
            let addr_ptr = libc::dlsym(libc::RTLD_DEFAULT, c_str.as_ptr());
            if !addr_ptr.is_null() {
                return Ok(addr_ptr as usize);
            }
            // Fallback: Android 14+ linker may refuse to promote LOCAL symbols
            // even with RTLD_GLOBAL|RTLD_NOLOAD. Resolve via the target image's
            // own handle instead (handle intentionally not dlclose'd — the library
            // stays loaded for the process lifetime, so the symbol pointer remains valid).
            if let Some(target) = crate::init::TARGET_IMAGE_NAME.get() {
                let c_name = match CString::new(target.as_str()) {
                    Ok(c) => c,
                    Err(_) => return Err(SymbolError::NotFound(symbol.into())),
                };
                let handle = libc::dlopen(c_name.as_ptr(), libc::RTLD_NOLOAD);
                if !handle.is_null() {
                    let addr = libc::dlsym(handle, c_str.as_ptr());
                    if !addr.is_null() {
                        return Ok(addr as usize);
                    }
                }
            }
            Err(SymbolError::NotFound(symbol.into()))
        }
    }
}

// Windows: use GetProcAddress with a null module handle to search the main executable.
#[cfg(target_os = "windows")]
mod platform {
    use super::SymbolError;
    use std::ffi::CString;
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};

    pub fn raw_resolve(symbol: &str) -> Result<usize, SymbolError> {
        let c_str = CString::new(symbol).map_err(|_| SymbolError::StringError)?;
        unsafe {
            // Prefer GameAssembly.dll explicitly: il2cpp_* exports live there,
            // not in the host exe (GetModuleHandleA(NULL) only searches the exe).
            let game = CString::new("GameAssembly.dll").unwrap();
            let handle = GetModuleHandleA(game.as_ptr() as *const u8);
            if !handle.is_null() {
                if let Some(f) = GetProcAddress(handle, c_str.as_ptr() as *const u8) {
                    return Ok(f as usize);
                }
            }
            // Fallback: process-wide search (custom embeddings).
            let handle = GetModuleHandleA(std::ptr::null());
            if handle.is_null() {
                return Err(SymbolError::NotFound(symbol.into()));
            }
            let addr = GetProcAddress(handle, c_str.as_ptr() as *const u8);
            match addr {
                Some(f) => Ok(f as usize),
                None => Err(SymbolError::NotFound(symbol.into())),
            }
        }
    }

    pub fn ensure_global_visibility(_library_name: &str) {}
}

// Unsupported platforms: always return an error.
#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "linux",
    target_os = "android",
    target_os = "windows",
)))]
mod platform {
    use super::SymbolError;

    pub fn raw_resolve(symbol: &str) -> Result<usize, SymbolError> {
        Err(SymbolError::NotFound(format!(
            "{} (unsupported platform)",
            symbol
        )))
    }

    pub fn ensure_global_visibility(_library_name: &str) {}
}
