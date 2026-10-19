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

use crate::renderer::Picture;

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
