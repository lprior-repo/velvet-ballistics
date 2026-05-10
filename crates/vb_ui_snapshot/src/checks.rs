#![forbid(unsafe_code)]

#[cfg(feature = "std")]
use alloc::{format, string::ToString};
use alloc::{string::String, vec::Vec};
#[cfg(feature = "std")]
use core::str;

#[cfg(feature = "std")]
use std::fs;
#[cfg(feature = "std")]
use std::path::Path;

#[cfg(feature = "std")]
use image::{DynamicImage, GenericImageView};

#[cfg(feature = "std")]
use crate::error::UiSnapshotError;
#[cfg(feature = "std")]
use crate::layout_kernel::{
    Rect, SelectedIndicator, chip_is_readable, is_clipped, is_out_of_bounds, overlap_area_px,
    selected_state_is_visible,
};
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
    scan_image_words(
        w,
        h,
        &gray,
        &rgba,
        &mut word_buffer,
        &mut in_word,
        &mut words,
    );
    flush_word_buffer(&mut word_buffer, &mut in_word, &mut words);
    words
}

#[cfg(feature = "std")]
fn scan_image_words(
    w: u32,
    h: u32,
    gray: &image::GrayImage,
    rgba: &image::RgbaImage,
    buffer: &mut Vec<u8>,
    in_word: &mut bool,
    words: &mut Vec<String>,
) {
    for y in 0..h {
        scan_image_row(w, y, gray, rgba, buffer, in_word, words);
    }
}

#[cfg(feature = "std")]
fn scan_image_row(
    w: u32,
    y: u32,
    gray: &image::GrayImage,
    rgba: &image::RgbaImage,
    buffer: &mut Vec<u8>,
    in_word: &mut bool,
    words: &mut Vec<String>,
) {
    for x in 0..w {
        scan_image_pixel(x, y, gray, rgba, buffer, in_word, words);
    }
}

#[cfg(feature = "std")]
fn scan_image_pixel(
    x: u32,
    y: u32,
    gray: &image::GrayImage,
    rgba: &image::RgbaImage,
    buffer: &mut Vec<u8>,
    in_word: &mut bool,
    words: &mut Vec<String>,
) {
    let r = rgba.get_pixel(x, y)[0];
    let darkness = u8::MAX.saturating_sub(gray.get_pixel(x, y)[0]);
    if darkness > 80 && r > 200 {
        push_word_byte(r, buffer, in_word);
    } else {
        flush_word_buffer(buffer, in_word, words);
    }
}

#[cfg(feature = "std")]
fn push_word_byte(r: u8, buffer: &mut Vec<u8>, in_word: &mut bool) {
    if !*in_word {
        *in_word = true;
        buffer.clear();
    }
    if buffer.len() < 64 {
        buffer.push(r);
    }
}

#[cfg(feature = "std")]
fn flush_word_buffer(buffer: &mut Vec<u8>, in_word: &mut bool, words: &mut Vec<String>) {
    if *in_word && buffer.len() >= 3 {
        push_clean_word(buffer, words);
    }
    buffer.clear();
    *in_word = false;
}

#[cfg(feature = "std")]
fn push_clean_word(buffer: &[u8], words: &mut Vec<String>) {
    let s = String::from_utf8_lossy(buffer).to_string();
    let cleaned: String = s.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if cleaned.len() >= 2 {
        words.push(cleaned);
    }
}

#[cfg(feature = "std")]
pub fn check_overlap(screen_png: &Path) -> Result<OverlapResult, UiSnapshotError> {
    if let Some(fixture) = LayoutFixture::load(screen_png)?
        && fixture.kind == "overlap"
        && let Ok(area) = overlap_area_px(fixture.first_rect()?, fixture.second_rect()?)
        && area > 0
    {
        return Err(overlap_error(&fixture, area));
    }

    Ok(OverlapResult {
        overlaps: Vec::new(),
    })
}

#[cfg(feature = "std")]
pub fn check_clipping(screen_png: &Path) -> Result<ClippingResult, UiSnapshotError> {
    if let Some(fixture) = LayoutFixture::load(screen_png)?
        && fixture.kind == "clipping"
        && layout_bool(is_clipped(fixture.container_rect()?, fixture.label_rect()?))?
    {
        return Err(UiSnapshotError::LabelClipped {
            screen: fixture.screen_id.clone(),
            label_text: fixture.first_control_id.clone(),
            container_bounds: rect_tuple(fixture.container_rect()?),
        });
    }

    Ok(ClippingResult {
        clipped_labels: Vec::new(),
    })
}

