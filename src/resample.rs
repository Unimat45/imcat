//! Downscaling images to terminal size.
//!
//! Every output pixel is the area-weighted average of exactly the source pixels
//! it covers, including fractional coverage at its edges. Averaging is done in
//! linear light with premultiplied alpha, so fine detail doesn't darken and
//! fully transparent pixels contribute no colour.

use image::RgbaImage;

use crate::colour::{LinearRgba, srgb_to_linear};

/// Output size in pixels for an image shown at most `max_width` pixels wide.
/// Images are never upscaled. Pixels are square because each terminal cell
/// (roughly twice as tall as it is wide) shows two of them stacked.
pub fn output_size(width: u32, height: u32, max_width: u32) -> (usize, usize) {
    let out_w = width.min(max_width).max(1) as usize;
    let out_h = (out_w as f64 * f64::from(height) / f64::from(width)).round();
    (out_w, (out_h as usize).max(1))
}

/// Resamples `img` to `out_w` x `out_h` pixels.
pub fn resample(img: &RgbaImage, out_w: usize, out_h: usize) -> Vec<LinearRgba> {
    let (in_w, in_h) = (img.width() as usize, img.height() as usize);
    let to_linear: [f32; 256] = std::array::from_fn(|v| srgb_to_linear(v as u8));
    let x_spans = spans(in_w, out_w);
    let y_spans = spans(in_h, out_h);

    // Horizontal pass: shrink every source row to `out_w` premultiplied linear pixels.
    let mut rows = Vec::with_capacity(in_h * out_w);
    for src_row in img.as_raw().chunks_exact(in_w * 4) {
        for span in &x_spans {
            let mut acc = [0.0f32; 4];
            for (i, &weight) in span.weights.iter().enumerate() {
                let p = &src_row[(span.start + i) * 4..][..4];
                let alpha = f32::from(p[3]) / 255.0 * weight;
                for c in 0..3 {
                    acc[c] += to_linear[usize::from(p[c])] * alpha;
                }
                acc[3] += alpha;
            }
            rows.push(acc);
        }
    }

    // Vertical pass: combine those rows into `out_h` output rows.
    let mut out = Vec::with_capacity(out_w * out_h);
    for span in &y_spans {
        let mut acc = vec![[0.0f32; 4]; out_w];
        for (i, &weight) in span.weights.iter().enumerate() {
            let row = &rows[(span.start + i) * out_w..][..out_w];
            for (a, p) in acc.iter_mut().zip(row) {
                for c in 0..4 {
                    a[c] += p[c] * weight;
                }
            }
        }
        out.extend(acc);
    }
    out
}

/// The source pixels covered by one output pixel along one axis.
#[derive(Debug)]
struct Span {
    /// Index of the first source pixel.
    start: usize,
    /// Weight of each source pixel from `start` on; they sum to 1.
    weights: Vec<f32>,
}

