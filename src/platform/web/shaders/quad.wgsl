// Quad shader for GPUI web renderer.
// Renders rounded rectangles with borders, solid color backgrounds, and content masking.

struct Globals {
    viewport_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;

struct QuadInstance {
    // Bounds: origin (x,y) and size (w,h) in scaled pixels
    bounds_origin: vec2<f32>,
    bounds_size: vec2<f32>,
    // Content mask (clip rect)
    clip_origin: vec2<f32>,
    clip_size: vec2<f32>,
    // Background color (HSLA converted to linear RGBA on CPU)
    background: vec4<f32>,
    // Border color
    border_color: vec4<f32>,
    // Corner radii (top_left, top_right, bottom_right, bottom_left)
    corner_radii: vec4<f32>,
    // Border widths (top, right, bottom, left)
    border_widths: vec4<f32>,
}

@group(0) @binding(1) var<storage, read> quads: array<QuadInstance>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) @interpolate(flat) instance_index: u32,
}

// Unit quad vertices (two triangles forming a quad)
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
    let quad = quads[instance_index];
    let unit_pos = UNIT_QUAD[vertex_index];

    // Expand unit quad to the quad's bounds
    let pixel_pos = quad.bounds_origin + unit_pos * quad.bounds_size;

    // Convert pixel position to clip space: [0, viewport] -> [-1, 1]
    let ndc = vec2<f32>(
        pixel_pos.x / globals.viewport_size.x * 2.0 - 1.0,
        1.0 - pixel_pos.y / globals.viewport_size.y * 2.0,
    );

    var output: VertexOutput;
    output.position = vec4<f32>(ndc, 0.0, 1.0);
    output.local_pos = unit_pos * quad.bounds_size;
    output.instance_index = instance_index;
    return output;
}

// Signed distance function for a rounded rectangle.
// `p` is position relative to rect center, `half_size` is half of rect dimensions,
// `radii` is (top_left, top_right, bottom_right, bottom_left).
fn sdf_rounded_rect(p: vec2<f32>, half_size: vec2<f32>, radii: vec4<f32>) -> f32 {
    // Select the correct radius based on which quadrant we're in
    var r: f32;
    if p.x < 0.0 {
        if p.y < 0.0 {
            r = radii.x; // top-left
        } else {
            r = radii.w; // bottom-left
        }
    } else {
        if p.y < 0.0 {
            r = radii.y; // top-right
        } else {
            r = radii.z; // bottom-right
        }
    }
    r = min(r, min(half_size.x, half_size.y));

    let q = abs(p) - half_size + vec2<f32>(r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - r;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let quad = quads[input.instance_index];

    // Position relative to quad center
    let half_size = quad.bounds_size * 0.5;
    let center_pos = input.local_pos - half_size;

    // Content mask clipping
    let pixel_pos = quad.bounds_origin + input.local_pos;
    let clip_min = quad.clip_origin;
    let clip_max = quad.clip_origin + quad.clip_size;
    if pixel_pos.x < clip_min.x || pixel_pos.x > clip_max.x ||
       pixel_pos.y < clip_min.y || pixel_pos.y > clip_max.y {
        discard;
    }

    // Outer shape SDF
    let outer_dist = sdf_rounded_rect(center_pos, half_size, quad.corner_radii);

    // Anti-aliased outer edge
    let outer_alpha = 1.0 - smoothstep(-0.5, 0.5, outer_dist);

    if outer_alpha < 0.001 {
        discard;
    }

    // Inner shape for border (inset by border widths)
    let inner_half_size = half_size - vec2<f32>(
        max(quad.border_widths.y, quad.border_widths.w) * 0.5,
        max(quad.border_widths.x, quad.border_widths.z) * 0.5,
    );
    let inner_offset = vec2<f32>(
        (quad.border_widths.w - quad.border_widths.y) * 0.5,
        (quad.border_widths.x - quad.border_widths.z) * 0.5,
    );
    let inner_radii = max(quad.corner_radii - max(
        vec4<f32>(quad.border_widths.x, quad.border_widths.y, quad.border_widths.z, quad.border_widths.w),
        vec4<f32>(0.0)
    ), vec4<f32>(0.0));
    let inner_dist = sdf_rounded_rect(center_pos - inner_offset, max(inner_half_size, vec2<f32>(0.0)), inner_radii);

    // Determine if we're in the border region
    let in_border = inner_dist > 0.0;
    let has_border = quad.border_widths.x > 0.0 || quad.border_widths.y > 0.0 ||
                     quad.border_widths.z > 0.0 || quad.border_widths.w > 0.0;

    var color: vec4<f32>;
    if has_border && in_border {
        color = quad.border_color;
    } else {
        color = quad.background;
    }

    // Apply outer alpha for anti-aliasing
    color.a *= outer_alpha;

    // Premultiply alpha
    return vec4<f32>(color.rgb * color.a, color.a);
}
