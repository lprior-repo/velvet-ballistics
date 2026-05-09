#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        missing_screens: Vec<String>,
    },
    TokenParseError(String),
    ImageError(String),
    IoError(String),
}

impl std::fmt::Display for UiSnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
            Self::ReportIncomplete { missing_screens } => {
                write!(f, "Report incomplete, missing: {:?}", missing_screens)
            }
            Self::TokenParseError(msg) => write!(f, "Token parse error: {msg}"),
            Self::ImageError(msg) => write!(f, "Image error: {msg}"),
            Self::IoError(msg) => write!(f, "IO error: {msg}"),
        }
    }
}

impl std::error::Error for UiSnapshotError {}

impl From<std::io::Error> for UiSnapshotError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.to_string())
    }
}

impl From<png::EncodingError> for UiSnapshotError {
    fn from(e: png::EncodingError) -> Self {
        Self::ImageError(e.to_string())
    }
}

impl From<image::ImageError> for UiSnapshotError {
    fn from(e: image::ImageError) -> Self {
        Self::ImageError(e.to_string())
    }
}
