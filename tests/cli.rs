//! End-to-end tests that run the imcat binary.
//!
//! Snapshot tests compare output with files in `tests/snapshots/`. After an
//! intentional change in output, inspect it, then refresh them with:
//!
//!     IMCAT_BLESS=1 cargo test
//!
//! Fixture images are made by `tests/fixtures/generate.py`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Runs imcat with a clean environment so the user's settings can't leak in.
fn imcat(args: &[&str]) -> Output {
    imcat_with_env(args, &[])
}

fn imcat_with_env(args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_imcat"));
    cmd.args(args)
        .env_remove("IMCATBG")
        .env_remove("IMCAT_WIDTH");
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().expect("failed to run imcat")
}

fn stdout_of(args: &[&str]) -> Vec<u8> {
    let out = imcat(args);
    assert!(
        out.status.success(),
        "imcat {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

/// (width in cells, number of lines) of rendered output.
fn dimensions(output: &[u8]) -> (usize, usize) {
    let text = String::from_utf8_lossy(output);
    let lines: Vec<&str> = text.lines().collect();
    let width = lines.first().map_or(0, |l| l.matches('▀').count());
    assert!(
        lines.iter().all(|l| l.matches('▀').count() == width),
        "ragged output"
    );
    (width, lines.len())
}

fn assert_snapshot(name: &str, actual: &[u8]) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(format!("{name}.ans"));
    if std::env::var_os("IMCAT_BLESS").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, actual).unwrap();
        return;
    }
    let expected = fs::read(&path).unwrap_or_else(|_| {
        panic!(
            "missing snapshot {}; run with IMCAT_BLESS=1",
            path.display()
        )
    });
    assert!(
        expected == actual,
        "output differs from {}; view both with `cat`, and if the change is intended rerun with IMCAT_BLESS=1",
        path.display()
    );
}

// --- Snapshots ---------------------------------------------------------------

#[test]
fn snapshot_downscaled() {
    let gradient = fixture("gradient.png");
    assert_snapshot(
        "gradient_w32",
        &stdout_of(&["-w", "32", gradient.to_str().unwrap()]),
    );
    assert_snapshot(
        "gradient_w23",
        &stdout_of(&["-w", "23", gradient.to_str().unwrap()]),
    );
}

#[test]
fn snapshot_native_size() {
    assert_snapshot(
        "gradient_native",
        &stdout_of(&["-w", "200", fixture("gradient.png").to_str().unwrap()]),
    );
}

#[test]
fn snapshot_alpha() {
    let alpha = fixture("alpha.png");
    // 31 rows: also covers the odd last line.
    assert_snapshot(
        "alpha_black",
        &stdout_of(&["-w", "100", alpha.to_str().unwrap()]),
    );
    assert_snapshot(
        "alpha_bg",
        &stdout_of(&["-w", "20", "--bg", "#dad9cc", alpha.to_str().unwrap()]),
    );
}

// --- Behaviour ---------------------------------------------------------------

#[test]
fn output_fits_requested_width() {
    let gradient = fixture("gradient.png");
    assert_eq!(
        dimensions(&stdout_of(&["-w", "32", gradient.to_str().unwrap()])),
        (32, 10)
    );
    assert_eq!(
        dimensions(&stdout_of(&["-w", "200", gradient.to_str().unwrap()])),
        (64, 20),
        "never upscales"
    );
    assert_eq!(
        dimensions(&stdout_of(&[
            "-w",
            "48",
            fixture("alpha.png").to_str().unwrap()
        ])),
        (48, 16)
    );
}

#[test]
fn environment_variables_match_flags() {
    let alpha = fixture("alpha.png");
    let alpha = alpha.to_str().unwrap();
    let by_flags = stdout_of(&["-w", "20", "--bg", "#dad9cc", alpha]);
    let by_env = imcat_with_env(&[alpha], &[("IMCAT_WIDTH", "20"), ("IMCATBG", "#dad9cc")]);
    assert!(by_env.status.success());
    assert_eq!(by_flags, by_env.stdout);

    let flag_wins = imcat_with_env(&["-w", "12", alpha], &[("IMCAT_WIDTH", "30")]);
    assert_eq!(dimensions(&flag_wins.stdout).0, 12);
}

