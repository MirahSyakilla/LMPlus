//! Unity Component system implementation
//!
//! This module contains wrappers for Unity's Component system:
//! - `GameObject`: The base class for all entities in Unity scenes
//! - `Transform`: Position, rotation and scale of an object
//! - `Component`: Base class for everything attached to GameObjects
//! - `UnityObject`: Base class for all objects that can be referenced
//! - Submodules for specific component types (Animation, Physics, Rendering, Scene)

pub mod animation;
pub mod core;
pub mod physics;
pub mod rendering;
pub mod scene;

// Re-exports for backward compatibility and convenience
pub use core::component::{Component, ComponentTrait};
pub use core::game_object::GameObject;
pub use core::mono_behaviour::MonoBehaviour;
pub use core::transform::Transform;
pub use core::unity_object::UnityObject;

pub use physics::collider::Collider;
pub use physics::rigidbody::Rigidbody;

pub use rendering::camera::Camera;
pub use rendering::renderer::Renderer;
pub use rendering::screen::Screen;

pub use animation::animator::Animator;

pub use scene::scene_management::SceneManager;
