import package::constants;
import package::constants::PI;

fn ease_in_cubic(t: f32) -> f32 {
    return t * t * t;
}

fn ease_in_quadratic(t: f32) -> f32 {
    return t * t;
}

fn ease_in_sine(t: f32) -> f32 {
    return 1.0 - cos(t * constants::HALF_PI);
}
