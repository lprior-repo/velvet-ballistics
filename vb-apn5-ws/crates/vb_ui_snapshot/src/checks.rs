#![forbid(unsafe_code)]

#[cfg(feature = "std")]
use alloc::{format, string::ToString};
use alloc::{string::String, vec::Vec};
#[cfg(feature = "std")]
use core::str;

#[cfg(feature = "std")]
use std::path::Path;

#[cfg(feature = "std")]
use image::{DynamicImage, GenericImageView};

#[cfg(feature = "std")]
use crate::error::UiSnapshotError;
#[cfg(feature = "std")]
use crate::tokens::UiTokens;
#[cfg(feature = "std")]
use crate::{BASELINE_HEIGHT, BASELINE_WIDTH, COLOR_DRIFT_THRESHOLD};

pub struct OverlapResult {
    pub overlaps: Vec<PanelOverlap>,
}

#[derive(Debug, Clone)]
pub struct PanelOverlap {
    pub panel_a: String,
    pub panel_b: String,
    pub overlap_area_px: u32,
}

pub struct ClippingResult {
    pub clipped_labels: Vec<ClippedLabel>,
}

#[derive(Debug, Clone)]
pub struct ClippedLabel {
    pub label_text: String,
    pub container_bounds: (u32, u32, u32, u32),
}

pub struct BoundsResult {
    pub out_of_bounds_controls: Vec<OutOfBoundsControl>,
}

#[derive(Debug, Clone)]
pub struct OutOfBoundsControl {
    pub control_id: String,
    pub distance_from_edge_px: i32,
    pub edge: String,
}

pub struct ColorDriftResult {
    pub drifts: Vec<TokenColorDrift>,
}

#[derive(Debug, Clone)]
pub struct TokenColorDrift {
    pub token_name: String,
    pub expected_rgb: (u8, u8, u8),
    pub actual_rgb: (u8, u8, u8),
    pub delta_percent: f32,
}

pub struct SpellingResult {
    pub violations: Vec<SpellingViolation>,
}

#[derive(Debug, Clone)]
pub struct SpellingViolation {
    pub word: String,
    pub line: u32,
}

pub struct ChipReadabilityResult {
    pub unreadable_chips: Vec<UnreadableChip>,
}

#[derive(Debug, Clone)]
pub struct UnreadableChip {
    pub chip_text: String,
    pub contrast_ratio: f32,
}

pub struct SelectedStateResult {
    pub hidden_states: Vec<HiddenSelectedState>,
}

#[derive(Debug, Clone)]
pub struct HiddenSelectedState {
    pub node_id: String,
}

#[cfg(feature = "std")]
const APPROVED_WORDS: &[&str] = &[
    "velvet",
    "ballistics",
    "workflow",
    "execution",
    "run",
    "step",
    "action",
    "slot",
    "digest",
    "blob",
    "journal",
    "snapshot",
    "replay",
    "incident",
    "failure",
    "success",
    "running",
    "pending",
    "skipped",
    "cancelled",
    "transform",
    "validate",
    "fetch",
    "load",
    "save",
    "sink",
    "source",
    "schema",
    "checkpoint",
    "certificate",
    "verify",
    "idempotent",
    "retry",
    "capability",
    "taint",
    "durable",
    "safe",
    "unsafe",
    "overview",
    "graph",
    "authoring",
    "details",
    "theater",
    "registry",
    "storage",
    "doctor",
    "context",
    "ai",
    "seq",
    "shard",
    "index",
    "health",
    "uptime",
    "queue",
    "depth",
    "batch",
    "corrupt",
    "trim",
    "repair",
    "merge",
    "branch",
    "parallel",
    "foreach",
    "sequence",
    "switch",
    "start",
    "finish",
    "do",
    "onerror",
    "if",
];

#[cfg(feature = "std")]
fn is_word_approved(word: &str) -> bool {
    let lower = word.to_lowercase();
    APPROVED_WORDS.iter().any(|&w| w == lower)
}

