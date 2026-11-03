@group(0) @binding(0) var source_tex: texture_2d<f32>;
@group(0) @binding(1) var source_samp: sampler;

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_blit(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let pos = positions[vertex_index];
    var out: VertexOut;
    out.clip = vec4<f32>(pos, 0.0, 1.0);
    // Texture rows start at the top. Stereogram uv.y = 0 is the bottom, same as WebGL.
    out.uv = vec2<f32>(0.5 + 0.5 * pos.x, 0.5 - 0.5 * pos.y);
    return out;
}

fn srgb_channel_to_linear(channel: f32) -> f32 {
    if (channel <= 0.04045) {
        return channel / 12.92;
    }
    return pow((channel + 0.055) / 1.055, 2.4);
}

@fragment
fn fs_blit(input: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(source_tex, source_samp, input.uv);
}

// The swapchain is sRGB, so it encodes on store. Stereogram values are already
// display bytes, the same as the WebGL canvas. Convert before writing.
@fragment
fn fs_blit_srgb(input: VertexOut) -> @location(0) vec4<f32> {
    let color = textureSample(source_tex, source_samp, input.uv);
    return vec4<f32>(
        srgb_channel_to_linear(color.r),
        srgb_channel_to_linear(color.g),
        srgb_channel_to_linear(color.b),
        color.a,
    );
}
