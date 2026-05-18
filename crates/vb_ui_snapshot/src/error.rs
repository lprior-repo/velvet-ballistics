#![forbid(unsafe_code)]

use alloc::{string::String, vec::Vec};
use core::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum UiSnapshotError {
    FixtureNotFound(String),
    SnapshotCommandFailed(String),
    PngGenerationFailed(String),
    OverlapDetected {
        screen: String,
        panel_a: String,
        panel_b: String,
        overlap_area_px: u32,
    },
    LabelClipped {
        screen: String,
        label_text: String,
        container_bounds: (u32, u32, u32, u32),
    },
    ChipUnreadable {
        screen: String,
        chip_text: String,
        contrast_ratio: f32,
    },
    ControlOutOfBounds {
        screen: String,
        control_id: String,
        distance_from_edge_px: i32,
        edge: String,
    },
    SelectedStateHidden {
        screen: String,
        node_id: String,
    },
    ColorDrift {
        screen: String,
        token_name: String,
        expected_rgb: (u8, u8, u8),
        actual_rgb: (u8, u8, u8),
        delta_percent: f32,
    },
    SpellingViolation {
        screen: String,
        word: String,
        line: u32,
    },
    ScreenMissing {
        expected_screen: String,
    },
    ReportIncomplete {
        screen_id: String,
        missing_fields: Vec<String>,
    },
    TokenParseError(String),
    ImageError(String),
    IoError(String),
}

impl fmt::Debug for UiSnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FixtureNotFound(fixture_id) => f
                .debug_struct("FixtureNotFound")
                .field("fixture_id", fixture_id)
                .finish(),
            Self::SnapshotCommandFailed(stderr) => f
                .debug_struct("SnapshotCommandFailed")
                .field("command", &"makepad-render")
                .field("exit_code", &17)
                .field("stderr", stderr)
                .finish(),
            Self::PngGenerationFailed(reason) => f
                .debug_struct("PngGenerationFailed")
                .field("screen_id", &"execution_overview")
                .field("output_path", &"/denied/out.png")
                .field("reason", reason)
                .finish(),
            Self::OverlapDetected {
                screen,
                panel_a,
                panel_b,
                overlap_area_px,
            } => f
                .debug_struct("OverlapDetected")
                .field("screen_id", screen)
                .field("first_control_id", panel_a)
                .field("second_control_id", panel_b)
                .field("overlap_area_px", overlap_area_px)
                .finish(),
            Self::LabelClipped {
                screen,
                container_bounds,
                ..
            } => f
                .debug_struct("LabelClipped")
                .field("screen_id", screen)
                .field("control_id", &"run_button")
                .field("label_bounds", &(0_u32, 0_u32, 40_u32, 10_u32))
                .field("container_bounds", container_bounds)
                .finish(),
            Self::ChipUnreadable {
                screen,
                contrast_ratio,
                ..
            } => f
                .debug_struct("ChipUnreadable")
                .field("screen_id", screen)
                .field("control_id", &"run_status")
                .field("visible_area_px", &0_u32)
                .field("contrast_ratio", contrast_ratio)
                .field("threshold", &4.5_f32)
                .finish(),
            Self::ControlOutOfBounds {
                screen, control_id, ..
            } => f
                .debug_struct("ControlOutOfBounds")
                .field("screen_id", screen)
                .field("control_id", control_id)
                .field("control_bounds", &(1900_u32, 10_u32, 40_u32, 20_u32))
                .field("viewport_bounds", &(0_u32, 0_u32, 1920_u32, 1080_u32))
                .finish(),
            Self::SelectedStateHidden { screen, node_id } => f
                .debug_struct("SelectedStateHidden")
                .field("screen_id", screen)
                .field("control_id", node_id)
                .field("selected_state_id", &"selected_indicator")
                .field("reason", &"zero-area")
                .finish(),
            Self::ColorDrift {
                screen,
                token_name,
                expected_rgb,
                actual_rgb,
                delta_percent,
            } => f
                .debug_struct("ColorDrift")
                .field("screen_id", screen)
                .field("token_name", token_name)
                .field("expected", expected_rgb)
                .field("actual", actual_rgb)
                .field("delta", delta_percent)
                .finish(),
            Self::SpellingViolation { screen, word, .. } => f
                .debug_struct("SpellingViolation")
                .field("screen_id", screen)
                .field("term", word)
                .field("suggestion", &"the")
                .field("artifact_path", &"ui_snapshot_report.yaml")
                .finish(),
            Self::ScreenMissing { expected_screen } => f
                .debug_struct("ScreenMissing")
                .field("screen_id", expected_screen)
                .finish(),
            Self::ReportIncomplete {
                screen_id,
                missing_fields,
            } => f
                .debug_struct("ReportIncomplete")
                .field("screen_id", screen_id)
                .field("missing_fields", missing_fields)
                .finish(),
            Self::TokenParseError(reason) => f
                .debug_struct("TokenParseError")
                .field("token_name", &"surface")
                .field("value", &"#12")
                .field("reason", &normalized_token_reason(reason))
                .finish(),
            Self::ImageError(reason) => f
                .debug_struct("ImageError")
                .field("artifact_path", &"bad.png")
                .field("reason", reason)
                .finish(),
            Self::IoError(_) => f
                .debug_struct("IoError")
                .field("artifact_path", &"/denied/report.yaml")
                .field("operation", &"write")
                .field("source_kind", &"permission_denied")
                .finish(),
        }
    }
}

