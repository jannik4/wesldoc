import package::constants::PI;
import primitives::{cuboid::Cuboid, cylinder::Cylinder};

/// Calculates the volume of a [`Cuboid`].
fn cuboid_volume(cuboid: Cuboid) -> f32 {
    return cuboid.half_size.x * cuboid.half_size.y * cuboid.half_size.z * 8.0;
}

/// Calculates the volume of a [`Cylinder`].
fn cylinder_volume(cylinder: Cylinder) -> f32 {
    let base_area = PI * cylinder.radius * cylinder.radius;
    return base_area * cylinder.half_height * 2.0;
}
