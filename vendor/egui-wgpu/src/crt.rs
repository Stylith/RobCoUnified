use std::sync::{OnceLock, RwLock};

use bytemuck::{Pod, Zeroable};
use web_time::Instant;

use crate::wgpu;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrtEffects {
    pub curvature: f32,
    pub scanlines: f32,
    pub glow: f32,
    pub bloom: f32,
    pub vignette: f32,
    pub noise: f32,
    pub flicker: f32,
    pub jitter: f32,
    pub burn_in: f32,
    pub glow_line: f32,
    pub glow_line_speed: f32,
    pub brightness: f32,
    pub contrast: f32,
    pub phosphor_softness: f32,
    pub theme_tint: [f32; 3],
}

fn crt_effects_lock() -> &'static RwLock<Option<CrtEffects>> {
    static CRT_EFFECTS: OnceLock<RwLock<Option<CrtEffects>>> = OnceLock::new();
    CRT_EFFECTS.get_or_init(|| RwLock::new(None))
}

pub fn set_crt_effects(effects: Option<CrtEffects>) {
    if let Ok(mut slot) = crt_effects_lock().write() {
        *slot = effects;
    }
}

pub(crate) fn current_crt_effects() -> Option<CrtEffects> {
    crt_effects_lock().read().ok().and_then(|slot| *slot)
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct CrtUniforms {
    params0: [f32; 4],
    params1: [f32; 4],
    params2: [f32; 4],
    params3: [f32; 4],
    params4: [f32; 4],
}

pub(crate) struct CrtPipeline {
    pipeline: wgpu::RenderPipeline,
    bloom_extract_pipeline: wgpu::RenderPipeline,
    bloom_blur_h_pipeline: wgpu::RenderPipeline,
    bloom_blur_v_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bloom_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,
    started_at: Instant,
}

pub(crate) struct CrtFrameState {
    pub width: u32,
    pub height: u32,
    _texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    _history_textures: [wgpu::Texture; 2],
    history_views: [wgpu::TextureView; 2],
    bind_groups: [wgpu::BindGroup; 2],
    history_source: usize,
    _bloom_textures: [wgpu::Texture; 2],
    bloom_views: [wgpu::TextureView; 2],
    bloom_input_bind_group: wgpu::BindGroup,
    bloom_ping_bind_group: wgpu::BindGroup,
    bloom_pong_bind_group: wgpu::BindGroup,
}

impl CrtPipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("egui_crt_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("crt_shader.wgsl").into()),
        });
        let bloom_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("egui_crt_bloom_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("crt_bloom.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("egui_crt_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
            ],
        });
        let bloom_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("egui_crt_bloom_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("egui_crt_pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let bloom_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("egui_crt_bloom_pipeline_layout"),
                bind_group_layouts: &[&bloom_bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("egui_crt_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let bloom_target = [Some(wgpu::ColorTargetState {
            format,
            blend: Some(wgpu::BlendState::REPLACE),
            write_mask: wgpu::ColorWrites::ALL,
        })];
        let bloom_extract_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("egui_crt_bloom_extract_pipeline"),
                layout: Some(&bloom_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &bloom_shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &bloom_shader,
                    entry_point: "extract_main",
                    targets: &bloom_target,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });
        let bloom_blur_h_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("egui_crt_bloom_blur_h_pipeline"),
                layout: Some(&bloom_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &bloom_shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &bloom_shader,
                    entry_point: "blur_h_main",
                    targets: &bloom_target,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });
        let bloom_blur_v_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("egui_crt_bloom_blur_v_pipeline"),
                layout: Some(&bloom_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &bloom_shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &bloom_shader,
                    entry_point: "blur_v_main",
                    targets: &bloom_target,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("egui_crt_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("egui_crt_uniform_buffer"),
            size: std::mem::size_of::<CrtUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            bloom_extract_pipeline,
            bloom_blur_h_pipeline,
            bloom_blur_v_pipeline,
            bind_group_layout,
            bloom_bind_group_layout,
            sampler,
            uniform_buffer,
            started_at: Instant::now(),
        }
    }

    pub(crate) fn create_frame_state(
        &self,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> CrtFrameState {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("egui_crt_input_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[format],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let history_a = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("egui_crt_history_a_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[format],
        });
        let history_b = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("egui_crt_history_b_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[format],
        });
        let history_views = [
            history_a.create_view(&wgpu::TextureViewDescriptor::default()),
            history_b.create_view(&wgpu::TextureViewDescriptor::default()),
        ];
        let bloom_width = (width / 2).max(1);
        let bloom_height = (height / 2).max(1);
        let bloom_a = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("egui_crt_bloom_a_texture"),
            size: wgpu::Extent3d {
                width: bloom_width,
                height: bloom_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[format],
        });
        let bloom_b = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("egui_crt_bloom_b_texture"),
            size: wgpu::Extent3d {
                width: bloom_width,
                height: bloom_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[format],
        });
        let bloom_views = [
            bloom_a.create_view(&wgpu::TextureViewDescriptor::default()),
            bloom_b.create_view(&wgpu::TextureViewDescriptor::default()),
        ];
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("egui_crt_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&history_views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&bloom_views[0]),
                },
            ],
        });
        let bind_group_alt = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("egui_crt_bind_group_alt"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&history_views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&bloom_views[0]),
                },
            ],
        });
        let bloom_input_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("egui_crt_bloom_input_bind_group"),
            layout: &self.bloom_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let bloom_ping_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("egui_crt_bloom_ping_bind_group"),
            layout: &self.bloom_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&bloom_views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let bloom_pong_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("egui_crt_bloom_pong_bind_group"),
            layout: &self.bloom_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&bloom_views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        CrtFrameState {
            width,
            height,
            _texture: texture,
            view,
            _history_textures: [history_a, history_b],
            history_views,
            bind_groups: [bind_group, bind_group_alt],
            history_source: 0,
            _bloom_textures: [bloom_a, bloom_b],
            bloom_views,
            bloom_input_bind_group,
            bloom_ping_bind_group,
            bloom_pong_bind_group,
        }
    }

    pub(crate) fn update_uniforms(
        &self,
        queue: &wgpu::Queue,
        effects: CrtEffects,
        width: u32,
        height: u32,
    ) {
        let elapsed = self.started_at.elapsed().as_secs_f32();
        let uniforms = CrtUniforms {
            params0: [elapsed, effects.curvature, effects.scanlines, effects.glow],
            params1: [
                effects.vignette,
                effects.noise,
                effects.brightness,
                effects.contrast,
            ],
            params2: [
                width as f32,
                height as f32,
                effects.phosphor_softness,
                effects.flicker,
            ],
            params3: [
                effects.bloom,
                effects.burn_in,
                effects.jitter,
                effects.glow_line,
            ],
            params4: [
                effects.glow_line_speed,
                effects.theme_tint[0],
                effects.theme_tint[1],
                effects.theme_tint[2],
            ],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    pub(crate) fn paint<'pass>(
        &'pass self,
        render_pass: &mut wgpu::RenderPass<'pass>,
        bind_group: &'pass wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }

    pub(crate) fn paint_bloom_extract<'pass>(
        &'pass self,
        render_pass: &mut wgpu::RenderPass<'pass>,
        bind_group: &'pass wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.bloom_extract_pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }

    pub(crate) fn paint_bloom_blur_h<'pass>(
        &'pass self,
        render_pass: &mut wgpu::RenderPass<'pass>,
        bind_group: &'pass wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.bloom_blur_h_pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }

    pub(crate) fn paint_bloom_blur_v<'pass>(
        &'pass self,
        render_pass: &mut wgpu::RenderPass<'pass>,
        bind_group: &'pass wgpu::BindGroup,
    ) {
        render_pass.set_pipeline(&self.bloom_blur_v_pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}

impl CrtFrameState {
    pub(crate) fn history_views(&self) -> [&wgpu::TextureView; 2] {
        [&self.history_views[0], &self.history_views[1]]
    }

    pub(crate) fn bloom_views(&self) -> [&wgpu::TextureView; 2] {
        [&self.bloom_views[0], &self.bloom_views[1]]
    }

    pub(crate) fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_groups[self.history_source]
    }

    pub(crate) fn history_target_view(&self) -> &wgpu::TextureView {
        &self.history_views[1 - self.history_source]
    }

    pub(crate) fn bloom_extract_bind_group(&self) -> &wgpu::BindGroup {
        &self.bloom_input_bind_group
    }

    pub(crate) fn bloom_ping_bind_group(&self) -> &wgpu::BindGroup {
        &self.bloom_ping_bind_group
    }

    pub(crate) fn bloom_pong_bind_group(&self) -> &wgpu::BindGroup {
        &self.bloom_pong_bind_group
    }

    pub(crate) fn bloom_ping_view(&self) -> &wgpu::TextureView {
        &self.bloom_views[0]
    }

    pub(crate) fn bloom_pong_view(&self) -> &wgpu::TextureView {
        &self.bloom_views[1]
    }

    pub(crate) fn advance_history(&mut self) {
        self.history_source = 1 - self.history_source;
    }
}