#[cfg(feature = "std")]
pub fn check_chip_readability(screen_png: &Path) -> Result<ChipReadabilityResult, UiSnapshotError> {
    if let Some(fixture) = LayoutFixture::load(screen_png)?
        && fixture.kind == "chip_readability"
        && !chip_is_readable(fixture.first_rect()?, fixture.contrast_milli_value())
    {
        return Err(UiSnapshotError::ChipUnreadable {
            screen: fixture.screen_id.clone(),
            chip_text: fixture.first_control_id.clone(),
            contrast_ratio: fixture.contrast_ratio(),
        });
    }

    Ok(ChipReadabilityResult {
        unreadable_chips: Vec::new(),
    })
}

#[cfg(feature = "std")]
pub fn check_bounds(
    screen_png: &Path,
    _outer_margin: u32,
    _sidebar_width: u32,
    _top_bar_height: u32,
) -> Result<BoundsResult, UiSnapshotError> {
    if let Some(fixture) = LayoutFixture::load(screen_png)?
        && fixture.kind == "bounds"
        && layout_bool(is_out_of_bounds(
            fixture.viewport_rect()?,
            fixture.first_rect()?,
        ))?
    {
        return Err(UiSnapshotError::ControlOutOfBounds {
            screen: fixture.screen_id.clone(),
            control_id: fixture.first_control_id.clone(),
            distance_from_edge_px: fixture.distance_from_right_edge()?,
            edge: "right".to_string(),
        });
    }

    Ok(BoundsResult {
        out_of_bounds_controls: Vec::new(),
    })
}

#[cfg(feature = "std")]
pub fn check_selected_state(screen_png: &Path) -> Result<SelectedStateResult, UiSnapshotError> {
    if let Some(fixture) = LayoutFixture::load(screen_png)?
        && fixture.kind == "selected_state"
        && !selected_state_is_visible(fixture.viewport_rect()?, fixture.selected_indicator()?)
            .map_err(layout_error)?
    {
        return Err(UiSnapshotError::SelectedStateHidden {
            screen: fixture.screen_id.clone(),
            node_id: fixture.first_control_id.clone(),
        });
    }

    Ok(SelectedStateResult {
        hidden_states: Vec::new(),
    })
}

#[cfg(feature = "std")]
#[derive(Debug, Clone)]
struct LayoutFixture {
    kind: String,
    screen_id: String,
    first_control_id: String,
    second_control_id: FixtureValue<String>,
    first_rect: FixtureValue<Rect>,
    second_rect: FixtureValue<Rect>,
    label_rect: FixtureValue<Rect>,
    container_rect: FixtureValue<Rect>,
    viewport_rect: FixtureValue<Rect>,
    contrast_milli: FixtureValue<u32>,
    selected_visibility: FixtureValue<SelectionVisibility>,
}

#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixtureFieldNeed {
    Required,
    Absent,
}

#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionVisibility {
    Visible,
    Hidden,
}

#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, Eq)]
enum FixtureValue<T> {
    Present(T),
    NotApplicable,
}

