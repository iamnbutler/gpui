// Shadow shader for GPUI web renderer.
// Renders soft shadows behind rounded rectangles using Gaussian blur approximation.

struct Globals {
    viewport_size: vec2<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;

struct ShadowInstance {
    blur_radius: f32,
    bounds_origin: vec2<f32>,
    bounds_size: vec2<f32>,
    corner_radii: vec4<f32>,
    clip_origin: vec2<f32>,
    clip_size: vec2<f32>,
    color: vec4<f32>,
}

@group(0) @binding(1) var<storage, read> shadows: array<ShadowInstance>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) instance_index: u32,
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
    let shadow = shadows[instance_index];
    let unit_pos = UNIT_QUAD[vertex_index];

    // Expand bounds by 3x blur radius to cover the full shadow spread
    let margin = 3.0 * shadow.blur_radius;
    let expanded_origin = shadow.bounds_origin - vec2<f32>(margin);
    let expanded_size = shadow.bounds_size + 2.0 * vec2<f32>(margin);

    let pixel_pos = expanded_origin + unit_pos * expanded_size;

    let ndc = vec2<f32>(
        pixel_pos.x / globals.viewport_size.x * 2.0 - 1.0,
        1.0 - pixel_pos.y / globals.viewport_size.y * 2.0,
    );

    var output: VertexOutput;
    output.position = vec4<f32>(ndc, 0.0, 1.0);
    output.instance_index = instance_index;
    return output;
}

// Standard Gaussian function
fn gaussian(x: f32, sigma: f32) -> f32 {
    return exp(-(x * x) / (2.0 * sigma * sigma)) / (sqrt(2.0 * M_PI_F) * sigma);
}

// Approximation of the error function for Gaussian integral
fn erf(v: vec2<f32>) -> vec2<f32> {
    let s = sign(v);
    let a = abs(v);
    let r1 = 1.0 + (0.278393 + (0.230389 + (0.000972 + 0.078108 * a) * a) * a) * a;
    let r2 = r1 * r1;
    return s - s / (r2 * r2);
}

// Compute blur along the x-axis for a rounded rect shape
fn blur_along_x(x: f32, y: f32, sigma: f32, corner: f32, half_size: vec2<f32>) -> f32 {
    let delta = min(half_size.y - corner - abs(y), 0.0);
    let curved = half_size.x - corner + sqrt(max(0.0, corner * corner - delta * delta));
    let integral = 0.5 + 0.5 * erf((x + vec2<f32>(-curved, curved)) * (sqrt(0.5) / sigma));
    return integral.y - integral.x;
}

// Select corner radius based on which quadrant the point is in
fn pick_corner_radius(center_to_point: vec2<f32>, radii: vec4<f32>) -> f32 {
    if center_to_point.x < 0.0 {
        if center_to_point.y < 0.0 {
            return radii.x; // top-left
        } else {
            return radii.w; // bottom-left
        }
    } else {
        if center_to_point.y < 0.0 {
            return radii.y; // top-right
        } else {
            return radii.z; // bottom-right
        }
    }
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let shadow = shadows[input.instance_index];

    // Content mask clipping
    let pixel_pos = input.position.xy;
    let clip_min = shadow.clip_origin;
    let clip_max = shadow.clip_origin + shadow.clip_size;
    if pixel_pos.x < clip_min.x || pixel_pos.x > clip_max.x ||
       pixel_pos.y < clip_min.y || pixel_pos.y > clip_max.y {
        discard;
    }

    let half_size = shadow.bounds_size / 2.0;
    let center = shadow.bounds_origin + half_size;
    let center_to_point = pixel_pos - center;

    let corner_radius = pick_corner_radius(center_to_point, shadow.corner_radii);

    // The signal is only non-zero in a limited range, so don't waste samples
    let low = center_to_point.y - half_size.y;
    let high = center_to_point.y + half_size.y;
    let start = clamp(-3.0 * shadow.blur_radius, low, high);
    let end = clamp(3.0 * shadow.blur_radius, low, high);

    // Accumulate 4 Gaussian samples along the y-axis
    let step = (end - start) / 4.0;
    var y = start + step * 0.5;
    var alpha = 0.0;
    for (var i = 0; i < 4; i += 1) {
        let blur = blur_along_x(
            center_to_point.x, center_to_point.y - y,
            shadow.blur_radius, corner_radius, half_size
        );
        alpha += blur * gaussian(y, shadow.blur_radius) * step;
        y += step;
    }

    let color = shadow.color;
    let final_alpha = color.a * alpha;
    return vec4<f32>(color.rgb * final_alpha, final_alpha);
}
