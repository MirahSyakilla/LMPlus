//! IL2CPP thread attachment and lifecycle helpers.
use super::super::api;
#[cfg(debug_assertions)]
use crate::logger;
use std::ffi::c_void;
use std::ptr;

/// Wrapper around an IL2CPP VM thread attachment.
///
/// Use this type whenever your code touches IL2CPP from a thread that was not
/// created by the crate's initialization flow. The common pattern is scoped
/// attachment via [`Thread::attach`] with `auto_detach = true`.
pub struct Thread {
    /// Pointer to the internal IL2CPP thread
    thread_ptr: *mut c_void,
    /// Whether to automatically detach the thread on drop
    auto_detach: bool,
}

impl Thread {
    /// Creates a Thread wrapper from a raw pointer
    ///
    /// # Arguments
    /// * `thread_ptr` - The raw IL2CPP thread pointer
    /// * `auto_detach` - Whether to detach the thread when this struct is dropped
    ///
    /// # Safety
    /// * `thread_ptr` must be a valid IL2CPP thread pointer.
    pub unsafe fn from_ptr(thread_ptr: *mut c_void, auto_detach: bool) -> Self {
        Self {
            thread_ptr,
            auto_detach,
        }
    }

    /// Returns the raw pointer to the thread
    pub fn as_ptr(&self) -> *mut c_void {
        self.thread_ptr
    }

    /// Manually detaches the thread from the IL2CPP domain
    ///
    /// This will detach the thread immediately and prevent auto-detachment on drop.
    pub fn detach(mut self) {
        if !self.thread_ptr.is_null() {
            unsafe {
                api::thread_detach(self.thread_ptr);
            }
            #[cfg(debug_assertions)]
            logger::info("Thread manually detached from IL2CPP");
            self.thread_ptr = ptr::null_mut();
        }
        self.auto_detach = false;
    }

    /// Checks if the thread is a VM thread
    pub fn is_vm_thread(&self) -> bool {
        if self.thread_ptr.is_null() {
            return false;
        }
        unsafe { api::is_vm_thread(self.thread_ptr) }
    }

    /// Gets the current attached thread
    ///
    /// # Returns
    /// * `Option<Self>` - The current thread wrapper if attached, or None
    pub fn current() -> Option<Self> {
        unsafe {
            let current = api::thread_current();
            if !current.is_null() {
                Some(Self::from_ptr(current, false))
            } else {
                None
            }
        }
    }

    /// Attaches the current OS thread to the IL2CPP domain.
    ///
    /// If the thread is already attached, the existing thread handle is reused.
    /// When `auto_detach` is `true`, dropping the returned value detaches the
    /// thread automatically.
    ///
    /// Returns `None` if the IL2CPP domain could not be resolved or the runtime
    /// rejected the attachment request.
    pub fn attach(auto_detach: bool) -> Option<Self> {
        unsafe {
            if Self::is_attached() {
                #[cfg(debug_assertions)]
                logger::info("Thread already attached to IL2CPP, returning existing thread");
                return Self::current();
            }

            let domain_ptr = api::domain_get();

            if domain_ptr.is_null() {
                #[cfg(debug_assertions)]
                logger::error("Failed to get IL2CPP domain for thread attachment");
                return None;
            }

            let thread_ptr = api::thread_attach(domain_ptr);

            if thread_ptr.is_null() {
                #[cfg(debug_assertions)]
                logger::error("Failed to attach thread to IL2CPP");
                None
            } else {
                #[cfg(debug_assertions)]
                logger::info("Thread successfully attached to IL2CPP");
                Some(Self::from_ptr(thread_ptr, auto_detach))
            }
        }
    }

    /// Checks if the current thread is attached to the IL2CPP domain
    pub fn is_attached() -> bool {
        unsafe {
            let current = api::thread_current();
            !current.is_null()
        }
    }

    /// Gets all attached threads
    ///
    /// # Returns
    /// * `Vec<Thread>` - A list of all threads currently attached to the IL2CPP domain
    pub fn all() -> Vec<Thread> {
        unsafe {
            let mut size: usize = 0;
            let threads_ptr = api::thread_get_all_attached_threads(&mut size as *mut usize);

            if threads_ptr.is_null() || size == 0 {
                return Vec::new();
            }

            let mut threads = Vec::with_capacity(size);
            for i in 0..size {
                let thread_ptr = *threads_ptr.add(i);
                if !thread_ptr.is_null() {
                    threads.push(Thread::from_ptr(thread_ptr, false));
                }
            }

            threads
        }
    }
}

impl Drop for Thread {
    /// Detaches the thread from the IL2CPP domain if auto-detach is enabled
    fn drop(&mut self) {
        if self.auto_detach && !self.thread_ptr.is_null() {
            unsafe {
                api::thread_detach(self.thread_ptr);
            }
            #[cfg(debug_assertions)]
            logger::info("Thread automatically detached from IL2CPP");
            self.thread_ptr = ptr::null_mut();
        }
    }
}

unsafe impl Send for Thread {}
unsafe impl Sync for Thread {}
