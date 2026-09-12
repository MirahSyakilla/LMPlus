//! Math structs and functions using glam
pub use glam::*;

pub type Vector2 = Vec2;
pub type Vector3 = Vec3;
pub type Vector4 = Vec4;
pub type Quaternion = Quat;
pub type Matrix4x4 = Mat4;
pub type Matrix3x3 = Mat3;
pub type Matrix2x2 = Mat2;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Red component (0.0 - 1.0)
    pub r: f32,
    /// Green component (0.0 - 1.0)
    pub g: f32,
    /// Blue component (0.0 - 1.0)
    pub b: f32,
    /// Alpha component (0.0 - 1.0)
    pub a: f32,
}

impl Color {
    /// Creates a new color
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    pub const RED: Self = Self::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Self = Self::new(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Self = Self::new(0.0, 0.0, 1.0, 1.0);
    pub const YELLOW: Self = Self::new(1.0, 1.0, 0.0, 1.0);
    pub const CYAN: Self = Self::new(0.0, 1.0, 1.0, 1.0);
    pub const MAGENTA: Self = Self::new(1.0, 0.0, 1.0, 1.0);
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);
}

impl From<Vec4> for Color {
    fn from(v: Vec4) -> Self {
        Self::new(v.x, v.y, v.z, v.w)
    }
}

impl From<Color> for Vec4 {
    fn from(c: Color) -> Self {
        Vec4::new(c.r, c.g, c.b, c.a)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// Center of the bounding box
    pub center: Vector3,
    /// Extents of the bounding box (half size)
    pub extents: Vector3,
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            center: Vector3::ZERO,
            extents: Vector3::ZERO,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    /// Origin point of the ray
    pub origin: Vector3,
    /// Direction vector of the ray
    pub direction: Vector3,
}

impl Default for Ray {
    fn default() -> Self {
        Self {
            origin: Vector3::ZERO,
            direction: Vector3::ZERO,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// X coordinate of the top-left corner
    pub x: f32,
    /// Y coordinate of the top-left corner
    pub y: f32,
    /// Width of the rectangle
    pub width: f32,
    /// Height of the rectangle
    pub height: f32,
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        }
    }
}

impl Rect {
    /// Creates a new rectangle
    ///
    /// # Arguments
    /// * `x` - The X coordinate of the top-left corner
    /// * `y` - The Y coordinate of the top-left corner
    /// * `width` - The width of the rectangle
    /// * `height` - The height of the rectangle
    ///
    /// # Returns
    /// * `Rect` - The new rectangle
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Plane {
    /// Normal vector of the plane
    pub normal: Vector3,
    /// Distance from the origin to the plane
    pub distance: f32,
}

impl Default for Plane {
    fn default() -> Self {
        Self {
            normal: Vector3::ZERO,
            distance: 0.0,
        }
    }
}

impl Plane {
    /// Creates a new plane
    ///
    /// # Arguments
    /// * `normal` - The normal vector of the plane
    /// * `distance` - The distance from the origin to the plane
    ///
    /// # Returns
    /// * `Plane` - The new plane
    pub fn new(normal: Vector3, distance: f32) -> Self {
        Self { normal, distance }
    }
}

use std::ffi::c_void;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaycastHit {
    /// The impact point in world space where the ray hit the collider
    pub point: Vector3,
    /// The normal of the surface the ray hit
    pub normal: Vector3,
    /// The barycentric coordinate of the triangle we hit
    pub barycentric_coordinate: Vector3,
    /// The distance from the ray's origin to the impact point
    pub distance: f32,
    /// The index of the triangle that was hit
    pub triangle_index: i32,
    /// The uv texture coordinate at the collision location
    pub texture_coord: Vector2,
    /// The secondary uv texture coordinate at the impact point
    pub texture_coord2: Vector2,
    /// The uv lightmap coordinate at the impact point
    pub lightmap_coord: Vector2,
    /// The Collider that was hit
    pub collider: *mut c_void,
    /// The Rigidbody of the collider that was hit
    pub rigidbody: *mut c_void,
    /// The ArticulationBody of the collider that was hit
    pub articulation_body: *mut c_void,
    /// The Transform of the rigidbody or collider that was hit
    pub transform: *mut c_void,
    /// EntityId of the Collider that was hit
    pub collider_entity_id: u32,
}

impl Default for RaycastHit {
    fn default() -> Self {
        Self {
            point: Vector3::ZERO,
            normal: Vector3::ZERO,
            barycentric_coordinate: Vector3::ZERO,
            distance: 0.0,
            triangle_index: 0,
            texture_coord: Vector2::ZERO,
            texture_coord2: Vector2::ZERO,
            lightmap_coord: Vector2::ZERO,
            collider: std::ptr::null_mut(),
            rigidbody: std::ptr::null_mut(),
            articulation_body: std::ptr::null_mut(),
            transform: std::ptr::null_mut(),
            collider_entity_id: 0,
        }
    }
}
