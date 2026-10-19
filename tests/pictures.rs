//! Drawing a frame and comparing it against one that was right. Spec 0062.
//!
//! **Because five hundred tests passed while the sun stopped casting
//! shadows.** Every other test in this engine checks arithmetic that can be
//! checked without a GPU, which is most of what the engine is and the wrong
//! shape for anything between an instance in a buffer and a lit pixel. That
//! has to be drawn and looked at.
//!
//! A failure writes what was drawn and the difference beside the reference
//! and says where they are, because the only useful thing to do with a
//! failed picture test is look at it. `BLESS=1 cargo test --test pictures`
//! writes what the renderer produced over the references, and a blessed
//! picture is looked at before it is committed, which is the rule this
//! project already has for its clips.

use blitzkit::camera::Camera;
use blitzkit::geometry::Geometry;
use blitzkit::lighting::PointLight;
use blitzkit::mesh::{MeshData, Transform};
use blitzkit::renderer::render_text::TextRenderer;
use blitzkit::renderer::scene::{MeshId, Scene};
use blitzkit::renderer::{Picture, Renderer};
use glam::{vec3, vec4, Vec3};

/// How big a picture is.
///
/// Small, because every one of these is committed and read by a person: a
/// shadow and a highlight are plain at this size and a diff of it is
/// something you can look at.
const WIDE: u32 = 320;
const TALL: u32 = 240;

/// How far a channel may differ before a pixel counts as changed, and how
/// many pixels may change before a picture does.
///
/// Two GPUs rarely agree to the bit, and a driver that rounds a shadow's
/// edge differently should not fail a test about whether there is a shadow.
/// Both numbers live here rather than at each test, so a picture that fails
/// fails for a reason rather than for a tolerance somebody tuned.
///
/// **The share started at a hundredth and was too loose to do its job.**
/// Turning the sun's shadow off changed 0.96% of the frame, so the box stood
/// on no shadow at all and this file called it a match. A hundredth of a 320
/// by 240 picture is 768 pixels, which is a shadow.
/// `the_tolerance_can_see_a_highlight` is what holds the number down now.
const CHANNEL: i32 = 8;
const SHARE: f32 = 0.002;

/// What the tests draw, and the one place a scene is described.
struct Shot {
    name: &'static str,
    scene: fn(&mut Scene, &Meshes),
    camera: fn(&mut Camera),
}

struct Meshes {
    cube: MeshId,
    sphere: MeshId,
    floor: MeshId,
}

fn shots() -> Vec<Shot> {
    vec![
        // the sun, the shadow map, and a cast shadow: the picture spec 0059
        // would have failed
        Shot {
            name: "a-lit-box-casts-its-shadow",
            scene: |scene, meshes| {
                // Low and across, so the shadow lies out on the floor where
                // the camera can see it. The engine's default sun is almost
                // overhead, and the first blessing of this picture had the
                // box standing on its own shadow with nothing to look at.
                scene.light.direction = vec3(-0.8, -0.9, 0.5).normalize();
                scene.push_colored(
                    meshes.floor,
                    &Transform::at(Vec3::ZERO).with_scale(vec3(20.0, 1.0, 20.0)),
                    vec4(0.6, 0.6, 0.62, 1.0),
                );
                scene.push_colored(
                    meshes.cube,
                    &Transform::at(vec3(0.0, 1.0, 0.0)).with_scale(Vec3::splat(2.0)),
                    vec4(0.8, 0.5, 0.3, 1.0),
                );
            },
            camera: |camera| {
                camera.position = vec3(4.2, 2.6, 5.4);
                camera.target = vec3(-0.4, 0.7, 0.0);
            },
        },
        // the specular highlight and the normal matrix spec 0060 rebuilt in
        // the vertex stage
        Shot {
            name: "a-sphere-has-a-highlight",
            scene: |scene, meshes| {
                scene.push_material(
                    meshes.sphere,
                    &Transform::at(vec3(0.0, 1.0, 0.0)).with_scale(Vec3::splat(2.0)),
                    vec4(0.9, 0.7, 0.2, 1.0),
                    96.0,
                );
            },
            camera: |camera| {
                camera.position = vec3(0.0, 2.0, 6.0);
                camera.target = vec3(0.0, 1.0, 0.0);
            },
        },
        // spec 0051's whole reason, which was tested by counting distinct
        // floats and is now a picture of a marble that either has a surface
        // or has seven steps
        Shot {
            name: "a-marble-at-the-far-wall",
            scene: |scene, meshes| {
                scene.push_material(
                    meshes.sphere,
                    &Transform::at(vec3(0.0, 0.0, 33_280.0)).with_scale(Vec3::splat(0.02)),
                    vec4(0.9, 0.7, 0.2, 1.0),
                    96.0,
                );
            },
            camera: |camera| {
                camera.position = vec3(0.0, 0.0, 33_280.0 - 0.08);
                camera.target = vec3(0.0, 0.0, 33_280.0);
                camera.near = 0.01;
                camera.far = 10.0;
            },
        },
        // a point light with a shadow, per spec 0022, which no arithmetic
        // test covers at all
        Shot {
            name: "a-lamp-throws-a-shadow",
            scene: |scene, meshes| {
                scene.light.intensity = 0.0;
                scene.light.ambient = Vec3::splat(0.02);
                scene.push_light(
                    PointLight::new(vec3(0.0, 4.0, 3.0), vec3(1.0, 0.95, 0.9), 3.0, 14.0).casting(),
                );
                scene.push_colored(
                    meshes.floor,
                    &Transform::at(Vec3::ZERO).with_scale(vec3(20.0, 1.0, 20.0)),
                    vec4(0.6, 0.6, 0.62, 1.0),
                );
                scene.push_colored(
                    meshes.cube,
                    &Transform::at(vec3(0.0, 1.0, 0.0)).with_scale(Vec3::splat(2.0)),
                    vec4(0.8, 0.5, 0.3, 1.0),
                );
            },
            camera: |camera| {
                camera.position = vec3(5.0, 4.0, 7.0);
                camera.target = vec3(0.0, 1.0, 0.0);
            },
        },
        // the see-through pass and its ordering, per spec 0018
        Shot {
            name: "glass-over-a-wall",
            scene: |scene, meshes| {
                scene.push_colored(
                    meshes.cube,
                    &Transform::at(vec3(0.0, 1.0, -2.0)).with_scale(vec3(6.0, 3.0, 0.4)),
                    vec4(0.7, 0.35, 0.3, 1.0),
                );
                scene.push_colored(
                    meshes.cube,
                    &Transform::at(vec3(0.0, 1.0, 1.0)).with_scale(vec3(3.0, 2.0, 0.1)),
                    vec4(0.5, 0.8, 0.9, 0.45),
                );
            },
            camera: |camera| {
                camera.position = vec3(0.0, 2.0, 7.0);
                camera.target = vec3(0.0, 1.0, 0.0);
            },
        },
    ]
}

