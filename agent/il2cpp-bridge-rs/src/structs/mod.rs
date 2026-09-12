//! Metadata, object, collection, and wrapper types exposed by the crate.
//!
//! The types in this module are the main building blocks once initialization
//! has completed:
//!
//! - metadata wrappers such as [`Assembly`], [`Class`], [`Method`], and [`Field`]
//! - runtime object wrappers such as [`Object`] and [`GameObject`]
//! - Unity-oriented wrappers under [`components`]
//! - collection and math helpers used by higher-level APIs
//!
//! Prefer starting from [`crate::api::cache`] to discover assemblies and classes,
//! then move into these types for inspection, invocation, and object access.

/// Collection types (List, Array, Dictionary, String)
pub mod collections;
/// Unity Component wrappers (GameObject, Transform, etc.)
pub mod components;
/// Core IL2CPP structures (Class, Method, Image, etc.)
pub mod core;
/// Mathematical structures (Vector3, Quaternion, etc.)
pub mod math;
/// Pointer utilities
pub mod ptr;

// Re-exports for easier access
pub use collections::Il2cppString;
pub use components::GameObject;
pub use components::MonoBehaviour;
pub use components::Transform;
pub use core::{
    Arg, Assembly, Class, Coroutine, Field, Image, Method, Object, Property, Type, ValueType,
};
pub use math::{Color, Matrix2x2, Matrix3x3, Matrix4x4, Quaternion, Vector2, Vector3, Vector4};
pub use ptr::MutPtr;
