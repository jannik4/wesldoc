struct Cylinder {
    /// The radius of the cylinder.
    radius: f32,
    /// The half height of the cylinder.
    half_height: f32,
}

fn create(radius: f32, height: f32) -> Cylinder {
    var cylinder: Cylinder;
    cylinder.radius = radius;
    cylinder.half_height = height / 2.0;
    return cylinder;
}
