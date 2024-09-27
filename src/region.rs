use std::{
    collections::{HashMap, HashSet},
    io::{Read, Seek},
};

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

        let mut block_set = HashSet::new();

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
                        let pos = u32::from_le_bytes([cx as u8, (cy + 4) as u8, cz as u8, 0]);

                        for x in 0..16 {
                            for y in 0..16 {
                                for z in 0..16 {
                                    if let Some(block) = chunk.block(x, y + cy * 16, z) {
                                        if block.name() != "minecraft:air" {
                                            let key_x = (x + cx * 16) as i16;
                                            let key_y = (y + (cy + 4) * 16) as i16;
                                            let key_z = (z + cz * 16) as i16;

                                            block_set.insert([key_x, key_y, key_z]);

                                            blocks
                                                .push((z << 8) as u32 | (y << 4) as u32 | x as u32);
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

        let chunks_iter = chunks
            .iter()
            .map(|&[len, pos]| (0..len).into_iter().map(move |_| pos))
            .flatten();

        let mut chunk_map: HashMap<_, _> = chunks.iter().map(|&[len, pos]| (pos, len)).collect();

        let blocks: Vec<_> = blocks
            .into_iter()
            .zip(chunks_iter)
            .filter_map(|(block, pos)| {
                let [cx, cy, cz, _] = pos.to_le_bytes();

                let x = block & 0b1111;
                let y = (block >> 4) & 0b1111;
                let z = (block >> 8) & 0b1111;

                let key_x = (x + cx as u32 * 16) as i16;
                let key_y = (y + cy as u32 * 16) as i16;
                let key_z = (z + cz as u32 * 16) as i16;

                let is_in_set = |key_x: i16, key_y: i16, key_z: i16| {
                    block_set.contains(&[key_x, key_y, key_z]) as u8
                };

                let count = is_in_set(key_x + 1, key_y, key_z)
                    + is_in_set(key_x - 1, key_y, key_z)
                    + is_in_set(key_x, key_y + 1, key_z)
                    + is_in_set(key_x, key_y - 1, key_z)
                    + is_in_set(key_x, key_y, key_z + 1)
                    + is_in_set(key_x, key_y, key_z - 1);

                if count < 6 {
                    Some(block)
                } else {
                    if let Some(len) = chunk_map.get_mut(&pos) {
                        *len -= 1;
                    }

                    None
                }
            })
            .collect();

        chunks
            .iter_mut()
            .for_each(|[len, pos]| *len = chunk_map[pos]);

        Some(Self { blocks, chunks })
    }

    pub fn chunks(&self) -> &[[u32; 2]] {
        &self.chunks
    }

    pub fn blocks(&self) -> &[u32] {
        &self.blocks
    }
}
