// A ray against the meshes the engine draws. Spec 0107, over spec 0098.
//
// **Its own bind group rather than the mesh shader's.** The camera's group
// carries sixty four lamps, four spots, two shadow tables and the contact
// march's planes, and a tracer reads none of it. Declaring that struct again
// here to borrow three fields from it would be a second copy of forty lines
// that nothing holds together, which is the fault spec 0067 wrote `shaders.rs`
// to stop. So this takes what it needs and no more.

// How deep a walk of either level may go before it gives up on a branch.
//
// **A stack and not recursion**, which WGSL has not got. Thirty two is far
// past what a middle split over nine thousand instances reaches, and a
// branch that would go deeper is dropped rather than overrunning the array:
// a ray that misses something is a wrong pixel and a write past the end is
// a wrong anything.
const DEEP: u32 = 32u;

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

// **What is drawn is relative to the eye and what is traced is not.**
// Spec 0051 hands every position over with the camera taken off it, because
// a world a hundred and fifty metres across loses the float precision a
// mirror's edge needs, and the fragment this shader runs on arrives that
// way. Spec 0107's structure is in the world instead, so that a frame which
// moved nothing need not build it again, so a ray gets the eye added back
// on to it before it is traced and the hit comes back in the world.
struct Tracing {
    view_projection: mat4x4<f32>,
    // xyz is where the eye is in the world, w is how many instances are real
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

// One node of either level. Thirty two bytes: the box, and in the padding
// of each corner the two numbers a walk needs. A leaf says where its first
// item is and how many it holds; a branch says where its right child is and
// that it holds none, its left child being the node straight after it.
//
// **Two four-float lanes and not a struct of vectors**, because a
// `vec3<f32>` in a storage array aligns to sixteen and would put the two
// counts in padding the Rust side does not pay for.
struct Node {
    least: vec3<f32>,
    first: u32,
    most: vec3<f32>,
    count: u32,
};

// One instance the top level holds: the inverse that takes a ray into its
// own space, three rows of four, then its colour and where the mesh under it
// keeps itself.
struct Placed {
    into_it_0: vec4<f32>,
    into_it_1: vec4<f32>,
    into_it_2: vec4<f32>,
    color: vec4<f32>,
    first_node: u32,
    first_tri: u32,
    first_index: u32,
    first_vertex: u32,
};

@group(0) @binding(1) var<storage, read> top: array<Node>;
@group(0) @binding(2) var<storage, read> placed: array<Placed>;
@group(0) @binding(3) var<storage, read> nodes: array<Node>;
@group(0) @binding(4) var<storage, read> tris: array<u32>;
@group(0) @binding(5) var<storage, read> indices: array<u32>;
// **Eight floats a vertex, read loose rather than as a struct.** A vertex is
// a position, a normal and a pair of texture coordinates packed tight, and
// a WGSL struct of three vectors would align each to sixteen and read the
// wrong bytes. The tracer wants the position and nothing else.
@group(0) @binding(6) var<storage, read> vertices: array<f32>;

fn corner(vertex: u32) -> vec3<f32> {
    let at = vertex * 8u;

    return vec3<f32>(vertices[at], vertices[at + 1u], vertices[at + 2u]);
}

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

// Whether a ray meets a node's box at all, and how far along if it does.
//
// The slab test, which is the whole of the arithmetic here as it was for the
// box list. `collision::met` in Rust is the other copy, and
// `renderer::tests::the_shader_and_the_slab_agree` holds them together.
//
// **A ray parallel to a slab it starts outside never meets the box, and the
// divide carries that**: the two ends come back as infinities of one sign
// and the near end wins nothing.
fn meets(least: vec3<f32>, most: vec3<f32>, start: vec3<f32>, way: vec3<f32>, best: f32) -> bool {
    let a = (least - start) / way;
    let b = (most - start) / way;
    let near = min(a, b);
    let far = max(a, b);
    let in_at = max(max(near.x, near.y), near.z);
    let out_at = min(min(far.x, far.y), far.z);

    return out_at >= max(in_at, 0.0) && in_at < best;
}

struct Hit {
    at: f32,
    normal: vec3<f32>,
    color: vec4<f32>,
    met: bool,
};

// Where a ray meets one triangle, and which way that triangle faces.
//
// Möller and Trumbore's: no plane equation and no division until the end.
// **Two sided**, because a reflection shows the inside of a wall and
// culling it would show the room behind.
fn met_a_triangle(a: vec3<f32>, b: vec3<f32>, c: vec3<f32>, start: vec3<f32>, way: vec3<f32>) -> vec2<f32> {
    let one = b - a;
    let two = c - a;
    let across = cross(way, two);
    let flat = dot(one, across);

    if (abs(flat) < 1e-12) {
        return vec2<f32>(-1.0, 0.0);
    }

    let back = 1.0 / flat;
    let to = start - a;
    let u = dot(to, across) * back;

    if (u < 0.0 || u > 1.0) {
        return vec2<f32>(-1.0, 0.0);
    }

    let other = cross(to, one);
    let v = dot(way, other) * back;

    if (v < 0.0 || u + v > 1.0) {
        return vec2<f32>(-1.0, 0.0);
    }

    return vec2<f32>(dot(two, other) * back, 1.0);
}

// What a ray meets inside one instance, in that instance's own space.
//
// **The nearest triangle and its own normal**, taken from the winding
// rather than from the vertices' stored normals: what the tracer is asked
// for is where the surface is, and a vertex normal is smoothed across a
// face it was never meant to answer for.
fn met_in_a_mesh(one: Placed, start: vec3<f32>, way: vec3<f32>, best: f32) -> vec2<f32> {
    var near = best;
    var which = -1.0;
    var stack: array<u32, DEEP>;
    var deep = 1u;

    stack[0] = one.first_node;

    while (deep > 0u) {
        deep = deep - 1u;

        let at = stack[deep];
        let node = nodes[at];

        if (!meets(node.least, node.most, start, way, near)) {
            continue;
        }

        if (node.count == 0u) {
            // the near child is the node straight after this one and the
            // far one is where this node says, which is the whole of why a
            // walk needs no second index
            if (deep + 2u <= DEEP) {
                stack[deep] = at + 1u;
                stack[deep + 1u] = node.first;
                deep = deep + 2u;
            }

            continue;
        }

        for (var n: u32 = 0u; n < node.count; n = n + 1u) {
            let tri = tris[node.first + n] * 3u;
            let a = corner(indices[tri] + one.first_vertex);
            let b = corner(indices[tri + 1u] + one.first_vertex);
            let c = corner(indices[tri + 2u] + one.first_vertex);
            let met = met_a_triangle(a, b, c, start, way);

            if (met.y > 0.5 && met.x > 1e-4 && met.x < near) {
                near = met.x;
                which = f32(tri);
            }
        }
    }

    return vec2<f32>(near, which);
}

// And what it meets in the world, which is the top level over the instances
// and then the mesh under whichever of them the ray reaches.
//
// **The ray is taken into each instance's own space** by the inverse that
// instance carries, which is what makes nine thousand instances of a
// hundred and sixty meshes cost a hundred and sixty hierarchies rather than
// nine thousand. The direction is not normalised on the way in on purpose:
// an instance that scales makes the ray longer or shorter in its own space,
// and leaving it alone is what keeps the distance meaning the same thing on
// both sides.
fn trace(ray: vec3<f32>, way: vec3<f32>) -> Hit {
    var best: Hit;
    best.met = false;
    best.at = 1e30;

    if (tracing.camera.w < 0.5) {
        return best;
    }

    let start = ray + tracing.camera.xyz;
    var stack: array<u32, DEEP>;
    var deep = 1u;

    stack[0] = 0u;

    while (deep > 0u) {
        deep = deep - 1u;

        let at = stack[deep];
        let node = top[at];

        if (!meets(node.least, node.most, start, way, best.at)) {
            continue;
        }

        if (node.count == 0u) {
            if (deep + 2u <= DEEP) {
                stack[deep] = at + 1u;
                stack[deep + 1u] = node.first;
                deep = deep + 2u;
            }

            continue;
        }

        for (var n: u32 = 0u; n < node.count; n = n + 1u) {
            let one = placed[node.first + n];
            let into = mat4x4<f32>(
                vec4<f32>(one.into_it_0.x, one.into_it_1.x, one.into_it_2.x, 0.0),
                vec4<f32>(one.into_it_0.y, one.into_it_1.y, one.into_it_2.y, 0.0),
                vec4<f32>(one.into_it_0.z, one.into_it_1.z, one.into_it_2.z, 0.0),
                vec4<f32>(one.into_it_0.w, one.into_it_1.w, one.into_it_2.w, 1.0),
            );
            let start = (into * vec4<f32>(start, 1.0)).xyz;
            let along = (into * vec4<f32>(way, 0.0)).xyz;
            let met = met_in_a_mesh(one, start, along, best.at);

            if (met.y < 0.0) {
                continue;
            }

            let tri = u32(met.y);
            let a = corner(indices[tri] + one.first_vertex);
            let b = corner(indices[tri + 1u] + one.first_vertex);
            let c = corner(indices[tri + 2u] + one.first_vertex);
            // a normal goes the other way about, by the transpose of the
            // inverse, which is the transpose of this
            let flat = cross(b - a, c - a);

            best.at = met.x;
            best.color = one.color;
            best.met = true;
            best.normal = normalize(vec3<f32>(
                dot(one.into_it_0.xyz, flat),
                dot(one.into_it_1.xyz, flat),
                dot(one.into_it_2.xyz, flat),
            ));
        }
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
        // a negative range is a lamp this surface's own mirror cannot see,
        // which is nothing to do with what the lamp lights
        let reach = abs(where_it_is.w);

        if (away > reach || away <= 0.0) {
            continue;
        }

        // the engine's own falloff, which is `(1 - d/range)` squared and
        // reaches exactly nothing at the range, the same as the mesh pass
        let left = 1.0 - away / reach;

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
        // **Only the lamps this thing can see.** There is no shadow in a
        // traced surface, so a lamp behind a floor put a highlight on
        // capriccio's ball from a storey down, with nothing in the room to
        // account for it. The renderer marks those with a negative range.
        if (where_it_is.w <= 0.0) {
            continue;
        }

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
