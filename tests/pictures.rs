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
use blitzkit::renderer::scene::{Instance, MeshId, Scene, TextureId};
use blitzkit::renderer::{Picture, Renderer};
use glam::{vec3, vec4, Vec3};

/// How big a picture is.
///
/// Small, because every one of these is committed and read by a person: a
/// shadow and a highlight are plain at this size and a diff of it is
/// something you can look at.
const WIDE: u32 = 320;
const TALL: u32 = 240;

/// How different is too different lives in the engine now, per the arcade's
/// spec 0027: a game cannot reach a test, and two tolerances would drift.
use blitzkit::picture::{CHANNEL, SHARE};

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
        // spec 0063: a run the game handed over once, far enough out that
        // an f32 world position would show its steps. What is drawn comes
        // out of the buffer it was written into, relative to the middle of
        // the run, with the eye off that middle rather than off each of
        // them.
        Shot {
            name: "a-kept-run-at-the-far-wall",
            scene: |scene, meshes| {
                let far = 33_280.0;
                let instances: Vec<Instance> = (0..4)
                    .map(|n| {
                        Instance::new(
                            &Transform::at(vec3(
                                (n as f32 - 1.5) * 0.09,
                                0.0,
                                far + (n % 2) as f32 * 0.05,
                            ))
                            .with_scale(Vec3::splat(0.05)),
                            vec4(0.85, 0.6, 0.25, 1.0),
                            64.0,
                        )
                    })
                    .collect();

                scene.keep(meshes.cube, TextureId::WHITE, &instances);
            },
            camera: |camera| {
                camera.position = vec3(0.0, 0.07, 33_280.0 - 0.35);
                camera.target = vec3(0.0, 0.0, 33_280.0);
                camera.near = 0.01;
                camera.far = 10.0;
            },
        },
        // spec 0067: something with enough in its mesh pass that the pass
        // reads heavier than one box does
        Shot {
            name: "a-thousand-boxes",
            scene: |scene, meshes| {
                scene.light.direction = vec3(-0.8, -0.9, 0.5).normalize();
                scene.push_colored(
                    meshes.floor,
                    &Transform::at(Vec3::ZERO).with_scale(vec3(60.0, 1.0, 60.0)),
                    vec4(0.6, 0.6, 0.62, 1.0),
                );

                for n in 0..1000 {
                    let n = n as f32;

                    scene.push_colored(
                        meshes.cube,
                        &Transform::at(vec3((n * 0.7).sin() * 18.0, 0.5, (n * 0.31).cos() * 18.0)),
                        vec4(0.8, 0.5, 0.3, 1.0),
                    );
                }
            },
            camera: |camera| {
                camera.position = vec3(0.0, 9.0, 26.0);
                camera.target = vec3(0.0, 0.0, 0.0);
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

    if let Err(why) = blitzkit::picture::looks_right(&picture, &beside(name, "")) {
        panic!("{}", why);
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

        /// Spec 0063: a kept run is drawn out of the buffer it was written
        /// into, and lands where it was put however far out that is.
        ///
        /// Every other picture here is pushed, so this is the only one that
        /// goes down the path spec 0063 built. Far out, because that is
        /// where storing a run relative to its own middle is the difference
        /// between a cube and a smear.
        #[test]
        fn a_kept_run_is_drawn_where_it_lies() {
            looks_right("a-kept-run-at-the-far-wall");
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

/// Spec 0065: the GPU says how long it took, and the number is believable.
///
/// **The one thing in this engine that cannot be checked without a device.**
/// The pair of timestamps a frame writes was resolved in the same submission
/// that wrote it, so the end tick came back holding the frame before's, the
/// pair went backwards, and a backwards pair is thrown away. The column was
/// blank on every frame fast enough for the CPU to run ahead, which after
/// specs 0063 and 0064 is every frame worth measuring.
///
/// A handful of frames rather than one, because the reading is some frames
/// behind the work by design: the GPU writes the ticks and they cannot be
/// read until the work has run.
mod clock {
    pub mod tests {
        use super::super::*;

        /// Spec 0067: each pass says what it took, and a pass that did not
        /// run says nothing.
        ///
        /// The occlusion pass of spec 0061 is the one that is off unless a
        /// game asks, so it is the one that has to be absent. If it were
        /// in here it would be holding another pass's time, which is the
        /// fault fixed slots exist to make impossible.
        #[test]
        fn every_pass_of_a_frame_is_timed() {
            let Some((passes, _)) = timed("a-lit-box-casts-its-shadow") else {
                return;
            };

            assert!(
                passes.iter().any(|(name, _)| *name == "mesh"),
                "the mesh pass did not say what it took"
            );
            assert!(
                passes.iter().any(|(name, _)| *name == "sun"),
                "the sun's shadow pass did not say what it took"
            );
            assert!(
                !passes.iter().any(|(name, _)| *name == "hidden"),
                "a pass that never ran reported a time"
            );

            // **And every pass that did run is in there.** This is the
            // check that would have caught the diagnostic left in spec
            // 0065, where every pass but the last stopped writing a
            // timestamp and the engine reported the 2D pass as the frame
            // for a day.
            for named in ["sun", "prepass", "mesh", "overlay"] {
                assert!(
                    passes.iter().any(|(name, _)| *name == named),
                    "the {} pass of a frame that drew one was not timed",
                    named
                );
            }

            for (name, seconds) in &passes {
                assert!(
                    *seconds > 0.0 && *seconds < 1.0,
                    "the {} pass says it took {} seconds",
                    name,
                    seconds
                );
            }
        }

        /// Spec 0067: no pass claims more than the frame it is part of.
        ///
        /// What this catches is slots crossed or written over, which is
        /// what forty four queries invite: a pair taken from two different
        /// frames reads longer than the frame did.
        ///
        /// It deliberately does not check that the passes add up to the
        /// frame. They do not: a pass's pair says when it began and ended
        /// on the device, and at four million instances those come to 23 ms
        /// inside a frame of 16. `every_pass_of_a_frame_is_timed` is what
        /// holds the thing that check was for.
        #[test]
        fn no_pass_is_longer_than_the_frame() {
            let Some((passes, span)) = timed("a-lit-box-casts-its-shadow") else {
                return;
            };
            for (name, seconds) in &passes {
                assert!(
                    *seconds <= span,
                    "the {} pass says it took {} of a {} second frame",
                    name,
                    seconds,
                    span
                );
            }
        }

        /// Spec 0067: a pass with more in it reads heavier.
        ///
        /// A clock that reports a plausible number for everything is a
        /// clock nobody can tell from a constant.
        #[test]
        fn a_heavier_pass_reads_heavier() {
            let (Some((few, _)), Some((many, _))) = (
                timed("a-lit-box-casts-its-shadow"),
                timed("a-thousand-boxes"),
            ) else {
                return;
            };
            let mesh = |passes: &[(&'static str, f32)]| {
                passes
                    .iter()
                    .find(|(name, _)| *name == "mesh")
                    .map(|(_, seconds)| *seconds)
                    .expect("a mesh pass")
            };

            assert!(
                mesh(&many) > mesh(&few),
                "a thousand boxes cost {} and one cost {}",
                mesh(&many),
                mesh(&few)
            );
        }

        /// Draws a shot for long enough that the clock reports, and hands
        /// back what each pass took and how long the GPU held the frame.
        fn timed(name: &str) -> Option<(Vec<(&'static str, f32)>, f32)> {
            let shots = shots();
            let shot = shots.iter().find(|shot| shot.name == name)?;
            let mut renderer = pollster::block_on(Renderer::offscreen(WIDE, TALL))?;

            if !renderer.times_frames() {
                eprintln!("this device will not time a frame, which is not a failure");

                return None;
            }

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

            let mut said = None;

            for _ in 0..24 {
                renderer.picture(&scene, &geometry, &text);

                if let Some(span) = renderer.gpu_seconds() {
                    let passes = renderer.gpu_passes();

                    if !passes.is_empty() {
                        said = Some((passes, span));
                    }
                }
            }

            Some(said.expect("the device timed twenty four frames and said nothing"))
        }

        #[test]
        fn the_gpu_says_how_long_it_took() {
            let shots = shots();
            let shot = shots
                .iter()
                .find(|shot| shot.name == "a-lit-box-casts-its-shadow")
                .expect("the lit box");
            let Some(mut renderer) = pollster::block_on(Renderer::offscreen(WIDE, TALL)) else {
                eprintln!("no adapter here, so nothing was timed");
                return;
            };

            if !renderer.times_frames() {
                eprintln!("this device will not time a frame, which is not a failure");
                return;
            }

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

            let mut said: Option<f32> = None;

            for _ in 0..24 {
                renderer.picture(&scene, &geometry, &text);

                if let Some(seconds) = renderer.gpu_seconds() {
                    said = Some(seconds);
                }
            }

            let seconds = said.expect("the device timed twenty four frames and said nothing");

            assert!(
                seconds > 0.0 && seconds < 1.0,
                "the GPU says a frame of a box on a floor took {} seconds",
                seconds
            );
        }
    }
}