/// Draws one shot, or nothing at all when this machine has no adapter.
fn drawn(shot: &Shot) -> Option<Picture> {
    let mut renderer = pollster::block_on(Renderer::offscreen(WIDE, TALL))?;
    let meshes = Meshes {
        cube: renderer.add_mesh(&MeshData::cube()),
        sphere: renderer.add_mesh(&MeshData::sphere(32, 24)),
        floor: renderer.add_mesh(&MeshData::plane()),
    };
    let mut scene = Scene::new();
    let geometry = Geometry::new();
    let text = TextRenderer::new();

    (shot.scene)(&mut scene, &meshes);

    let mut camera = Camera::new();

    (shot.camera)(&mut camera);
    renderer.set_camera(camera);

    Some(renderer.picture(&scene, &geometry, &text))
}

/// Where the references live, and where a failure writes what it saw.
fn beside(name: &str, what: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/pictures")
        .join(format!("{}{}.png", name, what))
}

fn write(path: &std::path::Path, picture: &Picture) {
    image::save_buffer(
        path,
        &picture.pixels,
        picture.width,
        picture.height,
        image::ColorType::Rgba8,
    )
    .expect("a picture that can be written");
}

/// How many pixels differ by more than a channel's worth, and a picture of
/// where.
fn apart(was: &image::RgbaImage, now: &Picture) -> (usize, image::RgbaImage) {
    let mut count = 0;
    let mut diff = image::RgbaImage::new(now.width, now.height);

    for y in 0..now.height {
        for x in 0..now.width {
            let at = ((y * now.width + x) * 4) as usize;
            let mine = &now.pixels[at..at + 4];
            let theirs = was.get_pixel(x, y).0;
            let off = (0..3)
                .map(|n| (mine[n] as i32 - theirs[n] as i32).abs())
                .max()
                .unwrap_or(0);

            if off > CHANNEL {
                count += 1;
                diff.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            } else {
                diff.put_pixel(
                    x,
                    y,
                    image::Rgba([mine[0] / 3, mine[1] / 3, mine[2] / 3, 255]),
                );
            }
        }
    }

    (count, diff)
}

/// Draws a shot and holds it against its reference.
fn looks_right(name: &str) {
    let shots = shots();
    let shot = shots
        .iter()
        .find(|shot| shot.name == name)
        .expect("a shot by that name");
    let Some(picture) = drawn(shot) else {
        eprintln!("no adapter here, so {} was not drawn", name);
        return;
    };

    assert!(
        !picture.pixels.is_empty(),
        "{} came back with no pixels at all",
        name
    );

    let reference = beside(name, "");

    if std::env::var("BLESS").is_ok() || !reference.exists() {
        write(&reference, &picture);
        eprintln!("blessed {}", reference.display());
        return;
    }

    let was = image::open(&reference)
        .expect("a reference that can be read")
        .to_rgba8();

    assert_eq!(
        (was.width(), was.height()),
        (picture.width, picture.height),
        "{} is a different size from its reference",
        name
    );

    let (count, diff) = apart(&was, &picture);
    let share = count as f32 / (picture.width * picture.height) as f32;

    if share > SHARE {
        let drawn_at = beside(name, "-drawn");
        let diff_at = beside(name, "-diff");

        write(&drawn_at, &picture);
        diff.save(&diff_at).expect("a diff that can be written");

        panic!(
            "{} differs from its reference in {:.1}% of its pixels. What was drawn is at {} and \
             the difference is at {}. If the change was meant, BLESS=1 cargo test --test pictures \
             and look at what it writes.",
            name,
            share * 100.0,
            drawn_at.display(),
            diff_at.display()
        );
    }
}

