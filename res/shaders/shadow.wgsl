// Depth only, from a light's point of view. No fragment stage: the depth buffer
// is the whole output. See specs/0015-shadows.md and 0021-spot-lights.md.
const MAX_SPOT_LIGHTS: u32 = 4u;

struct SpotLight {
    position_range: vec4<f32>,
    direction_cos_outer: vec4<f32>,
    color_intensity: vec4<f32>,
    cos_inner: vec4<f32>,
    view_projection: mat4x4<f32>,
};

struct PointLight {
    position_range: vec4<f32>,
    color_intensity: vec4<f32>,
};

struct Uniforms {
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    light_direction: vec4<f32>,
    light_color: vec4<f32>,
    ambient: vec4<f32>,
    light_view_projection: mat4x4<f32>,
    point_light_count: vec4<u32>,
    point_lights: array<PointLight, 8>,
    spot_light_count: vec4<u32>,
    spot_lights: array<SpotLight, MAX_SPOT_LIGHTS>,
    // these two are read by mesh.wgsl and not here, and they are declared
    // anyway because the field after them is read here and a uniform struct
    // is read by where its fields lie
    point_shadow: vec4<u32>,
    contact: vec4<f32>,
    // where the eye is, in the world's own coordinates. Spec 0059: a model
    // matrix arrives in world space and the eye comes off it here.
    eye: vec4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

// A model matrix with the eye taken off its translation, per spec 0059.
//
// Built from four columns of three, because the row that would make them
// four is nought, nought, nought, one for every transform this engine
// makes and is not carried. Spec 0060.
fn taken_off(
    model_0: vec3<f32>,
    model_1: vec3<f32>,
    model_2: vec3<f32>,
    model_3: vec3<f32>,
) -> mat4x4<f32> {
    return mat4x4<f32>(
        vec4<f32>(model_0, 0.0),
        vec4<f32>(model_1, 0.0),
        vec4<f32>(model_2, 0.0),
        vec4<f32>(model_3 - uniforms.eye.xyz, 1.0),
    );
}


// Which light this pass is filling the map for. Anything below MAX_SPOT_LIGHTS
// is that spot; anything at or above it is the sun. One pipeline fills every
// map, and this is the only thing that differs between the passes.
//
// A whole bind group for one number, because the alternative is immediate data,
// and that is a device capability this engine would then require everywhere.
struct Which {
    index: vec4<u32>,
};

@group(1) @binding(0) var<uniform> which: Which;

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(3) model_0: vec3<f32>,
    @location(4) model_1: vec3<f32>,
    @location(5) model_2: vec3<f32>,
    @location(6) model_3: vec3<f32>,
) -> @builtin(position) vec4<f32> {
    let model = taken_off(model_0, model_1, model_2, model_3);
    let world = model * vec4<f32>(position, 1.0);

    if which.index.x >= MAX_SPOT_LIGHTS {
        return uniforms.light_view_projection * world;
    }

    return uniforms.spot_lights[which.index.x].view_projection * world;
}

// --- point light faces, spec 0022 -------------------------------------------
//
// A lamp needs six of these passes, one along each axis, so what the pass is
// for is a layer number from nought to eleven rather than a light number. The
// matrix and the lamp both come from the layer.
//
// Unlike the sun and the spots, this one has a fragment stage: what goes in the
// map is distance from the lamp, not whatever depth the projection produced,
// because a reading has to mean the same thing on all six faces.

struct PointFace {
    view_projection: mat4x4<f32>,
    // xyz is the lamp, w is how far it reaches
    position_range: vec4<f32>,
};

struct PointFaces {
    faces: array<PointFace, 12>,
};

@group(2) @binding(0) var<uniform> point_faces: PointFaces;

struct PointOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
};

@vertex
fn vs_point(
    @location(0) position: vec3<f32>,
    @location(3) model_0: vec3<f32>,
    @location(4) model_1: vec3<f32>,
    @location(5) model_2: vec3<f32>,
    @location(6) model_3: vec3<f32>,
) -> PointOut {
    let model = taken_off(model_0, model_1, model_2, model_3);
    let world = model * vec4<f32>(position, 1.0);

    var out: PointOut;
    out.clip_position = point_faces.faces[which.index.x].view_projection * world;
    out.world_position = world.xyz;
    return out;
}

// Matches shadow::point_distance_fraction in Rust. Keep the two in step.
@fragment
fn fs_point(in: PointOut) -> @builtin(frag_depth) f32 {
    let lamp = point_faces.faces[which.index.x].position_range;
    if lamp.w <= 0.0 {
        return 1.0;
    }

    return clamp(distance(in.world_position, lamp.xyz) / lamp.w, 0.0, 1.0);
}