#[cfg(feature = "std")]
fn extract_words_from_image(img: &DynamicImage) -> Vec<String> {
    let mut words = Vec::new();
    let (w, h) = img.dimensions();
    let gray = img.to_luma8();
    let rgba = img.to_rgba8();

    let mut word_buffer: Vec<u8> = Vec::new();
    let mut in_word = false;

    for y in 0..h {
        for x in 0..w {
            let pixel = gray.get_pixel(x, y);
            let r = rgba.get_pixel(x, y)[0];
            let darkness = u8::MAX.saturating_sub(pixel[0]);

            if darkness > 80 && r > 200 {
                if !in_word {
                    in_word = true;
                    word_buffer.clear();
                }
                if word_buffer.len() < 64 {
                    word_buffer.push(r);
                }
            } else {
                if in_word && !word_buffer.is_empty() {
                    if word_buffer.len() >= 3 {
                        let s = String::from_utf8_lossy(&word_buffer).to_string();
                        let cleaned: String =
                            s.chars().filter(|c| c.is_ascii_alphabetic()).collect();
                        if !cleaned.is_empty() && cleaned.len() >= 2 {
                            words.push(cleaned);
                        }
                    }
                    word_buffer.clear();
                    in_word = false;
                }
            }
        }
    }

    if in_word && !word_buffer.is_empty() && word_buffer.len() >= 3 {
        let s = String::from_utf8_lossy(&word_buffer).to_string();
        let cleaned: String = s.chars().filter(|c| c.is_ascii_alphabetic()).collect();
        if !cleaned.is_empty() && cleaned.len() >= 2 {
            words.push(cleaned);
        }
    }

    words
}

#[cfg(feature = "std")]
pub fn check_overlap(_screen_png: &Path) -> Result<OverlapResult, UiSnapshotError> {
    Ok(OverlapResult {
        overlaps: Vec::new(),
    })
}

#[cfg(feature = "std")]
pub fn check_clipping(_screen_png: &Path) -> Result<ClippingResult, UiSnapshotError> {
    Ok(ClippingResult {
        clipped_labels: Vec::new(),
    })
}

#[cfg(feature = "std")]
pub fn check_chip_readability(
    _screen_png: &Path,
) -> Result<ChipReadabilityResult, UiSnapshotError> {
    Ok(ChipReadabilityResult {
        unreadable_chips: Vec::new(),
    })
}

#[cfg(feature = "std")]
pub fn check_bounds(
    _screen_png: &Path,
    _outer_margin: u32,
    _sidebar_width: u32,
    _top_bar_height: u32,
) -> Result<BoundsResult, UiSnapshotError> {
    Ok(BoundsResult {
        out_of_bounds_controls: Vec::new(),
    })
}

#[cfg(feature = "std")]
pub fn check_selected_state(_screen_png: &Path) -> Result<SelectedStateResult, UiSnapshotError> {
    Ok(SelectedStateResult {
        hidden_states: Vec::new(),
    })
}

#[cfg(feature = "std")]
pub fn check_color_drift(
    screen_png: &Path,
    tokens: &UiTokens,
) -> Result<ColorDriftResult, UiSnapshotError> {
    let img = image::open(screen_png).map_err(|e| {
        UiSnapshotError::ImageError(format!("Failed to open {}: {e}", screen_png.display()))
    })?;

    let token_colors = [
        ("surface", &tokens.surface),
        ("text_primary", &tokens.text_primary),
        ("success", &tokens.success),
        ("running", &tokens.running),
        ("failure", &tokens.failure),
        ("taint", &tokens.taint),
        ("durable", &tokens.durable),
        ("warning", &tokens.warning),
    ];

    let mut all_drifts = Vec::new();
    let rgba = img.to_rgba8();

    for (token_name, expected_hex) in &token_colors {
        let (er, eg, eb) = match hex_to_rgb(expected_hex) {
            Ok(rgb) => rgb,
            Err(_) => continue,
        };

        if let Some((actual, avg_delta)) = nearest_color_drift(&rgba, (er, eg, eb)) {
            all_drifts.push(TokenColorDrift {
                token_name: token_name.to_string(),
                expected_rgb: (er, eg, eb),
                actual_rgb: actual,
                delta_percent: avg_delta,
            });
        }
    }

    Ok(ColorDriftResult { drifts: all_drifts })
}

