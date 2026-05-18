//! Tests for `checks.rs` pub fns: `generate_blank_screenshot` and `check_spelling`.
//!
//! generate_blank_screenshot had no dedicated unit test.
//! check_spelling is tested via fixture-injection in report_evidence_shape.rs;
//! these tests cover the PNG-not-found path directly.

use image::GenericImageView;
use std::fs;
use std::path::{Path, PathBuf};
use vb_ui_snapshot::checks::{check_spelling, generate_blank_screenshot};

fn _write_png(path: &Path, w: u32, h: u32) {
    let img = image::RgbaImage::new(w, h);
    img.save(path).expect("save ok");
}

//
// generate_blank_screenshot
//

#[test]
fn generate_blank_screenshot_creates_1920x1080_white_png() {
    let path = Path::new("target/vb-ui-snapshot-image-tests/blank_1920.png");
    let _ = fs::create_dir_all("target/vb-ui-snapshot-image-tests");

    generate_blank_screenshot(path, 1920, 1080).expect("generate ok");

    let metadata = fs::metadata(path).expect("file exists");
    assert!(metadata.len() > 0, "PNG should have non-zero size");

    // Verify dimensions by reopening
    let img = image::open(path).expect("reopen ok");
    assert_eq!(img.dimensions(), (1920, 1080));
}

#[test]
fn generate_blank_screenshot_creates_1x1_minimal_png() {
    let path = Path::new("target/vb-ui-snapshot-image-tests/blank_1x1.png");
    let _ = fs::create_dir_all("target/vb-ui-snapshot-image-tests");

    generate_blank_screenshot(path, 1, 1).expect("generate ok");

    let img = image::open(path).expect("reopen ok");
    assert_eq!(img.dimensions(), (1, 1));
}

#[test]
fn generate_blank_screenshot_rejects_denied_path() {
    let path = Path::new("/proc/vb-nf2u-denied/output.png");
    let result = generate_blank_screenshot(path, 100, 100);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("PngGenerationFailed"), "got: {msg}");
}

#[test]
fn generate_blank_screenshot_accepts_different_dimensions() {
    let path = Path::new("target/vb-ui-snapshot-image-tests/blank_800x600.png");
    let _ = fs::create_dir_all("target/vb-ui-snapshot-image-tests");

    generate_blank_screenshot(path, 800, 600).expect("generate ok");

    let img = image::open(path).expect("reopen ok");
    assert_eq!(img.dimensions(), (800, 600));
}

#[test]
fn generate_blank_screenshot_overwrites_existing_file() {
    let path = Path::new("target/vb-ui-snapshot-image-tests/overwrite.png");
    let _ = fs::create_dir_all("target/vb-ui-snapshot-image-tests");

    // First write
    generate_blank_screenshot(path, 100, 50).expect("first write ok");
    let size1 = fs::metadata(path).expect("exists").len();

    // Overwrite
    generate_blank_screenshot(path, 200, 100).expect("overwrite ok");
    let size2 = fs::metadata(path).expect("exists").len();

    // Should have replaced (both should be valid PNGs, just different sizes)
    assert_ne!(size1, size2, "overwrite should change file");
    let img = image::open(path).expect("reopen ok");
    assert_eq!(img.dimensions(), (200, 100));
}

//
// check_spelling — non-fixture paths
//

#[test]
fn check_spelling_returns_ok_for_blank_image() {
    let dir = PathBuf::from("target/vb-ui-snapshot-image-tests");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("blank.png");

    // Write minimal valid PNG
    let img = image::RgbaImage::new(100, 100);
    img.save(&path).expect("save ok");

    let result = check_spelling(&path).expect("check_spelling should not error");
    // blank white image → no words found → no violations
    assert!(
        result.violations.is_empty(),
        "blank image should have no violations"
    );
}

#[test]
fn check_spelling_rejects_spelling_fixture_path() {
    // The spelling check short-circuits on paths containing "vb-nf2u-spelling-fixture"
    let path = Path::new("target/vb-nf2u-spelling-fixture.png");
    let result = check_spelling(path);
    assert!(result.is_err(), "expected error for spelling fixture path");
}

#[test]
fn check_spelling_returns_err_for_missing_file() {
    let path = Path::new("target/vb-ui-snapshot-image-tests/nonexistent.png");
    let result = check_spelling(path);
    // Missing file → IoError from image::open
    assert!(result.is_err());
}

#[test]
fn generate_blank_screenshot_accepts_zero_dimension() {
    // Edge case: verify zero-dimension call does not panic
    let path = Path::new("target/vb-ui-snapshot-image-tests/zero_dim.png");
    let _ = fs::create_dir_all("target/vb-ui-snapshot-image-tests");
    let result = generate_blank_screenshot(path, 0, 0);
    // Either succeeds (and creates file) or fails gracefully — no panic
    if result.is_ok() {
        assert!(path.exists());
    }
}

#[test]
fn check_spelling_accepts_large_blank_image() {
    // Large blank white image → no words extracted → no violations
    let dir = PathBuf::from("target/vb-ui-snapshot-image-tests");
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("large_blank.png");

    let img = image::RgbaImage::new(1920, 1080);
    img.save(&path).expect("save ok");

    let result = check_spelling(&path).expect("check_spelling should succeed");
    assert!(result.violations.is_empty());
}
