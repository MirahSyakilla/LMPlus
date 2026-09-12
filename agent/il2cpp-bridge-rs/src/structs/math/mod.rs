//! Math module exports
#[allow(clippy::module_inception)]
pub mod math;

pub use math::{
    Bounds, Color, Matrix2x2, Matrix3x3, Matrix4x4, Plane, Quaternion, Ray, RaycastHit, Rect,
    Vector2, Vector3, Vector4,
};
