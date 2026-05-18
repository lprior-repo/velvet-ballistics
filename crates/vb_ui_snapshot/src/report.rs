#![forbid(unsafe_code)]

#[cfg(feature = "std")]
use alloc::borrow::Cow;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use serde::{Deserialize, Serialize};

use crate::{REQUIRED_FIXTURES, UiSnapshotError};

#[cfg(feature = "std")]
use saphyr::{Mapping, Scalar, Yaml, YamlEmitter};

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
#[non_exhaustive]
pub enum CheckKind {
    Overlap,
    Clipping,
    ChipReadability,
    Bounds,
    SelectedState,
    Redaction,
    ColorDrift,
    Spelling,
    PngValidity,
}

impl fmt::Display for CheckKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overlap => write!(f, "overlap_check"),
            Self::Clipping => write!(f, "clipping_check"),
            Self::ChipReadability => write!(f, "chip_readability_check"),
            Self::Bounds => write!(f, "bounds_check"),
            Self::SelectedState => write!(f, "selected_state_check"),
            Self::Redaction => write!(f, "redaction_check"),
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

    #[cfg(feature = "std")]
    pub fn to_yaml(&self) -> anyhow::Result<String> {
        let doc = report_to_yaml(self)?;
        let mut output = String::new();
        let mut emitter = YamlEmitter::new(&mut output);
        emitter
            .dump(&doc)
            .map_err(|e| anyhow::anyhow!("Saphyr YAML emission failed: {e}"))?;
        Ok(output)
    }
}

#[cfg(feature = "std")]
fn report_to_yaml(report: &UiSnapshotReport) -> anyhow::Result<Yaml<'static>> {
    let mut mapping = Mapping::new();
    mapping.insert(yaml_key("status"), yaml_string(&report.status));
    mapping.insert(yaml_key("screens"), yaml_screens(&report.screens)?);
    mapping.insert(yaml_key("total_screens"), yaml_usize(report.total_screens)?);
    mapping.insert(
        yaml_key("passed_screens"),
        yaml_usize(report.passed_screens)?,
    );
    mapping.insert(
        yaml_key("failed_screens"),
        yaml_usize(report.failed_screens)?,
    );
    Ok(Yaml::Mapping(mapping))
}

#[cfg(feature = "std")]
fn yaml_screens(screens: &[ScreenResult]) -> anyhow::Result<Yaml<'static>> {
    screens
        .iter()
        .map(screen_to_yaml)
        .collect::<anyhow::Result<Vec<_>>>()
        .map(Yaml::Sequence)
}

#[cfg(feature = "std")]
fn screen_to_yaml(screen: &ScreenResult) -> anyhow::Result<Yaml<'static>> {
    let mut mapping = Mapping::new();
    mapping.insert(yaml_key("screen_name"), yaml_string(&screen.screen_name));
    mapping.insert(yaml_key("png_path"), yaml_option_string(&screen.png_path));
    mapping.insert(yaml_key("checks"), yaml_checks(&screen.checks)?);
    mapping.insert(yaml_key("passed"), yaml_bool(screen.passed));
    Ok(Yaml::Mapping(mapping))
}

#[cfg(feature = "std")]
fn yaml_checks(checks: &[CheckResult]) -> anyhow::Result<Yaml<'static>> {
    checks
        .iter()
        .map(check_to_yaml)
        .collect::<anyhow::Result<Vec<_>>>()
        .map(Yaml::Sequence)
}

#[cfg(feature = "std")]
fn check_to_yaml(check: &CheckResult) -> anyhow::Result<Yaml<'static>> {
    let mut mapping = Mapping::new();
    mapping.insert(yaml_key("kind"), yaml_string(check_kind_name(check.kind)));
    mapping.insert(yaml_key("passed"), yaml_bool(check.passed));
    mapping.insert(yaml_key("detail"), yaml_option_string(&check.detail));
    Ok(Yaml::Mapping(mapping))
}

#[cfg(feature = "std")]
fn check_kind_name(kind: CheckKind) -> &'static str {
    match kind {
        CheckKind::Overlap => "Overlap",
        CheckKind::Clipping => "Clipping",
        CheckKind::ChipReadability => "ChipReadability",
        CheckKind::Bounds => "Bounds",
        CheckKind::SelectedState => "SelectedState",
        CheckKind::Redaction => "Redaction",
        CheckKind::ColorDrift => "ColorDrift",
        CheckKind::Spelling => "Spelling",
        CheckKind::PngValidity => "PngValidity",
    }
}

#[cfg(feature = "std")]
fn yaml_key(key: &'static str) -> Yaml<'static> {
    yaml_borrowed_string(key)
}

#[cfg(feature = "std")]
fn yaml_option_string(value: &Option<String>) -> Yaml<'static> {
    value
        .as_ref()
        .map_or_else(yaml_null, |text| yaml_string(text))
}

#[cfg(feature = "std")]
fn yaml_string(value: &str) -> Yaml<'static> {
    Yaml::Value(Scalar::String(Cow::Owned(value.to_string())))
}

#[cfg(feature = "std")]
fn yaml_borrowed_string(value: &'static str) -> Yaml<'static> {
    Yaml::Value(Scalar::String(Cow::Borrowed(value)))
}

#[cfg(feature = "std")]
fn yaml_bool(value: bool) -> Yaml<'static> {
    Yaml::Value(Scalar::Boolean(value))
}

#[cfg(feature = "std")]
fn yaml_null() -> Yaml<'static> {
    Yaml::Value(Scalar::Null)
}

#[cfg(feature = "std")]
fn yaml_usize(value: usize) -> anyhow::Result<Yaml<'static>> {
    i64::try_from(value)
        .map(|integer| Yaml::Value(Scalar::Integer(integer)))
        .map_err(|e| anyhow::anyhow!("snapshot report count exceeded YAML integer range: {e}"))
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

pub fn validate_required_screens(screens: &[&str]) -> Result<(), UiSnapshotError> {
    for required in REQUIRED_FIXTURES.iter().rev() {
        if !screens.iter().any(|screen| screen == required) {
            return Err(UiSnapshotError::ScreenMissing {
                expected_screen: (*required).to_string(),
            });
        }
    }
    Ok(())
}

pub fn validate_report_fields(
    screen_id: &str,
    digest: Option<&str>,
    checks: Option<&[CheckResult]>,
) -> Result<(), UiSnapshotError> {
    let mut missing_fields = Vec::new();
    if digest.is_none() {
        missing_fields.push("digest".to_string());
    }
    if checks.is_none() {
        missing_fields.push("checks".to_string());
    }
    if missing_fields.is_empty() {
        Ok(())
    } else {
        Err(UiSnapshotError::ReportIncomplete {
            screen_id: screen_id.to_string(),
            missing_fields,
        })
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::{CheckKind, UiSnapshotReport, make_fail_result};
    use saphyr::LoadableYamlNode;

    #[test]
    fn saphyr_yaml_emits_parseable_report() -> anyhow::Result<()> {
        let mut report = UiSnapshotReport::new();
        report.add_screen(super::make_screen_result(
            "execution_overview",
            vec![make_fail_result(CheckKind::ColorDrift, "token drift")],
        ));
        report.finalize();

        let yaml = report.to_yaml()?;
        let docs = saphyr::Yaml::load_from_str(&yaml)?;
        let status = docs
            .first()
            .and_then(|doc| doc.as_mapping_get("status"))
            .and_then(saphyr::Yaml::as_str);

        if status != Some("fail") {
            anyhow::bail!("expected status=fail, got {status:?}");
        }
        Ok(())
    }
}
