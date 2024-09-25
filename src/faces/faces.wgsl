@group(0)
@binding(0)
var<storage> faces: array<u32>;
@group(0)
@binding(1)
var<uniform> clip_from_world: mat4x4<f32>;

fn vertexPos(face: u32, local_vertex_index: u32) -> array<vec3<f32>, 2> {
    let block_pos = vec3(
        f32(extractBits(face, 0u, 4u)),
        f32(extractBits(face, 4u, 9u)),
        f32(extractBits(face, 13u, 4u)),
    );

    var local_pos = array(0.0, 0.0, 0.0);

    let axis = extractBits(face, 18u, 2u);

    local_pos[axis] = f32(extractBits(face, 17u, 1u));
    local_pos[(axis + 1) % 3] = f32(extractBits(local_vertex_index, 0u, 1u));
    local_pos[(axis + 2) % 3] = f32(extractBits(local_vertex_index, 1u, 1u)
        | u32(local_vertex_index == 4));

    let pos = block_pos +  vec3(local_pos[0], local_pos[1], local_pos[2]);

    var norm = array(0.0, 0.0, 0.0);

    norm[axis] = select(-1.0, 1.0, bool(extractBits(face, 17u, 1u)));

    return array(pos, vec3(norm[0], norm[1], norm[2]));
}

struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) norm: vec3<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let pos_norm = vertexPos(faces[vertex_index / 6], vertex_index % 6);
    return VertexOutput(clip_from_world * vec4(pos_norm[0], 1.0), pos_norm[1]);
}

const SUN = vec3(0.2, 0.8, 0.5);
const AMBIENT = 0.3;
const DIFFUSE = 0.7;
const BLOCK_COLOR = vec3(1.0, 1.0, 1.0);

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let sun = max(0.0, dot(vertex.norm, normalize(SUN)));
    let color = (AMBIENT + DIFFUSE * sun) * BLOCK_COLOR;

    return vec4(color, 1.0);
}
