//! Writing pixels to the terminal as half-block characters.
//!
//! Each text cell shows two vertically stacked pixels: the upper one as the
//! foreground colour of `▀` (upper half block), the lower one as the background
//! colour, using 24-bit ANSI colour escapes.

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
        for (x, &[r, g, b]) in top.iter().enumerate() {
            write!(out, "\x1b[38;2;{r};{g};{b}m")?;
            match bottom.get(x) {
                Some(&[r, g, b]) => write!(out, "\x1b[48;2;{r};{g};{b}m")?,
                // Odd image height: the last line only has an upper pixel.
                None => out.write_all(b"\x1b[49m")?,
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
    fn odd_height_keeps_last_row_on_default_background() {
        let text = render_to_string(&[[1, 2, 3]; 6], 2);
        assert_eq!(text.lines().count(), 2);
        assert!(text.lines().last().unwrap().contains("\x1b[49m"));
    }
}
