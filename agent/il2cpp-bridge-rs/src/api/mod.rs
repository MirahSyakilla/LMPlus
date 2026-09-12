//! User-facing IL2CPP runtime access.
//!
//! This module re-exports the crate's primary runtime entry points:
//!
//! - [`cache`] for assembly and metadata lookup
//! - [`Thread`] for VM thread attachment
//! - [`invoke_method`] for low-level method invocation
//! - dump helpers such as [`dump`] and [`dump_all_to`]
//! - thin Unity wrappers such as [`Application`] and [`Time`]
//!
//! Most integrations should start here after calling [`crate::init`]. When you
//! need richer metadata or object wrappers, move into [`crate::structs`].

pub mod core;
pub use core::api::*;
pub use core::cache;
pub use core::caller::invoke_method;
pub use core::internals::{self, Internals};
pub use core::thread::Thread;

pub mod debugging;

pub use debugging::cs::{dump, dump_all, dump_all_to, dump_assembly, dump_to};

pub mod wrappers;
pub use wrappers::{application::Application, time::Time};
