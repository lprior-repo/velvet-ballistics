#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSnapshotReport {
    pub status: String,
    pub screens: Vec<ScreenResult>,
    pub total_screens: usize,
    pub passed_screens: usize,
    pub failed_screens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenResult {
    pub screen_name: String,
    pub png_path: Option<String>,
    pub checks: Vec<CheckResult>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub kind: CheckKind,
    pub passed: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum CheckKind {
    Overlap,
    Clipping,
    ChipReadability,
    Bounds,
    SelectedState,
    ColorDrift,
    Spelling,
    PngValidity,
}

impl std::fmt::Display for CheckKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Overlap => write!(f, "overlap_check"),
            Self::Clipping => write!(f, "clipping_check"),
            Self::ChipReadability => write!(f, "chip_readability_check"),
            Self::Bounds => write!(f, "bounds_check"),
            Self::SelectedState => write!(f, "selected_state_check"),
            Self::ColorDrift => write!(f, "color_drift_check"),
            Self::Spelling => write!(f, "spelling_check"),
            Self::PngValidity => write!(f, "png_validity_check"),
        }
    }
}

impl UiSnapshotReport {
    pub fn new() -> Self {
        Self {
            status: "pass".to_string(),
            screens: Vec::new(),
            total_screens: 0,
            passed_screens: 0,
            failed_screens: 0,
        }
    }

    pub fn add_screen(&mut self, result: ScreenResult) {
        if !result.passed {
            self.status = "fail".to_string();
        }
        self.screens.push(result);
    }

    pub fn finalize(&mut self) {
        self.total_screens = self.screens.len();
        self.passed_screens = self.screens.iter().filter(|s| s.passed).count();
        self.failed_screens = self.screens.iter().filter(|s| !s.passed).count();
    }

    pub fn to_yaml(&self) -> anyhow::Result<String> {
        serde_yaml::to_string(self).map_err(|e| anyhow::anyhow!("YAML serialization failed: {e}"))
    }
}

impl Default for UiSnapshotReport {
    fn default() -> Self {
        Self::new()
    }
}

pub fn make_screen_result(screen_name: &str, checks: Vec<CheckResult>) -> ScreenResult {
    let passed = checks.iter().all(|c| c.passed);
    ScreenResult {
        screen_name: screen_name.to_string(),
        png_path: None,
        checks,
        passed,
    }
}

pub fn make_pass_result(kind: CheckKind) -> CheckResult {
    CheckResult {
        kind,
        passed: true,
        detail: None,
    }
}

pub fn make_fail_result(kind: CheckKind, detail: &str) -> CheckResult {
    CheckResult {
        kind,
        passed: false,
        detail: Some(detail.to_string()),
    }
}
