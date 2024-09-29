@group(0)
@binding(0)
var<storage> blocks: array<u32>;
@group(0)
@binding(1)
var<storage> chunks: array<vec2<u32>>;
@group(0)
@binding(2)
var<storage> chunks_len: u32;
@group(0)
@binding(3)
var<storage, read_write> chunk_cursor: atomic<u32>;
@group(0)
@binding(4)
var<storage, read_write> faces: array<vec2<u32>>;
@group(0)
@binding(5)
var<storage, read_write> face_cursor: atomic<u32>;
@group(0)
@binding(6)
var<storage> eye: vec3<f32>;
@group(0)
@binding(7)
var<storage> clip_from_world_with_margin: mat4x4<f32>;

fn blockPos(block: u32) -> vec3<u32> {
    return vec3(
        extractBits(block, 0u, 4u),
        extractBits(block, 4u, 4u),
        extractBits(block, 8u, 4u),
    );
}

fn newFace(pos: vec3<u32>, i: u32) -> vec2<u32> {
    var face = vec2(0u);

    face.x = insertBits(face.x, pos.x, 0u, 9u);
    face.x = insertBits(face.x, pos.y, 9u, 9u);
    face.x = insertBits(face.x, pos.z, 18u, 9u);
    face.y = i;

    return face;
}

const WORKGROUP_SIZE = 256u;
const FACES_LEN = WORKGROUP_SIZE * 3;

var<workgroup> workgroup_face_cursor: atomic<u32>;
var<workgroup> workgroup_faces: array<vec2<u32>, FACES_LEN>;

var<workgroup> broadcast: u32;

fn genChunkFaces(chunk: vec2<u32>, block_index: u32, local_index: u32) {
    if local_index == 0 {
        atomicStore(&workgroup_face_cursor, 0u);
    }

    workgroupBarrier();

    if block_index < chunk.x {
        let block = blocks[block_index];
        let unpacked = unpack4xU8(chunk.y);
        let pos = blockPos(block) + unpacked.xyz * vec3(16);
        let mid = vec3<f32>(pos) + vec3(0.5);

        let clip_mid_h = clip_from_world_with_margin * vec4(mid, 1.0);
        let clip_mid = clip_mid_h.xyz / clip_mid_h.w;

        let max_dist = max(
            abs(clip_mid.x),
            max(abs(clip_mid.y), abs(clip_mid.z)),
        );

        if max_dist <= 1.0 {
            for (var i = 0u; i < 6; i++) {
                let has_face = bool(extractBits(block, i + 12, 1u));
                if has_face {
                    var axis_array = array(0.0, 0.0, 0.0);
                    axis_array[i >> 1] = select(-1.0, 1.0, bool(i & 1));

                    let axis = vec3(axis_array[0], axis_array[1], axis_array[2]);
                    let origin = fma(axis, vec3(0.5), mid);

                    if dot(normalize(eye - origin), axis) > 0.0 {
                        let face_index = atomicAdd(&workgroup_face_cursor, 1u);
                        workgroup_faces[face_index] = newFace(pos, i);
                    }
                }
            }
        }
    }

    workgroupBarrier();

    let len = atomicLoad(&workgroup_face_cursor);

    if local_index == 0 {
        broadcast = atomicAdd(&face_cursor, len);
    }

    workgroupBarrier();

    let face_start = broadcast;

    for (var stride = 0u; stride < FACES_LEN; stride += WORKGROUP_SIZE) {
        let index = local_index + stride;
        if index < len {
            let face_index = index + face_start;
            faces[face_index] = workgroup_faces[index];
        }
    }
}

@compute
@workgroup_size(WORKGROUP_SIZE)
fn genFaces(@builtin(local_invocation_index) local_index: u32) {
    loop {
        storageBarrier();

        if local_index == 0 {
            broadcast = atomicAdd(&chunk_cursor, 1u);
        }

        workgroupBarrier();

        let chunk_index = broadcast;

        if chunk_index >= chunks_len {
            break;
        }

        let chunk = chunks[chunk_index];
        let chunk_start = select(
            chunks[chunk_index - 1].x,
            0u,
            chunk_index == 0,
        );
        let chunk_len = chunk.x - chunk_start;

        let unpacked = unpack4xU8(chunk.y);
        if unpacked.w == 1 {
            continue;
        }

        for (var stride = 0u; stride < chunk_len; stride += WORKGROUP_SIZE) {
            let block_index = local_index + stride + chunk_start;
            genChunkFaces(chunk, block_index, local_index);
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
@binding(8)
var<storage, read_write> draw_indirect: DrawIndirect;

@compute
@workgroup_size(1)
fn writeVertexCount() {
    draw_indirect.vertex_count = atomicLoad(&face_cursor) * 6;
    draw_indirect.instance_count = 1u;
}
