use dashmap::DashMap;
use once_cell::sync::Lazy;

static CACHE: Lazy<DashMap<String, usize>> = Lazy::new(DashMap::new);

/// Returns the base address of the image with the given name, if loaded.
///
/// Results are cached after the first successful lookup.
pub fn get_image_base(name: &str) -> Option<usize> {
    if let Some(entry) = CACHE.get(name) {
        return Some(*entry);
    }

    let addr = platform::find_image_base(name)?;
    CACHE.insert(name.into(), addr);
    Some(addr)
}

/// Returns the full filesystem path of the loaded image matching `name`, if found.
///
/// On Linux/Android this walks loaded shared objects via `dl_iterate_phdr`.
/// On other platforms this always returns `None`.
pub fn get_image_path(name: &str) -> Option<String> {
    platform::find_image_path(name)
}

// macOS / iOS: iterate loaded dylibs via dyld to find the matching Mach-O header.
#[cfg(any(target_os = "macos", target_os = "ios"))]
mod platform {
    use mach2::dyld::{_dyld_get_image_header, _dyld_get_image_name, _dyld_image_count};
    use std::ffi::CStr;

    pub fn find_image_base(name: &str) -> Option<usize> {
        unsafe {
            let count = _dyld_image_count();
            for i in 0..count {
                let c_name = _dyld_get_image_name(i);
                if c_name.is_null() {
                    continue;
                }
                let path = CStr::from_ptr(c_name).to_string_lossy();
                if path.contains(name) {
                    let header = _dyld_get_image_header(i);
                    if !header.is_null() {
                        return Some(header as usize);
                    }
                }
            }
        }
        None
    }

    pub fn find_image_path(_name: &str) -> Option<String> {
        None
    }
}

// Linux / Android: iterate loaded shared objects via dl_iterate_phdr to find the matching base address.
#[cfg(any(target_os = "linux", target_os = "android"))]
mod platform {
    use std::ffi::CStr;

    pub fn find_image_base(name: &str) -> Option<usize> {
        struct CallbackData {
            name: String,
            result: Option<usize>,
        }

        unsafe extern "C" fn callback(
            info: *mut libc::dl_phdr_info,
            _size: libc::size_t,
            data: *mut libc::c_void,
        ) -> libc::c_int {
            let data = &mut *(data as *mut CallbackData);
            let dlpi_name = (*info).dlpi_name;
            if dlpi_name.is_null() {
                return 0;
            }
            let path = CStr::from_ptr(dlpi_name).to_string_lossy();
            if path.contains(&data.name) {
                data.result = Some((*info).dlpi_addr as usize);
                return 1;
            }
            0
        }

        let mut data = CallbackData {
            name: name.to_string(),
            result: None,
        };

        unsafe {
            libc::dl_iterate_phdr(Some(callback), &mut data as *mut _ as *mut libc::c_void);
        }

        data.result
    }

    pub fn find_image_path(name: &str) -> Option<String> {
        struct CallbackData {
            name: String,
            result: Option<String>,
        }

        unsafe extern "C" fn callback(
            info: *mut libc::dl_phdr_info,
            _size: libc::size_t,
            data: *mut libc::c_void,
        ) -> libc::c_int {
            let data = &mut *(data as *mut CallbackData);
            let dlpi_name = (*info).dlpi_name;
            if dlpi_name.is_null() {
                return 0;
            }
            let path = CStr::from_ptr(dlpi_name).to_string_lossy();
            if path.contains(&data.name) {
                data.result = Some(path.into_owned());
                return 1;
            }
            0
        }

        let mut data = CallbackData {
            name: name.to_string(),
            result: None,
        };

        unsafe {
            libc::dl_iterate_phdr(Some(callback), &mut data as *mut _ as *mut libc::c_void);
        }

        data.result
    }
}

// Windows: use GetModuleHandleA to retrieve the base address of a loaded module by name.
#[cfg(target_os = "windows")]
mod platform {
    use std::ffi::CString;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleA;

    pub fn find_image_base(name: &str) -> Option<usize> {
        let c_name = CString::new(name).ok()?;
        unsafe {
            let handle = GetModuleHandleA(c_name.as_ptr() as *const u8);
            if handle.is_null() {
                None
            } else {
                Some(handle as usize)
            }
        }
    }

    pub fn find_image_path(_name: &str) -> Option<String> {
        None
    }
}

// Unsupported platforms: always return None.
#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "linux",
    target_os = "android",
    target_os = "windows",
)))]
mod platform {
    pub fn find_image_base(_name: &str) -> Option<usize> {
        None
    }

    pub fn find_image_path(_name: &str) -> Option<String> {
        None
    }
}
