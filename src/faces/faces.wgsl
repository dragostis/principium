@group(0)
@binding(0)
var<storage> faces: array<u32>;
@group(0)
@binding(1)
var<uniform> clip_from_world: mat4x4<f32>;

fn vertexPos(face: u32, local_vertex_index: u32) -> vec3<f32> {
    let block_pos = vec3(
        f32(extractBits(face, 0u, 4u)),
        f32(extractBits(face, 4u, 4u)),
        f32(extractBits(face, 8u, 4u)),
    );

    var pos = array(0.0, 0.0, 0.0);

    let locked_axis = extractBits(face, 13u, 2u);

    pos[locked_axis] = f32(extractBits(face, 12u, 1u));
    pos[(locked_axis + 1) % 3] = f32(extractBits(local_vertex_index, 0u, 1u));
    pos[(locked_axis + 2) % 3] = f32(extractBits(local_vertex_index, 1u, 1u)
        | u32(local_vertex_index == 4));

    return block_pos +  vec3(pos[0], pos[1], pos[2]);
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    let pos = vertexPos(faces[vertex_index / 6], vertex_index % 6);
    return clip_from_world * vec4(pos, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
