// Collision boxes drawn over the finished scene, per spec 0046.
//
// Lines, unlit, in whatever colour the vertex carries. A debug view that
// takes the room's lighting goes dark in a dark room, which is where it is
// most wanted, so this one does not take it.

struct Uniforms {
    view_projection: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

// How far toward the camera an edge is pulled, in clip space.
//
// An outline traces a collider that usually agrees with the wall it is drawn
// over, and two surfaces at one depth fight: without this, the common case,
// the one where nothing is wrong, is the one that comes back as speckle. A
// pipeline depth bias would be the usual way and wgpu refuses one on
// anything that is not triangles, so it is done here.
//
// Scaled by w, so it is the same shift in normalised device coordinates
// wherever the edge is, rather than a shift that vanishes with distance.
const TOWARD: f32 = 0.0002;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_projection * vec4<f32>(in.position, 1.0);
    out.clip_position.z -= TOWARD * out.clip_position.w;
    out.color = in.color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
