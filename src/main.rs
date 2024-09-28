use std::{
    env,
    fs::File,
    mem,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Instant,
};

use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};

mod blocks;
mod camera;
mod chunks;
mod faces;
mod region;

use crate::{
    blocks::BlocksPipeline, camera::Camera, chunks::ChunksPipeline, faces::FacesPipeline,
    region::Region,
};

#[derive(Debug)]
struct Inner {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    depth_texture: wgpu::Texture,
    chunks_pipeline: ChunksPipeline,
    draw_indirect_buffer: wgpu::Buffer,
    blocks_pipeline: BlocksPipeline,
    faces_pipeline: FacesPipeline,
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

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let chunks_pipeline = ChunksPipeline::new(&device);
        let blocks_pipeline = BlocksPipeline::new(&device);
        let faces_pipeline = FacesPipeline::new(&device, swapchain_format);

        let draw_indirect_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("draw_indirect_buffer"),
            size: mem::size_of::<wgpu::util::DrawIndirectArgs>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });

        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();
        surface.configure(&device, &config);

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                ..Default::default()
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

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
            depth_texture,
            chunks_pipeline,
            draw_indirect_buffer,
            blocks_pipeline,
            faces_pipeline,
            camera,
            last_inst: None,
        }
    }
}

#[derive(Debug)]
struct App {
    region: Region,
    inner: Option<Inner>,
}

impl App {
    pub fn new(region: Region) -> Self {
        Self {
            region,
            inner: None,
        }
    }
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
            .create_window(Window::default_attributes().with_maximized(true))
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

                self.depth_texture.destroy();
                self.depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("depth_texture"),
                    size: wgpu::Extent3d {
                        width: new_size.width,
                        height: new_size.height,
                        ..Default::default()
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Depth24Plus,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                });

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

                let frame = self
                    .surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let color_view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let depth_view = self
                    .depth_texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                let (chunk_buffer, chunks_len_buffer) = self.chunks_pipeline.encode(
                    &self.device,
                    &mut encoder,
                    &self.region,
                    self.camera
                        .clip_from_world_with_margin(&self.config, 8.0 * 2.0f32.sqrt()),
                );
                let face_buffer = self.blocks_pipeline.encode(
                    &self.device,
                    &mut encoder,
                    &self.region,
                    chunk_buffer,
                    chunks_len_buffer,
                    self.camera.eye,
                    self.camera
                        .clip_from_world_with_margin(&self.config, 0.5 * 2.0f32.sqrt()),
                    &self.draw_indirect_buffer,
                );
                self.faces_pipeline.encode(
                    &self.device,
                    &mut encoder,
                    &face_buffer,
                    self.camera.clip_from_world(&self.config),
                    &self.draw_indirect_buffer,
                    &color_view,
                    &depth_view,
                );

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
    let path = env::args().nth(1).expect("provide .mca path as arg");

    let file = File::open(path).unwrap();

    let region = Region::new(file).unwrap();

    EventLoop::with_user_event()
        .build()
        .unwrap()
        .run_app(&mut App::new(region))
        .unwrap();
}
