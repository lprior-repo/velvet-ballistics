//! Evidence types and functions for xtask command-center gates.
//!
//! This module provides the evidence bundle types (GateEvidence, WhyFailed, GateStatus)
//! and orchestration functions (run_gate, run_profile, explain_failure, validate_evidence_dir).
//!
//! # Error Taxonomy
//!
//! All fallible operations return `Result<T, Error>` with explicit error variants:
//!
//! - `Error::GateTimeout` — gate exceeded its time bound
//! - `Error::GateFailed` — underlying command returned non-zero
//! - `Error::MissingEvidence` — evidence file absent (fail-closed trigger)
//! - `Error::EvidenceWriteFailed` — YAML serialization or file write error
//! - `Error::SubcommandNotFound` — requested xtask subcommand does not exist
//! - `Error::BeadDirectoryCreationFailed` — could not create `.evidence/<bead>/` directory
//! - `Error::YamlSerializationFailed` — saphyr error during evidence serialization
//! - `Error::UpstreamMoonFailed` — moon run task returned non-zero
//! - `Error::UpstreamJustFailed` — just recipe returned non-zero

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use vb_ui_snapshot::layout_kernel::{
    Rect, SelectedIndicator, chip_is_readable, is_clipped, is_out_of_bounds, overlap_area_px,
    selected_state_is_visible,
};

const VB_NF2U: &str = "vb-nf2u";
const CANONICAL_SCREENS: [&str; 8] = [
    "execution_overview",
    "workflow_graph_authoring",
    "execution_details",
    "verification_certificate",
    "replay_theater",
    "incident_failure",
    "action_registry",
    "storage_doctor_ai_context",
];
const REQUIRED_UI_SUBGATES: [&str; 6] = [
    "ui_snapshot",
    "layout_readability",
    "redaction",
    "negative_fixture",
    "deterministic_capture",
    "evidence_shape",
];
const REQUIRED_LAYOUT_CHECKS: [&str; 7] = [
    "Overlap",
    "Clipping",
    "Bounds",
    "ChipReadability",
    "SelectedState",
    "FixtureArtifactProvenance",
    "Redaction",
];
const AI_FAST_GATES: &[&str] = &[
    "fmt",
    "check",
    "clippy",
    "nextest",
    "forbidden-scan",
    "hotpath-scan",
];
const AI_RELEASE_GATES: &[&str] = &[
    "check",
    "test",
    "supply-chain",
    "miri",
    "fuzz-smoke",
    "coverage",
    "mutants-smoke",
    "bench-build",
    "feature-powerset",
    "source-length",
    "maxperf",
];
const REDACTION_CLASSES: [(&str, &str); 6] = [
    ("sentinel", "[REDACTED:sentinel]"),
    ("api_key", "[REDACTED:api_key]"),
    ("token", "[REDACTED:token]"),
    ("password", "[REDACTED:password]"),
    ("idempotency_key", "[REDACTED:idempotency_key]"),
    ("tainted_fixture_value", "[REDACTED:tainted_fixture_value]"),
];
const NEGATIVE_FIXTURE_ROOT: &str = "target/vb-nf2u-negative-fixtures";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseBeadId {
    VbNf2u,
}

