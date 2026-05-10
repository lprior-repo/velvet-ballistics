#![forbid(unsafe_code)]

use alloc::{string::String, vec::Vec};
use core::fmt;

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
            Self::ReportIncomplete { missing_screens } => {
                write!(f, "Report incomplete, missing: {:?}", missing_screens)
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

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::UiSnapshotError;
    use alloc::format;

    #[test]
    fn display_fixture_not_found() {
        let err = UiSnapshotError::FixtureNotFound("my_fixture".into());
        let display = format!("{err}");
        assert!(display.contains("my_fixture"));
        assert!(display.contains("Fixture not found"));
    }

    #[test]
    fn display_snapshot_command_failed() {
        let err = UiSnapshotError::SnapshotCommandFailed("timeout".into());
        let display = format!("{err}");
        assert!(display.contains("timeout"));
        assert!(display.contains("Snapshot command failed"));
    }

    #[test]
    fn display_png_generation_failed() {
        let err = UiSnapshotError::PngGenerationFailed("encode error".into());
        let display = format!("{err}");
        assert!(display.contains("encode error"));
        assert!(display.contains("PNG generation failed"));
    }

    #[test]
    fn display_overlap_detected() {
        let err = UiSnapshotError::OverlapDetected {
            screen: "screen_a".into(),
            panel_a: "panel1".into(),
            panel_b: "panel2".into(),
            overlap_area_px: 42,
        };
        let display = format!("{err}");
        assert!(display.contains("screen_a"));
        assert!(display.contains("panel1"));
        assert!(display.contains("panel2"));
        assert!(display.contains("42"));
    }

    #[test]
    fn display_label_clipped() {
        let err = UiSnapshotError::LabelClipped {
            screen: "scr".into(),
            label_text: "my label".into(),
            container_bounds: (10, 20, 30, 40),
        };
        let display = format!("{err}");
        assert!(display.contains("scr"));
        assert!(display.contains("my label"));
    }

    #[test]
    fn display_chip_unreadable() {
        let err = UiSnapshotError::ChipUnreadable {
            screen: "scr".into(),
            chip_text: "chip".into(),
            contrast_ratio: 1.5,
        };
        let display = format!("{err}");
        assert!(display.contains("chip"));
        assert!(display.contains("1.50"));
    }

    #[test]
    fn display_control_out_of_bounds() {
        let err = UiSnapshotError::ControlOutOfBounds {
            screen: "scr".into(),
            control_id: "btn_ok".into(),
            distance_from_edge_px: 15,
            edge: "right".into(),
        };
        let display = format!("{err}");
        assert!(display.contains("btn_ok"));
        assert!(display.contains("15"));
        assert!(display.contains("right"));
    }

    #[test]
    fn display_selected_state_hidden() {
        let err = UiSnapshotError::SelectedStateHidden {
            screen: "scr".into(),
            node_id: "node_42".into(),
        };
        let display = format!("{err}");
        assert!(display.contains("node_42"));
    }

    #[test]
    fn display_color_drift() {
        let err = UiSnapshotError::ColorDrift {
            screen: "scr".into(),
            token_name: "surface".into(),
            expected_rgb: (255, 0, 0),
            actual_rgb: (254, 1, 1),
            delta_percent: 0.4,
        };
        let display = format!("{err}");
        assert!(display.contains("surface"));
        assert!(display.contains("0.4"));
    }

    #[test]
    fn display_spelling_violation() {
        let err = UiSnapshotError::SpellingViolation {
            screen: "scr".into(),
            word: "teh".into(),
            line: 10,
        };
        let display = format!("{err}");
        assert!(display.contains("teh"));
        assert!(display.contains("10"));
    }

    #[test]
    fn display_screen_missing() {
        let err = UiSnapshotError::ScreenMissing {
            expected_screen: "ExecutionOverview".into(),
        };
        let display = format!("{err}");
        assert!(display.contains("ExecutionOverview"));
        assert!(display.contains("Screen missing"));
    }

    #[test]
    fn display_report_incomplete() {
        let err = UiSnapshotError::ReportIncomplete {
            missing_screens: vec!["scr1".into(), "scr2".into()],
        };
        let display = format!("{err}");
        assert!(display.contains("scr1"));
        assert!(display.contains("scr2"));
    }

    #[test]
    fn display_token_parse_error() {
        let err = UiSnapshotError::TokenParseError("bad hex".into());
        let display = format!("{err}");
        assert!(display.contains("bad hex"));
        assert!(display.contains("Token parse error"));
    }

    #[test]
    fn display_image_error() {
        let err = UiSnapshotError::ImageError("cannot decode".into());
        let display = format!("{err}");
        assert!(display.contains("cannot decode"));
        assert!(display.contains("Image error"));
    }

    #[test]
    fn display_io_error() {
        let err = UiSnapshotError::IoError("file not found".into());
        let display = format!("{err}");
        assert!(display.contains("file not found"));
        assert!(display.contains("IO error"));
    }

    #[test]
    fn from_std_io_error() {
        use std::io;
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file gone");
        let snapshot_err: UiSnapshotError = io_err.into();
        let display = format!("{snapshot_err}");
        assert!(display.contains("file gone"));
    }

    #[test]
    fn from_png_encoding_error() {
        use std::io;
        let io_err = io::Error::new(io::ErrorKind::Other, "png encoder failed");
        let png_err = png::EncodingError::IoError(io_err);
        let snapshot_err: UiSnapshotError = png_err.into();
        let display = format!("{snapshot_err}");
        assert!(display.contains("Image error"));
    }

    #[test]
    fn from_image_error_compiles() {
        // Verify the From<image::ImageError> impl compiles
        fn assert_implements_from<E: From<image::ImageError>>() {}
        assert_implements_from::<UiSnapshotError>();
    }

    #[test]
    fn error_is_debug_and_clone() {
        let err = UiSnapshotError::FixtureNotFound("test".into());
        let cloned = err.clone();
        assert_eq!(format!("{err:?}"), format!("{cloned:?}"));
    }
}