/// Splits `src_len` source pixels evenly over `dst_len` output pixels.
fn spans(src_len: usize, dst_len: usize) -> Vec<Span> {
    let scale = src_len as f64 / dst_len as f64;
    (0..dst_len)
        .map(|i| {
            let lo = i as f64 * scale;
            let hi = ((i + 1) as f64 * scale).min(src_len as f64);
            let start = (lo.floor() as usize).min(src_len - 1);
            let end = (hi.ceil() as usize).clamp(start + 1, src_len);
            let weights = (start..end)
                .map(|j| {
                    let overlap = hi.min((j + 1) as f64) - lo.max(j as f64);
                    (overlap.max(0.0) / (hi - lo)) as f32
                })
                .collect();
            Span { start, weights }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colour::composite;
    use image::Rgba;

    const BLACK: [f32; 3] = [0.0; 3];

    fn to_srgb(pixels: &[LinearRgba]) -> Vec<[u8; 3]> {
        pixels.iter().map(|&p| composite(p, BLACK)).collect()
    }

    #[test]
    fn output_size_caps_width_and_keeps_aspect() {
        assert_eq!(output_size(10, 6, 80), (10, 6), "never upscales");
        assert_eq!(output_size(400, 200, 100), (100, 50));
        assert_eq!(output_size(3000, 1, 100), (100, 1), "at least one row");
        assert_eq!(output_size(90, 1200, 80), (80, 1067));
    }

    #[test]
    fn spans_cover_every_source_pixel_with_unit_weight() {
        for (src, dst) in [(10, 10), (640, 237), (7, 3), (1000, 1), (5, 4)] {
            let spans = spans(src, dst);
            let mut coverage = vec![0.0f64; src];
            for span in &spans {
                let total: f32 = span.weights.iter().sum();
                assert!(
                    (total - 1.0).abs() < 1e-5,
                    "{src}->{dst}: weights sum to {total}"
                );
                for (i, &w) in span.weights.iter().enumerate() {
                    coverage[span.start + i] += f64::from(w);
                }
            }
            // Each source pixel contributes the same total amount (dst/src).
            for c in coverage {
                assert!((c - dst as f64 / src as f64).abs() < 1e-5, "{src}->{dst}");
            }
        }
    }

    #[test]
    fn native_size_is_an_exact_copy() {
        let img = RgbaImage::from_fn(13, 7, |x, y| {
            Rgba([(x * 19) as u8, (y * 37) as u8, ((x * y) % 256) as u8, 255])
        });
        let out = to_srgb(&resample(&img, 13, 7));
        let expected: Vec<_> = img.pixels().map(|p| [p[0], p[1], p[2]]).collect();
        assert_eq!(out, expected);
    }

    #[test]
    fn uniform_colour_stays_uniform_at_any_ratio() {
        let img = RgbaImage::from_pixel(97, 61, Rgba([12, 200, 99, 255]));
        let (w, h) = output_size(97, 61, 23);
        assert!(
            to_srgb(&resample(&img, w, h))
                .iter()
                .all(|&p| p == [12, 200, 99])
        );
    }

    #[test]
    fn fine_detail_averages_instead_of_aliasing() {
        // A 1-pixel checkerboard must come out as even grey at every ratio,
        // not as black or white (which is what point sampling produces).
        let img = RgbaImage::from_fn(64, 64, |x, y| {
            let v = if (x + y) % 2 == 0 { 255 } else { 0 };
            Rgba([v, v, v, 255])
        });
        for width in [32, 24, 21] {
            let (w, h) = output_size(64, 64, width);
            for p in to_srgb(&resample(&img, w, h)) {
                // A ~3x3 footprint holds 4/9 to 5/9 white: sRGB 179 to 197.
                assert!((175..=200).contains(&p[0]), "width {width}: got {p:?}");
            }
        }
        let (w, h) = output_size(64, 64, 32);
        assert!(
            to_srgb(&resample(&img, w, h))
                .iter()
                .all(|&p| p == [188; 3])
        );
    }

    #[test]
    fn samples_are_centred_on_their_footprint() {
        // Left half white, right half black: each half must map to its own output pixel.
        let img = RgbaImage::from_fn(8, 4, |x, _| {
            let v = if x < 4 { 255 } else { 0 };
            Rgba([v, v, v, 255])
        });
        assert_eq!(to_srgb(&resample(&img, 2, 1)), vec![[255; 3], [0; 3]]);
    }

    #[test]
    fn transparent_pixels_contribute_no_colour() {
        let img = RgbaImage::from_fn(2, 1, |x, _| {
            if x == 0 {
                Rgba([255, 0, 0, 0])
            } else {
                Rgba([0, 0, 255, 255])
            }
        });
        let [p] = resample(&img, 1, 1)[..] else {
            panic!()
        };
        assert_eq!(p[0], 0.0, "invisible red must not bleed in");
        assert!((p[3] - 0.5).abs() < 1e-6);
        assert_eq!(
            composite(p, [1.0; 3]),
            [188, 188, 255],
            "half blue over white"
        );
    }
}
