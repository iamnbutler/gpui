use crate::{BackgroundTag, DevicePixels, Hsla, Quad, Scene, Size};
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
    globals_buffer: wgpu::Buffer,
    globals_bind_group_layout: wgpu::BindGroupLayout,
}

impl WgpuRenderer {
    /// Create a new renderer from a wgpu device and queue.
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, surface_format: wgpu::TextureFormat) -> Self {
        let quad_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("quad_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/quad.wgsl").into(),
            ),
        });

        let globals_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("globals_bind_group_layout"),
                entries: &[
                    // Globals uniform
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
                    // Quad instances (storage buffer)
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
            label: Some("quad_pipeline_layout"),
            bind_group_layouts: &[&globals_bind_group_layout],
            push_constant_ranges: &[],
        });

        let quad_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("quad_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &quad_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &quad_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState {
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
                    }),
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
        });

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
            globals_buffer,
            globals_bind_group_layout,
        }
    }

    /// Render a scene to the given texture view.
    pub fn draw(&self, scene: &Scene, target: &wgpu::TextureView, viewport_size: Size<DevicePixels>) {
        // Update globals
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
                        self.draw_quads(&mut render_pass, quads, &globals);
                    }
                    crate::PrimitiveBatch::Shadows(_shadows) => {
                        // TODO: Implement shadow rendering
                    }
                    crate::PrimitiveBatch::Paths(_paths) => {
                        // TODO: Implement path rendering
                    }
                    crate::PrimitiveBatch::Underlines(_underlines) => {
                        // TODO: Implement underline rendering
                    }
                    crate::PrimitiveBatch::MonochromeSprites { .. } => {
                        // TODO: Implement monochrome sprite rendering
                    }
                    crate::PrimitiveBatch::PolychromeSprites { .. } => {
                        // TODO: Implement polychrome sprite rendering
                    }
                    crate::PrimitiveBatch::Surfaces(_surfaces) => {
                        // Surfaces are macOS-specific, skip on web.
                    }
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    fn draw_quads<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        quads: &[Quad],
        _globals: &Globals,
    ) {
        if quads.is_empty() {
            return;
        }

        // Convert scene quads to GPU format
        let gpu_quads: Vec<GpuQuad> = quads.iter().map(|q| quad_to_gpu(q)).collect();
        let quad_data = bytemuck::cast_slice(&gpu_quads);

        // Create instance buffer for this batch
        let quad_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad_instance_buffer"),
            contents: quad_data,
            usage: wgpu::BufferUsages::STORAGE,
        });

        // Create bind group for this draw call
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("quad_bind_group"),
            layout: &self.globals_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.globals_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: quad_buffer.as_entire_binding(),
                },
            ],
        });

        render_pass.set_pipeline(&self.quad_pipeline);
        render_pass.set_bind_group(0, Some(&bind_group), &[]);
        render_pass.draw(0..6, 0..quads.len() as u32);
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
