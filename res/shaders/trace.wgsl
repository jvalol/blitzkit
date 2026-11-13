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

// How many of the room's lamps a traced surface will take a highlight from.
// Held against the Rust by `scene::tests::the_tracer_holds_the_same_cap`.
//
// **Far fewer than the mesh pass's sixty four**, because this is not
// lighting a room: it is the handful of lamps near enough to a mirror to
// put a highlight on it, and every one is another loop over every fragment
// of the ball.
const MAX_TRACED_LAMPS: u32 = 32u;

// What a lamp looks like on a polished surface: a small white hot core
// with a soft halo round it, which is two lobes and not one.
//
// **One middling lobe reads as a smudge.** That is what was here, at 220,
// and Jake called the result soft. A real glint is far tighter than that
// and far brighter, bright enough to clip, because what a mirror shows of
// a lamp is the lamp. The halo is what says the surface is polished rather
// than painted, and on its own it is the smudge again.
const SHINE: f32 = 900.0;
const GLINT: f32 = 7.0;
const SHEEN: f32 = 32.0;
const SHEEN_LIT: f32 = 0.22;

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
    // x is how many lamps are real, the rest is spare
    lamps_used: vec4<u32>,
    // xyz where each lamp is, w how far it reaches; then rgb its colour and
    // a how hard it burns
    lamps: array<vec4<f32>, 64>,
};

@group(0) @binding(0) var<uniform> tracing: Tracing;

// **Nought is a box and one is a cylinder standing in it**, on the y axis,
// as wide as the box and as tall. It rides in the padding a `vec3<f32>`
// already costs, so a round shape is free in the list.
//
// Why there is a round shape at all: capriccio's colonnade is turned, and
// every stand-in built out of boxes read as a tube with corners. Jake went
// up to the ball and said the shape isn't true, which it was not and could
// not be.
struct Box {
    least: vec3<f32>,
    kind: f32,
    most: vec3<f32>,
    spare: f32,
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
    // **The radius is the scale times the mesh's own, not the scale.** This
    // read `length(model_0)` on the argument that a unit sphere scaled has
    // one column's length for its radius, and `MeshData::sphere` is a unit
    // *diameter*: every vertex of it is half a unit out. So every lens drawn
    // through here thought itself twice its real size, the exit point it
    // worked out was nowhere near the glass, and what came back was two flat
    // regions instead of a room.
    //
    // Taken off the vertex rather than written down, because a sphere's
    // vertices are all the same distance from its middle and that distance
    // is the thing being asked for. Spec 0064's room found this by building
    // on it; the engine's own picture was blessed by eye and a wrong bend
    // looks like a bend.
    out.radius = length(model_0) * length(position);
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

