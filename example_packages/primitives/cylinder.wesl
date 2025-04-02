struct Cylinder {
    radius: f32,
    half_height: f32,
}

fn new(radius: f32, height: f32) -> Cylinder {
    var cylinder: Cylinder;
    cylinder.radius = radius;
    cylinder.half_height = height / 2.0;
    return cylinder;
}
