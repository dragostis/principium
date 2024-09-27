use std::borrow::Cow;

use wgpu::util::DeviceExt;

use crate::region::Region;

#[derive(Debug)]
pub struct ChunksPipeline {
    cull_chunks_bind_group_layout: wgpu::BindGroupLayout,
    cull_chunks_pipeline: wgpu::ComputePipeline,
    prefix_sum_bind_group_layout: wgpu::BindGroupLayout,
    prefix_sum_pipeline: wgpu::ComputePipeline,
    write_block_count_bind_group_layout: wgpu::BindGroupLayout,
    write_block_count_pipeline: wgpu::ComputePipeline,
}

impl ChunksPipeline {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cull_chunks_shader_module"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("chunks.wgsl"))),
        });

        let cull_chunks_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("cull_chunks_pipeline"),
                layout: None,
                module: &shader_module,
                entry_point: "cullChunks",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let cull_chunks_bind_group_layout = cull_chunks_pipeline.get_bind_group_layout(0);

        let prefix_sum_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("prefix_sum_pipeline"),
                layout: None,
                module: &shader_module,
                entry_point: "prefixSum",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let prefix_sum_bind_group_layout = prefix_sum_pipeline.get_bind_group_layout(0);

        let write_block_count_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("write_block_count_pipeline"),
                layout: None,
                module: &shader_module,
                entry_point: "writeBlockCount",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let write_block_count_bind_group_layout =
            write_block_count_pipeline.get_bind_group_layout(0);

        Self {
            cull_chunks_bind_group_layout,
            cull_chunks_pipeline,
            prefix_sum_pipeline,
            prefix_sum_bind_group_layout,
            write_block_count_bind_group_layout,
            write_block_count_pipeline,
        }
    }

    pub fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        region: &Region,
        clip_from_world_with_margin: glam::Mat4,
        blocks_indirect_buffer: &wgpu::Buffer,
    ) -> (wgpu::Buffer, wgpu::Buffer) {
        let chunk_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chunk_buffer"),
            contents: bytemuck::cast_slice(region.chunks()),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let chunks_len_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chunks_buffer"),
            contents: bytemuck::bytes_of(&(region.chunks().len() as u32)),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let clip_from_world_with_margin_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("clip_from_world_with_margin_buffer"),
                contents: bytemuck::bytes_of(clip_from_world_with_margin.as_ref()),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let cull_chunks_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cull_chunks_bind_group"),
            layout: &self.cull_chunks_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: chunk_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: chunks_len_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: clip_from_world_with_margin_buffer.as_entire_binding(),
                },
            ],
        });
        let prefix_sum_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("prefix_sum_bind_group"),
            layout: &self.prefix_sum_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: chunk_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: chunks_len_buffer.as_entire_binding(),
                },
            ],
        });
        let write_block_count_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("write_block_count_bind_group"),
            layout: &self.write_block_count_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: chunk_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: chunks_len_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: blocks_indirect_buffer.as_entire_binding(),
                },
            ],
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("cull_chunks_pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&self.cull_chunks_pipeline);
            pass.set_bind_group(0, &cull_chunks_bind_group, &[]);

            pass.dispatch_workgroups(region.chunks().len().div_ceil(256) as u32, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("prefix_sum_pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&self.prefix_sum_pipeline);
            pass.set_bind_group(0, &prefix_sum_bind_group, &[]);

            pass.dispatch_workgroups(1, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("write_block_count_pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&self.write_block_count_pipeline);
            pass.set_bind_group(0, &write_block_count_bind_group, &[]);

            pass.dispatch_workgroups(1, 1, 1);
        }

        (chunk_buffer, chunks_len_buffer)
    }
}
