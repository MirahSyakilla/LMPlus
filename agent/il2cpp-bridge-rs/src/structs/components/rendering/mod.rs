//! Unity Rendering system wrappers
//!
//! This module contains wrappers for Unity's Rendering system:
//! - `Camera`: Controls what is rendered to screen
//! - `Renderer`: Base class for all renderers
//! - `Material`: Defines how an object is rendered
//! - `Shader`: Source code for shading (vertex/fragment)
//! - `Screen`: Access to display information

pub mod camera;
pub mod material;
pub mod renderer;
pub mod screen;
pub mod shader;

pub use camera::Camera;
pub use material::Material;
pub use renderer::Renderer;
pub use screen::Screen;
pub use shader::Shader;
