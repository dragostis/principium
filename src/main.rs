use std::{
    borrow::Cow,
    mem,
    num::NonZero,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Instant,
};

use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};

mod camera;

use camera::Camera;

#[derive(Debug)]
struct Inner {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    triangle_bind_group_layout: wgpu::BindGroupLayout,
    triangle_render_pipeline: wgpu::RenderPipeline,
    camera: Camera,
    last_inst: Option<Instant>,
}

impl Inner {
    pub async fn new(window: Window) -> Self {
        let window = Arc::new(window);

        let mut size = window.inner_size();
        size.width = size.width.max(1);
        size.height = size.height.max(1);

        let instance = wgpu::Instance::default();

        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let triangle_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("triangle_shader_module"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("triangle.wgsl"))),
        });

        let triangle_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("triangle_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            NonZero::new(mem::size_of::<glam::Mat4>() as u64).unwrap(),
                        ),
                    },
                    count: None,
                }],
            });

        let triangle_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("triangle_pipeline_layout"),
                bind_group_layouts: &[&triangle_bind_group_layout],
                push_constant_ranges: &[],
            });

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let triangle_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("triangle_render_pipeline"),
                layout: Some(&triangle_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &triangle_shader_module,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &triangle_shader_module,
                    entry_point: "fs_main",
                    compilation_options: Default::default(),
                    targets: &[Some(swapchain_format.into())],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();
        surface.configure(&device, &config);

        let camera = Camera::default();

        window
            .set_cursor_grab(CursorGrabMode::Locked)
            .unwrap_or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined).unwrap());
        window.set_cursor_visible(false);

        Self {
            window,
            device,
            queue,
            surface,
            config,
            triangle_bind_group_layout,
            triangle_render_pipeline,
            camera,
            last_inst: None,
        }
    }
}

#[derive(Debug, Default)]
struct App {
    inner: Option<Inner>,
}

impl Deref for App {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().expect("App has not been resumed yet")
    }
}

impl DerefMut for App {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().expect("App has not been resumed yet")
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();

        self.inner = Some(pollster::block_on(Inner::new(window)));
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.inner = None;
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        match event {
            DeviceEvent::MouseMotion { delta } => self.camera.handle_mouse_motion(delta),
            _ => (),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        state: ElementState::Pressed,
                        repeat: false,
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => self.camera.handle_key_event(event),
            WindowEvent::Resized(new_size) => {
                self.config.width = new_size.width.max(1);
                self.config.height = new_size.height.max(1);
                self.surface.configure(&self.device, &self.config);

                self.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = self
                    .last_inst
                    .map(|last_inst| now - last_inst)
                    .unwrap_or_default();

                self.last_inst = Some(now);

                self.camera.update(dt);

                let clip_from_world =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("clip_from_world"),
                            contents: bytemuck::cast_slice(
                                self.camera.clip_from_world(&self.config).as_ref(),
                            ),
                            usage: wgpu::BufferUsages::UNIFORM,
                        });

                let frame = self
                    .surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                {
                    let triangle_bind_group =
                        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("triangle_bind_group"),
                            layout: &self.triangle_bind_group_layout,
                            entries: &[wgpu::BindGroupEntry {
                                binding: 0,
                                resource: clip_from_world.as_entire_binding(),
                            }],
                        });

                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    rpass.set_pipeline(&self.triangle_render_pipeline);
                    rpass.set_bind_group(0, &triangle_bind_group, &[]);
                    rpass.draw(0..3, 0..1);
                }

                self.queue.submit(Some(encoder.finish()));

                frame.present();

                self.window.request_redraw();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => (),
        };
    }
}

fn main() {
    EventLoop::with_user_event()
        .build()
        .unwrap()
        .run_app(&mut App::default())
        .unwrap();
}
