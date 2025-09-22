struct Fog {
    base_color: vec4<f32>,
    @if(emissive)
    emissive: vec4<f32>,
    @if(fog_flags_unsigned)
    flags: u32,
    @elif(fog_flags_signed)
    flags: i32,
    @else
    flags: bool,
}

// Empty lines to make this page scrollable in the browser









// ...









// ...









// ...









// ...









// ...









// ...



@if(linear_fog)
fn linear_fog(
    fog: Fog,
    input_color: vec4<f32>,
    distance: f32,
    @if(fog_extra_flags)
    extra_flags: u32,
) -> vec4<f32> {
    return mix(input_color, fog.base_color, distance);
}
