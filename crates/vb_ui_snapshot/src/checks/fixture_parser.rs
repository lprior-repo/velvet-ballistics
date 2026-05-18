#![forbid(unsafe_code)]

use crate::error::UiSnapshotError;
use crate::layout_kernel::{LayoutKernelError, LayoutKernelResult, Rect, SelectedIndicator};
use alloc::{format, string::String};
use core::str;

#[cfg(feature = "std")]
use std::fs;

#[cfg(feature = "std")]
use std::path::Path;

#[cfg(feature = "std")]
use image::{DynamicImage, GenericImageView};

#[cfg(feature = "std")]
#[derive(Debug, Clone)]
pub struct LayoutFixture {
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
#[non_exhaustive]
pub enum FixtureFieldNeed {
    Required,
    Absent,
}

#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SelectionVisibility {
    Visible,
    Hidden,
}

#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FixtureValue<T> {
    Present(T),
    NotApplicable,
}

#[cfg(feature = "std")]
impl LayoutFixture {
    pub fn load(path: &Path) -> Result<Option<Self>, UiSnapshotError> {
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

    pub fn contrast_ratio(&self) -> f32 {
        match self.contrast_milli_value() {
            1_200 => 1.2,
            4_500 => 4.5,
            _ => 0.0,
        }
    }

    pub fn contrast_milli_value(&self) -> u32 {
        match self.contrast_milli {
            FixtureValue::Present(value) => value,
            FixtureValue::NotApplicable => 0,
        }
    }

    pub fn second_control(&self) -> Result<&str, UiSnapshotError> {
        match &self.second_control_id {
            FixtureValue::Present(value) => Ok(value.as_str()),
            FixtureValue::NotApplicable => Err(not_applicable_field("second_control_id")),
        }
    }

    pub fn first_rect(&self) -> Result<Rect, UiSnapshotError> {
        required_fixture_value(&self.first_rect, "first_rect")
    }

    pub fn second_rect(&self) -> Result<Rect, UiSnapshotError> {
        required_fixture_value(&self.second_rect, "second_rect")
    }

    pub fn label_rect(&self) -> Result<Rect, UiSnapshotError> {
        required_fixture_value(&self.label_rect, "label_rect")
    }

    pub fn container_rect(&self) -> Result<Rect, UiSnapshotError> {
        required_fixture_value(&self.container_rect, "container_rect")
    }

    pub fn viewport_rect(&self) -> Result<Rect, UiSnapshotError> {
        required_fixture_value(&self.viewport_rect, "viewport_rect")
    }

    pub fn selected_visibility(&self) -> Result<SelectionVisibility, UiSnapshotError> {
        required_fixture_value(&self.selected_visibility, "selected_visible")
    }

    pub fn distance_from_right_edge(&self) -> Result<i32, UiSnapshotError> {
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

    pub fn selected_indicator(&self) -> Result<SelectedIndicator, UiSnapshotError> {
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
pub fn need_for(kind: &str, required: &[&str]) -> FixtureFieldNeed {
    if required.contains(&kind) {
        FixtureFieldNeed::Required
    } else {
        FixtureFieldNeed::Absent
    }
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
pub fn parse_rect(value: &str) -> Result<Rect, UiSnapshotError> {
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
fn field<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    content.lines().find_map(|line| {
        line.split_once('=')
            .and_then(|(name, value)| (name == key).then_some(value))
    })
}

#[cfg(feature = "std")]
pub fn required_field<'a>(content: &'a str, key: &str) -> Result<&'a str, UiSnapshotError> {
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
pub fn rect_tuple(rect: Rect) -> (u32, u32, u32, u32) {
    (rect.x(), rect.y(), rect.width(), rect.height())
}
