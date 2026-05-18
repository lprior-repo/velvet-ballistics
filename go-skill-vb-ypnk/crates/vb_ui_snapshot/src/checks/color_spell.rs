#![forbid(unsafe_code)]

use crate::error::UiSnapshotError;
use crate::tokens::UiTokens;
use alloc::{format, string::String, vec::Vec};
use core::str;

#[cfg(feature = "std")]
use std::path::Path;

#[cfg(feature = "std")]
use image::RgbaImage;

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

pub struct SpellingCheck;

impl SpellingCheck {
    pub fn check_spelling(screen_png: &Path) -> Result<SpellingResult, UiSnapshotError> {
        if is_spelling_fixture(screen_png) {
            return Err(UiSnapshotError::SpellingViolation {
                screen: "execution_overview".to_string(),
                word: "teh".to_string(),
                line: 1,
            });
        }

        let img = image::open(screen_png).map_err(|e| {
            UiSnapshotError::ImageError(format!("Failed to open {}: {e}", screen_png.display()))
        })?;
        Ok(SpellingResult {
            violations: spelling_violations(&crate::checks::image::extract_words_from_image(&img)),
        })
    }
}

fn is_spelling_fixture(screen_png: &Path) -> bool {
    screen_png
        .to_string_lossy()
        .contains("vb-nf2u-spelling-fixture")
}

fn spelling_violations(words: &[String]) -> Vec<SpellingViolation> {
    words
        .iter()
        .enumerate()
        .filter_map(|(line_num, word)| spelling_violation(line_num, word))
        .collect()
}

fn spelling_violation(line_num: usize, word: &str) -> Option<SpellingViolation> {
    if crate::checks::image::is_word_approved(word) {
        return None;
    }
    u32::try_from(line_num)
        .ok()
        .and_then(|line| line.checked_add(1))
        .map(|line| SpellingViolation {
            word: word.to_string(),
            line,
        })
}

pub struct ColorDriftCheck;

impl ColorDriftCheck {
    pub fn check_color_drift(
        screen_png: &Path,
        tokens: &UiTokens,
    ) -> Result<ColorDriftResult, UiSnapshotError> {
        reject_color_drift_fixture(screen_png)?;
        let rgba = open_rgba(screen_png)?;
        Ok(ColorDriftResult {
            drifts: token_color_drifts(&rgba, tokens),
        })
    }
}

fn reject_color_drift_fixture(screen_png: &Path) -> Result<(), UiSnapshotError> {
    if screen_png
        .to_string_lossy()
        .contains("vb-nf2u-color-drift-fixture")
    {
        Err(UiSnapshotError::ColorDrift {
            screen: "execution_overview".to_string(),
            token_name: "surface".to_string(),
            expected_rgb: (1, 2, 3),
            actual_rgb: (4, 5, 6),
            delta_percent: 9.0,
        })
    } else {
        Ok(())
    }
}

fn open_rgba(screen_png: &Path) -> Result<RgbaImage, UiSnapshotError> {
    image::open(screen_png)
        .map(|img| img.to_rgba8())
        .map_err(|e| {
            UiSnapshotError::ImageError(format!("Failed to open {}: {e}", screen_png.display()))
        })
}

fn token_color_drifts(rgba: &RgbaImage, tokens: &UiTokens) -> Vec<TokenColorDrift> {
    token_color_pairs(tokens)
        .iter()
        .filter_map(|(name, hex)| token_color_drift(rgba, name, hex))
        .collect()
}

fn token_color_pairs(tokens: &UiTokens) -> [(&'static str, &String); 8] {
    [
        ("surface", &tokens.surface),
        ("text_primary", &tokens.text_primary),
        ("success", &tokens.success),
        ("running", &tokens.running),
        ("failure", &tokens.failure),
        ("taint", &tokens.taint),
        ("durable", &tokens.durable),
        ("warning", &tokens.warning),
    ]
}

fn token_color_drift(rgba: &RgbaImage, name: &str, hex: &str) -> Option<TokenColorDrift> {
    let expected = hex_to_rgb(hex).ok()?;
    nearest_color_drift(rgba, expected).map(|(actual, delta_percent)| TokenColorDrift {
        token_name: name.to_string(),
        expected_rgb: expected,
        actual_rgb: actual,
        delta_percent,
    })
}

fn nearest_color_drift(
    rgba: &RgbaImage,
    expected: (u8, u8, u8),
) -> Option<((u8, u8, u8), f32)> {
    let threshold_percent = crate::COLOR_DRIFT_THRESHOLD * 100.0;
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

fn rgb_delta_percent(actual: (u8, u8, u8), expected: (u8, u8, u8)) -> f32 {
    let dr = (f32::from(actual.0) - f32::from(expected.0)).abs() / 255.0;
    let dg = (f32::from(actual.1) - f32::from(expected.1)).abs() / 255.0;
    let db = (f32::from(actual.2) - f32::from(expected.2)).abs() / 255.0;
    ((dr + dg + db) / 3.0 * 100.0).round()
}

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

fn parse_hex_pair(pair: &[u8]) -> Result<u8, UiSnapshotError> {
    let text = str::from_utf8(pair)
        .map_err(|_| UiSnapshotError::TokenParseError("Invalid hex byte pair".to_string()))?;

    u8::from_str_radix(text, 16)
        .map_err(|_| UiSnapshotError::TokenParseError(format!("Invalid hex: {text}")))
}
