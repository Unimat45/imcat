//! imcat: preview any size image in a terminal window.
//!
//! Originally written in C by Abraham Stolk. This software is in the Public Domain.

mod colour;
mod render;
mod resample;
mod term;

use std::env;
use std::ffi::OsString;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;

use image::{DynamicImage, ImageDecoder, ImageReader, RgbaImage};

use colour::Rgb;

fn main() -> ExitCode {
    let files: Vec<OsString> = env::args_os().skip(1).collect();
    if files.is_empty() || files[0] == "--help" || files[0] == "-h" {
        eprintln!("Usage: imcat image [image2 .. imageN]");
        eprintln!();
        eprintln!("Environment:");
        eprintln!("  IMCATBG=#rrggbb   terminal background colour, enables alpha blending");
        eprintln!("  IMCAT_WIDTH=n     output width in columns (default: terminal width)");
        return ExitCode::SUCCESS;
    }

    let background = match env::var("IMCATBG") {
        Ok(s) => colour::parse_hex(&s).unwrap_or_else(|_| {
            eprintln!("imcat: ignoring invalid IMCATBG value {s:?} (expected #rrggbb)");
            [0, 0, 0]
        }),
        Err(_) => [0, 0, 0],
    };
    let background = colour::to_linear(background);

    term::enable_ansi_escapes();
    let max_width = env::var("IMCAT_WIDTH")
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .filter(|&w| w > 0)
        .unwrap_or_else(term::width);

    let mut out = BufWriter::new(io::stdout().lock());
    let mut all_loaded = true;

    for file in &files {
        let path = Path::new(file);
        let img = match load_image(path) {
            Ok(img) => img,
            Err(e) => {
                eprintln!("imcat: could not load image {}: {e}", path.display());
                all_loaded = false;
                continue;
            }
        };

        let (width, height) = resample::output_size(img.width(), img.height(), max_width);
        let pixels: Vec<Rgb> = resample::resample(&img, width, height)
            .into_iter()
            .map(|p| colour::composite(p, background))
            .collect();

        // Flush after every image so error messages interleave correctly with output.
        if let Err(e) = render::render(&mut out, &pixels, width).and_then(|()| out.flush()) {
            if e.kind() == io::ErrorKind::BrokenPipe {
                // e.g. `imcat big.png | head`: the reader went away, not an error.
                return ExitCode::SUCCESS;
            }
            eprintln!("imcat: write error: {e}");
            return ExitCode::FAILURE;
        }
    }

    if all_loaded {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Decodes an image, detecting its format from the file contents rather than
/// the extension, and rotates or flips it as its EXIF orientation says.
fn load_image(path: &Path) -> image::ImageResult<RgbaImage> {
    let mut decoder = ImageReader::open(path)?
        .with_guessed_format()?
        .into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut img = DynamicImage::from_decoder(decoder)?;
    img.apply_orientation(orientation);
    Ok(img.into_rgba8())
}
