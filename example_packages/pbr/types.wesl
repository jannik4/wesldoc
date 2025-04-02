struct StandardMaterial {
    base_color: vec4<f32>,
    emissive: vec4<f32>,
    metallic: f32,
    roughness: f32,
    custom_data: CustomData,
}

@if(custom_data_value_index)
struct CustomData {
    value: f32,
    index: u32,
}

@if(!custom_data_value_index)
struct CustomData {
    position: vec3<f32>,
    normal: vec3<f32>,
}