/// Draws a scene of its own with the camera of a named shot, and says what
/// share of the pixels differ from that shot's reference. `None` when there is
/// no adapter.
///
/// This is what the controls below are made of: a picture test is only worth
/// having if a known fault moves the number past the tolerance.
fn apart_from_the_shot(name: &str, scene: fn(&mut Scene, &Meshes)) -> Option<f32> {
    let shots = shots();
    let shot = shots
        .iter()
        .find(|shot| shot.name == name)
        .expect("a shot by that name");
    let changed = drawn(&Shot {
        name: shot.name,
        scene,
        camera: shot.camera,
    })?;
    let was = image::open(beside(name, ""))
        .expect("a reference that can be read")
        .to_rgba8();
    let (count, _) = apart(&was, &changed);

    Some(count as f32 / (changed.width * changed.height) as f32)
}

/// The tests, in a module of their own, so a spec can cite
/// `pictures::tests::<name>` and `check-specs` can find it: that check counts
/// a cited name as a test only when a segment of it ends in "tests".
mod pictures {
    pub mod tests {
        use super::super::*;

        #[test]
        fn a_lit_box_casts_its_shadow() {
            looks_right("a-lit-box-casts-its-shadow");
        }

        #[test]
        fn a_sphere_has_a_highlight() {
            looks_right("a-sphere-has-a-highlight");
        }

        #[test]
        fn a_marble_at_the_far_wall_has_a_surface() {
            looks_right("a-marble-at-the-far-wall");
        }

        #[test]
        fn a_lamp_throws_a_shadow() {
            looks_right("a-lamp-throws-a-shadow");
        }

        #[test]
        fn glass_is_drawn_over_what_is_behind_it() {
            looks_right("glass-over-a-wall");
        }

        /// Spec 0062: the comparison can fail.
        ///
        /// The lit box with the sun turned off, held against the picture of
        /// it with the sun on. A picture test that cannot fail is a file
        /// nobody reads.
        #[test]
        fn a_picture_test_can_fail() {
            let Some(share) = apart_from_the_shot("a-lit-box-casts-its-shadow", |scene, meshes| {
                let shots = shots();
                let lit = shots
                    .iter()
                    .find(|shot| shot.name == "a-lit-box-casts-its-shadow")
                    .expect("the lit shot");

                (lit.scene)(scene, meshes);
                scene.light.intensity = 0.0;
            }) else {
                return;
            };

            assert!(
                share > SHARE,
                "a scene with the sun off looks the same as one with it on, so this test checks \
                 nothing"
            );
        }

        /// Spec 0062: and it can fail on something the size of a highlight.
        ///
        /// The sun going out changes most of the frame, so a test that only
        /// watches for that would pass with any tolerance at all, which is
        /// what the first draft of this file did: the box lost its shadow and
        /// the comparison called it a match, because the shadow was 0.96% of
        /// the picture and the tolerance was 1%. This one takes the gold ball
        /// and dulls it, which moves a highlight and nothing else, and it is
        /// the smallest change any of these scenes is meant to catch.
        #[test]
        fn the_tolerance_can_see_a_highlight() {
            let Some(share) = apart_from_the_shot("a-sphere-has-a-highlight", |scene, meshes| {
                scene.push_material(
                    meshes.sphere,
                    &Transform::at(vec3(0.0, 1.0, 0.0)).with_scale(Vec3::splat(2.0)),
                    vec4(0.9, 0.7, 0.2, 1.0),
                    4.0,
                );
            }) else {
                return;
            };

            assert!(
                share > SHARE,
                "a ball with a different highlight counts as a match, so the tolerance is looser \
                 than the thing spec 0059 broke"
            );
        }

        /// Spec 0062: drawing the same thing twice gives the same pixels,
        /// which is what makes a reference worth keeping at all.
        #[test]
        fn blessing_twice_changes_nothing() {
            let shots = shots();
            let shot = shots
                .iter()
                .find(|shot| shot.name == "a-sphere-has-a-highlight")
                .expect("the sphere");
            let (Some(once), Some(twice)) = (drawn(shot), drawn(shot)) else {
                eprintln!("no adapter here, so nothing was drawn");
                return;
            };

            assert_eq!(
                once.pixels, twice.pixels,
                "the same scene drawn twice came out differently"
            );
        }

        /// Spec 0062: a machine with no adapter skips rather than fails.
        #[test]
        fn no_device_is_not_a_failure() {
            match pollster::block_on(Renderer::offscreen(16, 16)) {
                Some(_) => (),
                None => eprintln!("no adapter here, which is a skip and not a failure"),
            }
        }
    }
}