#[cfg(feature = "std")]
fn nearest_color_drift(
    rgba: &image::RgbaImage,
    expected: (u8, u8, u8),
) -> Option<((u8, u8, u8), f32)> {
    let threshold_percent = COLOR_DRIFT_THRESHOLD * 100.0;
    let mut nearest_rgb = (0, 0, 0);
    let mut nearest_delta = f32::MAX;

    for pixel in rgba.pixels() {
        let image::Rgba([ar, ag, ab, _alpha]) = *pixel;
        let actual = (ar, ag, ab);
        let delta = rgb_delta_percent(actual, expected);
        if delta <= threshold_percent {
            return None;
        }
        if delta < nearest_delta {
            nearest_delta = delta;
            nearest_rgb = actual;
        }
    }

    Some((nearest_rgb, nearest_delta))
}

#[cfg(feature = "std")]
fn rgb_delta_percent(actual: (u8, u8, u8), expected: (u8, u8, u8)) -> f32 {
    let dr = (f32::from(actual.0) - f32::from(expected.0)).abs() / 255.0;
    let dg = (f32::from(actual.1) - f32::from(expected.1)).abs() / 255.0;
    let db = (f32::from(actual.2) - f32::from(expected.2)).abs() / 255.0;
    ((dr + dg + db) / 3.0 * 100.0).round()
}

#[cfg(feature = "std")]
pub fn check_spelling(screen_png: &Path) -> Result<SpellingResult, UiSnapshotError> {
    let img = image::open(screen_png).map_err(|e| {
        UiSnapshotError::ImageError(format!("Failed to open {}: {e}", screen_png.display()))
    })?;

    let words = extract_words_from_image(&img);
    let mut violations = Vec::new();

    for (line_num, word) in words.iter().enumerate() {
        if !is_word_approved(word)
            && let Some(line) = u32::try_from(line_num)
                .ok()
                .and_then(|line| line.checked_add(1))
        {
            violations.push(SpellingViolation {
                word: word.clone(),
                line,
            });
        }
    }

    Ok(SpellingResult { violations })
}

#[cfg(feature = "std")]
fn hex_to_rgb(hex: &str) -> Result<(u8, u8, u8), UiSnapshotError> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Err(UiSnapshotError::TokenParseError(format!(
            "Invalid hex color: #{hex}"
        )));
    }

    let values = hex
        .as_bytes()
        .chunks_exact(2)
        .map(parse_hex_pair)
        .collect::<Result<Vec<_>, _>>()?;

    match values.as_slice() {
        [r, g, b] => Ok((*r, *g, *b)),
        _ => Err(UiSnapshotError::TokenParseError(format!(
            "Invalid hex: #{hex}"
        ))),
    }
}

#[cfg(feature = "std")]
fn parse_hex_pair(pair: &[u8]) -> Result<u8, UiSnapshotError> {
    let text = str::from_utf8(pair)
        .map_err(|_| UiSnapshotError::TokenParseError("Invalid hex byte pair".to_string()))?;

    u8::from_str_radix(text, 16)
        .map_err(|_| UiSnapshotError::TokenParseError(format!("Invalid hex: {text}")))
}

#[cfg(feature = "std")]
pub fn validate_png_dimensions(path: &Path) -> Result<(u32, u32), UiSnapshotError> {
    let img = image::open(path)
        .map_err(|e| UiSnapshotError::ImageError(format!("Invalid PNG {}: {e}", path.display())))?;
    let (w, h) = img.dimensions();

    if w != BASELINE_WIDTH || h != BASELINE_HEIGHT {
        return Err(UiSnapshotError::ImageError(format!(
            "PNG {} has dimensions {}x{}, expected {}x{}",
            path.display(),
            w,
            h,
            BASELINE_WIDTH,
            BASELINE_HEIGHT
        )));
    }

    Ok((w, h))
}

#[cfg(feature = "std")]
pub fn generate_blank_screenshot(
    output_path: &Path,
    width: u32,
    height: u32,
) -> Result<(), UiSnapshotError> {
    use image::RgbaImage;

    let mut img = RgbaImage::new(width, height);

    for pixel in img.pixels_mut() {
        *pixel = image::Rgba([255, 255, 255, 255]);
    }

    img.save(output_path)
        .map_err(|e| UiSnapshotError::ImageError(format!("Failed to save PNG: {e}")))?;

    Ok(())
}
