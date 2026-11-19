//! A picture of a mesh, drawn with no window.
//!
//! `picture-of <mesh.obj|cube|sphere|plane> <out.png>`, with `--size`,
//! `--turn` and `--pitch` for where to stand. Exits 0 when it wrote the
//! file, 1 when it could not draw, and 2 when it was asked wrongly, so a
//! shell can use it.
//!
//! The built-in names are here because the project holds no `.obj` file at
//! all, so a tool that could only read one could not be run.
//!
//! **It is here to be a consumer that is not a game.** Every other thing
//! built on this engine implements [`blitzkit::Game`] and hands itself to
//! [`blitzkit::start`], which takes the window, the event loop and the
//! renderer and does not give them back. Nothing had ever driven the
//! renderer from the outside except `picture`, which is a test harness and
//! lives in the crate. So the offscreen path was public and unused, and a
//! path nobody walks is a path nobody knows is broken.
//!
//! What it found is written at the foot of this file, because a tool that
//! was built to learn something should say what it learned.

use std::path::Path;
use std::process::ExitCode;

use blitzkit::camera::Camera;
use blitzkit::geometry::Geometry;
use blitzkit::lighting::PointLight;
use blitzkit::mesh::{MeshData, Transform};
use blitzkit::renderer::render_text::TextRenderer;
use blitzkit::renderer::scene::Scene;
use blitzkit::renderer::Renderer;
use glam::{Vec3, Vec4};

/// How far from the middle the camera stands, as a share of the model's
/// bounding sphere, before the field of view is taken into account.
///
/// A little over one, so the silhouette has air round it rather than
/// touching the frame.
const CLEARS: f32 = 1.25;

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(blitzkit::LOG))
        .init();

    let said: Vec<String> = std::env::args().skip(1).collect();
    let plain = plain(&said);

    let [from, to] = plain.as_slice() else {
        eprintln!(
            "picture-of <mesh.obj|cube|sphere|plane> <out.png> \
             [--size 1200x948] [--turn 35] [--pitch 20]"
        );

        return ExitCode::from(2);
    };

    let Some((wide, tall)) = size() else {
        eprintln!("--size is written as 1200x948");

        return ExitCode::from(2);
    };

    let mut mesh = match asked_for(from, (wide, tall)) {
        Ok(mesh) => mesh,
        Err(why) => {
            eprintln!("{}", why);

            return ExitCode::from(2);
        }
    };

    if mesh.vertices.is_empty() {
        eprintln!("{} has nothing in it", from);

        return ExitCode::from(2);
    }

    mesh.fill_missing_normals();

    match drawn(&mesh, (wide, tall), Path::new(to)) {
        Ok(()) => {
            println!("{} by {} of {} into {}", wide, tall, from, to);

            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("{}", why);

            ExitCode::FAILURE
        }
    }
}

/// Which options here are given a value, as `one_argument` reads them.
///
/// Written out because nothing can be asked. See the note at the foot.
const TAKES_A_VALUE: [&str; 3] = ["--size", "--turn", "--pitch"];

/// How many segments round a built-in sphere, for a picture whose short
/// side is `across` pixels.
///
/// **Off the picture, because the picture is what you look at.** The games
/// draw spheres of 16 to 24 segments, which is right for a bulb or a
/// marble a few score pixels wide and moving. Standing still and filling
/// the frame, 32 drew a silhouette whose angles Jake could count. A still
/// picture has no frame budget to protect, so the only question left is
/// how far the chord falls inside the arc.
///
/// That gap is about `r * pi^2 / (2n^2)` for `n` segments, so holding it
/// under half a pixel wants `n` of about `pi * sqrt(r)`, with `r` the drawn
/// radius in pixels. Capped at 512, which is a quarter of a million
/// triangles and already far past the point where more changes anything.
fn round_enough(across: u32) -> u32 {
    // the sphere stands about four fifths of the short side, per `CLEARS`
    let radius = (across as f32 * 0.4).max(1.0);

    ((std::f32::consts::PI * radius.sqrt()).ceil() as u32).clamp(32, 512)
}

