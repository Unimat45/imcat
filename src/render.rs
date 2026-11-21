//! Writing pixels to the terminal as half-block characters.
//!
//! Each text cell shows two vertically stacked pixels: the upper one as the
//! foreground colour of `▀` (upper half block), the lower one as the background
//! colour, using 24-bit ANSI colour escapes. A colour escape is only written
//! when the colour differs from the previous cell on the same line.

use std::io::{self, Write};

use crate::colour::Rgb;

const RESET: &str = "\x1b[0m";
const HALF_BLOCK: &str = "▀";

/// Writes `pixels` (row-major, `width` per row) as lines of half blocks.
pub fn render(out: &mut impl Write, pixels: &[Rgb], width: usize) -> io::Result<()> {
    if width == 0 {
        return Ok(());
    }
    for rows in pixels.chunks(width * 2) {
        let (top, bottom) = rows.split_at(width.min(rows.len()));
        // Colours currently set on this line; `None` means not set yet.
        let mut fg: Option<Rgb> = None;
        // `Some(None)` is the terminal's default background, used for the
        // last line of an image with an odd number of rows.
        let mut bg: Option<Option<Rgb>> = None;

        for (x, &upper) in top.iter().enumerate() {
            if fg != Some(upper) {
                let [r, g, b] = upper;
                write!(out, "\x1b[38;2;{r};{g};{b}m")?;
                fg = Some(upper);
            }
            let lower = bottom.get(x).copied();
            if bg != Some(lower) {
                match lower {
                    Some([r, g, b]) => write!(out, "\x1b[48;2;{r};{g};{b}m")?,
                    None => out.write_all(b"\x1b[49m")?,
                }
                bg = Some(lower);
            }
            out.write_all(HALF_BLOCK.as_bytes())?;
        }
        writeln!(out, "{RESET}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_to_string(pixels: &[Rgb], width: usize) -> String {
        let mut buf = Vec::new();
        render(&mut buf, pixels, width).unwrap();
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn two_pixel_rows_per_line() {
        let text = render_to_string(&[[1, 2, 3]; 12], 3);
        assert_eq!(text.lines().count(), 2);
        assert!(text.lines().all(|l| l.matches(HALF_BLOCK).count() == 3));
    }

    #[test]
    fn writes_each_colour_once_per_run() {
        let red = [255, 0, 0];
        let blue = [0, 0, 255];
        let pixels = [red, red, red, blue, blue, blue, blue, red];
        assert_eq!(
            render_to_string(&pixels, 4),
            "\x1b[38;2;255;0;0m\x1b[48;2;0;0;255m▀▀▀\x1b[38;2;0;0;255m\x1b[48;2;255;0;0m▀\x1b[0m\n"
        );
    }

    #[test]
    fn colours_are_restated_on_every_line() {
        let text = render_to_string(&[[9, 9, 9]; 8], 2);
        for line in text.lines() {
            assert!(
                line.starts_with("\x1b[38;2;9;9;9m\x1b[48;2;9;9;9m▀▀"),
                "{line:?}"
            );
        }
    }

    #[test]
    fn odd_height_keeps_last_row_on_default_background() {
        let text = render_to_string(&[[1, 2, 3]; 6], 2);
        assert_eq!(text.lines().count(), 2);
        assert_eq!(
            text.lines().last().unwrap(),
            "\x1b[38;2;1;2;3m\x1b[49m▀▀\x1b[0m"
        );
    }
}