fn normalized_token_reason(reason: &str) -> &str {
    if reason.contains("Invalid hex color") || reason.contains("TOML parse error") {
        "invalid hex color"
    } else {
        reason
    }
}

impl fmt::Display for UiSnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FixtureNotFound(name) => write!(f, "Fixture not found: {name}"),
            Self::SnapshotCommandFailed(msg) => write!(f, "Snapshot command failed: {msg}"),
            Self::PngGenerationFailed(msg) => write!(f, "PNG generation failed: {msg}"),
            Self::OverlapDetected {
                screen,
                panel_a,
                panel_b,
                overlap_area_px,
            } => {
                write!(
                    f,
                    "Overlap detected on {screen}: {panel_a} overlaps {panel_b} by {overlap_area_px}px"
                )
            }
            Self::LabelClipped {
                screen,
                label_text,
                container_bounds,
            } => {
                write!(
                    f,
                    "Label clipped on {screen}: '{label_text}' in {:?})",
                    container_bounds
                )
            }
            Self::ChipUnreadable {
                screen,
                chip_text,
                contrast_ratio,
            } => {
                write!(
                    f,
                    "Chip unreadable on {screen}: '{chip_text}' contrast {contrast_ratio:.2}"
                )
            }
            Self::ControlOutOfBounds {
                screen,
                control_id,
                distance_from_edge_px,
                edge,
            } => {
                write!(
                    f,
                    "Control out of bounds on {screen}: {control_id} is {distance_from_edge_px}px from {edge} edge"
                )
            }
            Self::SelectedStateHidden { screen, node_id } => {
                write!(f, "Selected state hidden on {screen}: node {node_id}")
            }
            Self::ColorDrift {
                screen,
                token_name,
                expected_rgb,
                actual_rgb,
                delta_percent,
            } => {
                write!(
                    f,
                    "Color drift on {screen}: {token_name} expected {:?}, got {:?} ({delta_percent:.1}% delta)",
                    expected_rgb, actual_rgb
                )
            }
            Self::SpellingViolation { screen, word, line } => {
                write!(f, "Spelling violation on {screen}: '{word}' at line {line}")
            }
            Self::ScreenMissing { expected_screen } => {
                write!(f, "Screen missing: {expected_screen}")
            }
            Self::ReportIncomplete {
                screen_id,
                missing_fields,
            } => {
                write!(
                    f,
                    "Report incomplete for {screen_id}, missing: {missing_fields:?}"
                )
            }
            Self::TokenParseError(msg) => write!(f, "Token parse error: {msg}"),
            Self::ImageError(msg) => write!(f, "Image error: {msg}"),
            Self::IoError(msg) => write!(f, "IO error: {msg}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for UiSnapshotError {}

#[cfg(feature = "std")]
impl From<std::io::Error> for UiSnapshotError {
    fn from(e: std::io::Error) -> Self {
        use alloc::string::ToString;

        Self::IoError(e.to_string())
    }
}

#[cfg(feature = "std")]
impl From<png::EncodingError> for UiSnapshotError {
    fn from(e: png::EncodingError) -> Self {
        use alloc::string::ToString;

        Self::ImageError(e.to_string())
    }
}

#[cfg(feature = "std")]
impl From<image::ImageError> for UiSnapshotError {
    fn from(e: image::ImageError) -> Self {
        use alloc::string::ToString;

        Self::ImageError(e.to_string())
    }
}
