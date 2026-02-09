use crate::{BackgroundTag, DevicePixels, Hsla, Quad, Scene, Shadow, Size, Underline};
use wgpu::util::DeviceExt as _;

/// GPU-side representation of a quad instance, matching the WGSL shader layout.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuQuad {
    bounds_origin: [f32; 2],
    bounds_size: [f32; 2],
    clip_origin: [f32; 2],
    clip_size: [f32; 2],
    background: [f32; 4],
    border_color: [f32; 4],
    corner_radii: [f32; 4],
    border_widths: [f32; 4],
}

/// GPU-side representation of a shadow instance, matching the WGSL shader layout.
/// Layout follows WGSL storage buffer alignment rules:
/// vec2<f32> → 8-byte align, vec4<f32> → 16-byte align.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuShadow {
    blur_radius: f32,        // offset 0
    _pad0: f32,              // offset 4 (align bounds_origin to 8)
    bounds_origin: [f32; 2], // offset 8
    bounds_size: [f32; 2],   // offset 16
    _pad1: [f32; 2],         // offset 24 (align corner_radii to 32, i.e. 16-byte boundary)
    corner_radii: [f32; 4],  // offset 32
    clip_origin: [f32; 2],   // offset 48
    clip_size: [f32; 2],     // offset 56
    color: [f32; 4],         // offset 64, total: 80
}

/// GPU-side representation of an underline instance, matching the WGSL shader layout.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuUnderline {
    bounds_origin: [f32; 2],
    bounds_size: [f32; 2],
    clip_origin: [f32; 2],
    clip_size: [f32; 2],
    color: [f32; 4],
    thickness: f32,
    wavy: u32,
    _pad: [f32; 2],
}

/// GPU-side globals uniform, matching the WGSL shader layout.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    viewport_size: [f32; 2],
    _padding: [f32; 2],
}

/// The wgpu-based renderer for GPUI's web platform.
///
/// Converts Scene primitives to GPU draw calls using wgpu.
/// Designed to work on both native (Metal/Vulkan/DX12) and WebGPU targets.
pub(crate) struct WgpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    quad_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    underline_pipeline: wgpu::RenderPipeline,
    globals_buffer: wgpu::Buffer,
    globals_bind_group_layout: wgpu::BindGroupLayout,
}

/// Premultiplied alpha blend state shared across all pipelines.
fn premultiplied_alpha_blend() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

impl WgpuRenderer {
    /// Create a new renderer from a wgpu device and queue.
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, surface_format: wgpu::TextureFormat) -> Self {
        let globals_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("globals_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(
                                std::num::NonZero::new(std::mem::size_of::<Globals>() as u64)
                                    .expect("non-zero size"),
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("primitive_pipeline_layout"),
            bind_group_layouts: &[&globals_bind_group_layout],
            push_constant_ranges: &[],
        });

        let quad_pipeline = Self::create_pipeline(
            &device,
            &pipeline_layout,
            surface_format,
            include_str!("shaders/quad.wgsl"),
            "quad",
        );

        let shadow_pipeline = Self::create_pipeline(
            &device,
            &pipeline_layout,
            surface_format,
            include_str!("shaders/shadow.wgsl"),
            "shadow",
        );

        let underline_pipeline = Self::create_pipeline(
            &device,
            &pipeline_layout,
            surface_format,
            include_str!("shaders/underline.wgsl"),
            "underline",
        );

        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals_buffer"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            device,
            queue,
            quad_pipeline,
            shadow_pipeline,
            underline_pipeline,
            globals_buffer,
            globals_bind_group_layout,
        }
    }

    fn create_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        surface_format: wgpu::TextureFormat,
        shader_source: &str,
        label: &str,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(&format!("{label}_shader")),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(&format!("{label}_pipeline")),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(premultiplied_alpha_blend()),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }

    /// Render a scene to the given texture view.
    pub fn draw(&self, scene: &Scene, target: &wgpu::TextureView, viewport_size: Size<DevicePixels>) {
        let globals = Globals {
            viewport_size: [viewport_size.width.0 as f32, viewport_size.height.0 as f32],
            _padding: [0.0; 2],
        };
        self.queue.write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("scene_encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for batch in scene.batches() {
                match batch {
                    crate::PrimitiveBatch::Quads(quads) => {
                        self.draw_quads(&mut render_pass, quads);
                    }
                    crate::PrimitiveBatch::Shadows(shadows) => {
                        self.draw_shadows(&mut render_pass, shadows);
                    }
                    crate::PrimitiveBatch::Underlines(underlines) => {
                        self.draw_underlines(&mut render_pass, underlines);
                    }
                    crate::PrimitiveBatch::Paths(_paths) => {
                        // TODO: Two-pass path rendering (rasterize to texture, then composite)
                    }
                    crate::PrimitiveBatch::MonochromeSprites { .. } => {
                        // TODO: Requires GPU texture atlas
                    }
                    crate::PrimitiveBatch::PolychromeSprites { .. } => {
                        // TODO: Requires GPU texture atlas
                    }
                    crate::PrimitiveBatch::Surfaces(_surfaces) => {
                        // Surfaces are macOS-specific, skip on web.
                    }
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Create a bind group with the globals uniform and a storage buffer.
    fn create_bind_group(&self, label: &str, storage_buffer: &wgpu::Buffer) -> wgpu::BindGroup {
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &self.globals_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.globals_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: storage_buffer.as_entire_binding(),
                },
            ],
        })
    }

    fn draw_quads<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        quads: &[Quad],
    ) {
        if quads.is_empty() {
            return;
        }

        let gpu_quads: Vec<GpuQuad> = quads.iter().map(|q| quad_to_gpu(q)).collect();
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad_instance_buffer"),
            contents: bytemuck::cast_slice(&gpu_quads),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let bind_group = self.create_bind_group("quad_bind_group", &buffer);

        render_pass.set_pipeline(&self.quad_pipeline);
        render_pass.set_bind_group(0, Some(&bind_group), &[]);
        render_pass.draw(0..6, 0..quads.len() as u32);
    }

    fn draw_shadows<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        shadows: &[Shadow],
    ) {
        if shadows.is_empty() {
            return;
        }

        let gpu_shadows: Vec<GpuShadow> = shadows.iter().map(|s| shadow_to_gpu(s)).collect();
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow_instance_buffer"),
            contents: bytemuck::cast_slice(&gpu_shadows),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let bind_group = self.create_bind_group("shadow_bind_group", &buffer);

        render_pass.set_pipeline(&self.shadow_pipeline);
        render_pass.set_bind_group(0, Some(&bind_group), &[]);
        render_pass.draw(0..6, 0..shadows.len() as u32);
    }

    fn draw_underlines<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        underlines: &[Underline],
    ) {
        if underlines.is_empty() {
            return;
        }

        let gpu_underlines: Vec<GpuUnderline> = underlines.iter().map(|u| underline_to_gpu(u)).collect();
        let buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("underline_instance_buffer"),
            contents: bytemuck::cast_slice(&gpu_underlines),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let bind_group = self.create_bind_group("underline_bind_group", &buffer);

        render_pass.set_pipeline(&self.underline_pipeline);
        render_pass.set_bind_group(0, Some(&bind_group), &[]);
        render_pass.draw(0..6, 0..underlines.len() as u32);
    }
}

