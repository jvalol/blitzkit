// A ray against a list of boxes. Spec 0098.
//
// **Its own bind group rather than the mesh shader's.** The camera's group
// carries sixty four lamps, four spots, two shadow tables and the contact
// march's planes, and a tracer reads none of it. Declaring that struct again
// here to borrow three fields from it would be a second copy of forty lines
// that nothing holds together, which is the fault spec 0067 wrote `shaders.rs`
// to stop. So this takes what it needs and no more.

// How many boxes a traced surface will look at. Held against the Rust by
// `renderer::tests::the_shaders_hold_the_same_caps`.
const MAX_TRACED: u32 = 1024u;

// How many times a ray may bend or bounce before it gives up and takes the
// ambient. Each one is a whole pass over the list.
const BOUNCES: u32 = 3u;

// **Everything here is relative to the eye, per spec 0051.** The engine
// hands every position over with the camera taken off it, because a world
// a hundred and fifty metres across loses the float precision a mirror's
// edge needs. So the boxes arrive shifted the same way the instances are,
// the camera is at the origin, and a ray starts where the fragment is.
//
// That is not a detail of this shader. It is a thing spec 0098 did not
// know and the renderer will have to do to the list before it uploads it.
struct Tracing {
    view_projection: mat4x4<f32>,
    // xyz is the camera, w is how many boxes are real
    camera: vec4<f32>,
    // rgb is what a ray that leaves the room comes back as, a is unused
    ambient: vec4<f32>,
    // xyz is the direction the light travels, w is unused
    light_direction: vec4<f32>,
    // rgb is the light's colour, a is its intensity
    light_color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> tracing: Tracing;

struct Box {
    least: vec3<f32>,
    most: vec3<f32>,
    color: vec4<f32>,
};

@group(0) @binding(1) var<storage, read> boxes: array<Box, MAX_TRACED>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world_position: vec3<f32>,
    // where the thing this fragment is on stands, and how big it is, which
    // is what lets the fragment know it is on a sphere
    @location(2) middle: vec3<f32>,
    @location(3) radius: f32,
    // how hard it bends, which rides in the instance's shininess because a
    // lens has no highlight and the alternative is a uniform the pass would
    // have to rewrite between draws
    @location(4) index: f32,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) model_0: vec3<f32>,
    @location(4) model_1: vec3<f32>,
    @location(5) model_2: vec3<f32>,
    @location(6) model_3: vec3<f32>,
    @location(7) color: vec4<f32>,
    @location(8) shininess: f32,
    @location(9) specular: f32,
) -> VertexOutput {
    let model = mat4x4<f32>(
        vec4<f32>(model_0, 0.0),
        vec4<f32>(model_1, 0.0),
        vec4<f32>(model_2, 0.0),
        vec4<f32>(model_3, 1.0),
    );
    let world = model * vec4<f32>(position, 1.0);

    var out: VertexOutput;
    out.clip_position = tracing.view_projection * world;
    out.color = color;
    out.world_position = world.xyz;
    out.middle = model_3;
    // a unit sphere scaled, so one column's length is the radius
    out.radius = length(model_0);
    out.index = shininess;

    return out;
}

// The slab test, which is the whole of the arithmetic. `collision::met` in
// Rust is the other copy, and
// `renderer::tests::the_shader_and_the_slab_agree` holds them together.
struct Hit {
    at: f32,
    normal: vec3<f32>,
    color: vec4<f32>,
    met: bool,
};

fn trace(ray: vec3<f32>, way: vec3<f32>) -> Hit {
    var best: Hit;
    best.met = false;
    best.at = 1e30;

    let count = u32(tracing.camera.w);

    for (var n: u32 = 0u; n < count; n = n + 1u) {
        let one = boxes[n];
        // a ray parallel to a slab it starts outside never meets the box,
        // and the divide below carries that: the two ends come back as
        // infinities of one sign and the near end wins nothing
        let a = (one.least - ray) / way;
        let b = (one.most - ray) / way;
        let near = min(a, b);
        let far = max(a, b);
        let in_at = max(max(near.x, near.y), near.z);
        let out_at = min(min(far.x, far.y), far.z);

        if (out_at < in_at || out_at < 1e-4 || in_at < 1e-4 || in_at >= best.at) {
            continue;
        }

        best.at = in_at;
        best.color = one.color;
        best.met = true;
        // whichever slab the ray entered through is the face it struck
        best.normal = -sign(way) * step(near.yzx, near.xyz) * step(near.zxy, near.xyz);
    }

    return best;
}

fn shade(hit: Hit) -> vec3<f32> {
    if (!hit.met) {
        return tracing.ambient.rgb;
    }

    let lamp = normalize(-tracing.light_direction.xyz);
    let lit = max(dot(normalize(hit.normal), lamp), 0.0) * tracing.light_color.a;

    return hit.color.rgb * (tracing.ambient.rgb + tracing.light_color.rgb * lit);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let index = in.index;
    var on = in.world_position;
    // the eye is the origin, per the note on `Seen`
    var way = normalize(on);
    let normal = normalize(on - in.middle);

    // in through the near face
    var inside = refract(way, normal, 1.0 / index);

    if (length(inside) < 1e-4) {
        // past the critical angle, which is a reflection rather than a bend
        inside = reflect(way, normal);
    }

    // and out through the far one, which this knows because it is a sphere
    let to = on - in.middle;
    let along = dot(to, inside);
    let under = along * along - dot(to, to) + in.radius * in.radius;
    let leaves = on + inside * (-along + sqrt(max(under, 0.0)));
    let out_normal = normalize(leaves - in.middle);
    var onward = refract(inside, -out_normal, index);

    if (length(onward) < 1e-4) {
        onward = reflect(inside, -out_normal);
    }

    var at = leaves + onward * 1e-3;
    var carried = vec3<f32>(1.0, 1.0, 1.0);
    var got = vec3<f32>(0.0, 0.0, 0.0);

    for (var n: u32 = 0u; n < BOUNCES; n = n + 1u) {
        let hit = trace(at, onward);

        got = got + carried * shade(hit);

        if (!hit.met) {
            break;
        }

        // and on, dimmer, which is what keeps a corner of a room from
        // trading light back and forth for ever
        carried = carried * 0.0;
        break;
    }

    return vec4<f32>(got * in.color.rgb, 1.0);
}
