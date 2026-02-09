// Underline shader for GPUI web renderer.
// Renders solid or wavy underlines with content masking.

struct Globals {
    viewport_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;

struct UnderlineInstance {
    bounds_origin: vec2<f32>,
    bounds_size: vec2<f32>,
    clip_origin: vec2<f32>,
    clip_size: vec2<f32>,
    color: vec4<f32>,
    thickness: f32,
    wavy: u32,
}

@group(0) @binding(1) var<storage, read> underlines: array<UnderlineInstance>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) @interpolate(flat) instance_index: u32,
}

const UNIT_QUAD: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(0.0, 1.0),
    vec2<f32>(0.0, 1.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(1.0, 1.0),
);

const M_PI_F: f32 = 3.1415926;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let underline = underlines[instance_index];
    let unit_pos = UNIT_QUAD[vertex_index];

    let pixel_pos = underline.bounds_origin + unit_pos * underline.bounds_size;

    let ndc = vec2<f32>(
        pixel_pos.x / globals.viewport_size.x * 2.0 - 1.0,
        1.0 - pixel_pos.y / globals.viewport_size.y * 2.0,
    );

    var output: VertexOutput;
    output.position = vec4<f32>(ndc, 0.0, 1.0);
    output.local_pos = unit_pos * underline.bounds_size;
    output.instance_index = instance_index;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let underline = underlines[input.instance_index];

    // Content mask clipping
    let pixel_pos = underline.bounds_origin + input.local_pos;
    let clip_min = underline.clip_origin;
    let clip_max = underline.clip_origin + underline.clip_size;
    if pixel_pos.x < clip_min.x || pixel_pos.x > clip_max.x ||
       pixel_pos.y < clip_min.y || pixel_pos.y > clip_max.y {
        discard;
    }

    let color = underline.color;

    // Solid underline: just output the color
    if (underline.wavy & 0xFFu) == 0u {
        let final_alpha = color.a;
        return vec4<f32>(color.rgb * final_alpha, final_alpha);
    }

    // Wavy underline: compute distance to sine wave
    const WAVE_FREQUENCY: f32 = 2.0;
    const WAVE_HEIGHT_RATIO: f32 = 0.8;

    let half_thickness = underline.thickness * 0.5;

    // st: normalized position within bounds, centered vertically
    let st = (pixel_pos - underline.bounds_origin) / underline.bounds_size.y - vec2<f32>(0.0, 0.5);
    let frequency = M_PI_F * WAVE_FREQUENCY * underline.thickness / underline.bounds_size.y;
    let amplitude = (underline.thickness * WAVE_HEIGHT_RATIO) / underline.bounds_size.y;

    let sine = sin(st.x * frequency) * amplitude;
    let d_sine = cos(st.x * frequency) * amplitude * frequency;
    let distance = (st.y - sine) / sqrt(1.0 + d_sine * d_sine);
    let distance_in_pixels = distance * underline.bounds_size.y;
    let distance_from_top_border = distance_in_pixels - half_thickness;
    let distance_from_bottom_border = distance_in_pixels + half_thickness;
    let alpha = saturate(0.5 - max(-distance_from_bottom_border, distance_from_top_border));

    let final_alpha = alpha * color.a;
    return vec4<f32>(color.rgb * final_alpha, final_alpha);
}