#[test]
fn background_only_changes_transparent_areas() {
    let gradient = fixture("gradient.png");
    let opaque = gradient.to_str().unwrap();
    assert_eq!(
        stdout_of(&["-w", "20", opaque]),
        stdout_of(&["-w", "20", "--bg", "#ff00ff", opaque])
    );
    let alpha = fixture("alpha.png");
    let alpha = alpha.to_str().unwrap();
    assert_ne!(
        stdout_of(&["-w", "20", alpha]),
        stdout_of(&["-w", "20", "--bg", "#ff00ff", alpha])
    );
}

#[test]
fn lossless_formats_render_identically() {
    let reference = stdout_of(&[
        "-w",
        "80",
        fixture("formats/reference.png").to_str().unwrap(),
    ]);
    assert_eq!(dimensions(&reference), (16, 8));
    for name in [
        "image.bmp",
        "image.gif",
        "image.ico",
        "image.ppm",
        "image.qoi",
        "image.tga",
        "image.tiff",
        "image.webp",
        "image.ff",
        "no_extension",
    ] {
        let out = stdout_of(&[
            "-w",
            "80",
            fixture(&format!("formats/{name}")).to_str().unwrap(),
        ]);
        assert!(
            out == reference,
            "{name} renders differently from reference.png"
        );
    }
}

#[test]
fn lossy_formats_decode() {
    for name in ["image.jpg", "image.dds"] {
        let out = stdout_of(&[
            "-w",
            "80",
            fixture(&format!("formats/{name}")).to_str().unwrap(),
        ]);
        assert_eq!(dimensions(&out), (16, 8), "{name}");
    }
}

#[test]
fn exif_orientation_is_applied() {
    // Stored as 40x20 with "rotate 90° clockwise"; displayed as 20x40 (20 lines).
    let out = stdout_of(&["-w", "80", fixture("orientation6.jpg").to_str().unwrap()]);
    assert_eq!(dimensions(&out), (20, 20));
    let text = String::from_utf8(out).unwrap();
    let first = text.lines().next().unwrap();
    let last = text.lines().last().unwrap();
    let fg = |line: &str| -> Vec<u8> {
        let start = line.find("\x1b[38;2;").unwrap() + 7;
        let end = start + line[start..].find('m').unwrap();
        line[start..end]
            .split(';')
            .map(|v| v.parse().unwrap())
            .collect()
    };
    let (top, bottom) = (fg(first), fg(last));
    assert!(
        top[0] > 200 && top[2] < 50,
        "top should be red, got {top:?}"
    );
    assert!(
        bottom[2] > 200 && bottom[0] < 50,
        "bottom should be blue, got {bottom:?}"
    );
}

#[test]
fn bad_files_are_reported_and_skipped() {
    let gradient = fixture("gradient.png");
    let not_an_image = fixture("generate.py");
    let out = imcat(&[
        "-w",
        "10",
        "missing.png",
        not_an_image.to_str().unwrap(),
        gradient.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("could not load image missing.png"),
        "{stderr}"
    );
    assert!(stderr.contains("generate.py"), "{stderr}");
    assert_eq!(
        dimensions(&out.stdout),
        (10, 3),
        "the good image is still shown"
    );
}

#[test]
fn usage_errors_exit_with_2() {
    let gradient = fixture("gradient.png");
    let gradient = gradient.to_str().unwrap();
    assert_eq!(imcat(&[]).status.code(), Some(2), "no images");
    assert_eq!(imcat(&["--bg", "blue", gradient]).status.code(), Some(2));
    assert_eq!(imcat(&["-w", "0", gradient]).status.code(), Some(2));
    assert_eq!(
        imcat_with_env(&[gradient], &[("IMCATBG", "nope")])
            .status
            .code(),
        Some(2)
    );
}

#[test]
fn version_and_help() {
    let version = stdout_of(&["--version"]);
    assert_eq!(
        String::from_utf8(version).unwrap().trim(),
        format!("imcat {}", env!("CARGO_PKG_VERSION"))
    );
    let help = String::from_utf8(stdout_of(&["--help"])).unwrap();
    for needle in ["--width", "--bg", "IMCAT_WIDTH", "IMCATBG"] {
        assert!(help.contains(needle), "help is missing {needle}");
    }
}
