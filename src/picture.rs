//! Holding a drawn frame against one that was right. Spec 0062.
//!
//! **Here rather than in a test, because a game cannot reach a test.** Spec
//! 0062 built this inside `tests/pictures.rs`, where it checked the engine's
//! own five scenes and nothing else could use it. The arcade wants the same
//! thing for its seven rooms, per that game's spec 0027, and the choice was
//! sixty lines copied into another repo or one tolerance with one account of
//! itself. Two places that say how different is too different will come
//! apart, which is a thing the arcade has written down four times under
//! other names.

use crate::camera::Camera;
use crate::geometry::Geometry;
use crate::renderer::render_text::TextRenderer;
use crate::renderer::scene::Scene;
use crate::renderer::{Picture, Renderer};

use std::path::{Path, PathBuf};

/// How far a channel may differ before a pixel counts as changed, and how
/// many pixels may change before a picture does.
///
/// Two GPUs rarely agree to the bit, and a driver that rounds a shadow's
/// edge differently should not fail a test about whether there is a shadow.
///
/// **The share started at a hundredth and was too loose to do its job.**
/// Turning the sun's shadow off changed 0.96% of a frame, so a box stood on
/// no shadow at all and the comparison called it a match. A hundredth of a
/// 320 by 240 picture is 768 pixels, which is a shadow.
pub const CHANNEL: i32 = 8;
pub const SHARE: f32 = 0.002;

/// Holds a drawn frame against the file at `reference`.
///
/// `Ok` when they match, when `BLESS` is set, or when there is no reference
/// yet: the last two write the file. Otherwise `Err` with a sentence saying
/// how far apart they were and where to look, having written what was drawn
/// and a picture of the difference beside the reference. **The only useful
/// thing to do with a failed picture test is look at it.**
///
/// A blessed picture is looked at before it is committed. That is a rule
/// this code cannot enforce and the reason these files are small.
pub fn looks_right(drawn: &Picture, reference: &Path) -> Result<(), String> {
    if drawn.pixels.is_empty() {
        return Err(format!(
            "{} came back with no pixels at all",
            reference.display()
        ));
    }

    if std::env::var("BLESS").is_ok() || !reference.exists() {
        write(reference, drawn)?;
        eprintln!("blessed {}", reference.display());

        return Ok(());
    }

    let was = image::open(reference)
        .map_err(|why| format!("{} could not be read: {}", reference.display(), why))?
        .to_rgba8();

    if (was.width(), was.height()) != (drawn.width, drawn.height) {
        return Err(format!(
            "{} is {} by {} and what was drawn is {} by {}",
            reference.display(),
            was.width(),
            was.height(),
            drawn.width,
            drawn.height
        ));
    }

    let (count, diff) = apart(&was, drawn);
    let share = count as f32 / (drawn.width * drawn.height) as f32;

    if share <= SHARE {
        return Ok(());
    }

    let drawn_at = beside(reference, "-drawn");
    let diff_at = beside(reference, "-diff");

    write(&drawn_at, drawn)?;
    diff.save(&diff_at)
        .map_err(|why| format!("{} could not be written: {}", diff_at.display(), why))?;

    Err(format!(
        "{} differs from its reference in {:.1}% of its pixels. What was drawn is at {} and the \
         difference is at {}. If the change was meant, BLESS=1 and look at what it writes.",
        reference.display(),
        share * 100.0,
        drawn_at.display(),
        diff_at.display()
    ))
}

/// What share of two drawn frames differ by more than a channel's worth.
///
/// How big a picture is kept, unless a caller has a reason of its own.
///
/// **Small, because every reference is committed and read by a person.** A
/// shadow and a highlight are plain at this size and a diff of it is
/// something you can look at, which is the only useful thing to do with a
/// failed picture test.
///
/// **And bigger is not sharper for this job.** Written down because the
/// opposite was assumed and measured wrong on 2026-11-02: two games were
/// given twice this on the argument that what tells their pieces apart,
/// the pips on a domino and the number on a ball, is about a pixel here
/// and would be lost. Both were then measured by taking the faces off
/// and asking how far the picture moved. Cascada: 7.7% at this size
/// against 7.3% at twice it. Poolhall: 0.3% against 0.2%. The share is
/// pixels changed over pixels in all, so the thing being watched for and
/// the frame it sits in grow together, and the bigger picture detects no
/// better while costing four times the bytes in somebody's repository.
///
/// A caller that wants another size still passes one. What this is, is
/// the answer when there is no reason.
pub const SIZE: (u32, u32) = (320, 240);

