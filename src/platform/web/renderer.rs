use crate::{BackgroundTag, DevicePixels, Hsla, Path, Quad, ScaledPixels, Scene, Shadow, Size, Underline};
use wgpu::util::DeviceExt as _;

// ---------------------------------------------------------------------------
// GPU struct definitions (must match WGSL shader layouts exactly)
// ---------------------------------------------------------------------------

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

/// WGSL alignment: blur_radius(f32) + pad + vec2 + vec2 + pad + vec4 + vec2 + vec2 + vec4 = 80 bytes
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuShadow {
    blur_radius: f32,
    _pad0: f32,
    bounds_origin: [f32; 2],
    bounds_size: [f32; 2],
    _pad1: [f32; 2],
    corner_radii: [f32; 4],
    clip_origin: [f32; 2],
    clip_size: [f32; 2],
    color: [f32; 4],
}

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

/// Path rasterization vertex (not instanced — 3 vertices per triangle).
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuPathVertex {
    xy_position: [f32; 2],
    st_position: [f32; 2],
    color: [f32; 4],
    clip_origin: [f32; 2],
    clip_size: [f32; 2],
}

/// Path sprite for compositing from intermediate texture.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuPathSprite {
    bounds_origin: [f32; 2],
    bounds_size: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    viewport_size: [f32; 2],
    _padding: [f32; 2],
}

// ---------------------------------------------------------------------------
// Renderer
// ---------------------------------------------------------------------------

/// The wgpu-based renderer for GPUI's web platform.
///
/// Converts Scene primitives to GPU draw calls using wgpu.
/// Designed to work on both native (Metal/Vulkan/DX12) and WebGPU targets.
pub(crate) struct WgpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_format: wgpu::TextureFormat,

    // Pipelines for instanced primitives (quads, shadows, underlines)
    quad_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    underline_pipeline: wgpu::RenderPipeline,

    // Path rendering pipelines (two-pass)
    path_rasterization_pipeline: wgpu::RenderPipeline,
    path_composite_pipeline: wgpu::RenderPipeline,

    // Shared resources
    globals_buffer: wgpu::Buffer,
    globals_bind_group_layout: wgpu::BindGroupLayout,
    path_composite_bind_group_layout: wgpu::BindGroupLayout,
    path_sampler: wgpu::Sampler,

    // Intermediate texture for path rendering (recreated on viewport resize)
    path_intermediate: Option<PathIntermediate>,
}

