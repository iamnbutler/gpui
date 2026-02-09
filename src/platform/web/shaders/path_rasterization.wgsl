// Path rasterization shader for GPUI web renderer.
// Renders path triangles to an intermediate texture using quadratic bezier SDF.
// Each vertex in the storage buffer is a triangle vertex (not instanced).

struct Globals {
    viewport_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;

struct PathVertex {
    xy_position: vec2<f32>,
    st_position: vec2<f32>,
    // Background color (solid RGBA for now)
    color: vec4<f32>,
    // Clipped bounds for content masking
    clip_origin: vec2<f32>,
    clip_size: vec2<f32>,
}

@group(0) @binding(1) var<storage, read> vertices: array<PathVertex>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) st_position: vec2<f32>,
    @location(1) @interpolate(flat) vertex_id: u32,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_id: u32) -> VertexOutput {
    let v = vertices[vertex_id];

    let ndc = vec2<f32>(
        v.xy_position.x / globals.viewport_size.x * 2.0 - 1.0,
        1.0 - v.xy_position.y / globals.viewport_size.y * 2.0,
    );

    var output: VertexOutput;
    output.position = vec4<f32>(ndc, 0.0, 1.0);
    output.st_position = v.st_position;
    output.vertex_id = vertex_id;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let v = vertices[input.vertex_id];

    // Content mask clipping
    let pixel_pos = input.position.xy;
    let clip_min = v.clip_origin;
    let clip_max = v.clip_origin + v.clip_size;
    if pixel_pos.x < clip_min.x || pixel_pos.x > clip_max.x ||
       pixel_pos.y < clip_min.y || pixel_pos.y > clip_max.y {
        discard;
    }

    // Quadratic bezier SDF
    let dx = dpdx(input.st_position);
    let dy = dpdy(input.st_position);

    var alpha: f32;
    if length(vec2<f32>(dx.x, dy.x)) < 0.001 {
        // Gradient too small — solid fill (triangle interior)
        alpha = 1.0;
    } else {
        let gradient = 2.0 * input.st_position.xx * vec2<f32>(dx.x, dy.x) - vec2<f32>(dx.y, dy.y);
        let f = input.st_position.x * input.st_position.x - input.st_position.y;
        let distance = f / length(gradient);
        alpha = saturate(0.5 - distance);
    }

    let color = v.color;
    let final_alpha = color.a * alpha;
    return vec4<f32>(color.rgb * final_alpha, final_alpha);
}