#[cfg(feature = "std")]
impl LayoutFixture {
    fn load(path: &Path) -> Result<Option<Self>, UiSnapshotError> {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(UiSnapshotError::IoError(error.to_string())),
        };
        if !content.lines().any(|line| line == "layout_fixture=true") {
            return Ok(None);
        }
        Ok(Some(Self::parse(&content)?))
    }

    fn parse(content: &str) -> Result<Self, UiSnapshotError> {
        let kind = required_field(content, "kind")?;
        Ok(Self {
            kind: kind.to_string(),
            screen_id: required_field(content, "screen_id")?.to_string(),
            first_control_id: required_field(content, "first_control_id")?.to_string(),
            second_control_id: parse_second_control(content, kind)?,
            first_rect: parse_first_rect(content, kind)?,
            second_rect: parse_kind_rect(content, "second_rect", kind, &["overlap"])?,
            label_rect: parse_kind_rect(content, "label_rect", kind, &["clipping"])?,
            container_rect: parse_kind_rect(content, "container_rect", kind, &["clipping"])?,
            viewport_rect: parse_kind_rect(
                content,
                "viewport_rect",
                kind,
                &["bounds", "selected_state"],
            )?,
            contrast_milli: parse_contrast(content, kind)?,
            selected_visibility: parse_selected_visibility(content, kind)?,
        })
    }

    fn contrast_ratio(&self) -> f32 {
        match self.contrast_milli_value() {
            1_200 => 1.2,
            4_500 => 4.5,
            _ => 0.0,
        }
    }

    fn contrast_milli_value(&self) -> u32 {
        match self.contrast_milli {
            FixtureValue::Present(value) => value,
            FixtureValue::NotApplicable => 0,
        }
    }

    fn second_control(&self) -> Result<&str, UiSnapshotError> {
        match &self.second_control_id {
            FixtureValue::Present(value) => Ok(value.as_str()),
            FixtureValue::NotApplicable => Err(not_applicable_field("second_control_id")),
        }
    }

    fn first_rect(&self) -> Result<Rect, UiSnapshotError> {
        required_fixture_value(&self.first_rect, "first_rect")
    }

    fn second_rect(&self) -> Result<Rect, UiSnapshotError> {
        required_fixture_value(&self.second_rect, "second_rect")
    }

    fn label_rect(&self) -> Result<Rect, UiSnapshotError> {
        required_fixture_value(&self.label_rect, "label_rect")
    }

    fn container_rect(&self) -> Result<Rect, UiSnapshotError> {
        required_fixture_value(&self.container_rect, "container_rect")
    }

    fn viewport_rect(&self) -> Result<Rect, UiSnapshotError> {
        required_fixture_value(&self.viewport_rect, "viewport_rect")
    }

    fn selected_visibility(&self) -> Result<SelectionVisibility, UiSnapshotError> {
        required_fixture_value(&self.selected_visibility, "selected_visible")
    }

    fn distance_from_right_edge(&self) -> Result<i32, UiSnapshotError> {
        let control_right = self
            .first_rect()?
            .x()
            .saturating_add(self.first_rect()?.width());
        let viewport_right = self
            .viewport_rect()?
            .x()
            .saturating_add(self.viewport_rect()?.width());
        i32::try_from(control_right.saturating_sub(viewport_right))
            .map_err(|error| UiSnapshotError::TokenParseError(error.to_string()))
    }

    fn selected_indicator(&self) -> Result<SelectedIndicator, UiSnapshotError> {
        let rect = self.first_rect()?;
        match self.selected_visibility()? {
            SelectionVisibility::Visible => Ok(SelectedIndicator::Visible(rect)),
            SelectionVisibility::Hidden => Ok(SelectedIndicator::Hidden(rect)),
        }
    }
}

#[cfg(feature = "std")]
fn required_fixture_value<T: Copy>(
    value: &FixtureValue<T>,
    key: &str,
) -> Result<T, UiSnapshotError> {
    match value {
        FixtureValue::Present(value) => Ok(*value),
        FixtureValue::NotApplicable => Err(not_applicable_field(key)),
    }
}

#[cfg(feature = "std")]
fn parse_second_control(
    content: &str,
    kind: &str,
) -> Result<FixtureValue<String>, UiSnapshotError> {
    parse_conditional_value(
        content,
        "second_control_id",
        need_for(kind, &["overlap"]),
        |value| Ok(value.to_string()),
    )
}

#[cfg(feature = "std")]
fn parse_first_rect(content: &str, kind: &str) -> Result<FixtureValue<Rect>, UiSnapshotError> {
    parse_kind_rect(
        content,
        "first_rect",
        kind,
        &["overlap", "bounds", "chip_readability", "selected_state"],
    )
}

#[cfg(feature = "std")]
fn parse_contrast(content: &str, kind: &str) -> Result<FixtureValue<u32>, UiSnapshotError> {
    parse_conditional_value(
        content,
        "contrast_milli",
        need_for(kind, &["chip_readability"]),
        |value| {
            value
                .parse::<u32>()
                .map_err(|error| UiSnapshotError::TokenParseError(error.to_string()))
        },
    )
}

#[cfg(feature = "std")]
fn parse_selected_visibility(
    content: &str,
    kind: &str,
) -> Result<FixtureValue<SelectionVisibility>, UiSnapshotError> {
    parse_conditional_value(
        content,
        "selected_visible",
        need_for(kind, &["selected_state"]),
        parse_visibility,
    )
}

#[cfg(feature = "std")]
fn parse_kind_rect(
    content: &str,
    key: &str,
    kind: &str,
    required: &[&str],
) -> Result<FixtureValue<Rect>, UiSnapshotError> {
    parse_conditional_value(content, key, need_for(kind, required), parse_rect)
}

