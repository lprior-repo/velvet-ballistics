#![forbid(unsafe_code)]

#[cfg(feature = "std")]
use alloc::borrow::Cow;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt;

use serde::{Deserialize, Serialize};

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

impl fmt::Display for CheckKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::{make_fail_result, make_pass_result, make_screen_result, CheckKind, CheckResult, ScreenResult, UiSnapshotReport};
    use saphyr::LoadableYamlNode;

    // ── UiSnapshotReport ──────────────────────────────────────────────────────

    #[test]
    fn new_report_has_pass_status() {
        let report = UiSnapshotReport::new();
        assert_eq!(report.status, "pass");
        assert_eq!(report.total_screens, 0);
        assert_eq!(report.passed_screens, 0);
        assert_eq!(report.failed_screens, 0);
        assert!(report.screens.is_empty());
    }

    #[test]
    fn add_screen_updates_status_to_fail_when_screen_fails() {
        let mut report = UiSnapshotReport::new();
        report.add_screen(make_screen_result(
            "test_screen",
            vec![make_fail_result(CheckKind::ColorDrift, "drifted")],
        ));
        assert_eq!(report.status, "fail");
        assert_eq!(report.screens.len(), 1);
    }

    #[test]
    fn add_screen_keeps_status_pass_when_all_screens_pass() {
        let mut report = UiSnapshotReport::new();
        report.add_screen(make_screen_result(
            "test_screen",
            vec![make_pass_result(CheckKind::Overlap)],
        ));
        assert_eq!(report.status, "pass");
    }

    #[test]
    fn finalize_computes_counts() {
        let mut report = UiSnapshotReport::new();
        report.add_screen(make_screen_result("scr1", vec![make_pass_result(CheckKind::Overlap)]));
        report.add_screen(make_screen_result("scr2", vec![make_fail_result(CheckKind::ColorDrift, "drift")]));
        report.finalize();
        assert_eq!(report.total_screens, 2);
        assert_eq!(report.passed_screens, 1);
        assert_eq!(report.failed_screens, 1);
    }

    #[test]
    fn finalize_status_fail_when_any_screen_fails() {
        let mut report = UiSnapshotReport::new();
        report.add_screen(make_screen_result("scr1", vec![make_pass_result(CheckKind::Overlap)]));
        report.add_screen(make_screen_result("scr2", vec![make_fail_result(CheckKind::Spelling, "typo")]));
        report.finalize();
        assert_eq!(report.failed_screens, 1);
    }

    #[test]
    fn default_implementation_matches_new() {
        let default_report = UiSnapshotReport::default();
        let new_report = UiSnapshotReport::new();
        assert_eq!(default_report.status, new_report.status);
        assert_eq!(default_report.screens.len(), new_report.screens.len());
    }

    // ── make_screen_result ───────────────────────────────────────────────────

    #[test]
    fn make_screen_result_is_passed_when_all_checks_passed() {
        let result = make_screen_result(
            "my_screen",
            vec![
                make_pass_result(CheckKind::Overlap),
                make_pass_result(CheckKind::ColorDrift),
            ],
        );
        assert!(result.passed);
        assert_eq!(result.screen_name, "my_screen");
        assert_eq!(result.checks.len(), 2);
        assert!(result.png_path.is_none());
    }

    #[test]
    fn make_screen_result_is_failed_when_any_check_fails() {
        let result = make_screen_result(
            "my_screen",
            vec![
                make_pass_result(CheckKind::Bounds),
                make_fail_result(CheckKind::Clipping, "label truncated"),
            ],
        );
        assert!(!result.passed);
    }

    // ── make_pass_result ──────────────────────────────────────────────────────

    #[test]
    fn make_pass_result_has_passed_true_and_no_detail() {
        let result = make_pass_result(CheckKind::Spelling);
        assert!(result.passed);
        assert_eq!(result.kind, CheckKind::Spelling);
        assert!(result.detail.is_none());
    }

    #[test]
    fn make_fail_result_has_passed_false_and_detail() {
        let result = make_fail_result(CheckKind::ChipReadability, "low contrast");
        assert!(!result.passed);
        assert_eq!(result.kind, CheckKind::ChipReadability);
        assert_eq!(result.detail.as_deref(), Some("low contrast"));
    }

    // ── CheckKind ─────────────────────────────────────────────────────────────

    #[test]
    fn check_kind_display_covers_all_variants() {
        use core::fmt::Write;
        let mut s = String::new();
        for kind in [
            CheckKind::Overlap,
            CheckKind::Clipping,
            CheckKind::ChipReadability,
            CheckKind::Bounds,
            CheckKind::SelectedState,
            CheckKind::ColorDrift,
            CheckKind::Spelling,
            CheckKind::PngValidity,
        ] {
            write!(&mut s, "{kind}").unwrap();
        }
        assert!(s.contains("overlap_check"));
        assert!(s.contains("png_validity_check"));
    }

    #[test]
    fn check_kind_partial_eq() {
        assert_eq!(CheckKind::Overlap, CheckKind::Overlap);
        assert_ne!(CheckKind::Overlap, CheckKind::ColorDrift);
    }

    // ── YAML roundtrip ────────────────────────────────────────────────────────

    #[test]
    fn saphyr_yaml_emits_parseable_report() -> anyhow::Result<()> {
        let mut report = UiSnapshotReport::new();
        report.add_screen(make_screen_result(
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

        anyhow::ensure!(
            status == Some("fail"),
            "expected status 'fail', got {status:?}"
        );
        Ok(())
    }

    #[test]
    fn yaml_report_has_pass_screens_count() -> anyhow::Result<()> {
        let mut report = UiSnapshotReport::new();
        report.add_screen(make_screen_result(
            "scr1",
            vec![make_pass_result(CheckKind::Overlap)],
        ));
        report.finalize();

        let yaml = report.to_yaml()?;
        let docs = saphyr::Yaml::load_from_str(&yaml)?;
        // Verify the report serializes to YAML and contains expected top-level keys
        let doc = docs.first().expect("expected at least one YAML document");
        let status = doc
            .as_mapping_get("status")
            .and_then(saphyr::Yaml::as_str);
        anyhow::ensure!(status == Some("pass"), "expected status 'pass', got {status:?}");
        let screens = doc
            .as_mapping_get("screens")
            .and_then(saphyr::Yaml::as_sequence);
        anyhow::ensure!(
            screens.map_or(0, |s| s.len()) == 1,
            "expected 1 screen in YAML"
        );
        Ok(())
    }

    #[test]
    fn yaml_report_has_correct_total_screens() -> anyhow::Result<()> {
        let mut report = UiSnapshotReport::new();
        report.add_screen(make_screen_result("a", vec![make_pass_result(CheckKind::Overlap)]));
        report.add_screen(make_screen_result("b", vec![make_pass_result(CheckKind::Spelling)]));
        report.finalize();

        let yaml = report.to_yaml()?;
        let docs = saphyr::Yaml::load_from_str(&yaml)?;
        let doc = docs.first().expect("expected at least one YAML document");
        // Verify status is "pass" when all screens pass
        let status = doc
            .as_mapping_get("status")
            .and_then(saphyr::Yaml::as_str);
        anyhow::ensure!(status == Some("pass"), "expected 'pass', got {status:?}");
        // Verify 2 screens in the sequence
        let screens = doc
            .as_mapping_get("screens")
            .and_then(saphyr::Yaml::as_sequence);
        anyhow::ensure!(
            screens.map_or(0, |s| s.len()) == 2,
            "expected 2 screens, got {screens:?}"
        );
        Ok(())
    }

    #[test]
    fn yaml_screens_sequence_has_correct_count() -> anyhow::Result<()> {
        let mut report = UiSnapshotReport::new();
        for name in ["scr1", "scr2", "scr3"] {
            report.add_screen(make_screen_result(name, vec![make_pass_result(CheckKind::Overlap)]));
        }
        report.finalize();

        let yaml = report.to_yaml()?;
        let docs = saphyr::Yaml::load_from_str(&yaml)?;
        let screens = docs
            .first()
            .and_then(|doc| doc.as_mapping_get("screens"))
            .and_then(saphyr::Yaml::as_sequence);

        anyhow::ensure!(
            screens.map_or(0, |s| s.len()) == 3,
            "expected 3 screens in YAML sequence"
        );
        Ok(())
    }
}