/// What was said that is not an option or an option's value.
///
/// **A filter on `--` is not enough.** `--turn 125` leaves `125`, which
/// then counts as a third positional and the whole run prints its usage and
/// exits. The engine's own reader already knows that `--turn` swallows what
/// follows it; it just has no way to say so, so this says it again.
fn plain(said: &[String]) -> Vec<&String> {
    let mut plain = Vec::new();
    let mut said = said.iter();

    while let Some(one) = said.next() {
        if !one.starts_with("--") {
            plain.push(one);

            continue;
        }

        // `--name=value` carries its value and eats nothing after it
        if !one.contains('=') && TAKES_A_VALUE.contains(&one.as_str()) {
            said.next();
        }
    }

    plain
}

/// The mesh behind a name: one of the engine's own, or a file to read.
///
/// A name is tried first and a file second, so `cube` is the built-in even
/// where a file of that name sits in the directory. The names are the ones
/// [`MeshData`] already has.
fn asked_for(name: &str, (wide, tall): (u32, u32)) -> Result<MeshData, String> {
    match name {
        "cube" => return Ok(MeshData::cube()),
        "sphere" => {
            let round = round_enough(wide.min(tall));

            return Ok(MeshData::sphere(round, round / 2));
        }
        "plane" => return Ok(MeshData::plane()),
        _ => {}
    }

    // A name carrying an extension was meant as a file, so say so plainly.
    // Only a bare word could have been a built-in spelt wrong.
    let bytes = std::fs::read(name).map_err(|why| {
        match (why.kind(), Path::new(name).extension().is_some()) {
            (std::io::ErrorKind::NotFound, true) => format!("{}: no such file", name),
            (std::io::ErrorKind::NotFound, false) => {
                format!("{} is neither a built-in nor a file", name)
            }
            _ => format!("{}: {}", name, why),
        }
    })?;

    MeshData::from_obj(&bytes)
        .map_err(|why| format!("{} is not an obj this can read: {}", name, why))
}

/// Draws it and writes it.
fn drawn(mesh: &MeshData, (wide, tall): (u32, u32), to: &Path) -> Result<(), String> {
    let Some(mut renderer) = pollster::block_on(Renderer::offscreen(wide, tall)) else {
        return Err("no adapter here, so nothing was drawn".to_string());
    };

    let bounds = mesh.bounds();
    let middle = (bounds.min + bounds.max) * 0.5;
    // **Measured off the vertices, not off the box.** The half-diagonal of
    // the bounding box is the right answer only for a shape that reaches
    // its corners: for a sphere it is 1.73 times too big, and the first
    // build of this framed one at two fifths of the height it should have
    // had.
    let radius = mesh
        .vertices
        .iter()
        .map(|one| (Vec3::from(one.position) - middle).length())
        .fold(0.0f32, f32::max)
        .max(1e-4);

    let id = renderer.add_mesh(mesh);

    let mut scene = Scene::new();
    scene.push_material(
        id,
        &Transform::new(),
        Vec4::new(0.74, 0.72, 0.69, 1.0),
        64.0,
    );

    let camera = standing(middle, radius, (wide, tall));

    // **Lit off the camera rather than off the world**, because this is
    // handed one object at a time with no say in which way it was modelled,
    // and a fixed direction leaves whichever face it misses in ambient.
    //
    // Across and down rather than along the eye: a light behind the viewer
    // flattens everything it touches, which is what the first build drew.
    let eye = (camera.position - middle).normalize();
    let across = eye.cross(Vec3::Y).normalize_or_zero();

    scene.light.direction = (-across - Vec3::Y * 0.8 - eye * 0.35).normalize();
    scene.light.ambient = Vec3::splat(0.17);
    // and one fill from the other side, so the shaded half is still a shape.
    //
    // **In front of the model rather than level with it.** Set out to the
    // side it grazes the limb, and a grazing Blinn-Phong highlight on a
    // faceted surface is a hard bright sliver one quad wide, which read as a
    // seam in the mesh and was not one.
    scene.push_light(PointLight::new(
        middle + (across * 0.8 + eye * 0.7 + Vec3::Y * 0.3).normalize() * radius * 3.0,
        Vec3::new(0.85, 0.88, 1.0),
        radius * 0.9,
        radius * 20.0,
    ));

    renderer.set_camera(camera);

    let picture = renderer.picture(&scene, &Geometry::new(), &TextRenderer::new());

    blitzkit::picture::save(&picture, to)
}