/// Draws one frame with no window and holds it against the file. Spec 0076.
///
/// `build` is handed the renderer, so that it can add its meshes and its
/// textures, and gives back what to draw and where from. The frame is the
/// 3D and nothing else: the `Geometry` and the `TextRenderer` are made here
/// and left empty, because a reference holding the 2D overlay changes every
/// time a line of help does, and at the size these are kept the help covers
/// the thing being tested. [`crate::renderer::Renderer::picture`] is still
/// there for a caller that wants the overlay in shot.
///
/// Panics with what [`looks_right`] said when the two differ, which is what
/// makes this one line at a call site. Says whether a frame was drawn at
/// all: `false` means this machine has no adapter and nothing was checked.
///
/// **The skip is the reason this is worth having.** A machine with no
/// adapter passes every picture test in the project by running none of
/// them, and that silence reads exactly like success. Written out in each
/// caller it cannot be changed anywhere; written here it can. So this says
/// on stderr which reference went unchecked, and hands back the fact so a
/// suite that wants to count can.
///
/// `PICTURES_REQUIRED` turns that skip into a panic, for a suite that would
/// rather not count. Spec 0077.
pub fn drawn_looks_right(
    reference: &Path,
    (wide, tall): (u32, u32),
    build: impl FnOnce(&mut Renderer) -> (Scene, Camera),
) -> bool {
    let Some(mut renderer) = pollster::block_on(Renderer::offscreen(wide, tall)) else {
        return no_adapter(reference, std::env::var("PICTURES_REQUIRED").is_ok());
    };

    let (scene, camera) = build(&mut renderer);

    renderer.set_camera(camera);

    let drawn = renderer.picture(&scene, &Geometry::new(), &TextRenderer::new());

    if let Err(why) = looks_right(&drawn, reference) {
        panic!("{}", why);
    }

    true
}

/// For a control: a test that breaks something on purpose and wants the
/// number rather than a pass or a fail. **It touches no file**, which is
/// the point. The arcade's control for spec 0027 held a darkened room
/// against the reference on disc, and on a run where that file did not
/// exist yet it blessed the dark room as the reference instead. A control
/// that can write what it is checking against is a control that can poison
/// it.
pub fn how_far_apart(one: &Picture, two: &Picture) -> f32 {
    if one.width != two.width || one.height != two.height || one.pixels.is_empty() {
        return 1.0;
    }

    let mut count = 0;

    for at in (0..one.pixels.len()).step_by(4) {
        let off = (0..3)
            .map(|n| (one.pixels[at + n] as i32 - two.pixels[at + n] as i32).abs())
            .max()
            .unwrap_or(0);

        if off > CHANNEL {
            count += 1;
        }
    }

    count as f32 / (one.width * one.height) as f32
}

/// How many pixels differ by more than a channel's worth, and a picture of
/// where: the difference in red over a dimmed copy of what was drawn, so
/// that what changed can be seen against what did not.
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

/// What a machine with no adapter gets. Taken out of `drawn_looks_right` so
/// it can be tested on a machine that has one, which every machine that runs
/// this project's gate does.
///
/// **`required` is the caller's, not the engine's.** A missing device
/// disables a feature here, so the engine's own answer is to skip and say
/// so. A suite is a different question: one that drew nothing at all drew no
/// conclusion either, and the only thing that knows whether that matters is
/// whatever ran it. `./check-all` sets it, because this machine has an
/// adapter and a run of it that found none has gone wrong somewhere else.
fn no_adapter(reference: &Path, required: bool) -> bool {
    if required {
        panic!(
            "no adapter here, so {} was not checked, and PICTURES_REQUIRED \
             says a run that checked nothing is a failed run",
            reference.display()
        );
    }

    eprintln!(
        "no adapter here, so {} was not checked",
        reference.display()
    );

    false
}

/// The same path with something added before the extension.
fn beside(reference: &Path, what: &str) -> PathBuf {
    let stem = reference
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();

    reference.with_file_name(format!("{}{}.png", stem, what))
}

fn write(path: &Path, picture: &Picture) -> Result<(), String> {
    image::save_buffer(
        path,
        &picture.pixels,
        picture.width,
        picture.height,
        image::ColorType::Rgba8,
    )
    .map_err(|why| format!("{} could not be written: {}", path.display(), why))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Both of these are the no-adapter arm, which is the one arm in this file
    // that a machine with an adapter can never reach. Spec 0076 left it to a
    // reading of the source and spec 0077 gave it an argument instead, so it
    // can be reached from here.

    #[test]
    fn a_skip_is_not_a_failure() {
        assert!(!no_adapter(Path::new("nowhere/at-all.png"), false));
    }

    #[test]
    #[should_panic(expected = "a run that checked nothing is a failed run")]
    fn a_skip_is_a_failure_when_pictures_are_required() {
        no_adapter(Path::new("nowhere/at-all.png"), true);
    }
}