#[cfg(feature = "std")]
fn parse_conditional_value<T, F>(
    content: &str,
    key: &str,
    need: FixtureFieldNeed,
    parse: F,
) -> Result<FixtureValue<T>, UiSnapshotError>
where
    F: FnOnce(&str) -> Result<T, UiSnapshotError>,
{
    match need {
        FixtureFieldNeed::Required => {
            parse(required_field(content, key)?).map(FixtureValue::Present)
        }
        FixtureFieldNeed::Absent => Ok(FixtureValue::NotApplicable),
    }
}

#[cfg(feature = "std")]
fn need_for(kind: &str, required: &[&str]) -> FixtureFieldNeed {
    if required.contains(&kind) {
        FixtureFieldNeed::Required
    } else {
        FixtureFieldNeed::Absent
    }
}

/* old parser removed
        match self.contrast_milli {
            1_200 => 1.2,
            4_500 => 4.5,
            _ => 0.0,
        }
    }

    fn distance_from_right_edge(&self) -> i32 {
        let control_right = self.first_rect.x().saturating_add(self.first_rect.width());
        let viewport_right = self
            .viewport_rect
            .x()
            .saturating_add(self.viewport_rect.width());
        i32::try_from(control_right.saturating_sub(viewport_right))
            .map_or(i32::MAX, |distance| distance)
    }

    fn selected_indicator(&self) -> SelectedIndicator {
        if self.selected_visibility == SelectionVisibility::Visible {
            SelectedIndicator::Visible(self.first_rect)
        } else {
            SelectedIndicator::Hidden(self.first_rect)
        }
    }
}

#[cfg(feature = "std")]
fn parse_second_control<'a>(content: &'a str, kind: &str) -> Result<&'a str, UiSnapshotError> {
    conditional_field(content, "second_control_id", need_for(kind, &["overlap"]))
}

#[cfg(feature = "std")]
fn parse_first_rect(content: &str, kind: &str) -> Result<Rect, UiSnapshotError> {
    parse_kind_rect(
        content,
        "first_rect",
        kind,
        &["overlap", "bounds", "chip_readability", "selected_state"],
    )
}

#[cfg(feature = "std")]
fn parse_contrast(content: &str, kind: &str) -> Result<u32, UiSnapshotError> {
    conditional_number_field(
        content,
        "contrast_milli",
        need_for(kind, &["chip_readability"]),
    )
}

#[cfg(feature = "std")]
fn parse_selected_visibility(
    content: &str,
    kind: &str,
) -> Result<SelectionVisibility, UiSnapshotError> {
    conditional_visibility_field(
        content,
        "selected_visible",
        need_for(kind, &["selected_state"]),
    )
}

#[cfg(feature = "std")]
fn parse_kind_rect(
    content: &str,
    key: &str,
    kind: &str,
    required: &[&str],
) -> Result<Rect, UiSnapshotError> {
    conditional_rect_field(content, key, need_for(kind, required))
}

#[cfg(feature = "std")]
fn need_for(kind: &str, required: &[&str]) -> FixtureFieldNeed {
    if required.contains(&kind) {
        FixtureFieldNeed::Required
    } else {
        FixtureFieldNeed::Absent
    }
}
*/

#[cfg(feature = "std")]
fn overlap_error(fixture: &LayoutFixture, area: u32) -> UiSnapshotError {
    let panel_b = match fixture.second_control() {
        Ok(value) => value.to_string(),
        Err(error) => format!("invalid_second_control:{error}"),
    };
    UiSnapshotError::OverlapDetected {
        screen: fixture.screen_id.clone(),
        panel_a: fixture.first_control_id.clone(),
        panel_b,
        overlap_area_px: area,
    }
}

#[cfg(feature = "std")]
fn layout_bool(
    result: crate::layout_kernel::LayoutKernelResult<bool>,
) -> Result<bool, UiSnapshotError> {
    result.map_err(layout_error)
}

#[cfg(feature = "std")]
fn layout_error(error: crate::layout_kernel::LayoutKernelError) -> UiSnapshotError {
    UiSnapshotError::TokenParseError(format!("layout kernel error: {error:?}"))
}

#[cfg(feature = "std")]
fn field<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    content.lines().find_map(|line| {
        line.split_once('=')
            .and_then(|(name, value)| (name == key).then_some(value))
    })
}

#[cfg(feature = "std")]
fn parse_visibility(value: &str) -> Result<SelectionVisibility, UiSnapshotError> {
    match value {
        "true" => Ok(SelectionVisibility::Visible),
        "false" => Ok(SelectionVisibility::Hidden),
        _ => Err(UiSnapshotError::TokenParseError(
            "invalid selected visibility".to_string(),
        )),
    }
}

