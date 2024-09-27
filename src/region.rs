use std::io::{Read, Seek};

use fastanvil::{complete::Chunk, Chunk as _, HeightMode};

#[derive(Debug)]
pub struct Region {
    chunks: Vec<[u32; 2]>,
    blocks: Vec<u32>,
}

impl Region {
    pub fn new<S>(stream: S) -> Option<Self>
    where
        S: Read + Seek,
    {
        let mut region = fastanvil::Region::from_stream(stream).ok()?;
        let mut blocks = Vec::new();
        let mut chunks = Vec::new();

        for cx in 0..32 {
            for cz in 0..32 {
                if let Some(chunk) = region
                    .read_chunk(cx, cz)
                    .ok()?
                    .and_then(|data| Chunk::from_bytes(&data).ok())
                {
                    let mut min = isize::MAX;
                    let mut max = isize::MIN;
                    for x in 0..16 {
                        for z in 0..16 {
                            let h = chunk.surface_height(x, z, HeightMode::Trust);

                            min = min.min(h);
                            max = max.max(h);
                        }
                    }

                    for cy in min / 16..=max / 16 {
                        let start = blocks.len() as u32;
                        let pos = u32::from_le_bytes([cx as u8, cy as u8, cz as u8, 0]);

                        for x in 0..16 {
                            for y in cy * 16..(cy + 1) * 16 {
                                for z in 0..16 {
                                    if let Some(block) = chunk.block(x, y, z) {
                                        if block.name() != "minecraft:air" {
                                            blocks.push(
                                                (z << 8) as u32
                                                    | ((y - cy * 16) << 4) as u32
                                                    | x as u32,
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        let end = blocks.len() as u32;
                        chunks.push([end - start, pos]);
                    }
                }
            }
        }

        Some(Self { blocks, chunks })
    }

    pub fn chunks(&self) -> &[[u32; 2]] {
        &self.chunks
    }

    pub fn blocks(&self) -> &[u32] {
        &self.blocks
    }
}
