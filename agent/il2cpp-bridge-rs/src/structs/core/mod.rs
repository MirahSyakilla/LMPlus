//! Core metadata and object wrappers.
//!
//! These types model the most important IL2CPP concepts used by the crate:
//!
//! - assemblies and images
//! - classes, methods, fields, and properties
//! - managed objects and value types
//! - coroutine handles
//!
//! They are re-exported at [`crate::structs`] for a flatter public API.

pub mod hierarchy;
pub mod members;
pub mod metadata;
pub mod runtime;

// Re-export types from submodules for flat access
pub use hierarchy::class::{Class, MethodSelector};
pub use hierarchy::object::{Il2cppObject, Object};
pub use hierarchy::r#type::Type;
pub use hierarchy::value_type::ValueType;

pub use members::field::Field;
pub use members::method::{Arg, Method};
pub use members::property::Property;

pub use metadata::assembly::Assembly;
pub use metadata::image::Image;

pub use runtime::coroutine::Coroutine;
