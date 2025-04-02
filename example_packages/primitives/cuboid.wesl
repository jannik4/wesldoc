struct Cuboid {
    half_size: vec3<f32>,
}

fn new(x_length: f32, y_length: f32, z_length: f32) -> Cuboid {
    return from_size(vec3<f32>(x_length, y_length, z_length));
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

@if(cuboid_all_constructors)
fn from_length(length: f32) -> Cuboid {
    var cuboid: Cuboid;
    cuboid.half_size = vec3<f32>(length / 2.0);
    return cuboid;
}
