use std::{borrow::Cow, mem};

use wgpu::util::DeviceExt;

use crate::region::Region;

#[derive(Debug)]
pub struct BlocksPipeline {
    gen_faces_bind_group_layout: wgpu::BindGroupLayout,
    gen_faces_pipeline: wgpu::ComputePipeline,
    write_vertex_count_bind_group_layout: wgpu::BindGroupLayout,
    write_vertex_count_pipeline: wgpu::ComputePipeline,
}

impl BlocksPipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gen_faces_shader_module"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("blocks.wgsl"))),
        });

        let gen_faces_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("gen_faces_pipeline"),
            layout: None,
            module: &shader_module,
            entry_point: "genFaces",
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let gen_faces_bind_group_layout = gen_faces_pipeline.get_bind_group_layout(0);

        let write_vertex_count_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("write_vertex_count_pipeline"),
                layout: None,
                module: &shader_module,
                entry_point: "writeVertexCount",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let write_vertex_count_bind_group_layout =
            write_vertex_count_pipeline.get_bind_group_layout(0);

        Self {
            gen_faces_bind_group_layout,
            gen_faces_pipeline,
            write_vertex_count_pipeline,
            write_vertex_count_bind_group_layout,
        }
    }

    pub fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        region: &Region,
        chunk_buffer: wgpu::Buffer,
        chunks_len_buffer: wgpu::Buffer,
        eye: glam::Vec3,
        clip_from_world_with_margin: glam::Mat4,
        draw_indirect_buffer: &wgpu::Buffer,
    ) -> wgpu::Buffer {
        let block_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("block_buffer"),
            contents: bytemuck::cast_slice(region.blocks()),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let blocks_len_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("blocks_buffer"),
            contents: bytemuck::bytes_of(&(region.blocks().len() as u32)),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let face_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("face_buffer"),
            size: (region.blocks().len() * 3 * mem::size_of::<[u32; 2]>()) as u64,
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
            contents: bytemuck::cast_slice(eye.extend(0.0).as_ref()),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let clip_from_world_with_margin_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("clip_from_world_with_margin_buffer"),
                contents: bytemuck::bytes_of(clip_from_world_with_margin.as_ref()),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let gen_faces_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gen_faces_bind_group"),
            layout: &self.gen_faces_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: block_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: blocks_len_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: chunk_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: chunks_len_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: face_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: cursor_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: eye_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: clip_from_world_with_margin_buffer.as_entire_binding(),
                },
            ],
        });
        let write_vertex_count_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("write_vertex_count_bind_group"),
            layout: &self.write_vertex_count_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: cursor_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: draw_indirect_buffer.as_entire_binding(),
                },
            ],
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("gen_faces_pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&self.gen_faces_pipeline);
            pass.set_bind_group(0, &gen_faces_bind_group, &[]);

            pass.dispatch_workgroups(region.blocks().len().div_ceil(256) as u32, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("write_vertex_count_pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&self.write_vertex_count_pipeline);
            pass.set_bind_group(0, &write_vertex_count_bind_group, &[]);

            pass.dispatch_workgroups(1, 1, 1);
        }

        face_buffer
    }
}
