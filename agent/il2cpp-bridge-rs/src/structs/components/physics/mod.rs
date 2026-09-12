//! Unity Physics system wrappers
//!
//! This module contains wrappers for Unity's Physics system:
//! - `Physics`: Global physics helper methods (Raycasts, etc.)
//! - `Rigidbody`: Physics body component
//! - `Collider`: Base class for physics colliders
//! - `LayerMask`: Bitmask for physics layers

pub mod collider;
pub mod layer_mask;
#[allow(clippy::module_inception)]
pub mod physics;
pub mod rigidbody;

pub use collider::Collider;
pub use layer_mask::LayerMask;
pub use physics::Physics;
pub use rigidbody::{ForceMode, Rigidbody};
