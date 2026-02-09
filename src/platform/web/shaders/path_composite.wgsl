// Path compositing shader for GPUI web renderer.
// Samples from the intermediate path texture and composites onto the main target.
// Each instance is a rectangular sprite covering a path's bounds.

struct Globals {
    viewport_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;

struct PathSprite {
    bounds_origin: vec2<f32>,
    bounds_size: vec2<f32>,
}

@group(0) @binding(1) var<storage, read> sprites: array<PathSprite>;
@group(0) @binding(2) var t_intermediate: texture_2d<f32>;
@group(0) @binding(3) var s_intermediate: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coords: vec2<f32>,
}

const UNIT_QUAD: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(0.0, 1.0),
    vec2<f32>(0.0, 1.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(1.0, 1.0),
);

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let sprite = sprites[instance_index];
    let unit_pos = UNIT_QUAD[vertex_index];

    let pixel_pos = sprite.bounds_origin + unit_pos * sprite.bounds_size;

    let ndc = vec2<f32>(
        pixel_pos.x / globals.viewport_size.x * 2.0 - 1.0,
        1.0 - pixel_pos.y / globals.viewport_size.y * 2.0,
    );

    // Map screen position to texture UV coordinates
    let texture_coords = pixel_pos / globals.viewport_size;

    var output: VertexOutput;
    output.position = vec4<f32>(ndc, 0.0, 1.0);
    output.texture_coords = texture_coords;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_intermediate, s_intermediate, input.texture_coords);
}
