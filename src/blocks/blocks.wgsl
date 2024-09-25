@group(0)
@binding(0)
var<storage> blocks: array<u32>;
@group(0)
@binding(1)
var<storage, read_write> faces: array<u32>;
@group(0)
@binding(2)
var<storage, read_write> cursor: atomic<u32>;

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

var<workgroup> workgroup_cursor: atomic<u32>;
var<workgroup> workgroup_faces: array<u32, WORKGROUP_SIZE>;

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

    let block = blocks[global_id.x];
    let pos = blockPos(block);

    for (var i = 0u; i < 6; i++) {
        let fi = atomicAdd(&workgroup_cursor, 1u);
        workgroup_faces[fi] = newFace(pos, i);
    }

    workgroupBarrier();

    let len = atomicLoad(&workgroup_cursor);

    if local_index == 0 {
        let i = atomicAdd(&cursor, len);
        atomicStore(&workgroup_cursor, i);
    }

    workgroupBarrier();

    if local_index < len {
        let i = atomicLoad(&workgroup_cursor) + local_index;
        faces[i] = workgroup_faces[local_index];
    }
}

struct DrawIndirect {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

@group(0)
@binding(3)
var<storage, read_write> draw_indirect: DrawIndirect;

@compute
@workgroup_size(1)
fn doubleFacesLen() {
    draw_indirect.vertex_count = atomicLoad(&cursor) * 6;
    draw_indirect.instance_count = 1u;
}