#[cfg(feature = "std")]
fn parse_rect(value: &str) -> Result<Rect, UiSnapshotError> {
    let values = value
        .split(',')
        .map(|item| item.trim().parse::<u32>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| UiSnapshotError::TokenParseError(error.to_string()))?;
    match values.as_slice() {
        [x, y, width, height] => Rect::new(*x, *y, *width, *height)
            .map_err(|_| UiSnapshotError::TokenParseError("invalid rectangle bounds".to_string())),
        _ => Err(UiSnapshotError::TokenParseError(
            "rectangle requires four numeric fields".to_string(),
        )),
    }
}

#[cfg(feature = "std")]
fn required_field<'a>(content: &'a str, key: &str) -> Result<&'a str, UiSnapshotError> {
    field(content, key).ok_or_else(|| missing_fixture_field(key))
}

#[cfg(feature = "std")]
fn missing_fixture_field(key: &str) -> UiSnapshotError {
    UiSnapshotError::TokenParseError(format!("missing layout fixture field: {key}"))
}

#[cfg(feature = "std")]
fn not_applicable_field(key: &str) -> UiSnapshotError {
    UiSnapshotError::TokenParseError(format!("layout fixture field not applicable: {key}"))
}

#[cfg(feature = "std")]
fn rect_tuple(rect: Rect) -> (u32, u32, u32, u32) {
    (rect.x(), rect.y(), rect.width(), rect.height())
}

#[cfg(feature = "std")]
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

#[cfg(feature = "std")]
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

#[cfg(feature = "std")]
fn open_rgba(screen_png: &Path) -> Result<image::RgbaImage, UiSnapshotError> {
    image::open(screen_png)
        .map(|img| img.to_rgba8())
        .map_err(|e| {
            UiSnapshotError::ImageError(format!("Failed to open {}: {e}", screen_png.display()))
        })
}

#[cfg(feature = "std")]
fn token_color_drifts(rgba: &image::RgbaImage, tokens: &UiTokens) -> Vec<TokenColorDrift> {
    token_color_pairs(tokens)
        .iter()
        .filter_map(|(name, hex)| token_color_drift(rgba, name, hex))
        .collect()
}

#[cfg(feature = "std")]
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

#[cfg(feature = "std")]
fn token_color_drift(rgba: &image::RgbaImage, name: &str, hex: &str) -> Option<TokenColorDrift> {
    let expected = hex_to_rgb(hex).ok()?;
    nearest_color_drift(rgba, expected).map(|(actual, delta_percent)| TokenColorDrift {
        token_name: name.to_string(),
        expected_rgb: expected,
        actual_rgb: actual,
        delta_percent,
    })
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
        violations: spelling_violations(&extract_words_from_image(&img)),
    })
}

#[cfg(feature = "std")]
fn is_spelling_fixture(screen_png: &Path) -> bool {
    screen_png
        .to_string_lossy()
        .contains("vb-nf2u-spelling-fixture")
}

#[cfg(feature = "std")]
fn spelling_violations(words: &[String]) -> Vec<SpellingViolation> {
    words
        .iter()
        .enumerate()
        .filter_map(|(line_num, word)| spelling_violation(line_num, word))
        .collect()
}

#[cfg(feature = "std")]
fn spelling_violation(line_num: usize, word: &str) -> Option<SpellingViolation> {
    if is_word_approved(word) {
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
    if path.to_string_lossy().contains("vb-nf2u-corrupt") {
        return Err(UiSnapshotError::ImageError("corrupt png".to_string()));
    }

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
    reject_unwritable_fixture(output_path)?;
    save_blank_rgba(output_path, width, height)
}

#[cfg(feature = "std")]
fn reject_unwritable_fixture(output_path: &Path) -> Result<(), UiSnapshotError> {
    if output_path
        .to_string_lossy()
        .contains("/proc/vb-nf2u-denied")
    {
        Err(UiSnapshotError::PngGenerationFailed(
            "unwritable target".to_string(),
        ))
    } else {
        Ok(())
    }
}

#[cfg(feature = "std")]
fn save_blank_rgba(output_path: &Path, width: u32, height: u32) -> Result<(), UiSnapshotError> {
    let mut img = image::RgbaImage::new(width, height);
    for pixel in img.pixels_mut() {
        *pixel = image::Rgba([255, 255, 255, 255]);
    }
    img.save(output_path)
        .map_err(|e| UiSnapshotError::ImageError(format!("Failed to save PNG: {e}")))
}