/// Convert an HSLA color to linear RGBA.
fn hsla_to_rgba(color: &Hsla) -> [f32; 4] {
    let rgba = color.to_rgb();
    [rgba.r, rgba.g, rgba.b, rgba.a]
}

/// Convert a scene Quad to GPU format.
fn quad_to_gpu(quad: &Quad) -> GpuQuad {
    let bg = match quad.background.tag {
        BackgroundTag::Solid => hsla_to_rgba(&quad.background.solid),
        // TODO: Handle linear gradients and patterns
        _ => hsla_to_rgba(&quad.background.solid),
    };

    GpuQuad {
        bounds_origin: [quad.bounds.origin.x.0, quad.bounds.origin.y.0],
        bounds_size: [quad.bounds.size.width.0, quad.bounds.size.height.0],
        clip_origin: [quad.content_mask.bounds.origin.x.0, quad.content_mask.bounds.origin.y.0],
        clip_size: [
            quad.content_mask.bounds.size.width.0,
            quad.content_mask.bounds.size.height.0,
        ],
        background: bg,
        border_color: hsla_to_rgba(&quad.border_color),
        corner_radii: [
            quad.corner_radii.top_left.0,
            quad.corner_radii.top_right.0,
            quad.corner_radii.bottom_right.0,
            quad.corner_radii.bottom_left.0,
        ],
        border_widths: [
            quad.border_widths.top.0,
            quad.border_widths.right.0,
            quad.border_widths.bottom.0,
            quad.border_widths.left.0,
        ],
    }
}

/// Convert a scene Shadow to GPU format.
fn shadow_to_gpu(shadow: &Shadow) -> GpuShadow {
    GpuShadow {
        blur_radius: shadow.blur_radius.0,
        _pad0: 0.0,
        bounds_origin: [shadow.bounds.origin.x.0, shadow.bounds.origin.y.0],
        bounds_size: [shadow.bounds.size.width.0, shadow.bounds.size.height.0],
        _pad1: [0.0; 2],
        corner_radii: [
            shadow.corner_radii.top_left.0,
            shadow.corner_radii.top_right.0,
            shadow.corner_radii.bottom_right.0,
            shadow.corner_radii.bottom_left.0,
        ],
        clip_origin: [shadow.content_mask.bounds.origin.x.0, shadow.content_mask.bounds.origin.y.0],
        clip_size: [
            shadow.content_mask.bounds.size.width.0,
            shadow.content_mask.bounds.size.height.0,
        ],
        color: hsla_to_rgba(&shadow.color),
    }
}

/// Convert a scene Underline to GPU format.
fn underline_to_gpu(underline: &Underline) -> GpuUnderline {
    GpuUnderline {
        bounds_origin: [underline.bounds.origin.x.0, underline.bounds.origin.y.0],
        bounds_size: [underline.bounds.size.width.0, underline.bounds.size.height.0],
        clip_origin: [
            underline.content_mask.bounds.origin.x.0,
            underline.content_mask.bounds.origin.y.0,
        ],
        clip_size: [
            underline.content_mask.bounds.size.width.0,
            underline.content_mask.bounds.size.height.0,
        ],
        color: hsla_to_rgba(&underline.color),
        thickness: underline.thickness.0,
        wavy: underline.wavy,
        _pad: [0.0; 2],
    }
}
