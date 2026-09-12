#![allow(clippy::missing_safety_doc)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

//! Runtime-first IL2CPP integration for Rust.
//!
//! `il2cpp-bridge-rs` is designed for code already running inside a process that
//! has Unity's IL2CPP runtime loaded. It resolves IL2CPP exports at runtime,
//! builds a metadata cache, and exposes higher-level wrappers for common tasks
//! such as:
//!
//! - locating assemblies and classes
//! - selecting and invoking methods
//! - reading fields and properties
//! - finding Unity objects in the scene
//! - dumping metadata into C#-like pseudo-code
//!
//! The crate does not hide the fact that IL2CPP integration is pointer-heavy and
//! runtime-sensitive. It aims to make the common paths easier and harder to
//! misuse, not to turn IL2CPP into a purely safe abstraction.
//!
//! # Recommended Flow
//!
//! 1. Call [`init`] once the target process has loaded IL2CPP.
//! 2. Use [`api::cache`] helpers to reach assemblies.
//! 3. Resolve [`structs::Class`] and [`structs::Method`] values from the cache.
//! 4. Prefer instance-bound lookups such as [`structs::Object::method`] for
//!    instance calls.
//! 5. Attach additional threads with [`api::Thread`] before doing runtime work
//!    outside the initialization callback.
//!
//! # Example
//!
//! ```no_run
//! use il2cpp_bridge_rs::{api, init};
//! use std::ffi::c_void;
//!
//! init("GameAssembly", || {
//!     let asm = api::cache::csharp();
//!     let player = asm
//!         .class("PlayerController")
//!         .expect("PlayerController should exist");
//!
//!     let method = player
//!         .method(("TakeDamage", ["System.Single"]))
//!         .expect("TakeDamage(float) should exist");
//!
//!     println!("Resolved {}::{} @ RVA 0x{:X}", player.name, method.name, method.rva);
//!
//!     if let Some(obj) = player.find_objects_of_type(false).into_iter().next() {
//!         let bound = obj
//!             .method(("TakeDamage", ["System.Single"]))
//!             .expect("instance method should exist");
//!
//!         let amount: f32 = 25.0;
//!         unsafe {
//!             let _: Result<(), _> =
//!                 bound.call(&[&amount as *const f32 as *mut c_void]);
//!         }
//!     }
//! });
//! ```
//!
//! This example is `no_run` because it depends on a live Unity IL2CPP runtime.
//! In normal usage, generated rustdoc should be treated as the canonical
//! signature-level reference, while the repository markdown guides explain
//! workflows and platform/runtime caveats.

pub mod api;
mod init;
pub mod logger;
pub mod memory;
pub mod structs;

pub use init::init;
pub use memory::symbol::promote_library_to_global;
