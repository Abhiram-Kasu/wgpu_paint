use std::sync::Arc;

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    *,
};
use winit::{event_loop::ActiveEventLoop, keyboard::KeyCode, window::Window};

use crate::shader;

pub struct MandelbrotState {
    pub center: [f32; 2],
    pub zoom: f32,
    pub max_iterations: u32,
    pub cursor_location: [f64; 2],
    pub prev_cursor_location: [f64; 2],
    pub dragging: bool,
    pub needs_update: bool,
}

impl Default for MandelbrotState {
    fn default() -> Self {
        Self {
            center: [-0.5, 0.0], // Default center of Mandelbrot set
            zoom: 1.0,
            max_iterations: 100,
            cursor_location: [0.0, 0.0],
            prev_cursor_location: [0.0, 0.0],
            dragging: false,
            needs_update: true,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct MandelbrotParams {
    center: [f32; 2],
    zoom: f32,
    max_iterations: u32,
    _padding: [f32; 2], // Align to 16 bytes
}

// Fullscreen quad vertices
const QUAD_VERTICES: &[shader::Vertex] = &[
    shader::Vertex {
        position: [-1.0, -1.0, 0.0],
        color: [0.0, 0.0, 0.0], // Will be replaced by texture
    },
    shader::Vertex {
        position: [1.0, -1.0, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    shader::Vertex {
        position: [1.0, 1.0, 0.0],
        color: [1.0, 1.0, 0.0],
    },
    shader::Vertex {
        position: [-1.0, -1.0, 0.0],
        color: [0.0, 0.0, 0.0],
    },
    shader::Vertex {
        position: [1.0, 1.0, 0.0],
        color: [1.0, 1.0, 0.0],
    },
    shader::Vertex {
        position: [-1.0, 1.0, 0.0],
        color: [0.0, 1.0, 0.0],
    },
];

pub struct State {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub is_surface_configured: bool,
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,

    pub mandelbrot_state: MandelbrotState,
    pub window: Arc<Window>,
    pub compute_pipeline: wgpu::ComputePipeline,

    // Paint textures (ping-pong between them)
    pub canvas_texture_a: wgpu::Texture,
    pub canvas_texture_b: wgpu::Texture,
    pub canvas_view_a: wgpu::TextureView,
    pub canvas_view_b: wgpu::TextureView,
    pub use_texture_a_as_input: bool,

    // Compute shader resources
    pub compute_bind_group_a_to_b: wgpu::BindGroup,
    pub compute_bind_group_b_to_a: wgpu::BindGroup,
    pub params_buffer: wgpu::Buffer,
    pub sampler: wgpu::Sampler,

    // Render resources
    pub render_bind_group_a: wgpu::BindGroup,
    pub render_bind_group_b: wgpu::BindGroup,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
                required_features: wgpu::Features::empty(),
            })
            .await?;

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .unwrap_or(&surface_capabilities.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format.clone(),
            width: size.width,
            height: size.height,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        // Create canvas textures
        let texture_desc = wgpu::TextureDescriptor {
            label: Some("Canvas Texture"),
            size: wgpu::Extent3d {
                width: size.width.max(1),
                height: size.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        };

        let canvas_texture_a = device.create_texture(&texture_desc);
        let canvas_texture_b = device.create_texture(&texture_desc);

        let canvas_view_a = canvas_texture_a.create_view(&wgpu::TextureViewDescriptor::default());
        let canvas_view_b = canvas_texture_b.create_view(&wgpu::TextureViewDescriptor::default());

        // Textures are initialized to zero by default

        // Create sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create compute shader
        let compute_shader_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
        });

        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: None,
            module: &compute_shader_module,
            entry_point: Some("compute"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // Create uniform buffer for compute parameters
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Compute Params Buffer"),
            size: std::mem::size_of::<MandelbrotParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Create compute bind groups
        let compute_bind_group_layout = compute_pipeline.get_bind_group_layout(0);

        let compute_bind_group_a_to_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mandelbrot Compute Bind Group A"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&canvas_view_a),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let compute_bind_group_b_to_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mandelbrot Compute Bind Group B"),
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&canvas_view_b),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create render pipeline
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader Pipeline"),
            source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Bind Group Layout"),
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

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[shader::Vertex::vertex_buffer_desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::all(),
                })],
                compilation_options: Default::default(),
            }),
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Create vertex buffer for fullscreen quad
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            usage: BufferUsages::VERTEX,
            contents: bytemuck::cast_slice(QUAD_VERTICES),
        });

        // Create render bind groups
        let render_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group A"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&canvas_view_a),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let render_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group B"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&canvas_view_b),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            render_pipeline,
            vertex_buffer,
            num_vertices: QUAD_VERTICES.len() as u32,
            mandelbrot_state: Default::default(),
            window,
            compute_pipeline,
            canvas_texture_a,
            canvas_texture_b,
            canvas_view_a,
            canvas_view_b,
            use_texture_a_as_input: true,
            compute_bind_group_a_to_b,
            compute_bind_group_b_to_a,
            params_buffer,
            sampler,
            render_bind_group_a,
            render_bind_group_b,
            texture_bind_group_layout,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;

            // Recreate textures with new size
            let texture_desc = wgpu::TextureDescriptor {
                label: Some("Canvas Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            };

            self.canvas_texture_a = self.device.create_texture(&texture_desc);
            self.canvas_texture_b = self.device.create_texture(&texture_desc);
            self.canvas_view_a = self
                .canvas_texture_a
                .create_view(&wgpu::TextureViewDescriptor::default());
            self.canvas_view_b = self
                .canvas_texture_b
                .create_view(&wgpu::TextureViewDescriptor::default());

            // Textures are initialized to zero by default
            // Trigger an update since we have new textures
            self.mandelbrot_state.needs_update = true;

            // Recreate bind groups
            let compute_bind_group_layout = self.compute_pipeline.get_bind_group_layout(0);

            self.compute_bind_group_a_to_b =
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Mandelbrot Compute Bind Group A"),
                    layout: &compute_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&self.canvas_view_a),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: self.params_buffer.as_entire_binding(),
                        },
                    ],
                });

            self.compute_bind_group_b_to_a =
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Mandelbrot Compute Bind Group B"),
                    layout: &compute_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&self.canvas_view_b),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: self.params_buffer.as_entire_binding(),
                        },
                    ],
                });

            self.render_bind_group_a = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Render Bind Group A"),
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.canvas_view_a),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });

            self.render_bind_group_b = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Render Bind Group B"),
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&self.canvas_view_b),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            });
        }
    }

    pub fn render(&mut self) -> Result<(), SurfaceError> {
        if !self.is_surface_configured {
            return Ok(());
        }

        self.window.request_redraw();

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Run compute shader to generate Mandelbrot set
        if self.mandelbrot_state.needs_update {
            let params = MandelbrotParams {
                center: self.mandelbrot_state.center,
                zoom: self.mandelbrot_state.zoom,
                max_iterations: self.mandelbrot_state.max_iterations,
                _padding: [0.0; 2],
            };

            self.queue
                .write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));

            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Mandelbrot Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);

            // Use texture A for output
            compute_pass.set_bind_group(0, &self.compute_bind_group_a_to_b, &[]);

            // Dispatch compute shader
            let workgroup_size = 8;
            let window_size = self.window.inner_size();
            let dispatch_x = (window_size.width + workgroup_size - 1) / workgroup_size;
            let dispatch_y = (window_size.height + workgroup_size - 1) / workgroup_size;

            compute_pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
            drop(compute_pass);

            self.mandelbrot_state.needs_update = false;
        }

        // Render the current canvas texture to screen
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(Color {
                        r: 0.1,
                        g: 0.1,
                        b: 0.1,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);

        // Always render texture A since that's where we compute the Mandelbrot set
        let render_bind_group = &self.render_bind_group_a;

        render_pass.set_bind_group(0, render_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.num_vertices, 0..1);

        drop(render_pass);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        // Update cursor position
        self.mandelbrot_state.prev_cursor_location = self.mandelbrot_state.cursor_location;

        Ok(())
    }

    pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, key: KeyCode, is_pressed: bool) {
        if !is_pressed {
            return;
        }

        match key {
            KeyCode::Escape => event_loop.exit(),
            KeyCode::KeyR => {
                // Reset to default view
                self.mandelbrot_state.center = [-0.5, 0.0];
                self.mandelbrot_state.zoom = 1.0;
                self.mandelbrot_state.needs_update = true;
                self.window.request_redraw();
            }
            KeyCode::Equal | KeyCode::NumpadAdd => {
                // Zoom in
                self.mandelbrot_state.zoom *= 1.5;
                self.mandelbrot_state.needs_update = true;
                self.window.request_redraw();
            }
            KeyCode::Minus | KeyCode::NumpadSubtract => {
                // Zoom out
                self.mandelbrot_state.zoom /= 1.5;
                self.mandelbrot_state.needs_update = true;
                self.window.request_redraw();
            }
            KeyCode::ArrowUp => {
                // Increase iterations
                self.mandelbrot_state.max_iterations = (self.mandelbrot_state.max_iterations + 50);
                self.mandelbrot_state.needs_update = true;
                self.window.request_redraw();
            }
            KeyCode::ArrowDown => {
                // Decrease iterations
                self.mandelbrot_state.max_iterations =
                    (self.mandelbrot_state.max_iterations.saturating_sub(50)).max(10);
                self.mandelbrot_state.needs_update = true;
                self.window.request_redraw();
            }
            _ => {}
        }
    }

    pub fn update(&mut self) {
        // Any per-frame updates can go here
    }
}
