@group(0)
@binding(0)
var<storage> blocks: array<u32>;
@group(0)
@binding(1)
var<storage> blocks_len: u32;
@group(0)
@binding(2)
var<storage, read_write> faces: array<u32>;
@group(0)
@binding(3)
var<storage, read_write> cursor: atomic<u32>;
@group(0)
@binding(4)
var<storage> eye: vec3<f32>;
@group(0)
@binding(5)
var<storage> clip_from_world_with_margin: mat4x4<f32>;

fn blockPos(block: u32) -> vec3<u32> {
    return vec3(
        extractBits(block, 0u, 4u),
        extractBits(block, 4u, 4u),
        extractBits(block, 8u, 4u),
    );
}

fn newFace(pos: vec3<u32>, i: u32) -> u32 {
    var face = 0u;

    face = insertBits(face, pos.x, 0u, 4u);
    face = insertBits(face, pos.y, 4u, 4u);
    face = insertBits(face, pos.z, 8u, 4u);
    face = insertBits(face, i, 12u, 3u);

    return face;
}

const WORKGROUP_SIZE = 256u;
const FACES_LEN = WORKGROUP_SIZE * 6;

var<workgroup> workgroup_cursor: atomic<u32>;
var<workgroup> workgroup_faces: array<u32, FACES_LEN>;

@compute
@workgroup_size(WORKGROUP_SIZE)
fn generateFaces(
    @builtin(local_invocation_index) local_index: u32,
    @builtin(global_invocation_id) global_id: vec3<u32>,
) {
    if local_index == 0 {
        atomicStore(&workgroup_cursor, 0u);
    }

    workgroupBarrier();

    if global_id.x < blocks_len {
        let block = blocks[global_id.x];
        let pos = blockPos(block);
        let mid = vec3<f32>(pos) + vec3(0.5);

        let clip_mid_h = clip_from_world_with_margin * vec4(mid, 1.0);
        let clip_mid = clip_mid_h.xyz / clip_mid_h.w;

        let max_dist = max(abs(clip_mid.x), max(abs(clip_mid.y), abs(clip_mid.z)));

        if max_dist <= 1.0 {
            for (var i = 0u; i < 6; i++) {
                var axis_array = array(0.0, 0.0, 0.0);
                axis_array[i >> 1] = select(-1.0, 1.0, bool(i & 1));

                let axis = vec3(axis_array[0], axis_array[1], axis_array[2]);
                let origin = fma(axis, vec3(0.5), mid);

                if dot(normalize(eye - origin), axis) > 0.0 {
                    let fi = atomicAdd(&workgroup_cursor, 1u);
                    workgroup_faces[fi] = newFace(pos, i);
                }
            }
        }
    }

    workgroupBarrier();

    let len = atomicLoad(&workgroup_cursor);

    if local_index == 0 {
        let i = atomicAdd(&cursor, len);
        atomicStore(&workgroup_cursor, i);
    }

    workgroupBarrier();

    for (var wg = 0u; wg < FACES_LEN; wg += WORKGROUP_SIZE) {
        let i = local_index + wg;
        if i < len {
            let fi = atomicLoad(&workgroup_cursor) + i;
            faces[fi] = workgroup_faces[i];
        }
    }
}

struct DrawIndirect {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

@group(0)
@binding(6)
var<storage, read_write> draw_indirect: DrawIndirect;

@compute
@workgroup_size(1)
fn doubleFacesLen() {
    draw_indirect.vertex_count = atomicLoad(&cursor) * 6;
    draw_indirect.instance_count = 1u;
}
