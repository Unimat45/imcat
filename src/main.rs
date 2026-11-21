//! imcat: preview any size image in a terminal window.
//!
//! Originally written in C by Abraham Stolk. This software is in the Public Domain.

mod colour;
mod render;
mod resample;
mod term;

use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use image::{DynamicImage, ImageDecoder, ImageReader, RgbaImage};

use colour::Rgb;

// The help text's description comes from `description` in Cargo.toml.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Images to display.
    #[arg(required = true, value_name = "IMAGE")]
    files: Vec<PathBuf>,

    /// Output width in columns [default: terminal width].
    #[arg(
        short,
        long,
        env = "IMCAT_WIDTH",
        value_name = "COLUMNS",
        value_parser = clap::value_parser!(u32).range(1..)
    )]
    width: Option<u32>,

    /// Terminal background colour; transparent areas are blended into it
    /// [default: black].
    #[arg(short, long, env = "IMCATBG", value_name = "#RRGGBB", value_parser = colour::parse_hex)]
    bg: Option<Rgb>,
}

fn main() -> ExitCode {
    let args = Args::parse();

    term::enable_ansi_escapes();
    let max_width = args.width.unwrap_or_else(term::width);
    let background = colour::to_linear(args.bg.unwrap_or([0, 0, 0]));

    let mut out = BufWriter::new(io::stdout().lock());
    let mut all_loaded = true;

    for path in &args.files {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_definition_is_valid() {
        use clap::CommandFactory;
        Args::command().debug_assert();
    }
}
