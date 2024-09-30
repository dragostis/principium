@group(0)
@binding(0)
var depth_texture: texture_depth_2d;
@group(0)
@binding(1)
var depth_compare: sampler_comparison;
@group(0)
@binding(2)
var<storage, read_write> active_tiles: array<atomic<u32>>;
@group(0)
@binding(3)
var<storage> width_in_tiles: u32;

var<workgroup> local_active_tiles: array<atomic<u32>, 4>;

@compute
@workgroup_size(16, 16)
fn activateTiles(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_index) local_index: u32,
) {
    if local_index < 4 {
        atomicStore(&local_active_tiles[local_index], 0u);
    }

    workgroupBarrier();

    let spaced_global_id = global_id * 2;

    let tile = spaced_global_id.xy / 16;
    let coords = vec2<f32>(spaced_global_id.xy) + vec2(0.5);

    let depth_is_zero = textureGatherCompare(depth_texture, depth_compare, coords, 0.0);
    let is_active = any(depth_is_zero == vec4(1.0));

    let local_tile_index = tile.x % 2 + (tile.y % 2) * 2;
    atomicOr(&local_active_tiles[local_tile_index], u32(is_active));

    workgroupBarrier();

    if local_index < 4 {
        let local_is_active = atomicLoad(&local_active_tiles[local_index]);
        let tile_index = tile.x + tile.y * width_in_tiles;

        atomicOr(&active_tiles[tile_index / 32], local_is_active << (tile_index % 32));
    }
}