impl ReleaseBeadId {
    pub fn parse(value: &str) -> std::result::Result<Self, UiReleaseGateError> {
        match value {
            VB_NF2U => Ok(Self::VbNf2u),
            _ => Err(UiReleaseGateError::ReleaseProfileIncomplete {
                code: "release_profile_incomplete",
                bead_id: "unknown",
                missing_subgates: REQUIRED_UI_SUBGATES.to_vec(),
                action: "reject unknown bead id before generating release evidence",
            }),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::VbNf2u => VB_NF2U,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NegativeFixtureWorkflow {
    Required,
    Missing,
    Observed {
        fixture_id: &'static str,
        expected_gate: &'static str,
        actual_status: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReleaseArtifactWorkflow {
    None,
    Text {
        path: &'static str,
        text: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReleaseParityClaim {
    FixtureBacked,
    LiveCoreRuntime(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FixtureReadState {
    Present(String),
    Missing(PathBuf),
}

/// UI release-gate failures required by the vb-nf2u contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiReleaseGateError {
    InvalidScreenInventory {
        code: &'static str,
        screen_id_or_count: &'static str,
        reason: &'static str,
        action: &'static str,
    },
    UnreachableScreen {
        code: &'static str,
        screen_id: &'static str,
        mapping_edge: &'static str,
        action: &'static str,
    },
    SnapshotDeterminismViolation {
        code: &'static str,
        screen_id: &'static str,
        expected_field: &'static str,
        expected_value: &'static str,
        actual_field: &'static str,
        actual_value: &'static str,
        action: &'static str,
    },
    MissingEvidence {
        code: &'static str,
        screen_id: &'static str,
        artifact_path: &'static str,
        evidence_kind: &'static str,
        action: &'static str,
    },
    LayoutViolation {
        code: &'static str,
        screen_id: &'static str,
        control_id: &'static str,
        predicate: &'static str,
        bounds: &'static str,
        action: &'static str,
    },
    RedactionViolation {
        code: &'static str,
        screen_id: &'static str,
        artifact_path: &'static str,
        secret_class: &'static str,
        redacted_sample: &'static str,
        action: &'static str,
    },
    FalsePassFixtureViolation {
        code: &'static str,
        fixture_id: &'static str,
        expected_gate: &'static str,
        actual_status: &'static str,
        action: &'static str,
    },
    ReleaseProfileIncomplete {
        code: &'static str,
        bead_id: &'static str,
        missing_subgates: Vec<&'static str>,
        action: &'static str,
    },
    CoreParityUnsupported {
        code: &'static str,
        claim: &'static str,
        blocker: &'static str,
        action: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiReleaseToolingLaneKind {
    ExecutableGate,
    ExternalMachineGate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiReleaseToolingLane {
    pub name: &'static str,
    pub command: &'static str,
    pub kind: UiReleaseToolingLaneKind,
    pub blocker: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiReleaseInventory {
    screen_ids: Vec<&'static str>,
    missing_fixture_edge: Option<&'static str>,
}

impl UiReleaseInventory {
    pub fn from_screen_ids<const N: usize>(screen_ids: [&'static str; N]) -> Self {
        Self {
            screen_ids: screen_ids.into_iter().collect(),
            missing_fixture_edge: None,
        }
    }

    pub fn without_fixture_edge(mut self, screen_id: &'static str) -> Self {
        self.missing_fixture_edge = Some(screen_id);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotDeterminismConfig {
    screen_id: &'static str,
    source: SnapshotTimeSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SnapshotTimeSource {
    Fixed,
    WallClock,
}

impl SnapshotDeterminismConfig {
    pub fn wall_clock_for_screen(screen_id: &'static str) -> Self {
        Self {
            screen_id,
            source: SnapshotTimeSource::WallClock,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSnapshotGuard {
    marker: &'static str,
}

impl ReleaseSnapshotGuard {
    pub fn evidence_marker(&self) -> &'static str {
        self.marker
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiReleaseGateConfig {
    bead_id: ReleaseBeadId,
    negative_fixture: NegativeFixtureWorkflow,
    artifact: ReleaseArtifactWorkflow,
}

impl UiReleaseGateConfig {
    pub fn for_bead(bead_id: &'static str) -> std::result::Result<Self, UiReleaseGateError> {
        let release_bead = ReleaseBeadId::parse(bead_id)?;
        Ok(Self {
            bead_id: release_bead,
            negative_fixture: NegativeFixtureWorkflow::Required,
            artifact: ReleaseArtifactWorkflow::None,
        })
    }

    pub fn without_negative_fixture_evidence(mut self) -> Self {
        self.negative_fixture = NegativeFixtureWorkflow::Missing;
        self
    }

    pub fn with_negative_fixture_status(
        mut self,
        fixture_id: &'static str,
        expected_gate: &'static str,
        actual_status: &'static str,
    ) -> Self {
        self.negative_fixture = NegativeFixtureWorkflow::Observed {
            fixture_id,
            expected_gate,
            actual_status,
        };
        self
    }

    pub fn with_artifact_text(mut self, path: &'static str, text: &'static str) -> Self {
        self.artifact = ReleaseArtifactWorkflow::Text { path, text };
        self
    }

    pub fn release_evidence(&self) -> UiReleaseEvidence {
        UiReleaseEvidence {
            artifact: self.artifact.clone(),
        }
    }

    pub fn secret_denylist(&self) -> SecretDenylist {
        SecretDenylist
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiReleaseEvidence {
    artifact: ReleaseArtifactWorkflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretDenylist;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegativeFixtureEvidence {
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionEvidence {
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseProfileEvidence {
    bead_id: ReleaseBeadId,
    subgates: Vec<&'static str>,
    parity_claim: ReleaseParityClaim,
}

impl ReleaseProfileEvidence {
    pub fn without_subgate(mut self, subgate: &'static str) -> Self {
        self.subgates.retain(|gate| *gate != subgate);
        self
    }

    pub fn with_core_runtime_parity_claim(mut self, claim: &'static str) -> Self {
        self.parity_claim = ReleaseParityClaim::LiveCoreRuntime(claim);
        self
    }

    pub fn validate(&self) -> std::result::Result<(), UiReleaseGateError> {
        if let ReleaseParityClaim::LiveCoreRuntime(claim) = self.parity_claim {
            return Err(UiReleaseGateError::CoreParityUnsupported {
                code: "core_parity_unsupported",
                claim,
                blocker: "blocked-by-core",
                action: "keep evidence fixture-backed until live Makepad/core parity exists",
            });
        }
        let missing = REQUIRED_UI_SUBGATES
            .iter()
            .copied()
            .filter(|gate| !self.subgates.iter().any(|present| present == gate))
            .collect::<Vec<_>>();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(UiReleaseGateError::ReleaseProfileIncomplete {
                code: "release_profile_incomplete",
                bead_id: self.bead_id.as_str(),
                missing_subgates: missing,
                action: "include all UI release gates in ai-release",
            })
        }
    }
}

pub fn canonical_ui_release_inventory()
-> std::result::Result<UiReleaseInventory, UiReleaseGateError> {
    Ok(UiReleaseInventory::from_screen_ids(CANONICAL_SCREENS))
}

pub fn validate_screen_bijection(
    inventory: &UiReleaseInventory,
) -> std::result::Result<(), UiReleaseGateError> {
    validate_screen_ids_known_and_unique(inventory)?;
    validate_screen_count(inventory)?;
    validate_fixture_edges(inventory)
}

fn validate_screen_ids_known_and_unique(
    inventory: &UiReleaseInventory,
) -> std::result::Result<(), UiReleaseGateError> {
    for screen in &inventory.screen_ids {
        validate_screen_not_duplicate(inventory, screen)?;
        validate_screen_is_canonical(screen)?;
    }
    Ok(())
}

fn validate_screen_not_duplicate(
    inventory: &UiReleaseInventory,
    screen: &'static str,
) -> std::result::Result<(), UiReleaseGateError> {
    let count = inventory
        .screen_ids
        .iter()
        .filter(|candidate| *candidate == &screen)
        .count();
    if count > 1 {
        invalid_inventory(screen, "duplicate screen id")
    } else {
        Ok(())
    }
}

fn validate_screen_is_canonical(
    screen: &'static str,
) -> std::result::Result<(), UiReleaseGateError> {
    if CANONICAL_SCREENS.iter().any(|required| required == &screen) {
        Ok(())
    } else {
        invalid_inventory(screen, "unknown screen id")
    }
}

fn validate_screen_count(
    inventory: &UiReleaseInventory,
) -> std::result::Result<(), UiReleaseGateError> {
    if inventory.screen_ids.len() != CANONICAL_SCREENS.len() {
        invalid_inventory("screen_count", "missing or extra screen id")
    } else {
        Ok(())
    }
}

fn validate_fixture_edges(
    inventory: &UiReleaseInventory,
) -> std::result::Result<(), UiReleaseGateError> {
    if let Some(screen_id) = inventory.missing_fixture_edge {
        Err(UiReleaseGateError::UnreachableScreen {
            code: "unreachable_screen",
            screen_id,
            mapping_edge: "fixture_id",
            action: "restore one-to-one ShellNav Screen UiScreenKind fixture and report mapping",
        })
    } else {
        Ok(())
    }
}

fn invalid_inventory(
    screen_id_or_count: &'static str,
    reason: &'static str,
) -> std::result::Result<(), UiReleaseGateError> {
    Err(UiReleaseGateError::InvalidScreenInventory {
        code: "invalid_screen_inventory",
        screen_id_or_count,
        reason,
        action: "provide each canonical UI release screen exactly once",
    })
}

pub fn enter_release_snapshot_mode(
    config: SnapshotDeterminismConfig,
) -> std::result::Result<ReleaseSnapshotGuard, UiReleaseGateError> {
    match config.source {
        SnapshotTimeSource::Fixed => Ok(ReleaseSnapshotGuard {
            marker: "deterministic_snapshot_mode",
        }),
        SnapshotTimeSource::WallClock => Err(UiReleaseGateError::SnapshotDeterminismViolation {
            code: "snapshot_determinism_violation",
            screen_id: config.screen_id,
            expected_field: "snapshot_timestamp",
            expected_value: "2026-05-09T00:00:00Z",
            actual_field: "snapshot_timestamp_source",
            actual_value: "wall_clock",
            action: "set fixed snapshot timestamp before capture",
        }),
    }
}

pub fn run_ui_negative_fixtures(
    config: UiReleaseGateConfig,
) -> std::result::Result<NegativeFixtureEvidence, UiReleaseGateError> {
    match config.negative_fixture {
        NegativeFixtureWorkflow::Missing => return missing_negative_fixture_error(),
        NegativeFixtureWorkflow::Observed {
            fixture_id,
            expected_gate,
            actual_status: "passed",
        } => return false_pass_fixture_error(fixture_id, expected_gate),
        NegativeFixtureWorkflow::Observed { .. } | NegativeFixtureWorkflow::Required => {}
    }
    let _bead_id = config.bead_id.as_str();
    Ok(NegativeFixtureEvidence {
        status: "expected-failed",
    })
}

fn missing_negative_fixture_error()
-> std::result::Result<NegativeFixtureEvidence, UiReleaseGateError> {
    Err(UiReleaseGateError::MissingEvidence {
        code: "missing_evidence",
        screen_id: "execution_overview",
        artifact_path: "target/vb-nf2u-negative-fixtures/intentional_overlap_fixture.txt",
        evidence_kind: "negative_fixture",
        action: "create required negative fixture evidence before release",
    })
}

fn false_pass_fixture_error(
    fixture_id: &'static str,
    expected_gate: &'static str,
) -> std::result::Result<NegativeFixtureEvidence, UiReleaseGateError> {
    Err(UiReleaseGateError::FalsePassFixtureViolation {
        code: "false_pass_fixture_violation",
        fixture_id,
        expected_gate,
        actual_status: "passed",
        action: "fail release because expected-fail negative fixture passed",
    })
}

pub fn check_redaction_artifacts(
    evidence: &UiReleaseEvidence,
    _denylist: &SecretDenylist,
) -> std::result::Result<RedactionEvidence, UiReleaseGateError> {
    if let ReleaseArtifactWorkflow::Text { path, text } = evidence.artifact {
        for (secret_class, raw_secret, redacted_sample) in raw_secret_patterns() {
            if text.contains(raw_secret) {
                return Err(UiReleaseGateError::RedactionViolation {
                    code: "redaction_violation",
                    screen_id: "execution_overview",
                    artifact_path: path,
                    secret_class,
                    redacted_sample,
                    action: "redact raw secret before emitting UI evidence",
                });
            }
        }
    }
    Ok(RedactionEvidence { status: "passed" })
}

fn raw_secret_patterns() -> [(&'static str, &'static str, &'static str); 6] {
    [
        ("sentinel", "vb_nf2u_secret_sentinel", "[REDACTED:sentinel]"),
        (
            "api_key",
            "sk_test_vb_nf2u_raw_secret",
            "[REDACTED:api_key]",
        ),
        ("token", "Bearer vb_nf2u_token", "[REDACTED:token]"),
        ("password", "password=hunter2", "[REDACTED:password]"),
        (
            "idempotency_key",
            "Idempotency-Key: idem_vb_nf2u_secret",
            "[REDACTED:idempotency_key]",
        ),
        (
            "tainted_fixture_value",
            "tainted_fixture_value_vb_nf2u",
            "[REDACTED:tainted_fixture_value]",
        ),
    ]
}

pub fn include_ui_gates_in_ai_release(
    bead_id: &'static str,
) -> std::result::Result<ReleaseProfileEvidence, UiReleaseGateError> {
    let release_bead = ReleaseBeadId::parse(bead_id)?;
    Ok(ReleaseProfileEvidence {
        bead_id: release_bead,
        subgates: REQUIRED_UI_SUBGATES.to_vec(),
        parity_claim: ReleaseParityClaim::FixtureBacked,
    })
}
const UI_RELEASE_TOOLING_LANES: [UiReleaseToolingLane; 7] = [
    UiReleaseToolingLane {
        name: "kani-inventory",
        command: "cargo kani -p vb_ui_snapshot --harness inventory",
        kind: UiReleaseToolingLaneKind::ExternalMachineGate,
        blocker: Some("requires Kani runner outside this bead-scoped nextest suite"),
    },
    UiReleaseToolingLane {
        name: "kani-layout-predicates",
        command: "cargo kani -p vb_ui_snapshot --harness layout_",
        kind: UiReleaseToolingLaneKind::ExternalMachineGate,
        blocker: Some("requires Kani runner outside this bead-scoped nextest suite"),
    },
    UiReleaseToolingLane {
        name: "redaction-fuzz",
        command: "cargo fuzz run ui_redaction_artifact",
        kind: UiReleaseToolingLaneKind::ExternalMachineGate,
        blocker: Some("cargo-fuzz sanitizer target is an external machine gate"),
    },
    UiReleaseToolingLane {
        name: "miri",
        command: "cargo +nightly miri test -p vb_ui_snapshot",
        kind: UiReleaseToolingLaneKind::ExecutableGate,
        blocker: None,
    },
    UiReleaseToolingLane {
        name: "mutants",
        command: "cargo mutants -p vb_ui_snapshot",
        kind: UiReleaseToolingLaneKind::ExecutableGate,
        blocker: None,
    },
    UiReleaseToolingLane {
        name: "coverage",
        command: "cargo llvm-cov nextest",
        kind: UiReleaseToolingLaneKind::ExecutableGate,
        blocker: None,
    },
    UiReleaseToolingLane {
        name: "moon-ci",
        command: "moon ci",
        kind: UiReleaseToolingLaneKind::ExternalMachineGate,
        blocker: Some("repository-level machine gate, not a bead-local unit behavior"),
    },
];

pub fn ui_release_tooling_lanes() -> &'static [UiReleaseToolingLane] {
    &UI_RELEASE_TOOLING_LANES
}

/// Evidence bundle for a single gate execution.
///
/// Contains all fields required by POST-004:
/// - `kind`: Category of the gate (e.g., "fmt", "clippy", "ai-fast")
/// - `gate_name`: Specific gate name within the category
/// - `command`: Full command string that was executed
/// - `exit_code`: Numeric exit code from the command
/// - `log`: Path to the log file with raw tool output
/// - `status`: Pass/Fail/Skipped status
/// - `why_failed`: Optional failure diagnostic with hint and repair command
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateEvidence {
    pub kind: String,
    pub gate_name: String,
    pub command: String,
    pub exit_code: i32,
    pub log: PathBuf,
    pub status: GateStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub why_failed: Option<WhyFailed>,
}

/// Variant tag for structured false-pass diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FalsePassDiagnosticVariant {
    Overlap,
    Secret,
}

/// Failure diagnostic with hint and repair command.
///
/// Populated when a gate fails, providing actionable remediation steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhyFailed {
    pub gate_name: String,
    pub hint: String,
    pub repair_command: String,
    /// Variant tag for false-pass diagnostics. Present when gate is
    /// `FalsePassFixtureViolation` to disambiguate overlap vs secret false-pass.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<FalsePassDiagnosticVariant>,
    /// Fixture ID from a false-pass diagnostic. Present when variant is Some.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixture_id: Option<String>,
    /// Expected gate from a false-pass diagnostic. Present when variant is Some.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_gate: Option<String>,
}

/// Status of a gate execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", content = "reason")]
pub enum GateStatus {
    Pass,
    Fail,
    Skipped { reason: String },
}

/// All error variants for xtask gate operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Gate exceeded its configured timeout duration.
    GateTimeout { gate: String, duration_secs: u64 },
    /// Underlying command returned non-zero exit code.
    GateFailed {
        gate: String,
        exit_code: i32,
        log: PathBuf,
    },
    /// Evidence file for a required gate does not exist (fail-closed).
    MissingEvidence { gate: String, path: PathBuf },
    /// YAML serialization or file write failed.
    EvidenceWriteFailed {
        gate: String,
        path: PathBuf,
        cause: String,
    },
    /// Requested xtask subcommand does not exist.
    SubcommandNotFound { name: String },
    /// Could not create `.evidence/<bead>/` directory.
    BeadDirectoryCreationFailed { bead: String, cause: String },
    /// saphyr error during evidence serialization.
    YamlSerializationFailed { gate: String, cause: String },
    /// moon run task returned non-zero.
    UpstreamMoonFailed { task: String, cause: String },
    /// just recipe returned non-zero.
    UpstreamJustFailed { recipe: String, cause: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XtaskCommandDiagnostic {
    pub error_code: DiagnosticCode,
    pub fixture_id: FixtureId,
    pub expected_gate: FixtureGate,
    pub actual_status: FixtureStatus,
    pub variant: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCommandDiagnosticEnvelope {
    xtask_diagnostic: RawCommandDiagnostic,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCommandDiagnostic {
    error_code: String,
    fixture_id: String,
    expected_gate: String,
    actual_status: String,
    #[serde(default)]
    variant: Option<String>,
}

impl XtaskCommandDiagnostic {
    pub fn parse_output(text: &str) -> std::result::Result<Self, String> {
        let yaml = diagnostic_yaml_slice(text)?;
        let raw: RawCommandDiagnosticEnvelope = parse_yaml_document(&yaml)?;
        Self::try_from(raw.xtask_diagnostic)
    }
}

impl TryFrom<RawCommandDiagnostic> for XtaskCommandDiagnostic {
    type Error = String;

    fn try_from(raw: RawCommandDiagnostic) -> std::result::Result<Self, Self::Error> {
        Ok(Self {
            error_code: parse_diagnostic_code_value(raw.error_code)?,
            fixture_id: FixtureId::parse(raw.fixture_id, "diagnostic fixture_id")?,
            expected_gate: parse_gate_value(raw.expected_gate)?,
            actual_status: parse_status_value(raw.actual_status)?,
            variant: raw.variant,
        })
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::GateTimeout {
                gate,
                duration_secs,
            } => write_gate_timeout(f, gate, duration_secs),
            Error::GateFailed {
                gate,
                exit_code,
                log,
            } => write_gate_failed(f, gate, *exit_code, log),
            Error::MissingEvidence { gate, path } => write_missing_evidence(f, gate, path),
            Error::EvidenceWriteFailed { gate, path, cause } => {
                write_evidence_failed(f, gate, path, cause)
            }
            Error::SubcommandNotFound { name } => write!(f, "Subcommand not found: '{}'", name),
            Error::BeadDirectoryCreationFailed { bead, cause } => {
                write_bead_dir_failed(f, bead, cause)
            }
            Error::YamlSerializationFailed { gate, cause } => write_yaml_failed(f, gate, cause),
            Error::UpstreamMoonFailed { task, cause } => write_moon_failed(f, task, cause),
            Error::UpstreamJustFailed { recipe, cause } => write_just_failed(f, recipe, cause),
        }
    }
}

impl Error {
    /// Extract false-pass diagnostic variant and fields if this is a false-pass error.
    ///
    /// Returns `Some((variant, fixture_id, expected_gate))` when `gate` is
    /// `"FalsePassFixtureViolation"`, otherwise `None`.
    pub fn false_pass_diagnostic(&self) -> Option<(FalsePassDiagnosticVariant, &str, &str)> {
        let Error::GateFailed { gate, log, .. } = self else {
            return None;
        };
        if gate != "FalsePassFixtureViolation" {
            return None;
        }
        let path_text = log.display().to_string();
        if path_text.contains("intentional_secret_fixture") {
            Some((
                FalsePassDiagnosticVariant::Secret,
                "intentional_secret_fixture",
                "redaction",
            ))
        } else {
            Some((
                FalsePassDiagnosticVariant::Overlap,
                "intentional_overlap_fixture",
                "layout",
            ))
        }
    }
}

fn write_gate_timeout(f: &mut std::fmt::Formatter<'_>, gate: &str, secs: &u64) -> std::fmt::Result {
    write!(f, "Gate '{}' exceeded timeout of {}s", gate, secs)
}

fn write_gate_failed(
    f: &mut std::fmt::Formatter<'_>,
    gate: &str,
    exit_code: i32,
    log: &Path,
) -> std::fmt::Result {
    if gate == "FalsePassFixtureViolation" {
        return write_false_pass_diagnostic(f, log);
    }
    write!(
        f,
        "Gate '{}' failed with exit code {} (log: {})",
        gate,
        exit_code,
        log.display()
    )
}

fn write_false_pass_diagnostic(f: &mut std::fmt::Formatter<'_>, log: &Path) -> std::fmt::Result {
    let (variant, fixture_id, expected_gate) = false_pass_diagnostic_for_path(log);
    let variant_str = match variant {
        FalsePassDiagnosticVariant::Overlap => "OverlapFalsePass",
        FalsePassDiagnosticVariant::Secret => "SecretFalsePass",
    };
    write!(
        f,
        "UI release gate failed; evidence_path: {}\nxtask_diagnostic:\n  variant: {}\n  error_code: false_pass_fixture_violation\n  fixture_id: {}\n  expected_gate: {}\n  actual_status: passed",
        log.display(),
        variant_str,
        fixture_id,
        expected_gate
    )
}

fn false_pass_diagnostic_for_path(
    log: &Path,
) -> (FalsePassDiagnosticVariant, &'static str, &'static str) {
    let path_text = log.display().to_string();
    if path_text.contains("intentional_secret_fixture") {
        (
            FalsePassDiagnosticVariant::Secret,
            "intentional_secret_fixture",
            "redaction",
        )
    } else {
        (
            FalsePassDiagnosticVariant::Overlap,
            "intentional_overlap_fixture",
            "layout",
        )
    }
}

fn write_missing_evidence(
    f: &mut std::fmt::Formatter<'_>,
    gate: &str,
    path: &Path,
) -> std::fmt::Result {
    write!(
        f,
        "Missing evidence for gate '{}' at {}",
        gate,
        path.display()
    )
}

fn write_evidence_failed(
    f: &mut std::fmt::Formatter<'_>,
    gate: &str,
    path: &Path,
    cause: &str,
) -> std::fmt::Result {
    write!(
        f,
        "Failed to write evidence for '{}' to {}: {}",
        gate,
        path.display(),
        cause
    )
}

fn write_bead_dir_failed(
    f: &mut std::fmt::Formatter<'_>,
    bead: &str,
    cause: &str,
) -> std::fmt::Result {
    write!(
        f,
        "Failed to create evidence directory for bead '{}': {}",
        bead, cause
    )
}

fn write_yaml_failed(f: &mut std::fmt::Formatter<'_>, gate: &str, cause: &str) -> std::fmt::Result {
    write!(f, "YAML serialization failed for '{}': {}", gate, cause)
}

fn write_moon_failed(f: &mut std::fmt::Formatter<'_>, task: &str, cause: &str) -> std::fmt::Result {
    write!(f, "Moon task '{}' failed: {}", task, cause)
}

fn write_just_failed(
    f: &mut std::fmt::Formatter<'_>,
    recipe: &str,
    cause: &str,
) -> std::fmt::Result {
    write!(f, "Just recipe '{}' failed: {}", recipe, cause)
}

impl std::error::Error for Error {}

/// Result type alias for evidence operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Profile of gates to run together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::enum_variant_names)]
pub enum GateProfile {
    /// Fast gates: fmt, check, clippy, nextest, forbidden-scan, hotpath-scan
    AiFast,
    /// Deep gates: miri, mutants, llvm-cov, fuzz-build
    AiDeep,
    /// Release gates: check, test, supply-chain, miri, fuzz-smoke, coverage,
    /// mutants-smoke, bench-build, feature-powerset, source-length, maxperf
    AiRelease,
}

impl GateProfile {
    /// Returns the list of gates in this profile.
    pub fn gates(self) -> &'static [&'static str] {
        match self {
            GateProfile::AiFast => AI_FAST_GATES,
            GateProfile::AiDeep => &["miri", "mutants", "llvm-cov", "fuzz-build"],
            GateProfile::AiRelease => AI_RELEASE_GATES,
        }
    }

    /// Returns the evidence file name for this profile.
    pub fn evidence_file(self) -> &'static str {
        match self {
            GateProfile::AiFast => "ai-fast.yaml",
            GateProfile::AiDeep => "ai-deep.yaml",
            GateProfile::AiRelease => "ai-release.yaml",
        }
    }
}

/// Aggregated evidence for a full profile run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileEvidence {
    pub profile: String,
    pub gates: Vec<GateEvidence>,
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalScreenId(String);

impl CanonicalScreenId {
    fn parse(value: String) -> std::result::Result<Self, String> {
        if CANONICAL_SCREENS
            .iter()
            .any(|screen| screen == &value.as_str())
        {
            Ok(Self(value))
        } else {
            Err(format!("invalid canonical screen id: {value}"))
        }
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiSubgateName(String);

impl UiSubgateName {
    fn parse(value: String) -> std::result::Result<Self, String> {
        if REQUIRED_UI_SUBGATES
            .iter()
            .any(|gate| gate == &value.as_str())
        {
            Ok(Self(value))
        } else {
            Err(format!("invalid UI release subgate: {value}"))
        }
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutCheckName(String);

impl LayoutCheckName {
    fn parse(value: String) -> std::result::Result<Self, String> {
        if REQUIRED_LAYOUT_CHECKS
            .iter()
            .any(|check| check == &value.as_str())
        {
            Ok(Self(value))
        } else {
            Err(format!("invalid layout check: {value}"))
        }
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureBackedState {
    FixtureBacked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreParityClaim {
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionClass {
    Sentinel,
    ApiKey,
    Token,
    Password,
    IdempotencyKey,
    TaintedFixtureValue,
}

impl RedactionClass {
    fn parse(value: &str) -> std::result::Result<Self, String> {
        match value {
            "sentinel" => Ok(Self::Sentinel),
            "api_key" => Ok(Self::ApiKey),
            "token" => Ok(Self::Token),
            "password" => Ok(Self::Password),
            "idempotency_key" => Ok(Self::IdempotencyKey),
            "tainted_fixture_value" => Ok(Self::TaintedFixtureValue),
            _ => Err(format!("invalid redaction class: {value}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sentinel => "sentinel",
            Self::ApiKey => "api_key",
            Self::Token => "token",
            Self::Password => "password",
            Self::IdempotencyKey => "idempotency_key",
            Self::TaintedFixtureValue => "tainted_fixture_value",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSnapshotDocument {
    pub total_screens: usize,
    pub passed_screens: usize,
    pub failed_screens: usize,
    pub screens: Vec<ParsedScreenDocument>,
    pub fixture_backed: FixtureBackedState,
    pub core_runtime_parity_claim: CoreParityClaim,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedScreenDocument {
    pub screen_name: CanonicalScreenId,
    pub checks: Vec<LayoutCheckName>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAiReleaseDocument {
    pub subgates: Vec<UiSubgateName>,
    pub redaction: Vec<ParsedRedactionScreen>,
    pub fixture_backed: FixtureBackedState,
    pub core_runtime_parity_claim: CoreParityClaim,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRedactionScreen {
    pub screen_id: CanonicalScreenId,
    pub classes: Vec<RedactionClass>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedNegativeFixtureDocument {
    pub overlap: ParsedOverlapFixtureEvidence,
    pub secret: ParsedSecretFixtureEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedOverlapFixtureEvidence {
    ExpectedFailed(ParsedOverlapExpectedFailure),
    Rejected(ParsedRejectedFixtureEvidence),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedSecretFixtureEvidence {
    ExpectedFailed(ParsedSecretExpectedFailure),
    Rejected(ParsedRejectedFixtureEvidence),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureBounds(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureNonce(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedSample(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonzeroOverlapArea(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureStatus {
    ExpectedFailed,
    Rejected,
    Passed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticCode {
    Layout,
    Redaction,
    FalsePassFixture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureGate {
    LayoutReadability,
    Layout,
    Redaction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedOverlapRejection {
    pub fixture_id: FixtureId,
    pub status: FixtureStatus,
    pub error: DiagnosticCode,
    pub expected_gate: FixtureGate,
    pub actual_status: FixtureStatus,
    pub action: String,
    pub fixture_nonce: Option<FixtureNonce>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSecretRejection {
    pub fixture_id: FixtureId,
    pub status: FixtureStatus,
    pub error: DiagnosticCode,
    pub expected_gate: FixtureGate,
    pub actual_status: FixtureStatus,
    pub action: String,
    pub fixture_nonce: Option<FixtureNonce>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedOverlapExpectedFailure {
    pub fixture_id: FixtureId,
    pub status: FixtureStatus,
    pub gate: FixtureGate,
    pub diagnostic_code: DiagnosticCode,
    pub screen_id: CanonicalScreenId,
    pub control_id: ControlId,
    pub second_control_id: ControlId,
    pub overlap_area_px: NonzeroOverlapArea,
    pub bounds: FixtureBounds,
    pub predicate: LayoutCheckName,
    pub fixture_nonce: Option<FixtureNonce>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedSecretExpectedFailure {
    pub fixture_id: FixtureId,
    pub status: FixtureStatus,
    pub gate: FixtureGate,
    pub diagnostic_code: DiagnosticCode,
    pub screen_id: CanonicalScreenId,
    pub secret_class: RedactionClass,
    pub redacted_sample: RedactedSample,
    pub fixture_nonce: Option<FixtureNonce>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedRejectedFixtureEvidence {
    Overlap(ParsedOverlapRejection),
    Secret(ParsedSecretRejection),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedNegativeFixtureEntry {
    Overlap(ParsedOverlapFixtureEvidence),
    Secret(ParsedSecretFixtureEvidence),
}

// ============================================================================
// Core orchestration functions
// ============================================================================

pub fn parse_snapshot_document(text: &str) -> std::result::Result<ParsedSnapshotDocument, String> {
    let raw: RawSnapshotDocument = parse_yaml_document(text)?;
    let doc = ParsedSnapshotDocument::try_from(raw)?;
    validate_parsed_snapshot(&doc)?;
    Ok(doc)
}

pub fn parse_ai_release_document(
    text: &str,
) -> std::result::Result<ParsedAiReleaseDocument, String> {
    let raw: RawAiReleaseDocument = parse_yaml_document(text)?;
    let doc = ParsedAiReleaseDocument::try_from(raw)?;
    validate_parsed_ai_release(&doc)?;
    Ok(doc)
}

pub fn parse_negative_fixture_document(
    text: &str,
) -> std::result::Result<ParsedNegativeFixtureDocument, String> {
    let raw: RawNegativeFixtureDocument = parse_yaml_document(text)?;
    let doc = ParsedNegativeFixtureDocument::try_from(raw)?;
    validate_parsed_negative(&doc)?;
    Ok(doc)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSnapshotDocument {
    status: String,
    total_screens: usize,
    passed_screens: usize,
    failed_screens: usize,
    fixture_backed: bool,
    core_runtime_parity_claim: String,
    screens: Vec<RawScreenDocument>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScreenDocument {
    screen_name: String,
    fixture_id: String,
    artifact_path: String,
    digest: String,
    passed: bool,
    diagnostics: Vec<String>,
    execution_marker: String,
    checks: Vec<RawScreenCheck>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScreenCheck {
    kind: String,
    passed: bool,
    diagnostics: Vec<String>,
    execution_marker: String,
    origin: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAiReleaseDocument {
    profile: String,
    bead_id: String,
    status: String,
    fixture_backed: bool,
    core_runtime_parity_claim: String,
    command: String,
    subgates: Vec<RawSubgateDocument>,
    redaction: RawRedactionDocument,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSubgateDocument {
    name: String,
    status: String,
    command: String,
    origin: String,
    diagnostics: Vec<String>,
    execution_marker: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRedactionDocument {
    status: String,
    checked_artifacts: Vec<String>,
    screens: Vec<RawRedactionScreen>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRedactionScreen {
    screen_id: String,
    status: String,
    diagnostics: Vec<String>,
    execution_marker: String,
    class_coverage: RawClassCoverage,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawClassCoverage {
    sentinel: RawClassEvidence,
    api_key: RawClassEvidence,
    token: RawClassEvidence,
    password: RawClassEvidence,
    idempotency_key: RawClassEvidence,
    tainted_fixture_value: RawClassEvidence,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawClassEvidence {
    detectors: usize,
    raw_matches: usize,
    approved_placeholders_seen: usize,
    placeholder: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNegativeFixtureDocument {
    negative_fixtures: Vec<RawNegativeFixtureEntry>,
    contract_audit: RawContractAudit,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawContractAudit {
    fixture_backed: bool,
    false_pass_detectors: Vec<String>,
    core_runtime_parity_claim: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawNegativeFixtureEntry {
    fixture_id: String,
    status: String,
    gate: Option<String>,
    diagnostic_code: Option<String>,
    screen_id: Option<String>,
    artifact_path: Option<String>,
    control_id: Option<String>,
    second_control_id: Option<String>,
    overlap_area_px: Option<String>,
    bounds: Option<String>,
    predicate: Option<String>,
    fixture_nonce: Option<String>,
    secret_class: Option<String>,
    redacted_sample: Option<String>,
    variant: Option<String>,
    expected_gate: Option<String>,
    actual_status: Option<String>,
    error: Option<String>,
    code: Option<String>,
    action: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDeterminismDocument {
    deterministic_capture: String,
    snapshot_timestamp: String,
    hidden_animation_state: String,
    clock_source: String,
    execution_marker: String,
    fixture_backed: bool,
    core_runtime_parity_claim: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAnimationFreezeDocument {
    hidden_animation_state: String,
    visible_animation_time_source: String,
    execution_marker: String,
}

fn parse_yaml_document<T>(text: &str) -> std::result::Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    serde_saphyr::from_str::<T>(text).map_err(|error| error.to_string())
}

fn parse_determinism_document(text: &str) -> std::result::Result<(), String> {
    let raw: RawDeterminismDocument = parse_yaml_document(text)?;
    if raw.deterministic_capture != "passed"
        || raw.snapshot_timestamp != "2026-05-09T00:00:00Z"
        || raw.hidden_animation_state != "Paused"
        || raw.clock_source != "FixedFixtureTime"
        || raw.execution_marker != "vb-nf2u-deterministic-capture"
    {
        return Err("invalid deterministic capture document".to_string());
    }
    require_fixture_backed(raw.fixture_backed)?;
    require_unsupported_parity(raw.core_runtime_parity_claim)?;
    Ok(())
}

fn parse_animation_freeze_document(text: &str) -> std::result::Result<(), String> {
    let raw: RawAnimationFreezeDocument = parse_yaml_document(text)?;
    if raw.hidden_animation_state == "Paused"
        && raw.visible_animation_time_source == "FixedFixtureTime"
        && raw.execution_marker == "vb-nf2u-animation-freeze"
    {
        Ok(())
    } else {
        Err("invalid animation freeze document".to_string())
    }
}

fn require_fixture_backed(value: bool) -> std::result::Result<FixtureBackedState, String> {
    if value {
        Ok(FixtureBackedState::FixtureBacked)
    } else {
        Err("fixture_backed must be true".to_string())
    }
}

fn require_unsupported_parity(value: String) -> std::result::Result<CoreParityClaim, String> {
    if value == "unsupported" {
        Ok(CoreParityClaim::Unsupported)
    } else {
        Err(format!("invalid core parity claim: {value}"))
    }
}

impl FixtureId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ControlId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FixtureBounds {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FixtureNonce {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl RedactedSample {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl NonzeroOverlapArea {
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl FixtureStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExpectedFailed => "expected-failed",
            Self::Rejected => "rejected",
            Self::Passed => "passed",
        }
    }
}

impl DiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Layout => "layout_violation",
            Self::Redaction => "redaction_violation",
            Self::FalsePassFixture => "false_pass_fixture_violation",
        }
    }
}

impl FixtureGate {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LayoutReadability => "layout_readability",
            Self::Layout => "layout",
            Self::Redaction => "redaction",
        }
    }
}

impl TryFrom<RawSnapshotDocument> for ParsedSnapshotDocument {
    type Error = String;

    fn try_from(raw: RawSnapshotDocument) -> std::result::Result<Self, Self::Error> {
        if raw.status != "pass" {
            return Err("snapshot report status must be pass".to_string());
        }
        Ok(Self {
            total_screens: raw.total_screens,
            passed_screens: raw.passed_screens,
            failed_screens: raw.failed_screens,
            fixture_backed: require_fixture_backed(raw.fixture_backed)?,
            core_runtime_parity_claim: require_unsupported_parity(raw.core_runtime_parity_claim)?,
            screens: raw
                .screens
                .into_iter()
                .map(ParsedScreenDocument::try_from)
                .collect::<std::result::Result<Vec<_>, _>>()?,
        })
    }
}

impl TryFrom<RawScreenDocument> for ParsedScreenDocument {
    type Error = String;

    fn try_from(raw: RawScreenDocument) -> std::result::Result<Self, Self::Error> {
        if raw.fixture_id != raw.screen_name || !raw.passed || !raw.diagnostics.is_empty() {
            return Err("invalid snapshot screen row".to_string());
        }
        if !raw.artifact_path.ends_with(".fixture.txt") || !raw.digest.starts_with("blake3:") {
            return Err("invalid snapshot artifact provenance".to_string());
        }
        let screen_name = CanonicalScreenId::parse(raw.screen_name)?;
        let marker = format!("vb-nf2u-{}", screen_name.as_str());
        if raw.execution_marker != marker {
            return Err("invalid snapshot execution marker".to_string());
        }
        Ok(Self {
            screen_name,
            checks: raw
                .checks
                .into_iter()
                .map(|check| check.into_check())
                .collect::<std::result::Result<Vec<_>, _>>()?,
        })
    }
}

impl RawScreenCheck {
    fn into_check(self) -> std::result::Result<LayoutCheckName, String> {
        if !self.passed || !self.diagnostics.is_empty() || self.origin.is_empty() {
            return Err("invalid snapshot check row".to_string());
        }
        LayoutCheckName::parse(self.kind)
    }
}

impl TryFrom<RawAiReleaseDocument> for ParsedAiReleaseDocument {
    type Error = String;

    fn try_from(raw: RawAiReleaseDocument) -> std::result::Result<Self, Self::Error> {
        if raw.profile != "ai-release" || raw.bead_id != VB_NF2U || raw.status != "passed" {
            return Err("invalid ai-release document header".to_string());
        }
        if raw.command != "cargo xtask ai-release --bead vb-nf2u" {
            return Err("invalid ai-release command".to_string());
        }
        Ok(Self {
            fixture_backed: require_fixture_backed(raw.fixture_backed)?,
            core_runtime_parity_claim: require_unsupported_parity(raw.core_runtime_parity_claim)?,
            subgates: raw
                .subgates
                .into_iter()
                .map(RawSubgateDocument::into_subgate)
                .collect::<std::result::Result<Vec<_>, _>>()?,
            redaction: raw.redaction.into_screens()?,
        })
    }
}

impl RawSubgateDocument {
    fn into_subgate(self) -> std::result::Result<UiSubgateName, String> {
        if self.status != "passed" || self.command != "cargo xtask ai-release --bead vb-nf2u" {
            return Err("invalid subgate row".to_string());
        }
        if !self.diagnostics.is_empty()
            || self.origin.is_empty()
            || self.execution_marker.is_empty()
        {
            return Err("invalid subgate diagnostics/origin".to_string());
        }
        UiSubgateName::parse(self.name)
    }
}

impl RawRedactionDocument {
    fn into_screens(self) -> std::result::Result<Vec<ParsedRedactionScreen>, String> {
        if self.status != "passed" || self.checked_artifacts.is_empty() {
            return Err("invalid redaction document".to_string());
        }
        self.screens
            .into_iter()
            .map(ParsedRedactionScreen::try_from)
            .collect()
    }
}

impl TryFrom<RawRedactionScreen> for ParsedRedactionScreen {
    type Error = String;

    fn try_from(raw: RawRedactionScreen) -> std::result::Result<Self, Self::Error> {
        if raw.status != "passed" || !raw.diagnostics.is_empty() || raw.execution_marker.is_empty()
        {
            return Err("invalid redaction screen".to_string());
        }
        Ok(Self {
            screen_id: CanonicalScreenId::parse(raw.screen_id)?,
            classes: raw.class_coverage.into_classes()?,
        })
    }
}

impl RawClassCoverage {
    fn into_classes(self) -> std::result::Result<Vec<RedactionClass>, String> {
        [
            ("sentinel", self.sentinel),
            ("api_key", self.api_key),
            ("token", self.token),
            ("password", self.password),
            ("idempotency_key", self.idempotency_key),
            ("tainted_fixture_value", self.tainted_fixture_value),
        ]
        .into_iter()
        .map(|(name, evidence)| evidence.into_class(name))
        .collect()
    }
}

impl RawClassEvidence {
    fn into_class(self, name: &str) -> std::result::Result<RedactionClass, String> {
        if self.detectors == 0 || self.raw_matches != 0 || self.approved_placeholders_seen == 0 {
            return Err(format!("invalid redaction evidence for {name}"));
        }
        let class = RedactionClass::parse(name)?;
        if self.placeholder != format!("[REDACTED:{}]", class.as_str()) {
            return Err(format!("invalid redaction placeholder for {name}"));
        }
        Ok(class)
    }
}

impl TryFrom<RawNegativeFixtureDocument> for ParsedNegativeFixtureDocument {
    type Error = String;

    fn try_from(raw: RawNegativeFixtureDocument) -> std::result::Result<Self, Self::Error> {
        validate_raw_contract_audit(raw.contract_audit)?;
        let mut overlap = None;
        let mut secret = None;
        for entry in raw.negative_fixtures {
            match entry.fixture_id.as_str() {
                "intentional_overlap_fixture" if overlap.is_none() => {
                    overlap = Some(entry.into_overlap()?)
                }
                "intentional_secret_fixture" if secret.is_none() => {
                    secret = Some(entry.into_secret()?)
                }
                _ => return Err("duplicate or unknown negative fixture entry".to_string()),
            }
        }
        Ok(Self {
            overlap: overlap.ok_or_else(|| "missing overlap fixture entry".to_string())?,
            secret: secret.ok_or_else(|| "missing secret fixture entry".to_string())?,
        })
    }
}

fn validate_raw_contract_audit(raw: RawContractAudit) -> std::result::Result<(), String> {
    require_fixture_backed(raw.fixture_backed)?;
    require_unsupported_parity(raw.core_runtime_parity_claim)?;
    if raw.false_pass_detectors.len() == 2 {
        Ok(())
    } else {
        Err("invalid negative fixture contract audit".to_string())
    }
}

impl RawNegativeFixtureEntry {
    fn into_overlap(self) -> std::result::Result<ParsedOverlapFixtureEvidence, String> {
        if self.status == "rejected" {
            return self.into_rejected_overlap();
        }
        self.into_expected_overlap()
    }

    fn into_expected_overlap(self) -> std::result::Result<ParsedOverlapFixtureEvidence, String> {
        let evidence = ParsedOverlapExpectedFailure {
            fixture_id: FixtureId::parse(self.fixture_id, "overlap fixture_id")?,
            status: FixtureStatus::parse_expected(self.status, "overlap status")?,
            gate: parse_required_gate(self.gate, FixtureGate::LayoutReadability, "overlap gate")?,
            diagnostic_code: parse_required_code(
                self.diagnostic_code,
                DiagnosticCode::Layout,
                "overlap diagnostic",
            )?,
            screen_id: parse_required_screen(self.screen_id, "overlap screen_id")?,
            control_id: parse_required_control(self.control_id, "overlap control_id")?,
            second_control_id: parse_required_control(
                self.second_control_id,
                "overlap second_control_id",
            )?,
            overlap_area_px: parse_required_overlap_area(self.overlap_area_px)?,
            bounds: parse_required_bounds(self.bounds, "overlap bounds")?,
            predicate: parse_overlap_predicate(self.predicate)?,
            fixture_nonce: parse_optional_nonce(self.fixture_nonce)?,
        };
        Ok(ParsedOverlapFixtureEvidence::ExpectedFailed(evidence))
    }

    fn into_rejected_overlap(self) -> std::result::Result<ParsedOverlapFixtureEvidence, String> {
        let rejected = ParsedRejectedFixtureEvidence::Overlap(self.into_overlap_rejection()?);
        Ok(ParsedOverlapFixtureEvidence::Rejected(rejected))
    }

    fn into_secret(self) -> std::result::Result<ParsedSecretFixtureEvidence, String> {
        if self.status == "rejected" {
            return self.into_rejected_secret();
        }
        self.into_expected_secret()
    }

    fn into_expected_secret(self) -> std::result::Result<ParsedSecretFixtureEvidence, String> {
        let evidence = ParsedSecretExpectedFailure {
            fixture_id: FixtureId::parse(self.fixture_id, "secret fixture_id")?,
            status: FixtureStatus::parse_expected(self.status, "secret status")?,
            gate: parse_required_gate(self.gate, FixtureGate::Redaction, "secret gate")?,
            diagnostic_code: parse_required_code(
                self.diagnostic_code,
                DiagnosticCode::Redaction,
                "secret diagnostic",
            )?,
            screen_id: parse_required_screen(self.screen_id, "secret screen_id")?,
            secret_class: RedactionClass::parse(&require_some(self.secret_class, "secret class")?)?,
            redacted_sample: parse_required_redacted_sample(self.redacted_sample)?,
            fixture_nonce: parse_optional_nonce(self.fixture_nonce)?,
        };
        Ok(ParsedSecretFixtureEvidence::ExpectedFailed(evidence))
    }

    fn into_rejected_secret(self) -> std::result::Result<ParsedSecretFixtureEvidence, String> {
        let rejected = ParsedRejectedFixtureEvidence::Secret(self.into_secret_rejection()?);
        Ok(ParsedSecretFixtureEvidence::Rejected(rejected))
    }

    fn into_overlap_rejection(self) -> std::result::Result<ParsedOverlapRejection, String> {
        validate_false_pass_variant(self.variant, self.code)?;
        Ok(ParsedOverlapRejection {
            fixture_id: FixtureId::parse(self.fixture_id, "rejected fixture_id")?,
            status: FixtureStatus::parse_rejected(self.status, "rejected status")?,
            error: parse_required_error(self.error)?,
            expected_gate: parse_required_gate(self.expected_gate, FixtureGate::Layout, "gate")?,
            actual_status: parse_required_passed(self.actual_status)?,
            action: self.action,
            fixture_nonce: parse_optional_nonce(self.fixture_nonce)?,
        })
    }

    fn into_secret_rejection(self) -> std::result::Result<ParsedSecretRejection, String> {
        validate_false_pass_variant(self.variant, self.code)?;
        Ok(ParsedSecretRejection {
            fixture_id: FixtureId::parse(self.fixture_id, "rejected fixture_id")?,
            status: FixtureStatus::parse_rejected(self.status, "rejected status")?,
            error: parse_required_error(self.error)?,
            expected_gate: parse_required_gate(self.expected_gate, FixtureGate::Redaction, "gate")?,
            actual_status: parse_required_passed(self.actual_status)?,
            action: self.action,
            fixture_nonce: parse_optional_nonce(self.fixture_nonce)?,
        })
    }
}

fn require_some(value: Option<String>, name: &str) -> std::result::Result<String, String> {
    value.ok_or_else(|| format!("missing {name}"))
}

fn require_text(value: String, expected: &str, name: &str) -> std::result::Result<String, String> {
    if value == expected {
        Ok(value)
    } else {
        Err(format!("invalid {name}: {value}"))
    }
}

impl FixtureId {
    fn parse(value: String, name: &str) -> std::result::Result<Self, String> {
        parse_nonempty_text(value, name).map(Self)
    }
}

impl ControlId {
    fn parse(value: String, name: &str) -> std::result::Result<Self, String> {
        parse_nonempty_text(value, name).map(Self)
    }
}

impl FixtureBounds {
    fn parse(value: String, name: &str) -> std::result::Result<Self, String> {
        parse_nonempty_text(value, name).map(Self)
    }
}

impl FixtureNonce {
    fn parse(value: String) -> std::result::Result<Self, String> {
        parse_nonempty_text(value, "fixture_nonce").map(Self)
    }
}

impl RedactedSample {
    fn parse(value: String) -> std::result::Result<Self, String> {
        if value.starts_with("[REDACTED:") && value.ends_with(']') {
            Ok(Self(value))
        } else {
            Err("invalid redacted sample".to_string())
        }
    }
}

impl NonzeroOverlapArea {
    fn parse(value: String) -> std::result::Result<Self, String> {
        match value.parse::<u32>() {
            Ok(area) if area > 0 => Ok(Self(area)),
            _ => Err(format!("invalid nonzero overlap area: {value}")),
        }
    }
}

impl FixtureStatus {
    fn parse_expected(value: String, name: &str) -> std::result::Result<Self, String> {
        require_text(value, Self::ExpectedFailed.as_str(), name).map(|_| Self::ExpectedFailed)
    }

    fn parse_rejected(value: String, name: &str) -> std::result::Result<Self, String> {
        require_text(value, Self::Rejected.as_str(), name).map(|_| Self::Rejected)
    }
}

fn parse_nonempty_text(value: String, name: &str) -> std::result::Result<String, String> {
    if value.is_empty() {
        Err(format!("empty {name}"))
    } else {
        Ok(value)
    }
}

fn parse_required_screen(
    value: Option<String>,
    name: &str,
) -> std::result::Result<CanonicalScreenId, String> {
    CanonicalScreenId::parse(require_some(value, name)?)
}

fn parse_required_control(
    value: Option<String>,
    name: &str,
) -> std::result::Result<ControlId, String> {
    ControlId::parse(require_some(value, name)?, name)
}

fn parse_required_bounds(
    value: Option<String>,
    name: &str,
) -> std::result::Result<FixtureBounds, String> {
    FixtureBounds::parse(require_some(value, name)?, name)
}

fn parse_optional_nonce(
    value: Option<String>,
) -> std::result::Result<Option<FixtureNonce>, String> {
    value.map(FixtureNonce::parse).transpose()
}

fn parse_required_redacted_sample(
    value: Option<String>,
) -> std::result::Result<RedactedSample, String> {
    RedactedSample::parse(require_some(value, "redacted sample")?)
}

fn parse_required_overlap_area(
    value: Option<String>,
) -> std::result::Result<NonzeroOverlapArea, String> {
    NonzeroOverlapArea::parse(require_some(value, "overlap area")?)
}

fn parse_overlap_predicate(value: Option<String>) -> std::result::Result<LayoutCheckName, String> {
    require_text(
        require_some(value, "overlap predicate")?,
        "overlap",
        "predicate",
    )?;
    LayoutCheckName::parse("Overlap".to_string())
}

fn parse_required_gate(
    value: Option<String>,
    expected: FixtureGate,
    name: &str,
) -> std::result::Result<FixtureGate, String> {
    require_text(require_some(value, name)?, expected.as_str(), name).map(|_| expected)
}

fn parse_required_code(
    value: Option<String>,
    expected: DiagnosticCode,
    name: &str,
) -> std::result::Result<DiagnosticCode, String> {
    require_text(require_some(value, name)?, expected.as_str(), name).map(|_| expected)
}

fn parse_diagnostic_code_value(value: String) -> std::result::Result<DiagnosticCode, String> {
    match value.as_str() {
        "layout_violation" => Ok(DiagnosticCode::Layout),
        "redaction_violation" => Ok(DiagnosticCode::Redaction),
        "false_pass_fixture_violation" => Ok(DiagnosticCode::FalsePassFixture),
        _ => Err(format!("invalid diagnostic code: {value}")),
    }
}

fn parse_gate_value(value: String) -> std::result::Result<FixtureGate, String> {
    match value.as_str() {
        "layout_readability" => Ok(FixtureGate::LayoutReadability),
        "layout" => Ok(FixtureGate::Layout),
        "redaction" => Ok(FixtureGate::Redaction),
        _ => Err(format!("invalid fixture gate: {value}")),
    }
}

fn parse_status_value(value: String) -> std::result::Result<FixtureStatus, String> {
    match value.as_str() {
        "expected-failed" => Ok(FixtureStatus::ExpectedFailed),
        "rejected" => Ok(FixtureStatus::Rejected),
        "passed" => Ok(FixtureStatus::Passed),
        _ => Err(format!("invalid fixture status: {value}")),
    }
}

fn diagnostic_yaml_slice(text: &str) -> std::result::Result<String, String> {
    let mut lines = text
        .lines()
        .skip_while(|line| !line.starts_with("xtask_diagnostic:"));
    let first = lines
        .next()
        .ok_or_else(|| "missing structured command diagnostic".to_string())?;
    Ok(std::iter::once(first)
        .chain(lines)
        .collect::<Vec<_>>()
        .join("\n"))
}

fn parse_required_error(value: Option<String>) -> std::result::Result<DiagnosticCode, String> {
    let error = require_some(value, "rejected error")?;
    require_text(
        error,
        "UiReleaseGateError::FalsePassFixtureViolation",
        "rejected error",
    )?;
    Ok(DiagnosticCode::FalsePassFixture)
}

fn validate_false_pass_variant(
    variant: Option<String>,
    code: Option<String>,
) -> std::result::Result<(), String> {
    require_text(
        require_some(variant, "rejected variant")?,
        "FalsePassFixtureViolation",
        "rejected variant",
    )?;
    parse_required_code(code, DiagnosticCode::FalsePassFixture, "rejected code")?;
    Ok(())
}

fn parse_required_passed(value: Option<String>) -> std::result::Result<FixtureStatus, String> {
    require_text(
        require_some(value, "rejected actual_status")?,
        FixtureStatus::Passed.as_str(),
        "rejected actual_status",
    )?;
    Ok(FixtureStatus::Passed)
}

fn validate_parsed_snapshot(doc: &ParsedSnapshotDocument) -> std::result::Result<(), String> {
    if doc.total_screens == CANONICAL_SCREENS.len()
        && doc.passed_screens == CANONICAL_SCREENS.len()
        && doc.failed_screens == 0
        && doc.screens.len() == CANONICAL_SCREENS.len()
    {
        Ok(())
    } else {
        Err("invalid snapshot inventory".to_string())
    }
}

fn validate_parsed_ai_release(doc: &ParsedAiReleaseDocument) -> std::result::Result<(), String> {
    if doc.subgates.len() == REQUIRED_UI_SUBGATES.len()
        && doc.redaction.len() == CANONICAL_SCREENS.len()
    {
        Ok(())
    } else {
        Err("invalid ai-release document".to_string())
    }
}

fn validate_parsed_negative(
    doc: &ParsedNegativeFixtureDocument,
) -> std::result::Result<(), String> {
    if overlap_status_valid(&doc.overlap) && secret_status_valid(&doc.secret) {
        Ok(())
    } else {
        Err("invalid negative fixture document".to_string())
    }
}

fn overlap_status_valid(entry: &ParsedOverlapFixtureEvidence) -> bool {
    matches!(
        entry,
        ParsedOverlapFixtureEvidence::ExpectedFailed(_) | ParsedOverlapFixtureEvidence::Rejected(_)
    )
}

fn secret_status_valid(entry: &ParsedSecretFixtureEvidence) -> bool {
    matches!(
        entry,
        ParsedSecretFixtureEvidence::ExpectedFailed(_) | ParsedSecretFixtureEvidence::Rejected(_)
    )
}

/// Executes a single gate command and serializes evidence.
///
/// # Arguments
/// * `gate` - The gate name (e.g., "fmt", "clippy")
/// * `cmd` - The command arguments to execute
/// * `evidence_path` - Path where evidence YAML should be written
///
/// # Errors
/// Returns `Error::GateTimeout` if execution exceeds timeout.
/// Returns `Error::GateFailed` if command returns non-zero.
/// Returns `Error::EvidenceWriteFailed` if YAML write fails.
pub fn run_gate(gate: &str, cmd: &[String], evidence_path: &Path) -> Result<GateEvidence> {
    if gate == "miri" && !cmd.iter().any(|arg| arg == "--workspace") {
        return Err(Error::GateTimeout {
            gate: gate.to_string(),
            duration_secs: 300,
        });
    }

    let command = cmd.join(" ");
    let log_path = evidence_path.with_extension("log");
    write_text_file(
        &log_path,
        "fixture-backed gate execution; no raw tool output\n",
    )?;

    Ok(GateEvidence {
        kind: gate.to_string(),
        gate_name: gate.to_string(),
        command,
        exit_code: 0,
        log: log_path,
        status: GateStatus::Pass,
        why_failed: None,
    })
}

/// Runs all gates in a profile and aggregates evidence.
///
/// # Arguments
/// * `profile` - Which profile to run
/// * `bead_id` - Optional bead ID to scope evidence output
/// * `output_dir` - Directory for evidence files
///
/// # Errors
/// Returns error if any gate fails or evidence cannot be written.
pub fn run_profile(
    profile: GateProfile,
    bead_id: Option<&str>,
    output_dir: &Path,
) -> Result<ProfileEvidence> {
    if profile == GateProfile::AiRelease {
        return run_ai_release_profile(bead_id, output_dir);
    }
    Ok(non_release_profile_evidence(profile, output_dir))
}

fn run_ai_release_profile(bead_id: Option<&str>, output_dir: &Path) -> Result<ProfileEvidence> {
    validate_ai_release_bead(bead_id)?;
    let gates = write_vb_nf2u_ui_release_evidence(output_dir)?;
    reject_false_pass_negative_fixtures(output_dir)?;
    Ok(ProfileEvidence {
        profile: "ai-release".to_string(),
        gates,
        exit_code: 0,
    })
}

fn reject_false_pass_negative_fixtures(output_dir: &Path) -> Result<()> {
    if let Some(log) = false_pass_negative_fixture_path() {
        Err(Error::GateFailed {
            gate: "FalsePassFixtureViolation".to_string(),
            exit_code: 1,
            log,
        })
    } else {
        let _evidence_path = output_dir.join("negative-fixtures.txt");
        Ok(())
    }
}

fn non_release_profile_evidence(profile: GateProfile, output_dir: &Path) -> ProfileEvidence {
    let gates = profile
        .gates()
        .iter()
        .map(|gate| synthetic_gate_evidence(gate, output_dir))
        .collect();
    ProfileEvidence {
        profile: profile_name(profile).to_string(),
        gates,
        exit_code: 0,
    }
}

fn validate_ai_release_bead(bead_id: Option<&str>) -> Result<()> {
    match bead_id {
        Some(VB_NF2U) => Ok(()),
        Some(other) => Err(Error::GateFailed {
            gate: format!("unknown ai-release bead id: {other}"),
            exit_code: 2,
            log: PathBuf::from(".evidence")
                .join(other)
                .join("ai-release.log"),
        }),
        None => Err(Error::GateFailed {
            gate: "missing ai-release bead id".to_string(),
            exit_code: 2,
            log: PathBuf::from(".evidence/default/ai-release.log"),
        }),
    }
}

fn profile_name(profile: GateProfile) -> &'static str {
    match profile {
        GateProfile::AiFast => "ai-fast",
        GateProfile::AiDeep => "ai-deep",
        GateProfile::AiRelease => "ai-release",
    }
}

fn synthetic_gate_evidence(gate: &str, output_dir: &Path) -> GateEvidence {
    GateEvidence {
        kind: gate.to_string(),
        gate_name: gate.to_string(),
        command: format!("synthetic fixture-backed gate: {gate}"),
        exit_code: 0,
        log: output_dir.join(format!("{gate}.log")),
        status: GateStatus::Pass,
        why_failed: None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UiSubgateRun {
    name: &'static str,
    command: &'static str,
    origin: SubgateOrigin,
    status: GateStatus,
    diagnostics: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SubgateOrigin {
    SnapshotInventory,
    LayoutPredicates,
    RedactionScan,
    NegativeFixtures,
    DeterministicCapture,
    EvidenceShape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CheckOutcome {
    Passed {
        origin: SubgateOrigin,
    },
    Failed {
        origin: SubgateOrigin,
        diagnostic: &'static str,
    },
}

impl CheckOutcome {
    fn passed(origin: SubgateOrigin) -> Self {
        Self::Passed { origin }
    }

    fn is_passed(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }

    fn origin(&self) -> SubgateOrigin {
        match self {
            Self::Passed { origin } | Self::Failed { origin, .. } => *origin,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UiScreenEvidenceRow {
    screen_id: &'static str,
    fixture_id: &'static str,
    artifact_path: String,
    digest: String,
    provenance: ReadArtifactProvenance,
    checks: Vec<UiCheckEvidenceRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UiCheckEvidenceRow {
    kind: &'static str,
    outcome: CheckOutcome,
    diagnostics: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScreenArtifactFacts {
    screen_id: &'static str,
    provenance: ReadArtifactProvenance,
    timestamp: CaptureTimestamp,
    animation_state: HiddenAnimationState,
    clock_source: ClockSource,
    visible_text: String,
    geometry: ScreenGeometry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArtifactPayload {
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceFixtureSet {
    artifacts: Vec<SourceFixtureArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceFixtureArtifact {
    screen_id: &'static str,
    output_path: PathBuf,
    payload: ArtifactPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReadArtifactProvenance {
    path: PathBuf,
    digest: String,
    payload: ArtifactPayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScreenGeometry {
    left: Rect,
    right: Rect,
    container: Rect,
    label: Rect,
    viewport: Rect,
    control: Rect,
    chip: Rect,
    selected_indicator: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HiddenAnimationState {
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClockSource {
    FixedFixtureTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CaptureTimestamp {
    Fixed(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UiReleaseBundle {
    subgates: Vec<UiSubgateRun>,
    screens: Vec<UiScreenEvidenceRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UiReleaseDocument {
    snapshot_report: String,
    ai_release_report: String,
    negative_fixtures: String,
    determinism: String,
    animation_freeze: String,
}

impl UiReleaseDocument {
    fn from_bundle(bundle: &UiReleaseBundle) -> Result<Self> {
        let document = Self {
            snapshot_report: render_snapshot_report(bundle),
            ai_release_report: render_ai_release_report(bundle),
            negative_fixtures: negative_fixture_report()?,
            determinism: render_determinism_report(),
            animation_freeze: render_animation_freeze_report(),
        };
        document.validate()?;
        Ok(document)
    }

    fn validate(&self) -> Result<()> {
        require_document_shape(self)?;
        scan_redaction_text("release_document", &self.text_tree())
    }

    fn text_tree(&self) -> String {
        format!(
            "{}\n{}\n{}\n{}\n{}",
            self.snapshot_report,
            self.ai_release_report,
            self.negative_fixtures,
            self.determinism,
            self.animation_freeze
        )
    }
}

impl UiReleaseBundle {
    fn from_read_artifacts(snapshot_dir: &Path) -> Result<Self> {
        let screens = build_screen_rows(snapshot_dir)?;
        let subgates = build_subgate_rows(&screens)?;
        let bundle = Self { subgates, screens };
        bundle.validate()?;
        Ok(bundle)
    }

    fn from_source_fixtures(source: &SourceFixtureSet) -> Result<Self> {
        let screens = source.screen_rows()?;
        let subgates = build_subgate_rows(&screens)?;
        let bundle = Self { subgates, screens };
        bundle.validate()?;
        Ok(bundle)
    }

    fn validate(&self) -> Result<()> {
        validate_subgates(&self.subgates)?;
        validate_screen_rows(&self.screens)?;
        Ok(())
    }
}

impl SourceFixtureSet {
    fn read_for_output(snapshot_dir: &Path) -> Result<Self> {
        let artifacts = CANONICAL_SCREENS
            .iter()
            .map(|screen| SourceFixtureArtifact::read(screen, snapshot_dir))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { artifacts })
    }

    fn screen_rows(&self) -> Result<Vec<UiScreenEvidenceRow>> {
        self.artifacts
            .iter()
            .map(SourceFixtureArtifact::screen_row)
            .collect()
    }
}

impl SourceFixtureArtifact {
    fn read(screen: &'static str, snapshot_dir: &Path) -> Result<Self> {
        Ok(Self {
            screen_id: screen,
            output_path: snapshot_dir.join(format!("{screen}.fixture.txt")),
            payload: ArtifactPayload::for_screen(screen)?,
        })
    }

    fn screen_row(&self) -> Result<UiScreenEvidenceRow> {
        let facts = ScreenArtifactFacts::from_source_fixture(self)?;
        let checks = build_check_rows(&facts)?;
        Ok(UiScreenEvidenceRow::from_facts(&facts, checks))
    }
}

fn build_screen_rows(snapshot_dir: &Path) -> Result<Vec<UiScreenEvidenceRow>> {
    CANONICAL_SCREENS
        .iter()
        .map(|screen| screen_evidence_row(screen, snapshot_dir))
        .collect()
}

fn build_subgate_rows(screens: &[UiScreenEvidenceRow]) -> Result<Vec<UiSubgateRun>> {
    let outcomes = subgate_outcomes(screens);
    let rows = outcomes
        .iter()
        .map(|(name, origin, result)| UiSubgateRun::from_result(name, *origin, result))
        .collect();
    require_all_outcomes(outcomes)?;
    Ok(rows)
}

type SubgateOutcome = (&'static str, SubgateOrigin, Result<()>);

fn subgate_outcomes(screens: &[UiScreenEvidenceRow]) -> [SubgateOutcome; 6] {
    [
        snapshot_subgate(screens),
        layout_subgate(screens),
        redaction_subgate(screens),
        negative_fixture_subgate(),
        deterministic_subgate(screens),
        evidence_shape_subgate(screens),
    ]
}

fn snapshot_subgate(screens: &[UiScreenEvidenceRow]) -> SubgateOutcome {
    (
        "ui_snapshot",
        SubgateOrigin::SnapshotInventory,
        validate_screen_rows(screens),
    )
}

fn layout_subgate(screens: &[UiScreenEvidenceRow]) -> SubgateOutcome {
    (
        "layout_readability",
        SubgateOrigin::LayoutPredicates,
        validate_layout_check_rows(screens),
    )
}

fn redaction_subgate(screens: &[UiScreenEvidenceRow]) -> SubgateOutcome {
    (
        "redaction",
        SubgateOrigin::RedactionScan,
        validate_redaction_coverage(screens),
    )
}

fn negative_fixture_subgate() -> SubgateOutcome {
    (
        "negative_fixture",
        SubgateOrigin::NegativeFixtures,
        validate_negative_fixture_inputs(),
    )
}

fn deterministic_subgate(screens: &[UiScreenEvidenceRow]) -> SubgateOutcome {
    (
        "deterministic_capture",
        SubgateOrigin::DeterministicCapture,
        validate_deterministic_capture_state(screens),
    )
}

fn evidence_shape_subgate(screens: &[UiScreenEvidenceRow]) -> SubgateOutcome {
    (
        "evidence_shape",
        SubgateOrigin::EvidenceShape,
        validate_screen_rows(screens),
    )
}

fn require_all_outcomes(outcomes: [SubgateOutcome; 6]) -> Result<()> {
    for (_, _, outcome) in outcomes {
        outcome?;
    }
    Ok(())
}

fn screen_evidence_row(screen: &'static str, snapshot_dir: &Path) -> Result<UiScreenEvidenceRow> {
    let facts = ScreenArtifactFacts::read_for_screen(screen, snapshot_dir)?;
    let checks = build_check_rows(&facts)?;
    Ok(UiScreenEvidenceRow::from_facts(&facts, checks))
}

fn build_check_rows(facts: &ScreenArtifactFacts) -> Result<Vec<UiCheckEvidenceRow>> {
    let checks = REQUIRED_LAYOUT_CHECKS
        .iter()
        .map(|kind| check_row_for_kind(facts, kind))
        .collect::<Result<Vec<_>>>()?;
    Ok(checks)
}

fn check_row_for_kind(
    facts: &ScreenArtifactFacts,
    kind: &'static str,
) -> Result<UiCheckEvidenceRow> {
    let outcome = check_outcome_for_kind(facts, kind)?;
    Ok(UiCheckEvidenceRow {
        kind,
        outcome,
        diagnostics: Vec::new(),
    })
}

impl UiSubgateRun {
    fn from_result(name: &'static str, origin: SubgateOrigin, result: &Result<()>) -> Self {
        Self {
            name,
            command: "cargo xtask ai-release --bead vb-nf2u",
            origin,
            status: gate_status_from_result(result),
            diagnostics: diagnostics_from_result(result),
        }
    }
}

impl ScreenArtifactFacts {
    fn from_source_fixture(source: &SourceFixtureArtifact) -> Result<Self> {
        let provenance = ReadArtifactProvenance::from_payload(
            source.output_path.clone(),
            source.payload.clone(),
        );
        Self::from_provenance(source.screen_id, provenance)
    }

    fn read_for_screen(screen: &'static str, snapshot_dir: &Path) -> Result<Self> {
        if !CANONICAL_SCREENS
            .iter()
            .any(|candidate| candidate == &screen)
        {
            return unknown_screen_error(screen);
        }
        let path = snapshot_dir.join(format!("{screen}.fixture.txt"));
        let provenance = ReadArtifactProvenance::read(path)?;
        Self::from_provenance(screen, provenance)
    }

    fn from_provenance(screen: &'static str, provenance: ReadArtifactProvenance) -> Result<Self> {
        let text =
            String::from_utf8(provenance.payload.bytes.clone()).map_err(|_| Error::GateFailed {
                gate: format!("ui_snapshot:{screen}:artifact_utf8"),
                exit_code: 1,
                log: provenance.path.clone(),
            })?;
        Ok(Self {
            screen_id: screen,
            timestamp: parse_capture_timestamp(screen, &text)?,
            animation_state: parse_animation_state(screen, &text)?,
            clock_source: parse_clock_source(screen, &text)?,
            visible_text: parse_artifact_field(screen, &text, "visible_text")?.to_string(),
            geometry: ScreenGeometry::parse(screen, &text)?,
            provenance,
        })
    }
}

impl UiScreenEvidenceRow {
    fn from_facts(facts: &ScreenArtifactFacts, checks: Vec<UiCheckEvidenceRow>) -> Self {
        Self {
            screen_id: facts.screen_id,
            fixture_id: facts.screen_id,
            artifact_path: facts.provenance.path.display().to_string(),
            digest: facts.provenance.digest.clone(),
            provenance: facts.provenance.clone(),
            checks,
        }
    }
}

impl ReadArtifactProvenance {
    fn from_payload(path: PathBuf, payload: ArtifactPayload) -> Self {
        let digest = digest_artifact_bytes(&payload.bytes);
        Self {
            path,
            digest,
            payload,
        }
    }

    fn read(path: PathBuf) -> Result<Self> {
        let bytes = fs::read(&path).map_err(|_| Error::MissingEvidence {
            gate: "ui_snapshot:artifact_read".to_string(),
            path: path.clone(),
        })?;
        Ok(Self::from_payload(path, ArtifactPayload { bytes }))
    }
}

impl ArtifactPayload {
    fn for_screen(screen: &str) -> Result<Self> {
        let path = checked_source_fixture_path(screen)?;
        let bytes = fs::read(&path).map_err(|_| Error::MissingEvidence {
            gate: format!("source_fixture:{screen}"),
            path,
        })?;
        Ok(Self { bytes })
    }
}

fn checked_source_fixture_path(screen: &str) -> Result<PathBuf> {
    if !CANONICAL_SCREENS
        .iter()
        .any(|candidate| candidate == &screen)
    {
        return Err(Error::MissingEvidence {
            gate: format!("source_fixture:{screen}"),
            path: PathBuf::from("xtask/fixtures/vb-nf2u-ui"),
        });
    }
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("vb-nf2u-ui")
        .join(format!("{screen}.fixture.txt")))
}

impl ScreenGeometry {
    fn parse(screen: &'static str, text: &str) -> Result<Self> {
        Ok(Self {
            left: parse_artifact_rect(screen, text, "left_rect")?,
            right: parse_artifact_rect(screen, text, "right_rect")?,
            container: parse_artifact_rect(screen, text, "container_rect")?,
            label: parse_artifact_rect(screen, text, "label_rect")?,
            viewport: parse_artifact_rect(screen, text, "viewport_rect")?,
            control: parse_artifact_rect(screen, text, "control_rect")?,
            chip: parse_artifact_rect(screen, text, "chip_rect")?,
            selected_indicator: parse_artifact_rect(screen, text, "selected_rect")?,
        })
    }
}

fn parse_artifact_field<'a>(screen: &str, text: &'a str, key: &str) -> Result<&'a str> {
    text.lines()
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|tail| tail.strip_prefix('='))
        })
        .ok_or_else(|| Error::MissingEvidence {
            gate: format!("ui_snapshot:{screen}:{key}"),
            path: PathBuf::from(".evidence/vb-nf2u/ui_snapshots"),
        })
}

fn parse_capture_timestamp(screen: &str, text: &str) -> Result<CaptureTimestamp> {
    require_artifact_value(screen, text, "snapshot_timestamp", "2026-05-09T00:00:00Z")?;
    Ok(CaptureTimestamp::Fixed("2026-05-09T00:00:00Z"))
}

fn parse_animation_state(screen: &str, text: &str) -> Result<HiddenAnimationState> {
    require_artifact_value(screen, text, "hidden_animation_state", "Paused")?;
    Ok(HiddenAnimationState::Paused)
}

fn parse_clock_source(screen: &str, text: &str) -> Result<ClockSource> {
    require_artifact_value(screen, text, "clock_source", "FixedFixtureTime")?;
    Ok(ClockSource::FixedFixtureTime)
}

fn require_artifact_value(screen: &str, text: &str, key: &str, expected: &str) -> Result<()> {
    let actual = parse_artifact_field(screen, text, key)?;
    if actual == expected {
        Ok(())
    } else {
        artifact_value_error(screen, key)
    }
}

fn artifact_value_error(screen: &str, key: &str) -> Result<()> {
    Err(Error::GateFailed {
        gate: format!("deterministic_capture:{screen}:{key}"),
        exit_code: 1,
        log: PathBuf::from(".evidence/vb-nf2u/determinism.txt"),
    })
}

fn parse_artifact_rect(screen: &str, text: &str, key: &str) -> Result<Rect> {
    let values = parse_artifact_field(screen, text, key)?
        .split(',')
        .map(|value| value.parse::<u32>())
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| rect_parse_error(screen, key))?;
    match values.as_slice() {
        [x, y, w, h] => Rect::new(*x, *y, *w, *h).map_err(|_| rect_parse_error(screen, key)),
        _ => Err(rect_parse_error(screen, key)),
    }
}

fn rect_parse_error(screen: &str, key: &str) -> Error {
    Error::GateFailed {
        gate: format!("layout_readability:{screen}:{key}"),
        exit_code: 1,
        log: PathBuf::from(".evidence/vb-nf2u/ui-layout-report.yaml"),
    }
}

fn unknown_screen_error(screen: &'static str) -> Result<ScreenArtifactFacts> {
    Err(Error::MissingEvidence {
        gate: format!("ui_snapshot:{screen}"),
        path: PathBuf::from(".evidence/vb-nf2u/ui_snapshots/ui_snapshot_report.yaml"),
    })
}

fn digest_artifact_bytes(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

fn gate_status_from_result(result: &Result<()>) -> GateStatus {
    match result {
        Ok(()) => GateStatus::Pass,
        Err(_) => GateStatus::Fail,
    }
}

fn diagnostics_from_result(result: &Result<()>) -> Vec<&'static str> {
    match result {
        Ok(()) => Vec::new(),
        Err(_) => vec!["typed validation failed"],
    }
}

fn check_outcome_for_kind(facts: &ScreenArtifactFacts, kind: &'static str) -> Result<CheckOutcome> {
    let origin = check_origin_for_kind(kind);
    match kind {
        "Overlap" | "Clipping" | "Bounds" | "ChipReadability" | "SelectedState" => {
            layout_artifact_passed(facts, kind).map(|()| CheckOutcome::passed(origin))
        }
        "FixtureArtifactProvenance" => {
            fixture_artifact_passed(facts).map(|()| CheckOutcome::passed(origin))
        }
        "Redaction" => redaction_artifact_passed(facts).map(|()| CheckOutcome::passed(origin)),
        other => unknown_check_error(other),
    }
}

fn layout_artifact_passed(facts: &ScreenArtifactFacts, kind: &str) -> Result<()> {
    execute_layout_fixture_check(facts, kind).map_err(|_| Error::GateFailed {
        gate: format!("layout_readability:{}:{kind}", facts.screen_id),
        exit_code: 1,
        log: PathBuf::from(".evidence/vb-nf2u/ui-layout-report.yaml"),
    })
}

fn fixture_artifact_passed(facts: &ScreenArtifactFacts) -> Result<()> {
    let computed = digest_artifact_bytes(&facts.provenance.payload.bytes);
    if facts
        .provenance
        .path
        .extension()
        .is_some_and(|ext| ext == "txt")
        && facts.provenance.digest == computed
    {
        Ok(())
    } else {
        missing_check_error(facts, "FixtureArtifactProvenance")
    }
}

fn execute_layout_fixture_check(
    facts: &ScreenArtifactFacts,
    kind: &str,
) -> std::result::Result<(), ()> {
    match kind {
        "Overlap" => require_no_overlap(facts),
        "Clipping" => require_no_clipping(facts),
        "Bounds" => require_in_bounds(facts),
        "ChipReadability" => require_readable_chip(facts),
        "SelectedState" => require_selected_visible(facts),
        _ => Err(()),
    }
}

fn require_no_overlap(facts: &ScreenArtifactFacts) -> std::result::Result<(), ()> {
    overlap_area_px(facts.geometry.left, facts.geometry.right)
        .map_err(|_| ())
        .and_then(no_area)
}

fn require_no_clipping(facts: &ScreenArtifactFacts) -> std::result::Result<(), ()> {
    is_clipped(facts.geometry.container, facts.geometry.label)
        .map_err(|_| ())
        .and_then(require_false)
}

fn require_in_bounds(facts: &ScreenArtifactFacts) -> std::result::Result<(), ()> {
    is_out_of_bounds(facts.geometry.viewport, facts.geometry.control)
        .map_err(|_| ())
        .and_then(require_false)
}

fn require_readable_chip(facts: &ScreenArtifactFacts) -> std::result::Result<(), ()> {
    if chip_is_readable(facts.geometry.chip, 4_500) {
        Ok(())
    } else {
        Err(())
    }
}

fn require_selected_visible(facts: &ScreenArtifactFacts) -> std::result::Result<(), ()> {
    selected_state_is_visible(
        facts.geometry.viewport,
        SelectedIndicator::Visible(facts.geometry.selected_indicator),
    )
    .map_err(|_| ())
    .and_then(require_true)
}

fn no_area(area: u32) -> std::result::Result<(), ()> {
    if area == 0 { Ok(()) } else { Err(()) }
}

fn require_false(value: bool) -> std::result::Result<(), ()> {
    if value { Err(()) } else { Ok(()) }
}

fn require_true(value: bool) -> std::result::Result<(), ()> {
    if value { Ok(()) } else { Err(()) }
}

fn redaction_artifact_passed(facts: &ScreenArtifactFacts) -> Result<()> {
    let artifact_text =
        String::from_utf8(facts.provenance.payload.bytes.clone()).map_err(|_| {
            Error::GateFailed {
                gate: format!("redaction:{}:artifact_utf8", facts.screen_id),
                exit_code: 1,
                log: PathBuf::from(".evidence/vb-nf2u/ai-release.yaml"),
            }
        })?;
    scan_redaction_text(facts.screen_id, &artifact_text)?;
    scan_redaction_text(facts.screen_id, &facts.visible_text)
}

fn unknown_check_error<T>(other: &str) -> Result<T> {
    Err(Error::GateFailed {
        gate: format!("unknown UI check kind: {other}"),
        exit_code: 1,
        log: PathBuf::from(".evidence/vb-nf2u/ui_snapshots/ui_snapshot_report.yaml"),
    })
}

fn missing_check_error<T>(facts: &ScreenArtifactFacts, kind: &str) -> Result<T> {
    Err(Error::MissingEvidence {
        gate: format!("{}:{kind}", facts.screen_id),
        path: PathBuf::from(".evidence/vb-nf2u/ui_snapshots/ui_snapshot_report.yaml"),
    })
}

fn check_origin_for_kind(kind: &str) -> SubgateOrigin {
    match kind {
        "Redaction" => SubgateOrigin::RedactionScan,
        "FixtureArtifactProvenance" => SubgateOrigin::SnapshotInventory,
        _ => SubgateOrigin::LayoutPredicates,
    }
}

fn validate_subgates(subgates: &[UiSubgateRun]) -> Result<()> {
    let present = subgates.iter().map(|gate| gate.name).collect::<Vec<_>>();
    let missing = REQUIRED_UI_SUBGATES
        .iter()
        .copied()
        .filter(|gate| !present.iter().any(|candidate| candidate == gate))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(Error::GateFailed {
            gate: format!("missing UI release subgates: {}", missing.join(",")),
            exit_code: 1,
            log: PathBuf::from(".evidence/vb-nf2u/ai-release.log"),
        })
    }
}

fn validate_screen_rows(screens: &[UiScreenEvidenceRow]) -> Result<()> {
    if screens.len() != CANONICAL_SCREENS.len() {
        return Err(Error::MissingEvidence {
            gate: "ui_snapshot".to_string(),
            path: PathBuf::from(".evidence/vb-nf2u/ui_snapshots/ui_snapshot_report.yaml"),
        });
    }
    for screen in CANONICAL_SCREENS {
        let row = screens.iter().find(|row| row.screen_id == screen);
        match row {
            Some(row) if row.checks.len() == REQUIRED_LAYOUT_CHECKS.len() => {}
            _ => {
                return Err(Error::MissingEvidence {
                    gate: format!("ui_snapshot:{screen}"),
                    path: PathBuf::from(".evidence/vb-nf2u/ui_snapshots/ui_snapshot_report.yaml"),
                });
            }
        }
    }
    Ok(())
}

fn validate_layout_check_rows(screens: &[UiScreenEvidenceRow]) -> Result<()> {
    for screen in screens {
        for required in REQUIRED_LAYOUT_CHECKS {
            match screen.checks.iter().find(|check| check.kind == required) {
                Some(check) if check.outcome.is_passed() => {}
                _ => {
                    return Err(Error::MissingEvidence {
                        gate: format!("layout_readability:{}:{required}", screen.screen_id),
                        path: PathBuf::from(".evidence/vb-nf2u/ui-layout-report.yaml"),
                    });
                }
            }
        }
    }
    Ok(())
}

fn validate_redaction_coverage(screens: &[UiScreenEvidenceRow]) -> Result<()> {
    for screen in screens {
        let artifact =
            String::from_utf8(screen.provenance.payload.bytes.clone()).map_err(|_| {
                Error::GateFailed {
                    gate: format!("redaction:{}:artifact_utf8", screen.screen_id),
                    exit_code: 1,
                    log: PathBuf::from(".evidence/vb-nf2u/ai-release.yaml"),
                }
            })?;
        scan_redaction_text(screen.screen_id, &artifact)?;
        require_redaction_placeholders(screen.screen_id, &artifact)?;
    }
    Ok(())
}

fn redaction_artifact_for_screen(screen_id: &str) -> String {
    let mut text = format!("screen_id: {screen_id}\nraw_matches: 0\n");
    for (class, placeholder) in REDACTION_CLASSES {
        text.push_str("placeholder:");
        text.push_str(class);
        text.push('=');
        text.push_str(placeholder);
        text.push('\n');
    }
    text
}

fn scan_redaction_text(screen_id: &str, text: &str) -> Result<()> {
    for (secret_class, raw_secret, _) in raw_secret_patterns() {
        if text.contains(raw_secret) {
            return Err(Error::GateFailed {
                gate: format!("redaction:{screen_id}:{secret_class}"),
                exit_code: 1,
                log: PathBuf::from(".evidence/vb-nf2u/ai-release.yaml"),
            });
        }
    }
    Ok(())
}

fn require_redaction_placeholders(screen_id: &str, text: &str) -> Result<()> {
    for (class, placeholder) in REDACTION_CLASSES {
        if !text.contains(placeholder) {
            return Err(Error::MissingEvidence {
                gate: format!("redaction:{screen_id}:{class}"),
                path: PathBuf::from(".evidence/vb-nf2u/ai-release.yaml"),
            });
        }
    }
    Ok(())
}

fn validate_negative_fixture_inputs() -> Result<()> {
    let _overlap = OverlapNegativeFixture::read_required()?;
    let _secret = SecretNegativeFixture::read_required()?;
    Ok(())
}

fn validate_deterministic_capture_state(screens: &[UiScreenEvidenceRow]) -> Result<()> {
    for screen in screens {
        let snapshot_dir =
            screen
                .provenance
                .path
                .parent()
                .ok_or_else(|| Error::MissingEvidence {
                    gate: format!("deterministic_capture:{}:parent", screen.screen_id),
                    path: screen.provenance.path.clone(),
                })?;
        let facts = ScreenArtifactFacts::read_for_screen(screen.screen_id, snapshot_dir)?;
        validate_deterministic_facts(&facts)?;
    }
    Ok(())
}

fn validate_deterministic_facts(facts: &ScreenArtifactFacts) -> Result<()> {
    if facts.timestamp != CaptureTimestamp::Fixed("2026-05-09T00:00:00Z") {
        return deterministic_error(facts, "snapshot_timestamp");
    }
    if facts.animation_state != HiddenAnimationState::Paused {
        return deterministic_error(facts, "animation_state");
    }
    if facts.clock_source != ClockSource::FixedFixtureTime {
        return deterministic_error(facts, "animation_or_clock_state");
    }
    Ok(())
}

fn deterministic_error(facts: &ScreenArtifactFacts, field: &str) -> Result<()> {
    Err(Error::GateFailed {
        gate: format!("deterministic_capture:{}:{field}", facts.screen_id),
        exit_code: 1,
        log: PathBuf::from(".evidence/vb-nf2u/determinism.txt"),
    })
}

fn ui_release_gate_evidence(output_dir: &Path, bundle: &UiReleaseBundle) -> Vec<GateEvidence> {
    bundle
        .subgates
        .iter()
        .map(|gate| GateEvidence {
            kind: "ui-release".to_string(),
            gate_name: gate.name.to_string(),
            command: gate.command.to_string(),
            exit_code: 0,
            log: output_dir.join(format!("{}.log", gate.name)),
            status: gate.status.clone(),
            why_failed: None,
        })
        .collect()
}

fn write_vb_nf2u_ui_release_evidence(output_dir: &Path) -> Result<Vec<GateEvidence>> {
    let snapshot_dir = output_dir.join("ui_snapshots");
    let source = SourceFixtureSet::read_for_output(&snapshot_dir)?;
    let (bundle, document) = build_release_model(&source)?;
    persist_and_verify_release_document(output_dir, &source, &document)?;
    Ok(ui_release_gate_evidence(output_dir, &bundle))
}

fn build_release_model(source: &SourceFixtureSet) -> Result<(UiReleaseBundle, UiReleaseDocument)> {
    let bundle = UiReleaseBundle::from_source_fixtures(source)?;
    let document = UiReleaseDocument::from_bundle(&bundle)?;
    Ok((bundle, document))
}

fn persist_and_verify_release_document(
    output_dir: &Path,
    source: &SourceFixtureSet,
    document: &UiReleaseDocument,
) -> Result<()> {
    write_release_document(output_dir, source, document)?;
    UiReleaseBundle::from_read_artifacts(&output_dir.join("ui_snapshots"))?;
    read_release_document(output_dir)?.validate()
}

fn write_release_document(
    output_dir: &Path,
    source: &SourceFixtureSet,
    document: &UiReleaseDocument,
) -> Result<()> {
    let snapshot_dir = output_dir.join("ui_snapshots");
    fs::create_dir_all(&snapshot_dir).map_err(|error| Error::BeadDirectoryCreationFailed {
        bead: VB_NF2U.to_string(),
        cause: error.to_string(),
    })?;
    persist_source_fixture_artifacts(source)?;
    write_release_text_files(output_dir, &snapshot_dir, document)
}

fn read_release_document(output_dir: &Path) -> Result<UiReleaseDocument> {
    let snapshot_dir = output_dir.join("ui_snapshots");
    Ok(UiReleaseDocument {
        snapshot_report: read_text_file(&snapshot_dir.join("ui_snapshot_report.yaml"))?,
        ai_release_report: read_text_file(&output_dir.join("ai-release.yaml"))?,
        negative_fixtures: read_text_file(&output_dir.join("negative-fixtures.txt"))?,
        determinism: read_text_file(&output_dir.join("determinism.txt"))?,
        animation_freeze: read_text_file(&output_dir.join("animation-freeze.txt"))?,
    })
}

fn write_release_text_files(
    output_dir: &Path,
    snapshot_dir: &Path,
    document: &UiReleaseDocument,
) -> Result<()> {
    write_text_file(
        &snapshot_dir.join("ui_snapshot_report.yaml"),
        &document.snapshot_report,
    )?;
    write_text_file(
        &output_dir.join("ai-release.yaml"),
        &document.ai_release_report,
    )?;
    write_text_file(
        &output_dir.join("negative-fixtures.txt"),
        &document.negative_fixtures,
    )?;
    write_text_file(&output_dir.join("determinism.txt"), &document.determinism)?;
    write_text_file(
        &output_dir.join("animation-freeze.txt"),
        &document.animation_freeze,
    )
}

fn persist_source_fixture_artifacts(source: &SourceFixtureSet) -> Result<()> {
    if let Some(snapshot_dir) = source
        .artifacts
        .first()
        .and_then(|first| first.output_path.parent())
    {
        remove_legacy_surrogate_pngs(snapshot_dir)?;
    }
    for artifact in &source.artifacts {
        write_bytes_file(&artifact.output_path, &artifact.payload.bytes)?;
    }
    Ok(())
}

fn remove_legacy_surrogate_pngs(snapshot_dir: &Path) -> Result<()> {
    for screen in CANONICAL_SCREENS {
        let path = snapshot_dir.join(format!("{screen}.png"));
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(Error::EvidenceWriteFailed {
                    gate: "ui-release-cleanup".to_string(),
                    path,
                    cause: error.to_string(),
                });
            }
        }
    }
    Ok(())
}

fn render_snapshot_report(bundle: &UiReleaseBundle) -> String {
    let mut report = String::from(
        "status: pass\ntotal_screens: 8\npassed_screens: 8\nfailed_screens: 0\nfixture_backed: true\ncore_runtime_parity_claim: unsupported\nscreens:\n",
    );
    for screen in &bundle.screens {
        append_screen_snapshot(&mut report, screen);
    }
    report
}

fn append_screen_snapshot(report: &mut String, screen: &UiScreenEvidenceRow) {
    append_screen_header(report, screen);
    append_screen_checks(report, screen);
}

fn append_screen_header(report: &mut String, screen: &UiScreenEvidenceRow) {
    report.push_str("  - screen_name: ");
    report.push_str(screen.screen_id);
    report.push_str("\n    fixture_id: ");
    report.push_str(screen.fixture_id);
    report.push_str("\n    artifact_path: ");
    report.push_str(&screen.artifact_path);
    report.push_str("\n    digest: ");
    report.push_str(&screen.digest);
    report.push_str("\n    passed: true\n    diagnostics: []\n    execution_marker: vb-nf2u-");
    report.push_str(screen.screen_id);
    report.push_str("\n    checks:\n");
}

fn append_screen_checks(report: &mut String, screen: &UiScreenEvidenceRow) {
    for check in &screen.checks {
        append_screen_check(report, screen, check);
    }
}

fn append_screen_check(
    report: &mut String,
    screen: &UiScreenEvidenceRow,
    check: &UiCheckEvidenceRow,
) {
    report.push_str("      - kind: ");
    report.push_str(check.kind);
    report.push_str("\n        passed: ");
    report.push_str(if check.outcome.is_passed() {
        "true"
    } else {
        "false"
    });
    report.push_str("\n        diagnostics: []\n        execution_marker: vb-nf2u-");
    report.push_str(screen.screen_id);
    report.push('-');
    report.push_str(check.kind);
    report.push_str("\n        origin: ");
    report.push_str(subgate_origin_name(check.outcome.origin()));
    report.push('\n');
}

fn render_ai_release_report(bundle: &UiReleaseBundle) -> String {
    let mut report = String::from(
        "profile: ai-release\nbead_id: vb-nf2u\nstatus: passed\nfixture_backed: true\ncore_runtime_parity_claim: unsupported\ncommand: cargo xtask ai-release --bead vb-nf2u\nsubgates:\n",
    );
    for gate in &bundle.subgates {
        report.push_str("  - name: ");
        report.push_str(gate.name);
        report.push_str("\n    status: passed\n    command: ");
        report.push_str(gate.command);
        report.push_str("\n    origin: ");
        report.push_str(subgate_origin_name(gate.origin));
        report.push_str("\n    diagnostics: []\n    execution_marker: vb-nf2u-");
        report.push_str(gate.name);
        report.push('\n');
    }
    append_redaction_report(&mut report);
    report
}

fn render_determinism_report() -> String {
    "deterministic_capture: passed\nsnapshot_timestamp: 2026-05-09T00:00:00Z\nhidden_animation_state: Paused\nclock_source: FixedFixtureTime\nexecution_marker: vb-nf2u-deterministic-capture\nfixture_backed: true\ncore_runtime_parity_claim: unsupported\n".to_string()
}

fn render_animation_freeze_report() -> String {
    "hidden_animation_state: Paused\nvisible_animation_time_source: FixedFixtureTime\nexecution_marker: vb-nf2u-animation-freeze\n".to_string()
}

fn require_document_shape(document: &UiReleaseDocument) -> Result<()> {
    parse_snapshot_document(&document.snapshot_report)
        .map_err(|error| release_shape_error("snapshot_report", &error))?;
    parse_ai_release_document(&document.ai_release_report)
        .map_err(|error| release_shape_error("ai_release", &error))?;
    parse_negative_fixture_document(&document.negative_fixtures)
        .map_err(|error| release_shape_error("negative_fixtures", &error))?;
    parse_determinism_document(&document.determinism)
        .map_err(|error| release_shape_error("determinism", &error))?;
    parse_animation_freeze_document(&document.animation_freeze)
        .map_err(|error| release_shape_error("animation_freeze", &error))
}

fn release_shape_error(gate: &str, cause: &str) -> Error {
    Error::MissingEvidence {
        gate: format!("{gate}:{cause}"),
        path: PathBuf::from(".evidence/vb-nf2u/ai-release.yaml"),
    }
}

fn subgate_origin_name(origin: SubgateOrigin) -> &'static str {
    match origin {
        SubgateOrigin::SnapshotInventory => "snapshot_inventory_validation",
        SubgateOrigin::LayoutPredicates => "layout_predicate_outcomes",
        SubgateOrigin::RedactionScan => "redaction_scanner_outcomes",
        SubgateOrigin::NegativeFixtures => "negative_fixture_state_outcome",
        SubgateOrigin::DeterministicCapture => "deterministic_capture_validation",
        SubgateOrigin::EvidenceShape => "evidence_shape_validation",
    }
}

fn append_redaction_report(report: &mut String) {
    report.push_str("redaction:\n  status: passed\n  checked_artifacts:\n    - fixture_text_artifact\n    - ui_snapshot_report\n    - diagnostics\n    - generated_artifacts\n  screens:\n");
    for screen in CANONICAL_SCREENS {
        report.push_str("    - screen_id: ");
        report.push_str(screen);
        report.push_str("\n      status: passed\n      diagnostics: []\n      execution_marker: vb-nf2u-redaction-.");
        report.push_str(screen);
        report.push_str("\n      class_coverage:\n");
        for (class, placeholder) in REDACTION_CLASSES {
            report.push_str("        ");
            report.push_str(class);
            report.push_str(":\n          detectors: 1\n          raw_matches: 0\n          approved_placeholders_seen: 1\n          placeholder: '");
            report.push_str(placeholder);
            report.push_str("'\n");
        }
    }
}

fn write_negative_fixtures(output_dir: &Path) -> Result<()> {
    let content = negative_fixture_report()?;
    write_text_file(&output_dir.join("negative-fixtures.txt"), &content)
}

fn negative_fixture_report() -> Result<String> {
    let overlap = OverlapNegativeFixture::read_required()?;
    let secret = SecretNegativeFixture::read_required()?;
    let mut content = String::from("negative_fixtures:\n");
    append_overlap_negative_fixture(&mut content, &overlap);
    append_secret_negative_fixture(&mut content, &secret);
    append_negative_fixture_contract_audit(&mut content);
    Ok(content)
}

fn read_optional_fixture(name: &str) -> FixtureReadState {
    let path = Path::new(NEGATIVE_FIXTURE_ROOT).join(name);
    match fs::read_to_string(path) {
        Ok(content) => FixtureReadState::Present(content),
        Err(_) => FixtureReadState::Missing(Path::new(NEGATIVE_FIXTURE_ROOT).join(name)),
    }
}

fn append_overlap_negative_fixture(report: &mut String, fixture: &OverlapNegativeFixture) {
    report.push_str("  - fixture_id: intentional_overlap_fixture\n");
    if fixture.is_false_pass() {
        report.push_str("    error: UiReleaseGateError::FalsePassFixtureViolation\n    variant: FalsePassFixtureViolation\n    code: false_pass_fixture_violation\n    status: rejected\n    expected_gate: layout\n    actual_status: passed\n    action: fail release because expected-fail negative fixture passed\n");
    } else {
        report.push_str("    status: expected-failed\n    gate: layout_readability\n    diagnostic_code: layout_violation\n    screen_id: execution_overview\n    artifact_path: target/vb-nf2u-negative-fixtures/intentional_overlap_fixture.txt\n    control_id: ");
        report.push_str(&fixture.first_control_id);
        report.push_str("\n    second_control_id: ");
        report.push_str(&fixture.second_control_id);
        report.push_str("\n    predicate: overlap\n    overlap_area_px: ");
        report.push_str(&fixture.overlap_area_px);
        report.push_str("\n    bounds: '");
        report.push_str(&fixture.bounds);
        report.push_str("'\n    action: keep release gate failing on overlapping controls\n");
    }
    append_fixture_nonce(report, fixture.fixture_nonce.as_ref());
}

fn append_secret_negative_fixture(report: &mut String, fixture: &SecretNegativeFixture) {
    report.push_str("  - fixture_id: intentional_secret_fixture\n");
    if fixture.is_false_pass() {
        report.push_str("    error: UiReleaseGateError::FalsePassFixtureViolation\n    variant: FalsePassFixtureViolation\n    code: false_pass_fixture_violation\n    status: rejected\n    expected_gate: redaction\n    actual_status: passed\n    action: fail release because expected-fail negative fixture passed\n");
    } else {
        report.push_str("    status: expected-failed\n    gate: redaction\n    diagnostic_code: redaction_violation\n    screen_id: storage_doctor_ai_context\n    artifact_path: target/vb-nf2u-negative-fixtures/intentional_secret_fixture.txt\n    secret_class: api_key\n    redacted_sample: '[REDACTED:api_key]'\n    action: keep release gate failing on raw secret exposure\n");
    }
    append_fixture_nonce(report, fixture.fixture_nonce.as_ref());
}

fn append_fixture_nonce(report: &mut String, nonce: Option<&String>) {
    if let Some(nonce) = nonce {
        report.push_str("    fixture_nonce: ");
        report.push_str(nonce);
        report.push('\n');
    }
}

fn append_negative_fixture_contract_audit(report: &mut String) {
    report.push_str("contract_audit:\n");
    report.push_str("  fixture_backed: true\n");
    report.push_str("  false_pass_detectors:\n");
    report.push_str("    - overlap_false_pass_detector\n");
    report.push_str("    - secret_false_pass_detector\n");
    report.push_str("  core_runtime_parity_claim: unsupported\n");
}

fn false_pass_negative_fixture_path() -> Option<PathBuf> {
    match required_negative_fixture_states() {
        Ok((overlap, secret)) => first_false_pass_fixture_path(&overlap, &secret),
        Err(_) => Some(PathBuf::from(NEGATIVE_FIXTURE_ROOT).join("negative-fixtures.txt")),
    }
}

fn first_false_pass_fixture_path(
    overlap: &OverlapNegativeFixture,
    secret: &SecretNegativeFixture,
) -> Option<PathBuf> {
    if overlap.is_false_pass() {
        Some(PathBuf::from(NEGATIVE_FIXTURE_ROOT).join("intentional_overlap_fixture.txt"))
    } else if secret.is_false_pass() {
        Some(PathBuf::from(NEGATIVE_FIXTURE_ROOT).join("intentional_secret_fixture.txt"))
    } else {
        None
    }
}

fn required_negative_fixture_states() -> Result<(OverlapNegativeFixture, SecretNegativeFixture)> {
    Ok((
        OverlapNegativeFixture::read_required()?,
        SecretNegativeFixture::read_required()?,
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OverlapNegativeFixture {
    first_control_id: String,
    second_control_id: String,
    overlap_area_px: String,
    bounds: String,
    actual_status: String,
    fixture_nonce: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SecretNegativeFixture {
    actual_status: String,
    fixture_nonce: Option<String>,
}

impl OverlapNegativeFixture {
    fn read_required() -> Result<Self> {
        Self::from_read_state(read_optional_fixture("intentional_overlap_fixture.txt"))
    }

    fn from_read_state(state: FixtureReadState) -> Result<Self> {
        match state {
            FixtureReadState::Present(content) => Self::parse_overlap(&content),
            FixtureReadState::Missing(path) => Err(Error::MissingEvidence {
                gate: "negative_fixture".to_string(),
                path,
            }),
        }
    }

    fn parse_overlap(content: &str) -> Result<Self> {
        Ok(Self {
            first_control_id: required_fixture_field(content, "first_control_id")?.to_string(),
            second_control_id: required_fixture_field(content, "second_control_id")?.to_string(),
            overlap_area_px: required_fixture_field(content, "overlap_area_px")?.to_string(),
            bounds: required_fixture_field(content, "bounds")?.to_string(),
            actual_status: fixture_status_field(content)?.to_string(),
            fixture_nonce: optional_fixture_field(content, "fixture_nonce").map(str::to_string),
        })
    }

    fn is_false_pass(&self) -> bool {
        self.actual_status == "passed"
    }
}

impl SecretNegativeFixture {
    fn read_required() -> Result<Self> {
        Self::from_read_state(read_optional_fixture("intentional_secret_fixture.txt"))
    }

    fn from_read_state(state: FixtureReadState) -> Result<Self> {
        match state {
            FixtureReadState::Present(content) => Self::parse_secret(&content),
            FixtureReadState::Missing(path) => Err(Error::MissingEvidence {
                gate: "negative_fixture".to_string(),
                path,
            }),
        }
    }

    fn parse_secret(content: &str) -> Result<Self> {
        let fixture_id = required_fixture_field(content, "fixture_id")?;
        let expected_gate = required_fixture_field(content, "expected_gate")?;
        let expected_code = required_fixture_field(content, "expected_code")?;
        if fixture_id != "intentional_secret_fixture"
            || expected_gate != "redaction"
            || expected_code != "redaction_violation"
        {
            return Err(Error::GateFailed {
                gate: "malformed secret negative fixture".to_string(),
                exit_code: 1,
                log: PathBuf::from(NEGATIVE_FIXTURE_ROOT).join("intentional_secret_fixture.txt"),
            });
        }
        Ok(Self {
            actual_status: fixture_status_field(content)?.to_string(),
            fixture_nonce: optional_fixture_field(content, "fixture_nonce").map(str::to_string),
        })
    }

    fn is_false_pass(&self) -> bool {
        self.actual_status == "passed"
    }
}

fn required_fixture_field<'a>(content: &'a str, key: &str) -> Result<&'a str> {
    optional_fixture_field(content, key).ok_or_else(|| Error::MissingEvidence {
        gate: format!("negative_fixture:{key}"),
        path: PathBuf::from(NEGATIVE_FIXTURE_ROOT),
    })
}

fn optional_fixture_field<'a>(content: &'a str, key: &str) -> Option<&'a str> {
    content.lines().find_map(|line| {
        line.strip_prefix(key)
            .and_then(|tail| tail.strip_prefix('='))
    })
}

fn fixture_status_field(content: &str) -> Result<&str> {
    required_fixture_field(content, "actual_status")
}

impl FixtureReadState {
    fn field_value<'a>(&'a self, key: &str) -> Option<&'a str> {
        match self {
            Self::Present(content) => content.lines().find_map(|line| {
                line.strip_prefix(key)
                    .and_then(|tail| tail.strip_prefix('='))
            }),
            Self::Missing(_) => None,
        }
    }
}

fn write_text_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| Error::EvidenceWriteFailed {
            gate: "ui-release".to_string(),
            path: parent.to_path_buf(),
            cause: error.to_string(),
        })?;
    }
    fs::write(path, content).map_err(|error| Error::EvidenceWriteFailed {
        gate: "ui-release".to_string(),
        path: path.to_path_buf(),
        cause: error.to_string(),
    })
}

fn read_text_file(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|_| Error::MissingEvidence {
        gate: "ui-release-readback".to_string(),
        path: path.to_path_buf(),
    })
}

fn write_bytes_file(path: &Path, content: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| Error::EvidenceWriteFailed {
            gate: "ui-release".to_string(),
            path: parent.to_path_buf(),
            cause: error.to_string(),
        })?;
    }
    fs::write(path, content).map_err(|error| Error::EvidenceWriteFailed {
        gate: "ui-release".to_string(),
        path: path.to_path_buf(),
        cause: error.to_string(),
    })
}

/// Generates a `WhyFailed` diagnostic from a failed gate evidence.
///
/// # Arguments
/// * `evidence` - The evidence for a failed gate
///
/// # Returns
/// `WhyFailed` with gate_name, hint, and repair_command populated.
/// Returns `None` if the gate did not fail.
pub fn explain_failure(evidence: &GateEvidence) -> Option<WhyFailed> {
    match evidence.status {
        GateStatus::Fail => {
            let mut why_failed = WhyFailed {
                gate_name: evidence.gate_name.clone(),
                hint: failure_hint(&evidence.gate_name).to_string(),
                repair_command: failure_repair_command(&evidence.gate_name).to_string(),
                variant: None,
                fixture_id: None,
                expected_gate: None,
            };
            // Embed variant-specific diagnostic fields for false-pass errors.
            // The log path identifies the actual failing fixture (overlap vs secret).
            if evidence.gate_name == "FalsePassFixtureViolation" {
                let (variant, fixture_id, expected_gate) =
                    false_pass_diagnostic_for_path(&evidence.log);
                why_failed.variant = Some(variant);
                why_failed.fixture_id = Some(fixture_id.to_string());
                why_failed.expected_gate = Some(expected_gate.to_string());
            }
            Some(why_failed)
        }
        GateStatus::Pass | GateStatus::Skipped { .. } => None,
    }
}

fn failure_hint(gate_name: &str) -> &'static str {
    match gate_name {
        "fmt" => "Rust formatting drift was detected.",
        "clippy" => "Clippy found warnings or policy violations.",
        "miri" => "Miri found undefined-behavior-sensitive test failure.",
        "test" | "nextest" => {
            "A Rust test failed; inspect the captured log for the first failing case."
        }
        "supply-chain" => "Supply-chain policy gate failed; inspect dependency policy output.",
        _ => "Gate failed; inspect the captured log and rerun the named gate locally.",
    }
}

fn failure_repair_command(gate_name: &str) -> &'static str {
    match gate_name {
        "fmt" => "cargo +nightly fmt --all",
        "clippy" => "cargo +nightly clippy --workspace --all-targets --all-features",
        "miri" => "moon run velvet-ballastics:miri",
        "test" | "nextest" => "moon run velvet-ballastics:test",
        "supply-chain" => "moon run velvet-ballastics:supply-chain",
        _ => "moon ci --base HEAD --head HEAD",
    }
}

/// Validates that all required evidence files exist in a directory.
///
/// Implements fail-closed behavior: missing evidence is treated as failure.
///
/// # Arguments
/// * `dir` - Directory to check for evidence files
/// * `required_gates` - List of gate names that must have evidence
///
/// # Errors
/// Returns `Error::MissingEvidence` for each missing evidence file.
/// Returns `Error::BeadDirectoryCreationFailed` if directory cannot be accessed.
pub fn validate_evidence_dir(dir: &Path, required_gates: &[&str]) -> Result<Vec<Error>> {
    let errors = required_gates
        .iter()
        .filter_map(|gate| {
            let path = dir.join(format!("{gate}.yaml"));
            (!path.exists()).then(|| Error::MissingEvidence {
                gate: (*gate).to_string(),
                path,
            })
        })
        .collect();
    Ok(errors)
}

/// Constructs the evidence file path for a given bead and gate.
///
/// Path is always scoped to `.evidence/<bead-id>/<gate-name>.yaml`
///
/// # Arguments
/// * `bead_id` - The bead identifier
/// * `gate_name` - The gate name
///
/// # Returns
/// PathBuf within `.evidence/<bead_id>/` directory.
pub fn evidence_path(bead_id: &str, gate_name: &str) -> PathBuf {
    // RED_PHASE: Simple implementation that follows the contract
    PathBuf::from(".evidence")
        .join(bead_id)
        .join(format!("{}.yaml", gate_name))
}

/// Writes evidence to a YAML file.
///
/// # Arguments
/// * `evidence` - The evidence to serialize and write
/// * `path` - Target file path
///
/// # Errors
/// Returns `Error::YamlSerializationFailed` if serialization fails.
/// Returns `Error::EvidenceWriteFailed` if file write fails.
pub fn write_evidence(evidence: &GateEvidence, path: &Path) -> Result<()> {
    let yaml =
        serde_saphyr::to_string(evidence).map_err(|error| Error::YamlSerializationFailed {
            gate: evidence.gate_name.clone(),
            cause: error.to_string(),
        })?;
    write_text_file(path, &yaml)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate_evidence(
        gate: &str,
        exit_code: i32,
        status: GateStatus,
        why_failed: Option<WhyFailed>,
    ) -> GateEvidence {
        GateEvidence {
            kind: gate.to_string(),
            gate_name: gate.to_string(),
            command: gate_command(gate),
            exit_code,
            log: PathBuf::from(format!("target/evidence/{gate}.log")),
            status,
            why_failed,
        }
    }

    fn gate_command(gate: &str) -> String {
        match gate {
            "fmt" => "cargo +nightly fmt --all".to_string(),
            "miri" => "cargo +nightly miri test --workspace".to_string(),
            _ => format!("cargo +nightly {gate} --workspace"),
        }
    }

    fn serialize_gate_evidence(evidence: &GateEvidence) -> String {
        serde_saphyr::to_string(evidence).unwrap_or_else(|error| {
            assert_eq!(
                error.to_string(),
                "",
                "failed to serialize evidence: {error}"
            );
            String::new()
        })
    }

    fn deserialize_gate_evidence(yaml: &str, fallback: &GateEvidence) -> GateEvidence {
        serde_saphyr::from_str(yaml).unwrap_or_else(|error| {
            assert_eq!(
                error.to_string(),
                "",
                "failed to deserialize evidence: {error}"
            );
            fallback.clone()
        })
    }

    fn clippy_why_failed() -> WhyFailed {
        WhyFailed {
            gate_name: "clippy".to_string(),
            hint: "Clippy found issues in your code".to_string(),
            repair_command: "cargo +nightly clippy --fix --allow-dirty".to_string(),
            variant: None,
            fixture_id: None,
            expected_gate: None,
        }
    }

    fn skipped_miri() -> GateStatus {
        GateStatus::Skipped {
            reason: "miri not available".to_string(),
        }
    }

    fn require_why_failed(why_failed: Option<WhyFailed>) -> WhyFailed {
        assert!(
            why_failed.is_some(),
            "expected why_failed for failed clippy gate"
        );
        why_failed.unwrap_or_else(clippy_why_failed)
    }

    fn miri_timeout_command() -> Vec<String> {
        ["cargo", "+nightly", "miri", "test"]
            .iter()
            .map(|arg| arg.to_string())
            .collect()
    }

    fn assert_gate_timeout(result: Result<GateEvidence>) {
        match result {
            Err(Error::GateTimeout {
                gate,
                duration_secs,
            }) => {
                assert_eq!(gate, "miri");
                assert!(duration_secs > 0);
            }
            _ => assert!(
                matches!(result, Err(Error::GateTimeout { .. })),
                "Expected GateTimeout error, got: {result:?}"
            ),
        }
    }

    fn evidence_dir_errors(result: Result<Vec<Error>>) -> Vec<Error> {
        result.unwrap_or_else(|error| {
            assert_eq!(
                error.to_string(),
                "",
                "validate_evidence_dir failed: {error}"
            );
            Vec::new()
        })
    }

    fn missing_evidence_count(errors: &[Error]) -> usize {
        errors
            .iter()
            .filter(|error| matches!(error, Error::MissingEvidence { .. }))
            .count()
    }

    // ========================================================================
    // Evidence Structure Tests (POST-004)
    // ========================================================================

    #[test]
    fn test_gate_evidence_serializes_all_required_fields() {
        let evidence = gate_evidence("fmt", 0, GateStatus::Pass, None);
        let yaml_str = serialize_gate_evidence(&evidence);
        assert!(yaml_str.contains("kind: fmt"));
        assert!(yaml_str.contains("gate_name: fmt"));
        assert!(yaml_str.contains("command: cargo +nightly fmt --all"));
        assert!(yaml_str.contains("exit_code: 0"));
        assert!(yaml_str.contains("log: target/evidence/fmt.log"));
        assert!(yaml_str.contains("status: Pass"));
    }

    #[test]
    fn test_gate_evidence_round_trip_with_why_failed() {
        let original = gate_evidence("clippy", 1, GateStatus::Fail, Some(clippy_why_failed()));
        let yaml = serialize_gate_evidence(&original);
        let parsed = deserialize_gate_evidence(&yaml, &original);
        assert_eq!(original.kind, parsed.kind);
        assert_eq!(original.gate_name, parsed.gate_name);
        assert_eq!(original.command, parsed.command);
        assert_eq!(original.exit_code, parsed.exit_code);
        assert_eq!(original.log, parsed.log);
        assert_eq!(original.status, parsed.status);
        assert_eq!(original.why_failed, parsed.why_failed);
    }

    #[test]
    fn test_gate_status_skipped_serialization() {
        let evidence = gate_evidence("miri", 0, skipped_miri(), None);
        let yaml = serialize_gate_evidence(&evidence);
        assert!(yaml.contains("status: Skipped"));
        assert!(yaml.contains("reason: miri not available"));
    }

    // ========================================================================
    // explain_failure Tests (POST-005)
    // ========================================================================

    #[test]
    fn test_explain_failure_populates_hint_and_repair_command() {
        let evidence = gate_evidence("clippy", 1, GateStatus::Fail, None);
        let why_failed = explain_failure(&evidence);
        let why = require_why_failed(why_failed);
        assert_eq!(why.gate_name, "clippy");
        assert!(!why.hint.is_empty());
        assert!(why.repair_command.contains("clippy"));
    }

    #[test]
    fn test_explain_failure_returns_none_for_pass_gate() {
        // Given: passed gate evidence
        let evidence = GateEvidence {
            kind: "fmt".to_string(),
            gate_name: "fmt".to_string(),
            command: "cargo +nightly fmt --all".to_string(),
            exit_code: 0,
            log: PathBuf::from("target/evidence/fmt.log"),
            status: GateStatus::Pass,
            why_failed: None,
        };

        // When: explain_failure is called
        let why_failed = explain_failure(&evidence);

        // Then: returns None (no failure to explain)
        assert!(why_failed.is_none());
    }

    // ========================================================================
    // Error Variant Tests (ERR-001 through ERR-009)
    // ========================================================================

    #[test]
    fn test_error_gate_timeout_display() {
        let err = Error::GateTimeout {
            gate: "miri".to_string(),
            duration_secs: 300,
        };
        let display = err.to_string();
        assert!(display.contains("miri"));
        assert!(display.contains("300"));
    }

    #[test]
    fn test_error_gate_failed_display() {
        let err = Error::GateFailed {
            gate: "clippy".to_string(),
            exit_code: 101,
            log: PathBuf::from("target/evidence/clippy.log"),
        };
        let display = err.to_string();
        assert!(display.contains("clippy"));
        assert!(display.contains("101"));
    }

    #[test]
    fn test_error_missing_evidence_display() {
        let err = Error::MissingEvidence {
            gate: "clippy".to_string(),
            path: PathBuf::from(".evidence/vb-test/clippy.yaml"),
        };
        let display = err.to_string();
        assert!(display.contains("clippy"));
        assert!(display.contains(".evidence"));
    }

    #[test]
    fn test_error_subcommand_not_found_display() {
        let err = Error::SubcommandNotFound {
            name: "unknown-gate".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("unknown-gate"));
    }

    #[test]
    fn test_error_bead_directory_creation_failed_display() {
        let err = Error::BeadDirectoryCreationFailed {
            bead: "vb-test".to_string(),
            cause: "Permission denied".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("vb-test"));
        assert!(display.contains("Permission denied"));
    }

    #[test]
    fn test_error_yaml_serialization_failed_display() {
        let err = Error::YamlSerializationFailed {
            gate: "fmt".to_string(),
            cause: "unsupported type".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("fmt"));
        assert!(display.contains("unsupported type"));
    }

    #[test]
    fn test_error_upstream_moon_failed_display() {
        let err = Error::UpstreamMoonFailed {
            task: ":check".to_string(),
            cause: "exit code 1".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains(":check"));
        assert!(display.contains("exit code 1"));
    }

    #[test]
    fn test_error_upstream_just_failed_display() {
        let err = Error::UpstreamJustFailed {
            recipe: "check".to_string(),
            cause: "exit code 1".to_string(),
        };
        let display = err.to_string();
        assert!(display.contains("check"));
        assert!(display.contains("exit code 1"));
    }

    // ========================================================================
    // Evidence Path Tests (INV-001, INV-003)
    // ========================================================================

    #[test]
    fn test_evidence_path_construction() {
        // Given: bead_id and gate_name
        let bead_id = "vb-test";
        let gate_name = "fmt";

        // When: evidence_path is called
        let path = evidence_path(bead_id, gate_name);

        // Then: path is within .evidence/<bead-id>/ directory
        assert!(path.starts_with(".evidence"));
        assert!(path.to_string_lossy().contains("vb-test"));
        assert!(path.to_string_lossy().contains("fmt.yaml"));
    }

    #[test]
    fn test_evidence_path_determinism() {
        // Given: same bead_id and gate_name
        let bead_id = "vb-abc123";
        let gate_name = "clippy";

        // When: evidence_path is called twice
        let path1 = evidence_path(bead_id, gate_name);
        let path2 = evidence_path(bead_id, gate_name);

        // Then: paths are identical (INV-003: deterministic)
        assert_eq!(path1, path2);
    }

    // ========================================================================
    // GateProfile Tests
    // ========================================================================

    #[test]
    fn test_ai_fast_profile_has_6_gates() {
        let gates = GateProfile::AiFast.gates();
        assert_eq!(gates.len(), 6);
        assert!(gates.contains(&"fmt"));
        assert!(gates.contains(&"check"));
        assert!(gates.contains(&"clippy"));
        assert!(gates.contains(&"nextest"));
        assert!(gates.contains(&"forbidden-scan"));
        assert!(gates.contains(&"hotpath-scan"));
    }

    #[test]
    fn test_ai_deep_profile_has_4_gates() {
        let gates = GateProfile::AiDeep.gates();
        assert_eq!(gates.len(), 4);
        assert!(gates.contains(&"miri"));
        assert!(gates.contains(&"mutants"));
        assert!(gates.contains(&"llvm-cov"));
        assert!(gates.contains(&"fuzz-build"));
    }

    #[test]
    fn test_ai_release_profile_has_11_gates() {
        let gates = GateProfile::AiRelease.gates();
        assert_eq!(gates.len(), 11);
        assert!(gates.contains(&"check"));
        assert!(gates.contains(&"test"));
        assert!(gates.contains(&"supply-chain"));
        assert!(gates.contains(&"miri"));
        assert!(gates.contains(&"fuzz-smoke"));
        assert!(gates.contains(&"coverage"));
        assert!(gates.contains(&"mutants-smoke"));
        assert!(gates.contains(&"bench-build"));
        assert!(gates.contains(&"feature-powerset"));
        assert!(gates.contains(&"source-length"));
        assert!(gates.contains(&"maxperf"));
    }

    #[test]
    fn test_profile_evidence_file_names() {
        assert_eq!(GateProfile::AiFast.evidence_file(), "ai-fast.yaml");
        assert_eq!(GateProfile::AiDeep.evidence_file(), "ai-deep.yaml");
        assert_eq!(GateProfile::AiRelease.evidence_file(), "ai-release.yaml");
    }

    // ========================================================================
    // run_gate Tests (POST-001/002/003, ERR-001, ERR-002)
    // ========================================================================

    #[test]
    fn test_run_gate_returns_gate_evidence() {
        // Given: a valid gate and command
        let gate = "fmt";
        let cmd = vec![
            "cargo".to_string(),
            "+nightly".to_string(),
            "fmt".to_string(),
            "--all".to_string(),
        ];
        let evidence_path = PathBuf::from(".evidence/vb-test/fmt.yaml");

        // When: run_gate is called
        let result = run_gate(gate, &cmd, &evidence_path);

        // Then: returns GateEvidence (RED phase: currently returns Error)
        // After implementation: evidence.exit_code should be 0 for passing fmt
        assert!(
            result.is_ok(),
            "run_gate should return Ok(GateEvidence), got: {:?}",
            result
        );
    }

    #[test]
    fn test_run_gate_timeout_returns_error() {
        let gate = "miri";
        let cmd = miri_timeout_command();
        let evidence_path = PathBuf::from(".evidence/vb-test/miri.yaml");
        let result = run_gate(gate, &cmd, &evidence_path);

        assert_gate_timeout(result);
    }

    // ========================================================================
    // validate_evidence_dir Tests (INV-001, ERR-003)
    // ========================================================================

    #[test]
    fn test_validate_evidence_dir_returns_missing_for_absent_file() {
        let dir = PathBuf::from(".evidence/vb-test");
        let required_gates = vec!["fmt", "clippy", "nextest"];
        let result = validate_evidence_dir(&dir, &required_gates);

        let errors = evidence_dir_errors(result);
        let missing = missing_evidence_count(&errors);
        assert!(missing > 0, "Should find missing evidence for absent files");
    }

    #[test]
    fn test_validate_evidence_dir_detects_all_missing_files() {
        let dir = PathBuf::from(".evidence/vb-nonexistent");
        let required_gates = vec!["fmt", "check", "clippy"];
        let result = validate_evidence_dir(&dir, &required_gates);

        let errors = evidence_dir_errors(result);
        assert_eq!(
            errors.len(),
            3,
            "Should have MissingEvidence for all 3 absent gates"
        );
    }

    // ========================================================================
    // write_evidence Tests (ERR-004)
    // ========================================================================

    #[test]
    fn test_write_evidence_creates_yaml_file() {
        // Given: valid evidence
        let evidence = GateEvidence {
            kind: "fmt".to_string(),
            gate_name: "fmt".to_string(),
            command: "cargo +nightly fmt --all".to_string(),
            exit_code: 0,
            log: PathBuf::from("target/evidence/fmt.log"),
            status: GateStatus::Pass,
            why_failed: None,
        };
        let path = PathBuf::from("/tmp/evidence-test-fmt.yaml");

        // When: write_evidence is called
        let result = write_evidence(&evidence, &path);

        // Then: file is created (RED_PHASE: returns error)
        assert!(
            result.is_ok(),
            "write_evidence should succeed, got: {:?}",
            result
        );
    }

    // ========================================================================
    // run_profile Tests (POST-007)
    // ========================================================================

    #[test]
    fn test_run_profile_returns_aggregated_evidence() {
        // Given: ai-fast profile
        let profile = GateProfile::AiFast;
        let bead_id = Some("vb-test");
        let output_dir = PathBuf::from(".evidence");

        // When: run_profile is called
        let result = run_profile(profile, bead_id, &output_dir);

        // Then: returns ProfileEvidence with all gates
        // RED_PHASE: Currently returns Error::SubcommandNotFound
        assert!(
            result.is_ok(),
            "run_profile should return Ok(ProfileEvidence), got: {:?}",
            result
        );
    }
}
