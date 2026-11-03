struct SceneUniforms {
    mvp: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> scene: SceneUniforms;

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) gl_clip: vec4<f32>,
}

@vertex
fn vs_mesh(@location(0) position: vec3<f32>) -> VertexOut {
    let gl_clip = scene.mvp * vec4<f32>(position, 1.0);
    var out: VertexOut;
    // glam's perspective uses OpenGL clip z (-1 to 1). wgpu clips z from 0 to 1.
    out.gl_clip = gl_clip;
    out.clip = vec4<f32>(gl_clip.x, gl_clip.y, 0.5 * (gl_clip.z + gl_clip.w), gl_clip.w);
    return out;
}

@fragment
fn fs_depth(input: VertexOut) -> @location(0) vec4<f32> {
    let depth = 0.5 + 0.5 * input.gl_clip.z / input.gl_clip.w;
    let value = 1.0 - depth;
    return vec4<f32>(value, value, value, 1.0);
}