struct PathIntermediate {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: Size<DevicePixels>,
}

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
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, surface_format: wgpu::TextureFormat) -> Self {
        // Bind group layout shared by quads, shadows, underlines, and path rasterization:
        // binding 0 = globals uniform, binding 1 = storage buffer
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

        // Bind group layout for path compositing:
        // binding 0 = globals, binding 1 = path sprites storage, binding 2 = texture, binding 3 = sampler
        let path_composite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("path_composite_bind_group_layout"),
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
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let primitive_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("primitive_pipeline_layout"),
            bind_group_layouts: &[&globals_bind_group_layout],
            push_constant_ranges: &[],
        });

        let path_composite_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("path_composite_pipeline_layout"),
            bind_group_layouts: &[&path_composite_bind_group_layout],
            push_constant_ranges: &[],
        });

        let quad_pipeline = Self::create_pipeline(
            &device, &primitive_layout, surface_format,
            include_str!("shaders/quad.wgsl"), "quad",
        );
        let shadow_pipeline = Self::create_pipeline(
            &device, &primitive_layout, surface_format,
            include_str!("shaders/shadow.wgsl"), "shadow",
        );
        let underline_pipeline = Self::create_pipeline(
            &device, &primitive_layout, surface_format,
            include_str!("shaders/underline.wgsl"), "underline",
        );
        let path_rasterization_pipeline = Self::create_pipeline(
            &device, &primitive_layout, surface_format,
            include_str!("shaders/path_rasterization.wgsl"), "path_rasterization",
        );
        let path_composite_pipeline = Self::create_pipeline(
            &device, &path_composite_layout, surface_format,
            include_str!("shaders/path_composite.wgsl"), "path_composite",
        );

        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals_buffer"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let path_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("path_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            device,
            queue,
            surface_format,
            quad_pipeline,
            shadow_pipeline,
            underline_pipeline,
            path_rasterization_pipeline,
            path_composite_pipeline,
            globals_buffer,
            globals_bind_group_layout,
            path_composite_bind_group_layout,
            path_sampler,
            path_intermediate: None,
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

    /// Ensure the intermediate path texture matches the viewport size.
    fn ensure_path_intermediate(&mut self, viewport_size: Size<DevicePixels>) {
        let needs_recreate = match &self.path_intermediate {
            Some(intermediate) => intermediate.size != viewport_size,
            None => true,
        };

        if needs_recreate {
            let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("path_intermediate_texture"),
                size: wgpu::Extent3d {
                    width: viewport_size.width.0 as u32,
                    height: viewport_size.height.0 as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.surface_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.path_intermediate = Some(PathIntermediate {
                texture,
                view,
                size: viewport_size,
            });
        }
    }

    /// Render a scene to the given texture view.
    pub fn draw(&mut self, scene: &Scene, target: &wgpu::TextureView, viewport_size: Size<DevicePixels>) {
        let globals = Globals {
            viewport_size: [viewport_size.width.0 as f32, viewport_size.height.0 as f32],
            _padding: [0.0; 2],
        };
        self.queue.write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("scene_encoder"),
        });

        // Track whether we've started the main render pass yet.
        // We use a closure-based approach: collect batches, then process them,
        // breaking the render pass when paths are encountered.
        let batches: Vec<_> = scene.batches().collect();
        let mut is_first_pass = true;

        let mut batch_idx = 0;
        while batch_idx < batches.len() {
            // Start or resume the main render pass
            {
                let load_op = if is_first_pass {
                    is_first_pass = false;
                    wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                } else {
                    wgpu::LoadOp::Load
                };

                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("scene_render_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: load_op,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });

                // Process non-path batches until we hit a path batch or run out
                while batch_idx < batches.len() {
                    match &batches[batch_idx] {
                        crate::PrimitiveBatch::Quads(quads) => {
                            self.draw_quads(&mut render_pass, quads);
                        }
                        crate::PrimitiveBatch::Shadows(shadows) => {
                            self.draw_shadows(&mut render_pass, shadows);
                        }
                        crate::PrimitiveBatch::Underlines(underlines) => {
                            self.draw_underlines(&mut render_pass, underlines);
                        }
                        crate::PrimitiveBatch::Paths(_) => {
                            // Break out of the render pass to handle paths
                            break;
                        }
                        crate::PrimitiveBatch::MonochromeSprites { .. } => {
                            // TODO: Requires GPU texture atlas
                        }
                        crate::PrimitiveBatch::PolychromeSprites { .. } => {
                            // TODO: Requires GPU texture atlas
                        }
                        crate::PrimitiveBatch::Surfaces(_) => {
                            // Surfaces are macOS-specific, skip on web.
                        }
                    }
                    batch_idx += 1;
                }
            }
            // Render pass is dropped here (ended)

            // If we stopped at a path batch, render it with two-pass approach
            if batch_idx < batches.len() {
                if let crate::PrimitiveBatch::Paths(paths) = &batches[batch_idx] {
                    self.draw_paths(&mut encoder, target, paths, viewport_size);
                    batch_idx += 1;
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    /// Two-pass path rendering: rasterize to intermediate texture, then composite.
    fn draw_paths(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        paths: &[Path<ScaledPixels>],
        viewport_size: Size<DevicePixels>,
    ) {
        if paths.is_empty() {
            return;
        }

        self.ensure_path_intermediate(viewport_size);
        let intermediate = self.path_intermediate.as_ref().expect("intermediate texture");

        // Pass 1: Rasterize path triangles to intermediate texture
        let mut vertices = Vec::new();
        for path in paths {
            let clipped_bounds = path.clipped_bounds();
            let bg_color = match path.color.tag {
                BackgroundTag::Solid => hsla_to_rgba(&path.color.solid),
                _ => hsla_to_rgba(&path.color.solid),
            };

            for v in &path.vertices {
                vertices.push(GpuPathVertex {
                    xy_position: [v.xy_position.x.0, v.xy_position.y.0],
                    st_position: [v.st_position.x, v.st_position.y],
                    color: bg_color,
                    clip_origin: [clipped_bounds.origin.x.0, clipped_bounds.origin.y.0],
                    clip_size: [clipped_bounds.size.width.0, clipped_bounds.size.height.0],
                });
            }
        }

        if vertices.is_empty() {
            return;
        }

        let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("path_vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let rasterization_bind_group = self.create_bind_group("path_rasterization_bind_group", &vertex_buffer);

        {
            let mut raster_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("path_rasterization_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &intermediate.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            raster_pass.set_pipeline(&self.path_rasterization_pipeline);
            raster_pass.set_bind_group(0, Some(&rasterization_bind_group), &[]);
            raster_pass.draw(0..vertices.len() as u32, 0..1);
        }

        // Pass 2: Composite from intermediate texture to main target
        // If all paths have the same draw order, copy each path's bounds individually.
        // Otherwise, copy a single spanning rect to avoid double-blending.
        let first_path = &paths[0];
        let sprites: Vec<GpuPathSprite> = if paths.last().map(|p| p.order) == Some(first_path.order) {
            paths.iter().map(|path| {
                let b = path.clipped_bounds();
                GpuPathSprite {
                    bounds_origin: [b.origin.x.0, b.origin.y.0],
                    bounds_size: [b.size.width.0, b.size.height.0],
                }
            }).collect()
        } else {
            let mut bounds = first_path.clipped_bounds();
            for path in paths.iter().skip(1) {
                bounds = bounds.union(&path.clipped_bounds());
            }
            vec![GpuPathSprite {
                bounds_origin: [bounds.origin.x.0, bounds.origin.y.0],
                bounds_size: [bounds.size.width.0, bounds.size.height.0],
            }]
        };

        let sprite_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("path_sprite_buffer"),
            contents: bytemuck::cast_slice(&sprites),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let composite_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("path_composite_bind_group"),
            layout: &self.path_composite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.globals_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: sprite_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&intermediate.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.path_sampler),
                },
            ],
        });

        {
            let mut composite_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("path_composite_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            composite_pass.set_pipeline(&self.path_composite_pipeline);
            composite_pass.set_bind_group(0, Some(&composite_bind_group), &[]);
            composite_pass.draw(0..6, 0..sprites.len() as u32);
        }
    }

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

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

fn hsla_to_rgba(color: &Hsla) -> [f32; 4] {
    let rgba = color.to_rgb();
    [rgba.r, rgba.g, rgba.b, rgba.a]
}

fn quad_to_gpu(quad: &Quad) -> GpuQuad {
    let bg = match quad.background.tag {
        BackgroundTag::Solid => hsla_to_rgba(&quad.background.solid),
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
