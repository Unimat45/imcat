//! Colour types and sRGB <-> linear-light conversion.

/// An 8-bit sRGB colour.
pub type Rgb = [u8; 3];

/// A linear-light colour with alpha, premultiplied (each colour channel is
/// already multiplied by alpha). Channels are in `0.0..=1.0`.
pub type LinearRgba = [f32; 4];

/// Converts an 8-bit sRGB channel to linear light.
pub fn srgb_to_linear(v: u8) -> f32 {
    let c = f32::from(v) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Converts a linear-light channel back to 8-bit sRGB, rounding to nearest.
pub fn linear_to_srgb(l: f32) -> u8 {
    let l = l.clamp(0.0, 1.0);
    let c = if l <= 0.003_130_8 {
        l * 12.92
    } else {
        1.055 * l.powf(1.0 / 2.4) - 0.055
    };
    (c * 255.0).round() as u8
}

/// Composites a premultiplied pixel over an opaque background
/// (`GL_ONE, GL_ONE_MINUS_SRC_ALPHA`) and converts the result to sRGB.
pub fn composite(pixel: LinearRgba, background: [f32; 3]) -> Rgb {
    let inv_alpha = 1.0 - pixel[3];
    std::array::from_fn(|c| linear_to_srgb(pixel[c] + background[c] * inv_alpha))
}

/// Converts an sRGB background colour to linear light, for [`composite`].
pub fn to_linear(rgb: Rgb) -> [f32; 3] {
    rgb.map(srgb_to_linear)
}

/// Parses `#rrggbb` (the `#` is optional). Used as a clap value parser.
pub fn parse_hex(s: &str) -> Result<Rgb, String> {
    let s = s.trim();
    let hex = s.strip_prefix('#').unwrap_or(s);
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("expected a colour like #rrggbb".into());
    }
    let v = u32::from_str_radix(hex, 16).map_err(|e| e.to_string())?;
    Ok([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srgb_round_trips_exactly() {
        for v in 0..=255u8 {
            assert_eq!(linear_to_srgb(srgb_to_linear(v)), v, "value {v}");
        }
    }

    #[test]
    fn linear_midpoint_is_brighter_than_srgb_midpoint() {
        // Half black, half white averages to 50% light, which is sRGB 188, not 128.
        assert_eq!(linear_to_srgb(0.5), 188);
    }

    #[test]
    fn composites_over_background() {
        let bg = to_linear([10, 20, 30]);
        assert_eq!(composite([0.0, 0.0, 0.0, 0.0], bg), [10, 20, 30]);
        let opaque = [
            srgb_to_linear(200),
            srgb_to_linear(100),
            srgb_to_linear(50),
            1.0,
        ];
        assert_eq!(composite(opaque, bg), [200, 100, 50]);
        // Out-of-range input (e.g. float error) is clamped rather than wrapping.
        assert_eq!(composite([1.2, 1.2, 1.2, 1.0], bg), [255, 255, 255]);
    }

    #[test]
    fn parses_hex_colours() {
        assert_eq!(parse_hex("#dad9cc"), Ok([0xda, 0xd9, 0xcc]));
        assert_eq!(parse_hex(" DAD9CC "), Ok([0xda, 0xd9, 0xcc]));
        assert!(parse_hex("#fff").is_err());
        assert!(parse_hex("#+abcde").is_err());
        assert!(parse_hex("blue").is_err());
    }
}
