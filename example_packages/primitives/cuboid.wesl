//! Module documentation for the [`Cuboid`] primitive.
//!
//! This module provides a cuboid primitive with various constructors.
//! It includes constructors for creating a [`Cuboid`](Cuboid) from its size, corners, and length.
//!
//! # Examples
//!
//! ```
//! import primitives::cuboid;
//!
//! fn main() {
//!    // Create a cuboid from its size
//!    let cuboid1 = cuboid::create(2.0, 3.0, 4.0);    
//! }
//! ```

/// Cuboid primitive
///
/// This is a cuboid primitive that can be used in various applications.
struct Cuboid {
    half_size: vec3<f32>,
}

fn create(x_length: f32, y_length: f32, z_length: f32) -> Cuboid {
    return from_size(vec3<f32>(x_length, y_length, z_length));
}

@if(signed_int)
fn from_int(int: i32) -> Cuboid {
    var cuboid: Cuboid;
    cuboid.half_size = f32(int) / 2.0;
    return cuboid;
}

@else
fn from_int(int: u32) -> Cuboid {
    var cuboid: Cuboid;
    cuboid.half_size = f32(int) / 2.0;
    return cuboid;
}

@if(cuboid_all_constructors)
fn from_size(size: Vec3) -> Cuboid {
    var cuboid: Cuboid;
    cuboid.half_size = size / 2.0;
    return cuboid;
}

@if(cuboid_all_constructors)
fn from_corners(point1: Vec3, point2: Vec3) -> Cuboid {
    var cuboid: Cuboid;
    cuboid.half_size = abs(point1 - point2) / 2.0;
    return cuboid;
}

@if(cuboid_all_constructors && (magic_flag || magic_flag2))
fn from_length(length: f32) -> Cuboid {
    var cuboid: Cuboid;
    cuboid.half_size = vec3<f32>(length / 2.0);
    return cuboid;
}
