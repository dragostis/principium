use std::{borrow::Cow, mem};

use wgpu::util::DeviceExt;

#[derive(Debug)]
pub struct BlocksPipeline {
    blocks_bind_group_layout: wgpu::BindGroupLayout,
    blocks_pipeline: wgpu::ComputePipeline,
    double_bind_group_layout: wgpu::BindGroupLayout,
    double_pipeline: wgpu::ComputePipeline,
}

impl BlocksPipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blocks_shader_module"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("blocks.wgsl"))),
        });

        let blocks_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("blocks_pipeline"),
            layout: None,
            module: &shader_module,
            entry_point: "generateFaces",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let blocks_bind_group_layout = blocks_pipeline.get_bind_group_layout(0);

        let double_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("double_pipeline"),
            layout: None,
            module: &shader_module,
            entry_point: "doubleFacesLen",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let double_bind_group_layout = double_pipeline.get_bind_group_layout(0);

        Self {
            blocks_bind_group_layout,
            blocks_pipeline,
            double_pipeline,
            double_bind_group_layout,
        }
    }

    pub fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        blocks: &[u32],
        eye: glam::Vec3,
        draw_indirect_buffer: &wgpu::Buffer,
    ) -> wgpu::Buffer {
        let block_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("block_buffer"),
            contents: bytemuck::cast_slice(blocks),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let face_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("face_buffer"),
            size: (blocks.len() * 3 * mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let cursor_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cursor_buffer"),
            size: mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let eye_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("eye_buffer"),
            contents: bytemuck::bytes_of(glam::Vec3A::from(eye).as_ref()),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let blocks_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blocks_bind_group"),
            layout: &self.blocks_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: block_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: face_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: cursor_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: eye_buffer.as_entire_binding(),
                },
            ],
        });
        let double_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("double_bind_group"),
            layout: &self.double_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: cursor_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: draw_indirect_buffer.as_entire_binding(),
                },
            ],
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("blocks_pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&self.blocks_pipeline);
            pass.set_bind_group(0, &blocks_bind_group, &[]);

            pass.dispatch_workgroups(blocks.len().div_ceil(256) as u32, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("double_pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&self.double_pipeline);
            pass.set_bind_group(0, &double_bind_group, &[]);

            pass.dispatch_workgroups(1, 1, 1);
        }

        face_buffer
    }
}
