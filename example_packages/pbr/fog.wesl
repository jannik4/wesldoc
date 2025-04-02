struct Fog {
    base_color: vec4<f32>,
    @if(fog_flags)
    flags: u32,
}

@if(linear_fog)
fn linear_fog(
    fog: Fog,
    input_color: vec4<f32>,
    distance: f32,
) -> vec4<f32> {
    return mix(input_color, fog.base_color, distance);
}
