struct Params {
    tile_color: f32,
    tile_height: f32,
    tile_scale_x: f32,
    tile_scale_y: f32,
    show_uv: f32,
    main_stripe: f32,
    depth_factor: f32,
    invert: f32,
    heightmap_scale_x: f32,
    heightmap_scale_y: f32,
    stripes_count: f32,
    show_heightmap: f32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var heightmap_tex: texture_2d<f32>;
@group(0) @binding(2) var heightmap_samp: sampler;
@group(0) @binding(3) var tile_tex: texture_2d<f32>;
@group(0) @binding(4) var tile_samp: sampler;

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_fullscreen(@builtin(vertex_index) vertex_index: u32) -> VertexOut {
    // One triangle covering the viewport. uv.y = 0 stays at the bottom, as in WebGL.
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let pos = positions[vertex_index];
    var out: VertexOut;
    out.clip = vec4<f32>(pos, 0.0, 1.0);
    out.uv = vec2<f32>(0.5 + 0.5 * pos.x, 0.5 + 0.5 * pos.y);
    return out;
}

fn sample_heightmap(position: vec2<f32>) -> f32 {
    let scaled = vec2<f32>(0.5, 0.5) + (position - vec2<f32>(0.5, 0.5)) * vec2<f32>(params.heightmap_scale_x, params.heightmap_scale_y);
    if (scaled.x >= 0.0 && scaled.x <= 1.0 && scaled.y >= 0.0 && scaled.y <= 1.0) {
        let texel = textureSampleLevel(heightmap_tex, heightmap_samp, scaled, 0.0);
        var value = dot(vec3<f32>(1.0 / 3.0), texel.rgb);
        value = mix(value, 1.0 - value, params.invert);
        return params.depth_factor * value;
    }
    return 0.0;
}

fn compute_tile_uv(position_in: vec2<f32>) -> vec2<f32> {
    var position = position_in;
    let stripes = max(params.stripes_count, 1.0);
    let main_width = 1.0 - sample_heightmap(vec2<f32>((params.main_stripe + 0.5) / (stripes + 1.0), position.y)) * 0.45;
    let main_left = params.main_stripe + 0.5 * (1.0 - main_width);
    let main_right = params.main_stripe + 1.0 - 0.5 * (1.0 - main_width);

    position.x = position.x * (stripes + 1.0);

    // 48 covers the web shader maximum: ceil(2 * 24).
    for (var i = 0; i < 48; i++) {
        if (f32(i) >= ceil(2.0 * stripes)) {
            break;
        }
        if (position.x < main_left) {
            let previous = vec2<f32>((position.x - 0.25) / stripes, position.y);
            position.x = position.x + 1.0 - sample_heightmap(previous) * 0.45;
        } else if (position.x >= main_right) {
            let previous = vec2<f32>((position.x - 1.0) / stripes, position.y);
            position.x = position.x - (1.0 - sample_heightmap(previous) * 0.45);
        } else {
            break;
        }
    }

    position.x = (position.x - main_left) / max(main_width, 0.0001);
    position.y = fract(position.y / max(params.tile_height, 0.0001));
    return position;
}

fn sample_tile(coords_in: vec2<f32>) -> vec4<f32> {
    let coords = vec2<f32>(0.5, 0.5) + (coords_in - vec2<f32>(0.5, 0.5)) * vec2<f32>(params.tile_scale_x, params.tile_scale_y);
    return textureSampleLevel(tile_tex, tile_samp, coords, 0.0);
}

@fragment
fn fs_stereogram(input: VertexOut) -> @location(0) vec4<f32> {
    if (params.show_heightmap > 0.5) {
        let value = sample_heightmap(input.uv);
        return vec4<f32>(value, value, value, 1.0);
    }

    let tile_uv = compute_tile_uv(input.uv);
    let tile_sample = sample_tile(tile_uv);
    let mono = vec4<f32>(tile_sample.r, tile_sample.r, tile_sample.r, 1.0);
    let color = mix(mono, tile_sample, params.tile_color);
    let color_uv = vec4<f32>(tile_uv, 0.0, 1.0);
    return mix(color, color_uv, params.show_uv);
}
