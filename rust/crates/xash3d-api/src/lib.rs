//! Common definitions, FFI types, and math routines for Xash3D.

use std::ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// A 3D vector representing coordinates or velocities in GoldSrc units (inches).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vector3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };
    pub const UP: Self = Self { x: 0.0, y: 0.0, z: 1.0 };

    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn from_slice(slice: &[f32; 3]) -> Self {
        Self { x: slice[0], y: slice[1], z: slice[2] }
    }

    #[inline]
    pub fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    #[inline]
    pub fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    #[inline]
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    #[inline]
    pub fn normalize(self) -> Self {
        let len = self.length();
        if len > 1e-6 {
            self * (1.0 / len)
        } else {
            Self::ZERO
        }
    }
}

impl Add for Vector3 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self { x: self.x + rhs.x, y: self.y + rhs.y, z: self.z + rhs.z }
    }
}

impl AddAssign for Vector3 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl Sub for Vector3 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self { x: self.x - rhs.x, y: self.y - rhs.y, z: self.z - rhs.z }
    }
}

impl SubAssign for Vector3 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}

impl Mul<f32> for Vector3 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self { x: self.x * rhs, y: self.y * rhs, z: self.z * rhs }
    }
}

impl MulAssign<f32> for Vector3 {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
        self.z *= rhs;
    }
}

impl Neg for Vector3 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self { x: -self.x, y: -self.y, z: -self.z }
    }
}

/// Plane description matching GoldSrc `mplane_t` layout passed via FFI.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PlaneFfi {
    pub normal: [f32; 3],
    pub dist: f32,
    pub plane_type: byte_t,
    pub signbits: byte_t,
    pub pad: [byte_t; 2],
}

#[allow(non_camel_case_types)]
pub type byte_t = u8;

/// Brush description matching GoldSrc brush structure passed via FFI.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct BrushFfi {
    pub first_plane: u32,
    pub num_planes: u32,
    pub mins: [f32; 3],
    pub maxs: [f32; 3],
    pub contents: i32,
}

/// Trace result returned to the C engine.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TraceResultFfi {
    pub allsolid: i32,
    pub startsolid: i32,
    pub inopen: i32,
    pub inwater: i32,
    pub fraction: f32,
    pub endpos: [f32; 3],
    pub plane_normal: [f32; 3],
    pub plane_dist: f32,
    pub hit_ent: i32,
}

/// User command flags and buttons matching GoldSrc headers.
pub const IN_ATTACK: i32 = 1 << 0;
pub const IN_JUMP: i32 = 1 << 1;
pub const IN_DUCK: i32 = 1 << 2;
pub const IN_FORWARD: i32 = 1 << 3;
pub const IN_BACK: i32 = 1 << 4;
pub const IN_USE: i32 = 1 << 5;
pub const IN_CANCEL: i32 = 1 << 6;
pub const IN_LEFT: i32 = 1 << 7;
pub const IN_RIGHT: i32 = 1 << 8;
pub const IN_MOVELEFT: i32 = 1 << 9;
pub const IN_MOVERIGHT: i32 = 1 << 10;
pub const IN_ATTACK2: i32 = 1 << 11;
pub const IN_RUN: i32 = 1 << 12;
pub const IN_RELOAD: i32 = 1 << 13;
pub const IN_ALT1: i32 = 1 << 14;
pub const IN_SCORE: i32 = 1 << 15;

/// Edict / Entity flags.
pub const FL_ONGROUND: i32 = 1 << 9;
pub const FL_DUCKING: i32 = 1 << 14;

/// Shape types for dynamic bodies.
pub const SHAPE_BOX: i32 = 0;
pub const SHAPE_SPHERE: i32 = 1;
pub const SHAPE_CAPSULE: i32 = 2;
pub const SHAPE_CONVEX: i32 = 3;
pub const SHAPE_MESH: i32 = 4;