/// Where to stand to hold the whole of it.
///
/// `near` and `far` come off the model instead of keeping [`Camera::new`]'s
/// 0.1 and 100, which are a room's numbers: a teapot of 0.3 across gets a
/// far plane of 100 and all its depth in the first thousandth of the
/// buffer, and anything bigger than a hundred metres is simply cut off.
fn standing(middle: Vec3, radius: f32, (wide, tall): (u32, u32)) -> Camera {
    let mut camera = Camera::new();

    let turn = degrees("--turn", 35.0).to_radians();
    let pitch = degrees("--pitch", 20.0).to_radians();

    // the vertical field is the one that is fixed, so a frame wider than it
    // is tall holds this and a frame taller than it is wide has to back off
    let across = (wide as f32 / tall as f32).min(1.0);
    let out = radius * CLEARS / ((camera.fov_y * 0.5).sin() * across);

    camera.position = middle
        + Vec3::new(
            turn.sin() * pitch.cos(),
            pitch.sin(),
            turn.cos() * pitch.cos(),
        ) * out;
    camera.target = middle;
    camera.near = (out - radius * 2.0).max(radius * 0.01);
    camera.far = out + radius * 4.0;

    camera
}

/// `--size 1200x948`, or the engine's own picture size.
fn size() -> Option<(u32, u32)> {
    let Some(said) = blitzkit::one_argument("--size") else {
        return Some(blitzkit::PICTURE_SIZE);
    };

    let (wide, tall) = said.split_once('x')?;
    let (Ok(wide), Ok(tall)) = (wide.parse(), tall.parse()) else {
        return None;
    };

    (wide > 0 && tall > 0).then_some((wide, tall))
}

/// One of the angles, in degrees, or what it is when nobody says.
///
/// A number that will not parse is the default rather than a refusal,
/// because the engine's own reader already warns on `--turn --pitch` and
/// there is nothing here worth a second account of it.
fn degrees(name: &str, by_default: f32) -> f32 {
    blitzkit::one_argument(name)
        .and_then(|said| said.trim().parse().ok())
        .unwrap_or(by_default)
}

// ## What this found
//
// **Nothing here needed `Game` and nothing here needed a window**, which
// was the question. The engine's shape is a single-window process that owns
// its own loop, and that shape is `start`'s, not the renderer's.
// `Renderer::offscreen`, `set_camera`, `picture` and `picture::save` are
// all public and all behave. `set_camera` does the quiet work: it sets the
// camera's viewport off the surface config, so the aspect is right without
// being asked for.
//
// **The command line cannot be shared.** This is the one that bit.
// `one_argument` reads `std::env::args` itself and returns only the value,
// so a caller is never told which tokens were taken. Anything that wants a
// positional argument of its own has to know, separately, which options
// swallow what follows them: `picture-of sphere out.png --turn 125` read
// `125` as a third positional and printed its usage, and `TAKES_A_VALUE`
// above exists because of it. That list is a second account of what
// `one_of` already knows, which is the fault this project writes down as
// two lists of one thing. `one_of` takes an iterator and is private; the
// fix is to make it public, or to have the engine hand back what it did
// not consume.
//
// **`Camera::new`'s near and far are a room's numbers.** 0.1 and 100 are
// the arcade's scale. Anything handed a model it has not seen has to
// replace both, which is what `standing` does.
//
// **Not a finding, kept because it looked like one.** A hard bright sliver
// down the sphere's limb read as a seam in `MeshData::sphere`. It was this
// tool's own fill light grazing a faceted surface, and it moved when the
// light moved. The check that settled it was two renders from different
// angles: the mark held its place on screen rather than on the model.
