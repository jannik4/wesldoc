struct FragmentInput {
    @location(0) blend_color: vec4<f32>,
}

@fragment
fn fragment(input: FragmentInput) -> @location(0) vec4<f32> {
    return input.blend_color;
}
