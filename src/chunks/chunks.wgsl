@group(0)
@binding(0)
var<storage, read_write> chunks: array<vec2<u32>>;
@group(0)
@binding(1)
var<storage> chunks_len: u32;
@group(0)
@binding(2)
var<storage> clip_from_world_with_margin: mat4x4<f32>;

const WORKGROUP_SIZE = 256u;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn cullChunks(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if global_id.x >= chunks_len {
        return;
    }

    let chunk = chunks[global_id.x];
    var unpacked = unpack4xU8(chunk.y);
    let chunk_mid = fma(vec3<f32>(unpacked.xyz), vec3(16.0), vec3(8.0));

    let clip_mid_h = clip_from_world_with_margin * vec4(chunk_mid, 1.0);
    let clip_mid = clip_mid_h.xyz / clip_mid_h.w;

    let max_dist = max(abs(clip_mid.x), max(abs(clip_mid.y), abs(clip_mid.z)));

    if max_dist > 1.0 {
        unpacked.w = 1u;
        chunks[global_id.x].y = pack4xU8(unpacked);
    }
}


var<workgroup> prefix: array<u32, WORKGROUP_SIZE>;

fn workgroupPrefixSum(val: u32, local_index: u32) -> u32 {
    var sum = 0u;
    var shift = 1u;

    let shifted = (local_index + shift) & (WORKGROUP_SIZE - 1);
    prefix[shifted] = select(val, 0u, shifted < shift);

    loop {
        workgroupBarrier();

        sum += prefix[local_index];

        if shift == WORKGROUP_SIZE { break; }

        workgroupBarrier();

        let shifted = (local_index + shift) & (WORKGROUP_SIZE - 1);
        prefix[shifted] = select(sum, 0u, shifted < shift);

        shift <<= 1u;
    }

    return sum;
}

var<workgroup> carry: u32;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn prefixSum(
    @builtin(local_invocation_index) local_index: u32,
) {
    if local_index == 0 {
        carry = 0u;
    }

    prefix[local_index] = 0u;

    workgroupBarrier();

    for (var stride = 0u; stride < chunks_len; stride += WORKGROUP_SIZE) {
        let index = local_index + stride;

        var val: u32;
        if index < chunks_len {
            val = chunks[index].x;
        }

        let sum = workgroupPrefixSum(val, local_index);

        if index < chunks_len {
            chunks[index].x += sum + carry;
        }

        workgroupBarrier();

        if local_index == WORKGROUP_SIZE - 1 {
            carry += val + sum;
        }
    }
}
