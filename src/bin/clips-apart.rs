//! Says whether two clips of the same thing are the same clip. Spec 0084.
//!
//! Reads two streams of raw rgba frames, which is what `ffmpeg` hands back
//! when it decodes an mp4, and answers with `blitzkit::picture`'s own
//! tolerance rather than a second account of it. Exits 0 when a refilmed
//! clip can be thrown away and 1 when it cannot, so a shell can use it as a
//! condition.
//!
//! **Both sides have to have been through the encoder.** A freshly drawn
//! frame against a decoded one differs everywhere from the compression
//! alone, so `refresh-examples` encodes first and decodes both.

use std::fs::File;
use std::process::ExitCode;

fn main() -> ExitCode {
    let said: Vec<String> = std::env::args().skip(1).collect();
    let [one, two, wide, tall] = said.as_slice() else {
        eprintln!("clips-apart <one.rgba> <two.rgba> <width> <height>");

        return ExitCode::from(2);
    };
    let (Ok(wide), Ok(tall)) = (wide.parse::<u32>(), tall.parse::<u32>()) else {
        eprintln!("the width and the height are numbers");

        return ExitCode::from(2);
    };

    let (Ok(mut one), Ok(mut two)) = (File::open(one), File::open(two)) else {
        eprintln!("both of those have to be readable");

        return ExitCode::from(2);
    };

    match blitzkit::picture::clips_apart(&mut one, &mut two, wide, tall) {
        Ok(apart) => {
            println!(
                "{} frames, {} differing, worst {:.5}",
                apart.frames, apart.differing, apart.worst
            );

            if apart.the_same() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(why) => {
            eprintln!("{}", why);

            ExitCode::from(2)
        }
    }
}