        if (one.kind > 0.5) {
            let middle = (one.least + one.most) * 0.5;
            let across = (one.most.x - one.least.x) * 0.5;
            let off = vec2<f32>(ray.x - middle.x, ray.z - middle.z);
            let along = vec2<f32>(way.x, way.z);
            let aa = dot(along, along);
            let bb = 2.0 * dot(off, along);
            let cc = dot(off, off) - across * across;
            let under = bb * bb - 4.0 * aa * cc;

            // a ray straight up the axis meets no side of it
            if (aa < 1e-9 || under < 0.0) {
                continue;
            }

            let root = sqrt(under);
            var met = (-bb - root) / (2.0 * aa);

            if (met < 1e-4) {
                met = (-bb + root) / (2.0 * aa);
            }
            if (met < 1e-4 || met >= best.at) {
                continue;
            }

            let on = ray + way * met;

            // **The side and not the ends.** A column's ends are covered by
            // the blocks above and below it, so the caps would be two more
            // tests for something nothing can see.
            if (on.y < one.least.y || on.y > one.most.y) {
                continue;
            }

            best.at = met;
            best.color = one.color;
            best.met = true;
            best.normal = normalize(vec3<f32>(on.x - middle.x, 0.0, on.z - middle.z));

            continue;
        }

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

// **And the room's own lamps, not just the sun.** This had the ambient and
// the one directional light, which is a sky's worth of lighting and nothing
// a room has: every surface inside the ball came back at one flat value, so
// the reflection read as a wash of grey whatever the room was doing. Jake
// said bland three times before this was taken seriously.
//
// A lit room is a run of pools with dark between them. That is what a lamp
// with a falloff gives and what an ambient cannot, and in a mirror it is
// most of what tells you the thing is showing a room at all.
fn shade(hit: Hit, on: vec3<f32>) -> vec3<f32> {
    if (!hit.met) {
        return tracing.ambient.rgb;
    }

    let normal = normalize(hit.normal);
    let sun = normalize(-tracing.light_direction.xyz);
    let lit = max(dot(normal, sun), 0.0) * tracing.light_color.a;
    var got = tracing.ambient.rgb + tracing.light_color.rgb * lit;
    let lamps = min(tracing.lamps_used.x, MAX_TRACED_LAMPS);

    for (var n: u32 = 0u; n < lamps; n = n + 1u) {
        let where_it_is = tracing.lamps[n * 2u];
        let what_it_is = tracing.lamps[n * 2u + 1u];
        let offset = where_it_is.xyz - on;
        let away = length(offset);

        if (away > where_it_is.w || away <= 0.0) {
            continue;
        }

        // the engine's own falloff, which is `(1 - d/range)` squared and
        // reaches exactly nothing at the range, the same as the mesh pass
        let left = 1.0 - away / where_it_is.w;

        got = got
            + what_it_is.rgb
                * what_it_is.a
                * left
                * left
                * max(dot(normal, offset / away), 0.0);
    }

    return hit.color.rgb * got;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let index = in.index;
    var on = in.world_position;
    // the eye is the origin, per the note on `Seen`
    var way = normalize(on);
    let normal = normalize(on - in.middle);

    var onward: vec3<f32>;
    var at: vec3<f32>;

    if (index <= 0.0) {
        // **A mirror rather than a lens.** An index at or under nought is
        // not a refractive index at all, which is what makes it a free
        // sentinel: every real one is over one.
        //
        // The difference is what the ball is for. A solid glass sphere is a
        // lens: it bends the room twice, turns it over and squeezes it into
        // a disc, which is true and reads as a smear of bands rather than
        // as a room. A mirror shows the room the right way up and in its
        // own colours, and a room is what capriccio's spec 0064 built one
        // of these to show. Jake, 2026-11-12: the reflections aren't
        // accurate, and it could look a lot more like a mirror.
        onward = reflect(way, normal);
        at = on + onward * 1e-3;
    } else {
        // in through the near face
        var inside = refract(way, normal, 1.0 / index);

        if (length(inside) < 1e-4) {
            // past the critical angle, which is a reflection rather than a
            // bend
            inside = reflect(way, normal);
        }

        // and out through the far one, which this knows because it is a
        // sphere
        let to = on - in.middle;
        let along = dot(to, inside);
        let under = along * along - dot(to, to) + in.radius * in.radius;
        let leaves = on + inside * (-along + sqrt(max(under, 0.0)));
        let out_normal = normalize(leaves - in.middle);

        onward = refract(inside, -out_normal, index);

        if (length(onward) < 1e-4) {
            onward = reflect(inside, -out_normal);
        }

        at = leaves + onward * 1e-3;
    }
    var carried = vec3<f32>(1.0, 1.0, 1.0);
    var got = vec3<f32>(0.0, 0.0, 0.0);

    for (var n: u32 = 0u; n < BOUNCES; n = n + 1u) {
        let hit = trace(at, onward);

        got = got + carried * shade(hit, at + onward * hit.at);

        if (!hit.met) {
            break;
        }

        // and on, dimmer, which is what keeps a corner of a room from
        // trading light back and forth for ever
        carried = carried * 0.0;
        break;
    }

    // **And the room's own lamps on the surface itself.** Everything above
    // is what the ray found; this is what the ball is lit by, which a lens
    // had no need of and a mirror does. Jake, 2026-11-12: there should be
    // specular highlights on the sphere from the lighting in the room.
    //
    // The lamps are not in the box list and could not be: a lamp is a point
    // and the list is boxes. What is in the list is whatever the game draws
    // round them, so the bays show up in the ball as the lit panels they
    // are, and this is the glint off the glass that no panel can give.
    var shine = vec3<f32>(0.0);
    let to_viewer = normalize(-on);
    let lamps = min(tracing.lamps_used.x, MAX_TRACED_LAMPS);

    for (var n: u32 = 0u; n < lamps; n = n + 1u) {
        let where_it_is = tracing.lamps[n * 2u];
        let what_it_is = tracing.lamps[n * 2u + 1u];
        let offset = where_it_is.xyz - on;
        let away = length(offset);

        if (away > where_it_is.w || away <= 0.0) {
            continue;
        }

        let to_light = offset / away;

        // a lamp behind the surface lights nothing on it, which the half
        // vector alone does not say
        if (dot(normal, to_light) <= 0.0) {
            continue;
        }

        let left = 1.0 - away / where_it_is.w;
        let half_way = normalize(to_light + to_viewer);
        let square = max(dot(normal, half_way), 0.0);

        shine = shine
            + what_it_is.rgb
                * what_it_is.a
                * left
                * left
                * (pow(square, SHINE) * GLINT + pow(square, SHEEN) * SHEEN_LIT);
    }

    // **How much it reflects depends on how square on you are to it**,
    // which is what makes a ball read as a ball rather than as a disc with
    // a picture on it. Straight on, a surface gives back its own colour;
    // at a grazing angle every surface there is gives back nearly all of
    // the light, so the rim of a sphere is a bright ring and the face is
    // not. Jake, 2026-11-12: it reads more like a reflective circle, I'm
    // looking for the kind of lighting that reads as a sphere.
    //
    // Schlick's, which is the cheap form everybody uses: the tint is what
    // it gives back face on and the fifth power carries it to all of it at
    // the edge.
    let facing = clamp(dot(normal, -way), 0.0, 1.0);
    let edge = pow(1.0 - facing, 5.0);
    let mirrored = in.color.rgb + (vec3<f32>(1.0) - in.color.rgb) * edge;

    return vec4<f32>(got * mirrored + shine, 1.0);
}
