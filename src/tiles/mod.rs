use std::borrow::Cow;

use wgpu::util::DeviceExt;

#[derive(Debug)]
pub struct TilesPipeline {
    activate_tiles_bind_group_layout: wgpu::BindGroupLayout,
    activate_tiles_pipeline: wgpu::ComputePipeline,
    depth_compare_sampler: wgpu::Sampler,
}

impl TilesPipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("activate_tiles_shader_module"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("tiles.wgsl"))),
        });

        let activate_tiles_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("activate_tiles_pipeline"),
                layout: None,
                module: &shader_module,
                entry_point: "activateTiles",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let depth_compare_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("depth_compare_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            compare: Some(wgpu::CompareFunction::Equal),
            ..Default::default()
        });

        let activate_tiles_bind_group_layout = activate_tiles_pipeline.get_bind_group_layout(0);

        Self {
            activate_tiles_bind_group_layout,
            activate_tiles_pipeline,
            depth_compare_sampler,
        }
    }

    pub fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        depth_view: &wgpu::TextureView,
        config: &wgpu::SurfaceConfiguration,
    ) -> wgpu::Buffer {
        let size = glam::UVec2::new(config.width, config.height);
        let tiles = glam::UVec2::new(config.width.div_ceil(16), config.height.div_ceil(16));

        let active_tile_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("active_tile_buffer"),
            size: (tiles.x * tiles.y).div_ceil(32) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("size_buffer"),
            contents: bytemuck::cast_slice(size.as_ref()),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let activate_tiles_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("activate_tiles_bind_group"),
            layout: &self.activate_tiles_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(depth_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.depth_compare_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: active_tile_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: size_buffer.as_entire_binding(),
                },
            ],
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("activate_tiles_pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&self.activate_tiles_pipeline);
            pass.set_bind_group(0, &activate_tiles_bind_group, &[]);

            pass.dispatch_workgroups(tiles.x.div_ceil(2), tiles.y.div_ceil(2), 1);
        }

        active_tile_buffer
    }
}
