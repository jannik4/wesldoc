@id(0) override has_directional_light: bool = true;

@id(100) override has_spot_light: bool = true;

/// The magnitude of the speed. This must be overriden.
override speed_magnitude: f32;
override direction: vec3;

/// The speed. This is computed from the [`speed_magnitude`] and [`direction`].
override speed = speed_magnitude * direction;

