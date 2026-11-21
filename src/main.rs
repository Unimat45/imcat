//! imcat: preview any size image in a terminal window.
//!
//! Originally written in C by Abraham Stolk. This software is in the Public Domain.
//!
//! Each text cell shows two vertically stacked pixels: the upper one as the
//! foreground colour of a `▀` (upper half block) and the lower one as the
//! background colour, using 24-bit ANSI colour escapes.

use std::env;
use std::ffi::OsString;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;

use image::{ImageReader, RgbaImage};

const RESET: &str = "\x1b[0m";
const DEFAULT_BG: &str = "\x1b[49m";
const HALF_BLOCK: char = '▀';
const DEFAULT_WIDTH: u32 = 80;

type Rgb = [u8; 3];
type Rgba = [u8; 4];

/// A downsampled image whose colour channels are premultiplied by alpha.
struct Frame {
    width: usize,
    pixels: Vec<Rgba>,
}

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
        Ok(s) => {
            let bg = parse_background(&s);
            if bg.is_none() {
                eprintln!("imcat: ignoring invalid IMCATBG value {s:?} (expected #rrggbb)");
            }
            bg
        }
        Err(_) => None,
    };

    enable_ansi_escapes();
    let width = terminal_width();

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

        let frame = downsample(&img, width);
        // Flush after every image so error messages interleave correctly with output.
        if let Err(e) = render(&mut out, &frame, background).and_then(|()| out.flush()) {
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

/// Decodes an image, detecting its format from the file contents (like stb_image)
/// rather than from the extension.
fn load_image(path: &Path) -> image::ImageResult<RgbaImage> {
    Ok(ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?
        .to_rgba8())
}

/// Parses `#rrggbb` (the `#` is optional).
fn parse_background(s: &str) -> Option<Rgb> {
    let hex = s.trim().strip_prefix('#').unwrap_or(s.trim());
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(hex, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

/// Output width in columns: `IMCAT_WIDTH` if set, otherwise the size of whichever
/// standard stream is attached to a terminal (so `imcat x.png | less -R` still
/// sizes to the terminal), otherwise 80.
fn terminal_width() -> u32 {
    if let Some(w) = env::var("IMCAT_WIDTH")
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .filter(|&w| w > 0)
    {
        return w;
    }

    use terminal_size::{Width, terminal_size_of};
    [
        terminal_size_of(io::stdout()),
        terminal_size_of(io::stderr()),
        terminal_size_of(io::stdin()),
    ]
    .into_iter()
    .flatten()
    .map(|(Width(w), _)| u32::from(w))
    .find(|&w| w > 0)
    .unwrap_or(DEFAULT_WIDTH)
}

/// Shrinks the image to at most `term_width` columns using an odd-sized box
/// filter, premultiplying colour by alpha. Images narrower than the terminal
/// are kept at their native size.
fn downsample(img: &RgbaImage, term_width: u32) -> Frame {
    let (imw, imh) = (img.width() as usize, img.height() as usize);
    let src = img.as_raw();

    let pixels_per_char = (imw as f32 / term_width as f32).max(1.0);
    let mut kernel_size = pixels_per_char.floor() as usize;
    if kernel_size.is_multiple_of(2) {
        kernel_size -= 1;
    }
    let radius = kernel_size.saturating_sub(1) / 2;

    let outw = imw.min(term_width as usize);
    let outh = (outw as f32 * imh as f32 / imw as f32).round() as usize;

    let mut pixels = Vec::with_capacity(outw * outh);
    for y in 0..outh {
        let cy = ((pixels_per_char * y as f32).round() as usize).min(imh - 1);
        let rows = cy.saturating_sub(radius)..=(cy + radius).min(imh - 1);
        for x in 0..outw {
            let cx = ((pixels_per_char * x as f32).round() as usize).min(imw - 1);
            let cols = cx.saturating_sub(radius)..=(cx + radius).min(imw - 1);

            let mut acc = [0u32; 4];
            let mut samples = 0u32;
            for yy in rows.clone() {
                let row = &src[yy * imw * 4..(yy + 1) * imw * 4];
                for xx in cols.clone() {
                    let p = &row[xx * 4..xx * 4 + 4];
                    let a = u32::from(p[3]);
                    acc[0] += a * u32::from(p[0]) / 255;
                    acc[1] += a * u32::from(p[1]) / 255;
                    acc[2] += a * u32::from(p[2]) / 255;
                    acc[3] += a;
                    samples += 1;
                }
            }
            pixels.push(acc.map(|v| (v / samples) as u8));
        }
    }

    Frame {
        width: outw,
        pixels,
    }
}

/// Composites a premultiplied pixel over the background (GL_ONE, GL_ONE_MINUS_SRC_ALPHA).
fn blend([r, g, b, a]: Rgba, bg: Rgb) -> Rgb {
    let inv_alpha = 255 - u32::from(a);
    let mix =
        |c: u8, bgc: u8| ((u32::from(c) * 255 + u32::from(bgc) * inv_alpha) / 255).min(255) as u8;
    [mix(r, bg[0]), mix(g, bg[1]), mix(b, bg[2])]
}

fn render(out: &mut impl Write, frame: &Frame, background: Option<Rgb>) -> io::Result<()> {
    if frame.width == 0 {
        return Ok(());
    }
    let colour = |p: Rgba| match background {
        Some(bg) => blend(p, bg),
        None => [p[0], p[1], p[2]],
    };

    for rows in frame.pixels.chunks(frame.width * 2) {
        let (top, bottom) = rows.split_at(frame.width.min(rows.len()));
        for (x, &upper) in top.iter().enumerate() {
            let [r, g, b] = colour(upper);
            write!(out, "\x1b[38;2;{r};{g};{b}m")?;
            match bottom.get(x) {
                Some(&lower) => {
                    let [r, g, b] = colour(lower);
                    write!(out, "\x1b[48;2;{r};{g};{b}m{HALF_BLOCK}")?;
                }
                // Odd image height: the last line only has an upper pixel.
                None => write!(out, "{DEFAULT_BG}{HALF_BLOCK}")?,
            }
        }
        writeln!(out, "{RESET}")?;
    }
    Ok(())
}

/// Windows consoles need virtual terminal processing turned on to interpret ANSI escapes.
#[cfg(windows)]
fn enable_ansi_escapes() {
    use windows_sys::Win32::System::Console::{
        ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetStdHandle, STD_OUTPUT_HANDLE,
        SetConsoleMode,
    };
    // SAFETY: plain Win32 calls on the process's own stdout handle; failures are harmless.
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode = 0;
        if GetConsoleMode(handle, &mut mode) != 0 {
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    }
}

#[cfg(not(windows))]
fn enable_ansi_escapes() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_background() {
        assert_eq!(parse_background("#dad9cc"), Some([0xda, 0xd9, 0xcc]));
        assert_eq!(parse_background("DAD9CC"), Some([0xda, 0xd9, 0xcc]));
        assert_eq!(parse_background("#fff"), None);
        assert_eq!(parse_background("#+abcde"), None);
        assert_eq!(parse_background("nope"), None);
    }

    #[test]
    fn blend_never_overflows() {
        assert_eq!(blend([255, 255, 255, 0], [255, 255, 255]), [255, 255, 255]);
        assert_eq!(blend([0, 0, 0, 0], [10, 20, 30]), [10, 20, 30]);
        assert_eq!(blend([200, 100, 50, 255], [10, 20, 30]), [200, 100, 50]);
    }

    #[test]
    fn small_images_keep_native_size() {
        let img = RgbaImage::from_pixel(10, 6, image::Rgba([255, 0, 0, 255]));
        let frame = downsample(&img, 80);
        assert_eq!(frame.width, 10);
        assert_eq!(frame.pixels.len(), 60);
        assert!(frame.pixels.iter().all(|&p| p == [255, 0, 0, 255]));
    }

    #[test]
    fn wide_images_shrink_to_terminal() {
        let img = RgbaImage::from_pixel(400, 200, image::Rgba([0, 0, 255, 255]));
        let frame = downsample(&img, 100);
        assert_eq!(frame.width, 100);
        assert_eq!(frame.pixels.len(), 100 * 50);
    }

    #[test]
    fn odd_height_keeps_last_row() {
        let frame = Frame {
            width: 2,
            pixels: vec![[1, 2, 3, 255]; 6],
        };
        let mut buf = Vec::new();
        render(&mut buf, &frame, None).unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(text.lines().last().unwrap().contains(DEFAULT_BG));
    }
}
