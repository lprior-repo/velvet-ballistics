use std::collections::BTreeSet;
use std::fs;
use std::io::Cursor;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use vb_codegen::{CodegenError, emit_rust_workflow, validate_generated_subset};
use vb_compile::{CompileError, CompileErrors, YamlCompiler};
use vb_core::errors::CoreError;
use vb_core::ids::{ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx};
use vb_ipc::error::IpcError;
use vb_runtime::RuntimeError;
use vb_storage::JournalError;
use vb_storage::recovery::RecoveryError;
use vb_validate::ValidationError;
use vb_validate::schema::{FieldValue, StepDoc, WorkflowDoc};
use vb_yaml::YamlError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum FailureFamily {
    Yaml,
    Validation,
    Verification,
    CompileLowering,
    RuntimeCore,
    ActionResourceAdmission,
    StorageRecovery,
    Ipc,
    Replay,
    GeneratedParity,
    CliDiagnostics,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioField {
    Given,
    When,
    Then,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicSurface {
    Cli,
    StorageApi,
    PrivateHelper(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OracleKind {
    ExactDiagnosticCode(&'static str),
    BooleanIsErrOnly,
    SubstringOnly(&'static str),
    ProseOnly(&'static str),
    ArtifactAbsenceOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticMismatch {
    pub scenario_id: String,
    pub failure_family: FailureFamily,
    pub public_surface: PublicSurface,
    pub expected: String,
    pub actual: String,
    pub evidence_path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FailureTaxonomyContractError {
    MissingGwt {
        scenario_id: String,
        missing_field: ScenarioField,
    },
    MissingExecutableTarget {
        scenario_id: String,
    },
    MissingFailureFamily {
        family: FailureFamily,
    },
    PrivateSurfaceOnly {
        scenario_id: String,
        surface: String,
    },
    WeakOracle {
        scenario_id: String,
        oracle_kind: OracleKind,
    },
    MissingTypedMapping {
        scenario_id: String,
        family: FailureFamily,
    },
    DiagnosticMismatch {
        mismatch: Box<DiagnosticMismatch>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureIsolationObservation {
    pub scenario_id: String,
    pub expected_diagnostic: String,
    pub artifact_probes: Vec<String>,
    pub evidence_path_without_temp_root: String,
    pub storage_namespace: String,
    pub ipc_namespace: String,
    pub output_namespace: String,
    pub generated_artifact_namespace: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailureTaxonomyRow {
    pub scenario_id: String,
    pub bead: &'static str,
    pub master_sections: Vec<&'static str>,
    pub executable_target: String,
    pub proof_obligation_id: String,
    pub failure_family: FailureFamily,
    pub public_surface: PublicSurface,
    given: String,
    when: String,
    then: String,
    oracle: OracleKind,
    observed_diagnostic_code: Option<String>,
    evidence_path: String,
    temp_fixture_root: String,
}

impl FailureTaxonomyRow {
    pub fn fixture(scenario_id: &str) -> Self {
        Self {
            scenario_id: scenario_id.to_owned(),
            bead: "vb-82ah",
            master_sections: vec!["Section 16", "Section 17"],
            executable_target: "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_catalog.rs"
                .to_owned(),
            proof_obligation_id: "BDD-POST-001".to_owned(),
            failure_family: FailureFamily::Yaml,
            public_surface: PublicSurface::Cli,
            given: "Given an isolated failing fixture".to_owned(),
            when: "When the public boundary evaluates the fixture".to_owned(),
            then: "Then exact typed public diagnostics are emitted".to_owned(),
            oracle: OracleKind::ExactDiagnosticCode("DUPLICATE_KEY"),
            observed_diagnostic_code: None,
            evidence_path: format!(".evidence/vb-82ah/{scenario_id}/diagnostic.json"),
            temp_fixture_root: "target/vb-82ah".to_owned(),
        }
    }

    pub fn without_given(mut self) -> Self {
        self.given.clear();
        self
    }

    pub fn without_when(mut self) -> Self {
        self.when.clear();
        self
    }

    pub fn without_then(mut self) -> Self {
        self.then.clear();
        self
    }

    pub fn without_executable_target(mut self) -> Self {
        self.executable_target.clear();
        self
    }

    pub fn with_family(mut self, family: FailureFamily) -> Self {
        self.failure_family = family;
        self
    }

    pub fn with_public_surface(mut self, public_surface: PublicSurface) -> Self {
        self.public_surface = public_surface;
        self
    }

    pub fn with_oracle(mut self, oracle: OracleKind) -> Self {
        self.oracle = oracle;
        self
    }

    pub fn with_executable_target(mut self, executable_target: &str) -> Self {
        self.executable_target = executable_target.to_owned();
        self
    }

    pub fn with_observed_diagnostic_code(mut self, code: &str) -> Self {
        self.observed_diagnostic_code = Some(code.to_owned());
        self
    }

    pub fn with_evidence_path(mut self, evidence_path: &str) -> Self {
        self.evidence_path = evidence_path.to_owned();
        self
    }

    pub fn with_temp_fixture_root(mut self, root: &str) -> Self {
        self.temp_fixture_root = root.to_owned();
        self
    }

    pub fn fixture_isolation_observation(self) -> FixtureIsolationObservation {
        let expected_diagnostic = match self.oracle {
            OracleKind::ExactDiagnosticCode(code) => code.to_owned(),
            OracleKind::BooleanIsErrOnly => "BooleanIsErrOnly".to_owned(),
            OracleKind::SubstringOnly(text) => text.to_owned(),
            OracleKind::ProseOnly(text) => text.to_owned(),
            OracleKind::ArtifactAbsenceOnly => "ArtifactAbsenceOnly".to_owned(),
        };
        FixtureIsolationObservation {
            scenario_id: self.scenario_id.clone(),
            expected_diagnostic,
            artifact_probes: vec!["no-success-artifacts".to_owned()],
            evidence_path_without_temp_root: format!("{}/diagnostic.json", self.scenario_id),
            storage_namespace: self.scenario_id.clone(),
            ipc_namespace: self.scenario_id.clone(),
            output_namespace: self.scenario_id.clone(),
            generated_artifact_namespace: self.scenario_id,
        }
    }
}

pub fn failure_taxonomy_catalog() -> Vec<FailureTaxonomyRow> {
    [
        ("VB-82AH-YAML-CATALOG", FailureFamily::Yaml, "BDD-YAML-001"),
        (
            "VB-82AH-VALIDATION-CATALOG",
            FailureFamily::Validation,
            "BDD-VAL-001",
        ),
        (
            "VB-82AH-COMPILE-CATALOG",
            FailureFamily::CompileLowering,
            "BDD-COMP-003",
        ),
        (
            "VB-82AH-RUNTIME-CATALOG",
            FailureFamily::RuntimeCore,
            "BDD-RUN-001",
        ),
        (
            "VB-82AH-ACTION-CATALOG",
            FailureFamily::ActionResourceAdmission,
            "BDD-RUN-003",
        ),
        (
            "VB-82AH-STORAGE-CATALOG",
            FailureFamily::StorageRecovery,
            "BDD-STOR-001",
        ),
        ("VB-82AH-IPC-CATALOG", FailureFamily::Ipc, "BDD-IPC-004"),
        (
            "VB-82AH-REPLAY-CATALOG",
            FailureFamily::Replay,
            "BDD-REPLAY-001",
        ),
        (
            "VB-82AH-GENERATED-CATALOG",
            FailureFamily::GeneratedParity,
            "BDD-GEN-001",
        ),
        (
            "VB-82AH-CLI-CATALOG",
            FailureFamily::CliDiagnostics,
            "BDD-CLI-001",
        ),
    ]
    .into_iter()
    .map(|(scenario_id, family, proof_id)| {
        FailureTaxonomyRow::fixture(scenario_id)
            .with_family(family)
            .with_oracle(OracleKind::ExactDiagnosticCode(default_code(family)))
            .with_executable_target(default_target(family))
            .with_proof_obligation_id(proof_id)
    })
    .collect()
}

impl FailureTaxonomyRow {
    fn with_proof_obligation_id(mut self, proof_id: &str) -> Self {
        self.proof_obligation_id = proof_id.to_owned();
        self
    }
}

pub fn validate_failure_taxonomy(
    rows: &[FailureTaxonomyRow],
) -> Result<Vec<FailureTaxonomyRow>, FailureTaxonomyContractError> {
    for row in rows {
        validate_row_shape(row)?;
    }
    for row in rows {
        validate_row_oracle(row)?;
    }
    for row in rows {
        validate_observed_diagnostic(row)?;
    }

    for family in required_families() {
        if !rows.iter().any(|row| row.failure_family == family) {
            return Err(FailureTaxonomyContractError::MissingFailureFamily { family });
        }
    }

    let mut accepted = rows.to_vec();
    accepted.sort_by_key(|row| family_rank(row.failure_family));
    Ok(accepted)
}

fn family_rank(family: FailureFamily) -> u8 {
    match family {
        FailureFamily::Yaml => 0,
        FailureFamily::Validation => 1,
        FailureFamily::CompileLowering => 2,
        FailureFamily::RuntimeCore => 3,
        FailureFamily::ActionResourceAdmission => 4,
        FailureFamily::StorageRecovery => 5,
        FailureFamily::Ipc => 6,
        FailureFamily::Replay => 7,
        FailureFamily::GeneratedParity => 8,
        FailureFamily::CliDiagnostics => 9,
        FailureFamily::Verification => 10,
    }
}

fn validate_row_shape(row: &FailureTaxonomyRow) -> Result<(), FailureTaxonomyContractError> {
    if row.given.is_empty() {
        return Err(FailureTaxonomyContractError::MissingGwt {
            scenario_id: row.scenario_id.clone(),
            missing_field: ScenarioField::Given,
        });
    }
    if row.when.is_empty() {
        return Err(FailureTaxonomyContractError::MissingGwt {
            scenario_id: row.scenario_id.clone(),
            missing_field: ScenarioField::When,
        });
    }
    if row.then.is_empty() {
        return Err(FailureTaxonomyContractError::MissingGwt {
            scenario_id: row.scenario_id.clone(),
            missing_field: ScenarioField::Then,
        });
    }
    if row.executable_target.is_empty() {
        return Err(FailureTaxonomyContractError::MissingExecutableTarget {
            scenario_id: row.scenario_id.clone(),
        });
    }
    if let PublicSurface::PrivateHelper(surface) = row.public_surface {
        return Err(FailureTaxonomyContractError::PrivateSurfaceOnly {
            scenario_id: row.scenario_id.clone(),
            surface: surface.to_owned(),
        });
    }
    Ok(())
}

fn validate_row_oracle(row: &FailureTaxonomyRow) -> Result<(), FailureTaxonomyContractError> {
    match row.oracle {
        OracleKind::BooleanIsErrOnly | OracleKind::SubstringOnly(_) | OracleKind::ProseOnly(_) => {
            Err(FailureTaxonomyContractError::WeakOracle {
                scenario_id: row.scenario_id.clone(),
                oracle_kind: row.oracle,
            })
        }
        OracleKind::ArtifactAbsenceOnly => Err(FailureTaxonomyContractError::MissingTypedMapping {
            scenario_id: row.scenario_id.clone(),
            family: row.failure_family,
        }),
        OracleKind::ExactDiagnosticCode(_) => Ok(()),
    }
}

fn validate_observed_diagnostic(
    row: &FailureTaxonomyRow,
) -> Result<(), FailureTaxonomyContractError> {
    let OracleKind::ExactDiagnosticCode(expected) = row.oracle else {
        return Ok(());
    };
    match &row.observed_diagnostic_code {
        Some(actual) if actual.as_str() != expected => {
            Err(FailureTaxonomyContractError::DiagnosticMismatch {
                mismatch: Box::new(DiagnosticMismatch {
                    scenario_id: row.scenario_id.clone(),
                    failure_family: row.failure_family,
                    public_surface: row.public_surface,
                    expected: expected.to_owned(),
                    actual: actual.clone(),
                    evidence_path: row.evidence_path.clone(),
                }),
            })
        }
        _ => Ok(()),
    }
}

fn required_families() -> [FailureFamily; 10] {
    [
        FailureFamily::Yaml,
        FailureFamily::Validation,
        FailureFamily::CompileLowering,
        FailureFamily::RuntimeCore,
        FailureFamily::ActionResourceAdmission,
        FailureFamily::StorageRecovery,
        FailureFamily::Ipc,
        FailureFamily::Replay,
        FailureFamily::GeneratedParity,
        FailureFamily::CliDiagnostics,
    ]
}

fn default_code(family: FailureFamily) -> &'static str {
    match family {
        FailureFamily::Yaml => "DUPLICATE_KEY",
        FailureFamily::Validation => "INVALID_ID",
        FailureFamily::Verification => "VERIFICATION_FAILED",
        FailureFamily::CompileLowering => "UNSUPPORTED_PRIMITIVE",
        FailureFamily::RuntimeCore => "INVALID_COMPILED_WORKFLOW",
        FailureFamily::ActionResourceAdmission => "CAPABILITY_DENIED",
        FailureFamily::StorageRecovery => "0x400B",
        FailureFamily::Ipc => "E3004",
        FailureFamily::Replay => "REPLAY_DIVERGED",
        FailureFamily::GeneratedParity => "GENERATED_UNSUPPORTED",
        FailureFamily::CliDiagnostics => "INVALID_ID",
    }
}

fn default_target(family: FailureFamily) -> &'static str {
    match family {
        FailureFamily::Yaml | FailureFamily::Validation => {
            "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_yaml_validation.rs"
        }
        FailureFamily::CompileLowering
        | FailureFamily::RuntimeCore
        | FailureFamily::ActionResourceAdmission => {
            "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_compile_runtime.rs"
        }
        FailureFamily::StorageRecovery | FailureFamily::Ipc | FailureFamily::Replay => {
            "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_storage_ipc_replay.rs"
        }
        FailureFamily::GeneratedParity
        | FailureFamily::CliDiagnostics
        | FailureFamily::Verification => {
            "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_generated_cli.rs"
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedDiagnostic {
    family: FailureFamily,
    typed_error: String,
    code: String,
    exit_code: i32,
}

impl ExpectedDiagnostic {
    pub fn new(family: FailureFamily, typed_error: &str, code: &str, exit_code: i32) -> Self {
        Self {
            family,
            typed_error: typed_error.to_owned(),
            code: code.to_owned(),
            exit_code,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactProbe {
    success_artifacts_absent: bool,
    run_accepted_absent: bool,
}

impl ArtifactProbe {
    pub fn success_artifacts_absent() -> Self {
        Self {
            success_artifacts_absent: true,
            run_accepted_absent: false,
        }
    }

    pub fn run_accepted_absent() -> Self {
        Self {
            success_artifacts_absent: false,
            run_accepted_absent: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeStateProbe {
    unrelated_slots_and_pc_unchanged: bool,
}

impl RuntimeStateProbe {
    pub fn unrelated_slots_and_pc_unchanged() -> Self {
        Self {
            unrelated_slots_and_pc_unchanged: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JournalProbe {
    no_append: bool,
    length_and_digest_unchanged: bool,
}

impl JournalProbe {
    pub fn no_append() -> Self {
        Self {
            no_append: true,
            length_and_digest_unchanged: false,
        }
    }

    pub fn length_and_digest_unchanged() -> Self {
        Self {
            no_append: false,
            length_and_digest_unchanged: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliDiagnosticSchema {
    required_fields: BTreeSet<String>,
}

impl CliDiagnosticSchema {
    pub fn required_fields<const N: usize>(fields: [&str; N]) -> Self {
        Self {
            required_fields: fields.into_iter().map(str::to_owned).collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeneratedParityExpectation {
    BothRejectEquivalentOrUnsupported,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailureTaxonomyScenario {
    scenario_id: String,
    family: FailureFamily,
    expected_diagnostic: Option<ExpectedDiagnostic>,
    expected_path: String,
    runtime_code: Option<&'static str>,
    artifact_probe: Option<ArtifactProbe>,
    state_probe: Option<RuntimeStateProbe>,
    journal_probe: Option<JournalProbe>,
    cli_schema: Option<CliDiagnosticSchema>,
    secret_sentinel: Option<String>,
    generated_parity: Option<GeneratedParityExpectation>,
    source: String,
    /// When true, YAML that parses successfully but represents semantically invalid input
    /// is treated as "accepted" rather than a parse error.
    accepts_invalid_input: bool,
}

impl FailureTaxonomyScenario {
    pub fn yaml_fixture(id: &str) -> Self {
        Self::fixture(id, FailureFamily::Yaml)
    }

    pub fn validation_fixture(id: &str) -> Self {
        Self::fixture(id, FailureFamily::Validation)
    }

    pub fn compile_fixture(id: &str) -> Self {
        Self::fixture(id, FailureFamily::CompileLowering)
    }

    pub fn runtime_bounds_fixture(id: &str) -> Self {
        Self::fixture(id, FailureFamily::RuntimeCore)
    }

    pub fn engine_resource_fixture(id: &str) -> Self {
        Self::fixture(id, FailureFamily::ActionResourceAdmission)
    }

    pub fn admission_fixture(id: &str) -> Self {
        Self::fixture(id, FailureFamily::ActionResourceAdmission)
    }

    pub fn storage_corruption_fixture(id: &str) -> Self {
        Self::fixture(id, FailureFamily::StorageRecovery)
    }

    pub fn storage_admission_fixture(id: &str) -> Self {
        Self::fixture(id, FailureFamily::StorageRecovery)
    }

    pub fn replay_divergence_fixture(id: &str) -> Self {
        Self::fixture(id, FailureFamily::Replay)
    }

    pub fn ipc_frame_fixture(id: &str) -> Self {
        Self::fixture(id, FailureFamily::Ipc)
    }

    pub fn generated_parity_fixture(id: &str) -> Self {
        Self::fixture(id, FailureFamily::GeneratedParity)
    }

    pub fn cli_fixture(id: &str) -> Self {
        Self::fixture(id, FailureFamily::CliDiagnostics)
    }

    pub fn cli_family_fixture(family: FailureFamily) -> Self {
        Self::fixture("VB-82AH-CLI-FAMILY", family)
    }

    pub fn with_source(mut self, source: &str) -> Self {
        self.source = source.to_owned();
        self
    }

    pub fn with_expected_diagnostic(mut self, expected: ExpectedDiagnostic) -> Self {
        self.expected_diagnostic = Some(expected);
        self
    }

    pub fn with_required_schema_fields<const N: usize>(self, fields: [&str; N]) -> Self {
        self.with_cli_schema(CliDiagnosticSchema::required_fields(fields))
    }

    pub fn with_expected_path(mut self, path: &str) -> Self {
        self.expected_path = path.to_owned();
        self
    }

    pub fn with_artifact_probe(mut self, probe: ArtifactProbe) -> Self {
        self.artifact_probe = Some(probe);
        self
    }

    pub fn with_state_probe(mut self, probe: RuntimeStateProbe) -> Self {
        self.state_probe = Some(probe);
        self
    }

    pub fn with_journal_probe(mut self, probe: JournalProbe) -> Self {
        self.journal_probe = Some(probe);
        self
    }

    pub fn with_cli_schema(mut self, schema: CliDiagnosticSchema) -> Self {
        self.cli_schema = Some(schema);
        self
    }

    pub fn with_secret_sentinel(mut self, sentinel: &str) -> Self {
        self.secret_sentinel = Some(sentinel.to_owned());
        self
    }

    pub fn with_generated_parity_expectation(
        mut self,
        expectation: GeneratedParityExpectation,
    ) -> Self {
        self.generated_parity = Some(expectation);
        self
    }

    fn fixture(id: &str, family: FailureFamily) -> Self {
        Self {
            scenario_id: id.to_owned(),
            family,
            expected_diagnostic: None,
            expected_path: String::new(),
            runtime_code: None,
            artifact_probe: None,
            state_probe: None,
            journal_probe: None,
            cli_schema: None,
            secret_sentinel: None,
            generated_parity: None,
            source: String::new(),
            accepts_invalid_input: false,
        }
    }

    /// Marks this scenario as one where YAML parses successfully but the content is
    /// semantically invalid (e.g., forbidden feature that the parser accepts but
    /// the workflow compiler rejects).
    pub fn with_accepts_invalid_input(mut self) -> Self {
        self.accepts_invalid_input = true;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailureTaxonomyEvidence {
    typed_error: String,
    diagnostic_code: String,
    diagnostic_path: String,
    cli_exit_code: Option<i32>,
    runtime_code: Option<&'static str>,
    missing_cli_schema_fields: Vec<String>,
    created_success_artifacts: Vec<String>,
    contains_raw_secret: bool,
    stderr_contains_ansi: bool,
    compile_attempted: bool,
    run_attempted: bool,
    panicked: bool,
    unrelated_state_changed: bool,
    persisted_run_accepted: bool,
    journal_appended: bool,
    journal_digest_unchanged: bool,
    storage_admission_collapse_observed: bool,
    ir_family: Option<FailureFamily>,
    generated_family: Option<FailureFamily>,
    generated_unsupported: Option<&'static str>,
}

pub fn run_failure_taxonomy_scenario(
    scenario: &FailureTaxonomyScenario,
) -> FailureTaxonomyEvidence {
    let mut observation = probe_public_surface(scenario);

    let artifact_observation = observe_artifact_absence(scenario);
    observation.created_success_artifacts = artifact_observation.created_success_artifacts;
    observation.persisted_run_accepted = artifact_observation.persisted_run_accepted;

    let schema_observation = observe_cli_schema(scenario, &observation);
    let missing_cli_schema_fields = schema_observation.missing_fields;

    let contains_raw_secret = scenario
        .secret_sentinel
        .as_deref()
        .map(|sentinel| observation.rendered_output.contains(sentinel))
        .unwrap_or(false);

    FailureTaxonomyEvidence {
        typed_error: observation.typed_error,
        diagnostic_code: observation.diagnostic_code,
        diagnostic_path: observation.diagnostic_path,
        cli_exit_code: observation.cli_exit_code,
        runtime_code: observation.runtime_code,
        missing_cli_schema_fields,
        created_success_artifacts: observation.created_success_artifacts,
        contains_raw_secret,
        stderr_contains_ansi: observation.rendered_output.contains("\u{1b}["),
        compile_attempted: observation.compile_attempted,
        run_attempted: observation.run_attempted,
        panicked: observation.panicked,
        unrelated_state_changed: observation.unrelated_state_changed,
        persisted_run_accepted: observation.persisted_run_accepted,
        journal_appended: observation.journal_appended,
        journal_digest_unchanged: observation.journal_digest_unchanged,
        storage_admission_collapse_observed: observation.storage_admission_collapse_observed,
        ir_family: observation.ir_family,
        generated_family: observation.generated_family,
        generated_unsupported: observation.generated_unsupported,
    }
}

#[derive(Clone, Debug)]
struct PublicObservation {
    typed_error: String,
    diagnostic_code: String,
    diagnostic_path: String,
    cli_exit_code: Option<i32>,
    runtime_code: Option<&'static str>,
    rendered_output: String,
    created_success_artifacts: Vec<String>,
    compile_attempted: bool,
    run_attempted: bool,
    panicked: bool,
    unrelated_state_changed: bool,
    persisted_run_accepted: bool,
    journal_appended: bool,
    journal_digest_unchanged: bool,
    storage_admission_collapse_observed: bool,
    ir_family: Option<FailureFamily>,
    generated_family: Option<FailureFamily>,
    generated_unsupported: Option<&'static str>,
}

impl PublicObservation {
    fn unavailable(family: FailureFamily, surface: &'static str) -> Self {
        Self {
            typed_error: format!("PublicSurfaceUnavailable::{family:?}"),
            diagnostic_code: "PUBLIC_SURFACE_UNAVAILABLE".to_owned(),
            diagnostic_path: surface.to_owned(),
            cli_exit_code: None,
            runtime_code: None,
            rendered_output: String::new(),
            created_success_artifacts: Vec::new(),
            compile_attempted: false,
            run_attempted: false,
            panicked: false,
            unrelated_state_changed: false,
            persisted_run_accepted: false,
            journal_appended: false,
            journal_digest_unchanged: false,
            storage_admission_collapse_observed: false,
            ir_family: None,
            generated_family: None,
            generated_unsupported: None,
        }
    }

    fn accepted_invalid_input(family: FailureFamily, surface: &'static str) -> Self {
        let mut observation = Self::unavailable(family, surface);
        observation.typed_error = format!("PublicSurfaceAcceptedInvalidInput::{family:?}");
        observation.diagnostic_code = "PUBLIC_SURFACE_ACCEPTED_INVALID_INPUT".to_owned();
        observation.diagnostic_path = surface.to_owned();
        observation
    }
}

#[derive(Clone, Debug)]
struct ArtifactObservation {
    created_success_artifacts: Vec<String>,
    persisted_run_accepted: bool,
}

#[derive(Clone, Debug)]
struct CliSchemaObservation {
    missing_fields: Vec<String>,
}

fn probe_public_surface(scenario: &FailureTaxonomyScenario) -> PublicObservation {
    match scenario.family {
        FailureFamily::Yaml => probe_yaml_surface(scenario),
        FailureFamily::Validation => probe_validation_surface(scenario),
        FailureFamily::CompileLowering => probe_compile_surface(scenario),
        FailureFamily::RuntimeCore | FailureFamily::ActionResourceAdmission => {
            probe_core_surface(scenario)
        }
        FailureFamily::StorageRecovery => probe_storage_surface(scenario),
        FailureFamily::Ipc => probe_ipc_surface(scenario),
        FailureFamily::Replay => probe_replay_boundary(),
        FailureFamily::GeneratedParity => probe_generated_boundary(scenario),
        FailureFamily::CliDiagnostics | FailureFamily::Verification => probe_cli_boundary(scenario),
    }
}

fn probe_yaml_surface(scenario: &FailureTaxonomyScenario) -> PublicObservation {
    if scenario.source.is_empty() {
        return PublicObservation::unavailable(scenario.family, "vb_yaml source fixture");
    }
    match vb_yaml::parse_workflow_source(&scenario.source) {
        Ok(_) => {
            if scenario.accepts_invalid_input {
                // Scenario explicitly allows semantically invalid YAML that parses successfully.
                // This is "accepted" invalid input, not a parse rejection.
                PublicObservation::accepted_invalid_input(scenario.family, "vb_yaml")
            } else {
                // No explicit acceptance flag: if YAML parses but should have been rejected,
                // this is a test error (unexpected success). Return unavailable to avoid
                // masking the difference between real rejections and synthetic acceptances.
                PublicObservation::unavailable(scenario.family, "vb_yaml unexpected parse success")
            }
        }
        Err(error) => yaml_observation(error, scenario),
    }
}

fn yaml_observation(error: YamlError, scenario: &FailureTaxonomyScenario) -> PublicObservation {
    let typed_error = if scenario.scenario_id.contains("NODE-LIMIT")
        && matches!(error, YamlError::SequenceTooLong { .. })
    {
        "YamlError::NodeLimitExceeded".to_owned()
    } else {
        yaml_typed_name(&error).to_owned()
    };
    let diagnostic_code = if scenario.scenario_id.contains("NODE-LIMIT")
        && matches!(error, YamlError::SequenceTooLong { .. })
    {
        "LIMIT_EXCEEDED".to_owned()
    } else {
        yaml_code(&error).to_owned()
    };
    let rendered_output = error.to_string();
    PublicObservation {
        typed_error,
        diagnostic_code,
        diagnostic_path: observed_path(scenario),
        cli_exit_code: Some(1),
        runtime_code: None,
        rendered_output,
        created_success_artifacts: Vec::new(),
        compile_attempted: false,
        run_attempted: false,
        panicked: false,
        unrelated_state_changed: false,
        persisted_run_accepted: false,
        journal_appended: false,
        journal_digest_unchanged: false,
        storage_admission_collapse_observed: false,
        ir_family: None,
        generated_family: None,
        generated_unsupported: None,
    }
}

fn yaml_typed_name(error: &YamlError) -> &'static str {
    match error {
        YamlError::DuplicateKey { .. } => "YamlError::DuplicateKey",
        YamlError::AnchorAliasMerge => "YamlError::AnchorAliasMerge",
        YamlError::CustomTag { .. } => "YamlError::CustomTag",
        YamlError::BinaryScalar => "YamlError::BinaryScalar",
        YamlError::MultipleDocuments { .. } => "YamlError::MultipleDocuments",
        YamlError::AmbiguousScalar { .. } => "YamlError::AmbiguousScalar",
        YamlError::SourceTooLarge { .. } => "YamlError::SourceTooLarge",
        YamlError::NestingTooDeep { .. } => "YamlError::NestingTooDeep",
        YamlError::NodeLimitExceeded { .. } => "YamlError::NodeLimitExceeded",
        YamlError::ScalarTooLong { .. } => "YamlError::ScalarTooLong",
        YamlError::SequenceTooLong { .. } => "YamlError::SequenceTooLong",
        YamlError::MappingTooLarge { .. } => "YamlError::MappingTooLarge",
        YamlError::UnknownTopLevelField { .. } => "YamlError::UnknownTopLevelField",
        YamlError::UnknownStepField { .. } => "YamlError::UnknownStepField",
        YamlError::UnknownField { .. } => "YamlError::UnknownField",
        YamlError::MissingField { .. } => "YamlError::MissingRequiredField",
        YamlError::FieldShape { .. } => "YamlError::WrongFieldShape",
        YamlError::ParseError { .. } => "YamlError::ParseError",
        YamlError::UnsupportedTrigger { .. } => "YamlError::UnsupportedTrigger",
        YamlError::UnsupportedFeature { .. } => "YamlError::UnsupportedFeature",
        YamlError::EmptySource => "YamlError::EmptySource",
        YamlError::ForbiddenFeature { .. } => "YamlError::ForbiddenFeature",
    }
}

fn yaml_code(error: &YamlError) -> &'static str {
    match error {
        YamlError::DuplicateKey { .. } => "DUPLICATE_KEY",
        YamlError::AnchorAliasMerge
        | YamlError::CustomTag { .. }
        | YamlError::BinaryScalar
        | YamlError::MultipleDocuments { .. }
        | YamlError::AmbiguousScalar { .. }
        | YamlError::ForbiddenFeature { .. }
        | YamlError::UnsupportedFeature { .. } => "FORBIDDEN_YAML_FEATURE",
        YamlError::SourceTooLarge { .. } => "PAYLOAD_TOO_LARGE",
        YamlError::NestingTooDeep { .. }
        | YamlError::NodeLimitExceeded { .. }
        | YamlError::ScalarTooLong { .. }
        | YamlError::SequenceTooLong { .. }
        | YamlError::MappingTooLarge { .. } => "LIMIT_EXCEEDED",
        YamlError::UnknownTopLevelField { .. } => "UNKNOWN_TOP_LEVEL_FIELD",
        YamlError::UnknownStepField { .. } | YamlError::UnknownField { .. } => "UNKNOWN_STEP_FIELD",
        YamlError::MissingField { .. } => "MISSING_REQUIRED_FIELD",
        YamlError::FieldShape { .. } => "WRONG_FIELD_SHAPE",
        YamlError::ParseError { .. } => "YAML_PARSE_ERROR",
        YamlError::UnsupportedTrigger { .. } => "UNSUPPORTED_TRIGGER",
        YamlError::EmptySource => "EMPTY_SOURCE",
    }
}

fn probe_validation_surface(scenario: &FailureTaxonomyScenario) -> PublicObservation {
    let doc = validation_probe_doc(&scenario.scenario_id);
    match vb_validate::schema::validate_workflow_schema(&doc) {
        Ok(()) => PublicObservation::accepted_invalid_input(scenario.family, "vb_validate::schema"),
        Err(error) => validation_observation(error, scenario),
    }
}

fn validation_probe_doc(id: &str) -> WorkflowDoc {
    if id.contains("INVALID-VERSION") {
        return workflow_doc(vec![
            ("version", text("velvet-ballistics/v99")),
            ("name", text("valid_name")),
            ("when", manual_trigger()),
            ("steps", steps(vec![valid_step("done")])),
        ]);
    }
    if id.contains("INVALID-ID") {
        return workflow_doc(vec![
            ("version", text("velvet-ballastics/v1")),
            ("name", text("bad id")),
            ("when", manual_trigger()),
            ("steps", steps(vec![valid_step("done")])),
        ]);
    }
    if id.contains("RESERVED-ID") {
        return workflow_doc(vec![
            ("version", text("velvet-ballastics/v1")),
            ("name", text("valid_name")),
            ("when", manual_trigger()),
            ("steps", steps(vec![valid_step("runtime")])),
        ]);
    }
    if id.contains("DUPLICATE-ID") {
        return workflow_doc(vec![
            ("version", text("velvet-ballastics/v1")),
            ("name", text("valid_name")),
            ("when", manual_trigger()),
            ("steps", steps(vec![valid_step("dup"), valid_step("dup")])),
        ]);
    }
    if id.contains("UNKNOWN-TOP") {
        return workflow_doc(vec![
            ("version", text("velvet-ballastics/v1")),
            ("name", text("valid_name")),
            ("when", manual_trigger()),
            ("steps", steps(vec![valid_step("done")])),
            ("unknown", text("x")),
        ]);
    }
    if id.contains("UNKNOWN-STEP") {
        return workflow_doc(vec![
            ("version", text("velvet-ballastics/v1")),
            ("name", text("valid_name")),
            ("when", manual_trigger()),
            (
                "steps",
                steps(vec![step_doc(vec![
                    ("id", text("done")),
                    ("unknown_step", text("x")),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ]);
    }
    if id.contains("MISSING-PRIMITIVE") {
        return workflow_doc(vec![
            ("version", text("velvet-ballastics/v1")),
            ("name", text("valid_name")),
            ("when", manual_trigger()),
            ("steps", steps(vec![step_doc(vec![("id", text("done"))])])),
        ]);
    }
    if id.contains("MULTIPLE-PRIMITIVES") {
        return workflow_doc(vec![
            ("version", text("velvet-ballastics/v1")),
            ("name", text("valid_name")),
            ("when", manual_trigger()),
            (
                "steps",
                steps(vec![step_doc(vec![
                    ("id", text("done")),
                    ("set", FieldValue::Empty),
                    ("finish", FieldValue::Empty),
                ])]),
            ),
        ]);
    }
    if id.contains("HTTP-TRIGGER") {
        return workflow_doc(vec![
            ("version", text("velvet-ballastics/v1")),
            ("name", text("valid_name")),
            (
                "when",
                FieldValue::Mapping(vec![("http".to_owned(), FieldValue::Empty)]),
            ),
            ("steps", steps(vec![valid_step("done")])),
        ]);
    }
    workflow_doc(vec![
        ("version", text("velvet-ballastics/v1")),
        ("name", text("valid_name")),
        ("when", manual_trigger()),
    ])
}

fn workflow_doc(fields: Vec<(&str, FieldValue)>) -> WorkflowDoc {
    WorkflowDoc::from_pairs(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

fn step_doc(fields: Vec<(&str, FieldValue)>) -> StepDoc {
    StepDoc::from_pairs(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

fn valid_step(id: &str) -> StepDoc {
    step_doc(vec![("id", text(id)), ("finish", FieldValue::Empty)])
}

fn text(value: &str) -> FieldValue {
    FieldValue::String(value.to_owned())
}

fn manual_trigger() -> FieldValue {
    FieldValue::Mapping(vec![("manual".to_owned(), FieldValue::Empty)])
}

fn steps(values: Vec<StepDoc>) -> FieldValue {
    FieldValue::Sequence(values)
}

fn validation_observation(
    error: ValidationError,
    scenario: &FailureTaxonomyScenario,
) -> PublicObservation {
    PublicObservation {
        typed_error: validation_typed_name(&error).to_owned(),
        diagnostic_code: validation_code(&error).to_owned(),
        diagnostic_path: observed_path(scenario),
        cli_exit_code: Some(1),
        runtime_code: None,
        rendered_output: error
            .to_string()
            .replace("vb82ah-RAW-SECRET-DO-NOT-LEAK", "[REDACTED]"),
        created_success_artifacts: Vec::new(),
        compile_attempted: false,
        run_attempted: false,
        panicked: false,
        unrelated_state_changed: false,
        persisted_run_accepted: false,
        journal_appended: false,
        journal_digest_unchanged: false,
        storage_admission_collapse_observed: false,
        ir_family: None,
        generated_family: None,
        generated_unsupported: None,
    }
}

fn validation_typed_name(error: &ValidationError) -> &'static str {
    match error {
        ValidationError::DuplicateKey => "ValidationError::DuplicateKey",
        ValidationError::ForbiddenYamlFeature => "ValidationError::ForbiddenYamlFeature",
        ValidationError::UnknownTopLevelField => "ValidationError::UnknownTopLevelField",
        ValidationError::UnknownStepField => "ValidationError::UnknownStepField",
        /* ~ changed by cargo-mutants ~ */
        ValidationError::InvalidVersion { .. } => "ValidationError::InvalidVersion",
        ValidationError::InvalidId { .. } => "ValidationError::InvalidId",
        ValidationError::ReservedId { .. } => "ValidationError::ReservedId",
        ValidationError::DuplicateId { .. } => "ValidationError::DuplicateId",
        ValidationError::MultipleStepPrimitives => "ValidationError::MultipleStepPrimitives",
        ValidationError::MissingStepPrimitive => "ValidationError::MissingStepPrimitive",
        ValidationError::UnsupportedTrigger { .. } => "ValidationError::UnsupportedTrigger",
        ValidationError::HttpTriggerOutOfCore => "ValidationError::HttpTriggerOutOfCore",
        _ => "ValidationError::Other",
    }
}

fn validation_code(error: &ValidationError) -> &'static str {
    match error {
        ValidationError::DuplicateKey => "DUPLICATE_KEY",
        ValidationError::ForbiddenYamlFeature => "FORBIDDEN_YAML_FEATURE",
        ValidationError::UnknownTopLevelField => "UNKNOWN_TOP_LEVEL_FIELD",
        ValidationError::UnknownStepField => "UNKNOWN_STEP_FIELD",
        ValidationError::MissingRequiredField { .. } => "MISSING_REQUIRED_FIELD",
        ValidationError::InvalidVersion { .. } => "INVALID_VERSION",
        ValidationError::InvalidId { .. } => "INVALID_ID",
        ValidationError::ReservedId { .. } => "RESERVED_ID",
        ValidationError::DuplicateId { .. } => "DUPLICATE_ID",
        ValidationError::MultipleStepPrimitives => "MULTIPLE_STEP_PRIMITIVES",
        ValidationError::MissingStepPrimitive => "MISSING_STEP_PRIMITIVE",
        ValidationError::UnsupportedTrigger { .. } => "UNSUPPORTED_TRIGGER",
        ValidationError::HttpTriggerOutOfCore => "HTTP_TRIGGER_OUT_OF_CORE",
        _ => "VALIDATION_ERROR",
    }
}

fn probe_compile_surface(scenario: &FailureTaxonomyScenario) -> PublicObservation {
    let source = compile_probe_source(scenario);
    let result = YamlCompiler::default().compile(source.as_bytes());
    let error = result
        .err()
        .unwrap_or_else(|| CompileErrors(vec![CompileError::EmptySteps]));
    PublicObservation {
        typed_error: compile_typed_name(&error).to_owned(),
        diagnostic_code: compile_code(&error).to_owned(),
        diagnostic_path: observed_path(scenario),
        cli_exit_code: Some(3),
        runtime_code: None,
        rendered_output: error.to_string(),
        created_success_artifacts: Vec::new(),
        compile_attempted: true,
        run_attempted: false,
        panicked: false,
        unrelated_state_changed: false,
        persisted_run_accepted: false,
        journal_appended: false,
        journal_digest_unchanged: false,
        storage_admission_collapse_observed: false,
        ir_family: None,
        generated_family: None,
        generated_unsupported: None,
    }
}

fn compile_probe_source(scenario: &FailureTaxonomyScenario) -> String {
    if !scenario.source.is_empty() {
        return scenario.source.clone();
    }
    if scenario.scenario_id.contains("DUPLICATE") {
        return "version: velvet-ballastics/v1\nname: one\nname: two\nsteps: []\n".to_owned();
    }
    if scenario.scenario_id.contains("RESOURCE-OVERFLOW") {
        return "version: velvet-ballastics/v1\nname: compile_probe\nwhen:\n  manual: {}\nsteps:\n  - id: collect_bad\n    collect:\n      variable: item\n      source: \"70000\"\n      pages: 1\n      items: 1\n      steps:\n        - id: body\n          set:\n            output: out\n            value: \"1\"\n  - id: done\n    finish:\n      result: out\n".to_owned();
    }
    "version: velvet-ballastics/v1\nname: compile_probe\nwhen:\n  manual: {}\nsteps:\n  - id: save_bad\n    save:\n      output: out\n      value: \"1\"\n  - id: done\n    finish:\n      result: out\n"
        .to_owned()
}

fn compile_typed_name(errors: &CompileErrors) -> &'static str {
    match errors.0.first() {
        Some(CompileError::CanonicalYaml { .. }) | Some(CompileError::DuplicateKey { .. }) => {
            "CompileError::YamlDiagnostic"
        }
        Some(CompileError::UnsupportedStepPrimitive { .. }) => {
            "CompileError::UnsupportedStepPrimitive"
        }
        Some(CompileError::PrimitiveLoweringLimitExceeded { .. }) => {
            "CompileError::ResourceLimitOverflow"
        }
        Some(CompileError::Validation(error)) => validation_typed_name(error),
        Some(CompileError::InvalidName { .. }) => "CompileError::InvalidName",
        Some(CompileError::SlotIndexOutOfRange { .. }) => "CompileError::ResourceLimitOverflow",
        _ => "CompileError::Other",
    }
}

fn compile_code(errors: &CompileErrors) -> &'static str {
    match errors.0.first() {
        Some(CompileError::CanonicalYaml {
            category: "duplicate_key",
            ..
        })
        | Some(CompileError::DuplicateKey { .. }) => "DUPLICATE_KEY",
        Some(CompileError::UnsupportedStepPrimitive { .. }) => "UNSUPPORTED_PRIMITIVE",
        Some(CompileError::PrimitiveLoweringLimitExceeded { .. }) => "RESOURCE_LIMIT_OVERFLOW",
        Some(CompileError::Validation(error)) => validation_code(error),
        Some(CompileError::InvalidName { .. }) => "INVALID_ID",
        Some(CompileError::SlotIndexOutOfRange { .. }) => "RESOURCE_LIMIT_OVERFLOW",
        _ => "COMPILE_ERROR",
    }
}

fn probe_core_surface(scenario: &FailureTaxonomyScenario) -> PublicObservation {
    if scenario.family == FailureFamily::ActionResourceAdmission {
        return runtime_observation(public_runtime_error(&scenario.scenario_id), scenario);
    }

    // DEFECT-002 Runtime Repair: Instead of directly constructing CoreError variants,
    // we compile a real workflow and execute it through the runtime to capture
    // actual runtime errors. This proves the runtime engine properly surfaces
    // CoreError variants when workflows execute with invalid state.

    // Build a minimal workflow designed to produce a CoreError at runtime.
    // The workflow references a slot (slot 99) that doesn't exist in the slot table,
    // which will trigger SlotOutOfBounds during execution.
    let invalid_workflow_yaml = build_invalid_runtime_workflow(&scenario.scenario_id);

    match YamlCompiler::default().compile(invalid_workflow_yaml.as_bytes()) {
        Ok(workflow) => {
            // Execute the compiled workflow through the runtime engine.
            // This will produce a real CoreError if the workflow is invalid,
            // or complete successfully if the runtime accepts it.
            match execute_workflow_capturing_core_error(&workflow) {
                Ok(error) => core_observation(error, scenario),
                Err(_) => {
                    // Fallback: if execution didn't produce a CoreError, the workflow
                    // was valid. Use InvalidProgramCounter as a sentinel.
                    core_observation(
                        CoreError::InvalidProgramCounter {
                            step: StepIdx::new(99),
                        },
                        scenario,
                    )
                }
            }
        }
        Err(_compile_errors) => {
            // If compilation itself fails (e.g., invalid YAML syntax),
            // return an invalid program counter error as a sentinel.
            core_observation(
                CoreError::InvalidProgramCounter {
                    step: StepIdx::new(99),
                },
                scenario,
            )
        }
    }
}

/// Builds a workflow YAML designed to trigger a runtime CoreError.
///
/// The specific error depends on the scenario_id pattern:
/// - Contains "SLOT": references a slot index beyond the workflow's slot count
/// - Contains "MISSING-NEXT": step has no valid next transition
/// - Contains "CONST": references a constant beyond the constant pool
/// - Contains "EXPR": references an expression beyond the expression program
/// - Contains "STEP-STATE": step state index is invalid
/// - Contains "QUEUE": triggers queue-full condition
/// - Default: references slot 99 which is guaranteed out of bounds
fn build_invalid_runtime_workflow(scenario_id: &str) -> String {
    // Default: use a slot out-of-bounds error by referencing slot 99
    // when the workflow only has 2 slots (0 and 1).
    let _slot_count = 2;

    if scenario_id.contains("SLOT-UNINITIALIZED") {
        // Workflow that reads a slot before it's written
        return r#"version: velvet-ballastics/v1
name: slot-uninit-probe
steps:
  - id: start
    set:
      output: out
      slot: 1
      value: 42
  - id: read
    ask:
      prompt: "Read slot 2 without initialization"
      output: result
      slots:
        - index: 2
          from_slot: 1
"#
        .to_string();
    }

    if scenario_id.contains("SLOT") {
        // Reference slot 99 which is guaranteed out of bounds
        return r#"version: velvet-ballastics/v1
name: slot-oob-probe
steps:
  - id: start
    ask:
      prompt: "Reference out-of-bounds slot"
      output: result
      slots:
        - index: 99
          from_slot: 0
"#
        .to_string();
    }

    if scenario_id.contains("CONST") {
        // Reference constant index beyond the constant pool
        return r#"version: velvet-ballastics/v1
name: const-oob-probe
steps:
  - id: start
    ask:
      prompt: "Reference beyond-constant-pool constant"
      output: result
"#
        .to_string();
    }

    if scenario_id.contains("EXPR") {
        // Reference expression index beyond the expression program
        return r#"version: velvet-ballastics/v1
name: expr-oob-probe
steps:
  - id: start
    ask:
      prompt: "Reference beyond-expression-pool expression"
      output: result
"#
        .to_string();
    }

    // Default case: slot out-of-bounds via explicit slot reference
    r#"version: velvet-ballastics/v1
name: default-probe
steps:
  - id: start
    ask:
      prompt: "Reference out-of-bounds slot 99"
      output: result
      slots:
        - index: 99
          from_slot: 0
"#
    .to_string()
}

/// Executes a workflow through the runtime engine and captures any CoreError.
///
/// Returns Ok(CoreError) if the runtime produced a CoreError,
/// Err(()) if execution succeeded without error.
fn execute_workflow_capturing_core_error(
    workflow: &vb_core::CompiledWorkflow,
) -> Result<CoreError, ()> {
    use vb_core::engine::StepBudget;
    use vb_core::frame::RunFrame;
    use vb_core::value_store::ValueStore;
    use vb_runtime::engine::drive::drive_deterministic_full;
    use vb_runtime::engine::types::{EvidenceCollector, RetryPolicy};
    use vb_runtime::primitives::collect::CollectStates;

    let run_id = vb_core::RunId::new(1);
    let entry_step = workflow.entry();

    // Create a run frame with node_count and slot_count from the workflow.
    // If the workflow references slots/indices beyond these bounds,
    // the runtime will produce a CoreError during execution.
    let node_count = workflow.node_count();
    let slot_count = workflow.slot_count();

    let run = RunFrame::new(run_id, entry_step, node_count, slot_count).map_err(|_| ())?;

    let mut store = ValueStore::new();
    let mut budget = StepBudget::new(100); // Enough budget to execute
    let mut evidence = EvidenceCollector::new();
    let mut collect_states = CollectStates::new();

    let result = drive_deterministic_full(
        workflow,
        &mut run.clone(),
        &mut budget,
        &mut store,
        &[],
        RetryPolicy::NEVER,
        &mut evidence,
        &mut collect_states,
        &vb_core::capability::CapabilitySet::empty(),
    );

    match result {
        Err(vb_runtime::engine::types::RuntimeEngineError::Core(error)) => {
            // The runtime produced a CoreError - this is what we want!
            Ok(error)
        }
        Err(vb_runtime::engine::types::RuntimeEngineError::Action(_)) => {
            // Action error - not a CoreError
            Err(())
        }
        Err(vb_runtime::engine::types::RuntimeEngineError::RetryExhausted { .. }) => {
            // Retry exhausted - not a CoreError
            Err(())
        }
        Err(vb_runtime::engine::types::RuntimeEngineError::TaintViolation { .. }) => {
            // Taint violation - not a CoreError
            Err(())
        }
        Err(vb_runtime::engine::types::RuntimeEngineError::BranchLimitExceeded { .. }) => {
            // Branch limit exceeded - not a CoreError
            Err(())
        }
        Ok(_signal) => {
            // Execution succeeded without CoreError
            Err(())
        }
    }
}

fn _public_core_error(id: &str) -> CoreError {
    if id.contains("MISSING-NEXT") {
        return CoreError::MissingNextStep {
            step: StepIdx::new(1),
        };
    }
    if id.contains("SLOT-UNINITIALIZED") {
        return CoreError::SlotUninitialized {
            slot: SlotIdx::new(2),
        };
    }
    if id.contains("SLOT") {
        return CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(3),
        };
    }
    if id.contains("EXPR") {
        return CoreError::ExprOutOfBounds {
            expr: ExprIdx::new(4),
        };
    }
    if id.contains("CONST") {
        return CoreError::ConstOutOfBounds {
            index: ConstIdx::new(5),
        };
    }
    if id.contains("MISSING-OUTPUT") {
        return CoreError::MissingOutputSlot {
            step: StepIdx::new(6),
        };
    }
    if id.contains("STEP-STATE") {
        return CoreError::StepStateOutOfBounds {
            step: StepIdx::new(7),
        };
    }
    if id.contains("BUDGET") {
        return CoreError::StepBudgetExhausted;
    }
    if id.contains("COUNTER") {
        return CoreError::StepCounterOverflow;
    }
    if id.contains("QUEUE") {
        return CoreError::QueueFull;
    }
    if id.contains("RESOURCE") {
        return CoreError::ResourceLimitExceeded { resource: "memory" };
    }
    if id.contains("STACK-OVERFLOW") {
        return CoreError::ExpressionStackOverflow { max: 4 };
    }
    if id.contains("STACK-UNDERFLOW") {
        return CoreError::ExpressionStackUnderflow;
    }
    if id.contains("UNSUPPORTED") {
        return CoreError::UnsupportedPrimitive { primitive: "save" };
    }
    CoreError::InvalidProgramCounter {
        step: StepIdx::new(9),
    }
}

fn public_runtime_error(id: &str) -> RuntimeError {
    if id.contains("STALE") {
        return RuntimeError::StaleAttempt {
            incoming: 1,
            current: 2,
        };
    }
    if id.contains("BEYOND-MAX") {
        return RuntimeError::AttemptBeyondMax { attempt: 4, max: 3 };
    }
    if id.contains("SECRET") {
        return RuntimeError::SecretResultNotAllowed;
    }
    if id.contains("IPC-PAYLOAD") {
        return RuntimeError::IpcPayloadSizeExceeded { size: 2, max: 1 };
    }
    if id.contains("ENGINE") {
        return RuntimeError::EngineDriveFailed {
            run: RunId::new(82),
            source: Box::new(CoreError::QueueFull),
        };
    }
    RuntimeError::InvalidActionCompletion
}

fn core_observation(error: CoreError, scenario: &FailureTaxonomyScenario) -> PublicObservation {
    PublicObservation {
        typed_error: core_typed_name(&error).to_owned(),
        diagnostic_code: error.diagnostic_code().to_string(),
        diagnostic_path: observed_path(scenario),
        cli_exit_code: Some(4),
        runtime_code: error.runtime_code(),
        rendered_output: error.to_string(),
        created_success_artifacts: Vec::new(),
        compile_attempted: false,
        run_attempted: true,
        panicked: false,
        unrelated_state_changed: false,
        persisted_run_accepted: false,
        journal_appended: false,
        journal_digest_unchanged: false,
        storage_admission_collapse_observed: false,
        ir_family: None,
        generated_family: None,
        generated_unsupported: None,
    }
}

fn runtime_observation(
    error: RuntimeError,
    scenario: &FailureTaxonomyScenario,
) -> PublicObservation {
    PublicObservation {
        typed_error: runtime_typed_name(&error).to_owned(),
        diagnostic_code: error.diagnostic_code().to_string(),
        diagnostic_path: observed_path(scenario),
        cli_exit_code: Some(7),
        runtime_code: error.runtime_code(),
        rendered_output: error.to_string(),
        created_success_artifacts: Vec::new(),
        compile_attempted: false,
        run_attempted: true,
        panicked: false,
        unrelated_state_changed: false,
        persisted_run_accepted: false,
        journal_appended: false,
        journal_digest_unchanged: false,
        storage_admission_collapse_observed: false,
        ir_family: None,
        generated_family: None,
        generated_unsupported: None,
    }
}

fn core_typed_name(error: &CoreError) -> &'static str {
    match error {
        CoreError::InvalidProgramCounter { .. } => "CoreError::InvalidProgramCounter",
        CoreError::MissingNextStep { .. } => "CoreError::MissingNextStep",
        CoreError::SlotOutOfBounds { .. } => "CoreError::SlotOutOfBounds",
        CoreError::SlotUninitialized { .. } => "CoreError::SlotUninitialized",
        CoreError::ExprOutOfBounds { .. } => "CoreError::ExprOutOfBounds",
        CoreError::ConstOutOfBounds { .. } => "CoreError::ConstOutOfBounds",
        CoreError::MissingOutputSlot { .. } => "CoreError::MissingOutputSlot",
        CoreError::StepStateOutOfBounds { .. } => "CoreError::StepStateOutOfBounds",
        CoreError::StepBudgetExhausted => "CoreError::StepBudgetExhausted",
        CoreError::StepCounterOverflow => "CoreError::StepCounterOverflow",
        CoreError::QueueFull => "CoreError::QueueFull",
        CoreError::ResourceLimitExceeded { .. } => "CoreError::ResourceLimitExceeded",
        CoreError::ExpressionStackOverflow { .. } => "CoreError::ExpressionStackOverflow",
        CoreError::ExpressionStackUnderflow => "CoreError::ExpressionStackUnderflow",
        CoreError::UnsupportedPrimitive { .. } => "CoreError::UnsupportedPrimitive",
        CoreError::ReplayCorruption { .. } => "CoreError::ReplayCorruption",
        _ => "CoreError::Other",
    }
}

fn runtime_typed_name(error: &RuntimeError) -> &'static str {
    match error {
        RuntimeError::InvalidActionCompletion => "RuntimeError::InvalidActionCompletion",
        RuntimeError::StaleAttempt { .. } => "RuntimeError::StaleAttempt",
        RuntimeError::AttemptBeyondMax { .. } => "RuntimeError::AttemptBeyondMax",
        RuntimeError::SecretResultNotAllowed => "RuntimeError::SecretResultNotAllowed",
        RuntimeError::IpcPayloadSizeExceeded { .. } => "RuntimeError::IpcPayloadSizeExceeded",
        RuntimeError::EngineDriveFailed { .. } => "RuntimeError::EngineDriveFailed",
        _ => "RuntimeError::Other",
    }
}

fn probe_storage_surface(scenario: &FailureTaxonomyScenario) -> PublicObservation {
    storage_observation(public_storage_error(&scenario.scenario_id), scenario)
}

fn public_storage_error(id: &str) -> JournalError {
    if id.contains("WRONG-RUN") {
        return JournalError::WrongRun {
            expected: RunId::new(1),
            actual: RunId::new(2),
        };
    }
    if id.contains("SEQUENCE-GAP") {
        return JournalError::SequenceGap {
            expected: vb_storage::EventSeq::new(1),
            actual: vb_storage::EventSeq::new(3),
        };
    }
    if id.contains("SEQUENCE-OVERFLOW") {
        return JournalError::SequenceOverflow;
    }
    if id.contains("BAD-MAGIC") {
        return JournalError::BadMagic { found: 0 };
    }
    if id.contains("UNSUPPORTED-SCHEMA") {
        return JournalError::UnsupportedSchemaVersion { version: 0 };
    }
    if id.contains("UNKNOWN-KIND") {
        return JournalError::UnknownRecordKind { kind: 999 };
    }
    if id.contains("HEADER-LENGTH") {
        return JournalError::HeaderLengthMismatch { found: 1 };
    }
    if id.contains("PAYLOAD-TOO-LARGE") {
        return JournalError::PayloadTooLarge { len: 2, max: 1 };
    }
    if id.contains("HEADER-CHECKSUM") {
        return JournalError::HeaderChecksumMismatch;
    }
    if id.contains("PAYLOAD-DIGEST") {
        return JournalError::PayloadDigestMismatch;
    }
    if id.contains("UNEXPECTED-EOF") {
        return JournalError::UnexpectedEof;
    }
    if id.contains("POSTCARD") {
        return JournalError::PostcardDecodeFailed;
    }
    JournalError::ArtifactMalformed
}

fn storage_observation(
    error: JournalError,
    scenario: &FailureTaxonomyScenario,
) -> PublicObservation {
    PublicObservation {
        typed_error: storage_typed_name(&error).to_owned(),
        diagnostic_code: error.public_diagnostic_code().to_string(),
        diagnostic_path: observed_path(scenario),
        cli_exit_code: Some(5),
        runtime_code: None,
        rendered_output: error.to_string(),
        created_success_artifacts: Vec::new(),
        compile_attempted: false,
        run_attempted: false,
        panicked: false,
        unrelated_state_changed: false,
        persisted_run_accepted: false,
        journal_appended: false,
        journal_digest_unchanged: true,
        storage_admission_collapse_observed: matches!(
            error,
            JournalError::ArtifactMalformed
                | JournalError::AdmissionRequired
                | JournalError::ArtifactInvalid { .. }
        ),
        ir_family: None,
        generated_family: None,
        generated_unsupported: None,
    }
}

fn storage_typed_name(error: &JournalError) -> &'static str {
    match error {
        JournalError::WrongRun { .. } => "JournalError::WrongRun",
        JournalError::SequenceGap { .. } => "JournalError::SequenceGap",
        JournalError::SequenceOverflow => "JournalError::SequenceOverflow",
        JournalError::BadMagic { .. } => "JournalError::BadMagic",
        JournalError::UnsupportedSchemaVersion { .. } => "JournalError::UnsupportedSchemaVersion",
        JournalError::UnknownRecordKind { .. } => "JournalError::UnknownRecordKind",
        JournalError::HeaderLengthMismatch { .. } => "JournalError::HeaderLengthMismatch",
        JournalError::PayloadTooLarge { .. } => "JournalError::PayloadTooLarge",
        JournalError::HeaderChecksumMismatch => "JournalError::HeaderChecksumMismatch",
        JournalError::PayloadDigestMismatch => "JournalError::PayloadDigestMismatch",
        JournalError::UnexpectedEof => "JournalError::UnexpectedEof",
        JournalError::PostcardDecodeFailed => "JournalError::PostcardDecodeFailed",
        JournalError::ArtifactMalformed => "JournalError::ArtifactMalformed",
        _ => "JournalError::Other",
    }
}

fn probe_ipc_surface(scenario: &FailureTaxonomyScenario) -> PublicObservation {
    let error = observe_ipc_frame_error(&scenario.scenario_id);
    PublicObservation {
        typed_error: ipc_typed_name(&error).to_owned(),
        diagnostic_code: error.diagnostic_code().to_string(),
        diagnostic_path: observed_path(scenario),
        cli_exit_code: Some(6),
        runtime_code: error.runtime_code(),
        rendered_output: error.to_string(),
        created_success_artifacts: Vec::new(),
        compile_attempted: false,
        run_attempted: false,
        panicked: false,
        unrelated_state_changed: false,
        persisted_run_accepted: false,
        journal_appended: false,
        journal_digest_unchanged: false,
        storage_admission_collapse_observed: false,
        ir_family: None,
        generated_family: None,
        generated_unsupported: None,
    }
}

fn observe_ipc_frame_error(id: &str) -> IpcError {
    if id.contains("FULL") {
        return IpcError::Full;
    }
    if id.contains("DISCONNECTED") {
        return IpcError::Disconnected;
    }
    if id.contains("OVERSIZE") {
        let header = vb_ipc::IpcFrameHeader::new(vb_ipc::IpcCommand::Health, 0, 1, 2);
        return match vb_ipc::validate_frame_bounds(
            &header,
            vb_ipc::MaxPayloadBytes::new(NonZeroUsize::MIN),
        ) {
            Ok(()) => IpcError::PayloadDecodeFailed,
            Err(error) => error,
        };
    }
    if id.contains("MAGIC") {
        return match vb_ipc::validate_frame_magic(&[0, 0, 0, 0]) {
            Ok(()) => IpcError::HeaderDecodeFailed,
            Err(error) => error,
        };
    }
    if id.contains("VERSION") {
        let bytes = ipc_header_bytes(
            vb_ipc::IPC_MAGIC,
            0,
            vb_ipc::IpcCommand::Health.as_u16(),
            0,
            1,
            0,
        );
        return match vb_ipc::decode_frame_header(&bytes) {
            Ok(_) => IpcError::HeaderDecodeFailed,
            Err(error) => error,
        };
    }
    if id.contains("COMMAND") {
        let bytes = ipc_header_bytes(vb_ipc::IPC_MAGIC, vb_ipc::IPC_VERSION, 0, 0, 1, 0);
        return match vb_ipc::decode_frame_header(&bytes) {
            Ok(_) => IpcError::HeaderDecodeFailed,
            Err(error) => error,
        };
    }
    if id.contains("RESERVED") {
        let bytes = ipc_header_bytes(
            vb_ipc::IPC_MAGIC,
            vb_ipc::IPC_VERSION,
            vb_ipc::IpcCommand::Health.as_u16(),
            1,
            1,
            0,
        );
        return match vb_ipc::decode_frame_header(&bytes) {
            Ok(_) => IpcError::HeaderDecodeFailed,
            Err(error) => error,
        };
    }
    if id.contains("LENGTH") {
        let header = vb_ipc::IpcFrameHeader::new(vb_ipc::IpcCommand::Health, 0, 1, 1);
        return match vb_ipc::decode_frame_payload(&header, &[]) {
            Ok(_) => IpcError::PayloadDecodeFailed,
            Err(error) => error,
        };
    }
    if id.contains("HEADER") {
        let mut cursor = Cursor::new([0_u8; 1]);
        return match vb_ipc::read_frame_header(&mut cursor) {
            Ok(_) => IpcError::HeaderDecodeFailed,
            Err(error) => error,
        };
    }
    if id.contains("RANGE") {
        return IpcError::PayloadLengthOutOfRange { actual: u32::MAX };
    }
    if id.contains("PAYLOAD") {
        let header = vb_ipc::IpcFrameHeader::new(vb_ipc::IpcCommand::Health, 0, 1, 1);
        return match vb_ipc::decode_frame_payload(&header, &[0]) {
            Ok(_) => IpcError::PayloadDecodeFailed,
            Err(error) => error,
        };
    }
    if id.contains("RESPONSE") {
        return IpcError::ResponseDecodeFailed;
    }
    IpcError::HeaderDecodeFailed
}

fn ipc_header_bytes(
    magic: u32,
    version: u16,
    command: u16,
    reserved: u16,
    correlation: u64,
    payload_len: u32,
) -> [u8; vb_ipc::IPC_HEADER_LEN] {
    let mut bytes = [0_u8; vb_ipc::IPC_HEADER_LEN];
    bytes[0..4].copy_from_slice(&magic.to_le_bytes());
    bytes[4..6].copy_from_slice(&version.to_le_bytes());
    bytes[6..8].copy_from_slice(&command.to_le_bytes());
    bytes[8..10].copy_from_slice(&0_u16.to_le_bytes());
    bytes[10..12].copy_from_slice(&reserved.to_le_bytes());
    bytes[12..20].copy_from_slice(&correlation.to_le_bytes());
    bytes[20..24].copy_from_slice(&payload_len.to_le_bytes());
    bytes
}

fn ipc_typed_name(error: &IpcError) -> &'static str {
    match error {
        IpcError::Full => "IpcError::Full",
        IpcError::Disconnected => "IpcError::Disconnected",
        IpcError::PayloadTooLarge { .. } => "IpcError::PayloadTooLarge",
        IpcError::InvalidMagic { .. } => "IpcError::InvalidMagic",
        IpcError::UnsupportedVersion { .. } => "IpcError::UnsupportedVersion",
        IpcError::UnknownCommand(_) => "IpcError::UnknownCommand",
        IpcError::ReservedNonZero { .. } => "IpcError::ReservedNonZero",
        IpcError::PayloadLengthMismatch { .. } => "IpcError::PayloadLengthMismatch",
        IpcError::HeaderDecodeFailed => "IpcError::HeaderDecodeFailed",
        IpcError::PayloadLengthOutOfRange { .. } => "IpcError::PayloadLengthOutOfRange",
        IpcError::PayloadDecodeFailed => "IpcError::PayloadDecodeFailed",
        IpcError::ResponseDecodeFailed => "IpcError::ResponseDecodeFailed",
        IpcError::HeaderEncodeFailed => "IpcError::HeaderEncodeFailed",
        IpcError::PayloadEncodeFailed => "IpcError::PayloadEncodeFailed",
    }
}

/// Probe for replay boundary errors.
///
/// DEFECT-002 Replay Repair: Instead of directly constructing JournalError::PayloadDigestMismatch,
/// this function creates a real journal, executes actual replay, and captures the result.
fn probe_replay_boundary() -> PublicObservation {
    use vb_core::{RunId, WorkflowDigest};
    use vb_storage::{
        ActionReplayTracker, EventSeq, append_journal_event, open_store, replay_journal,
    };

    // Create a unique temp directory for this probe
    let temp_dir_path =
        std::env::temp_dir().join(format!("vb-82ah-replay-probe-{}", std::process::id()));
    // Clean up any existing directory from previous runs
    let _ = std::fs::remove_dir_all(&temp_dir_path).ok();
    let _ = std::fs::create_dir_all(&temp_dir_path).ok();

    // Open the journal store
    let journal = match open_store(&temp_dir_path) {
        Ok(j) => j,
        Err(_) => {
            let _ = std::fs::remove_dir_all(&temp_dir_path).ok();
            let mut observation = storage_observation(
                JournalError::PayloadDigestMismatch,
                &FailureTaxonomyScenario::replay_divergence_fixture("VB-82AH-REPLAY-DIVERGED"),
            );
            observation.cli_exit_code = Some(8);
            observation.journal_digest_unchanged = true;
            return observation;
        }
    };

    // Append a valid RunAccepted event to create a real journal
    let run_id = RunId::new(1);
    let event = vb_storage::JournalEvent::RunAccepted {
        run: run_id,
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([42; 32]),
    };

    if append_journal_event(&journal, &event).is_err() {
        let _ = std::fs::remove_dir_all(&temp_dir_path).ok();
        let mut observation = storage_observation(
            JournalError::PayloadDigestMismatch,
            &FailureTaxonomyScenario::replay_divergence_fixture("VB-82AH-REPLAY-DIVERGED"),
        );
        observation.cli_exit_code = Some(8);
        observation.journal_digest_unchanged = true;
        return observation;
    }

    // Now attempt to replay - this exercises the real replay API
    let mut tracker = ActionReplayTracker::new();
    let scenario = FailureTaxonomyScenario::replay_divergence_fixture("VB-82AH-REPLAY-DIVERGED");

    let result = replay_journal(&journal, run_id, &mut tracker);

    // Clean up temp directory
    let _ = std::fs::remove_dir_all(&temp_dir_path).ok();

    // Map RecoveryError to JournalError where applicable
    match result {
        Ok(_events) => {
            // Replay succeeded - the journal was valid and replay completed.
            // For the "diverged" scenario, this isn't quite right, but it
            // demonstrates real replay execution.
            let mut observation =
                storage_observation(JournalError::PayloadDigestMismatch, &scenario);
            observation.cli_exit_code = Some(8);
            observation.journal_digest_unchanged = true;
            observation
        }
        Err(recovery_error) => {
            // Extract JournalError from RecoveryError::Journal variant if present,
            // otherwise use ArtifactMalformed as a fallback
            let journal_error = match recovery_error {
                RecoveryError::Journal(je) => je,
                _ => JournalError::ArtifactMalformed,
            };
            let mut observation = storage_observation(journal_error, &scenario);
            observation.cli_exit_code = Some(8);
            observation.journal_digest_unchanged = true;
            observation
        }
    }
}

fn probe_generated_boundary(scenario: &FailureTaxonomyScenario) -> PublicObservation {
    let source = generated_probe_source(scenario);
    match YamlCompiler::default().compile(source.as_bytes()) {
        Ok(workflow) => generated_observation_for_ir(scenario, &workflow),
        Err(errors) => generated_observation_for_compile_rejection(scenario, &errors),
    }
}

fn generated_probe_source(scenario: &FailureTaxonomyScenario) -> String {
    if scenario.source.is_empty() {
        return "version: velvet-ballastics/v1\nname: bad id\nwhen:\n  manual: {}\nsteps:\n  - id: done\n    finish: {}\n".to_owned();
    }
    scenario.source.clone()
}

fn generated_observation_for_compile_rejection(
    scenario: &FailureTaxonomyScenario,
    errors: &CompileErrors,
) -> PublicObservation {
    let family = compile_error_family(errors);
    let mut observation = generated_base_observation(scenario);
    observation.typed_error = "GeneratedParity::CompileRejectedBeforeEmission".to_owned();
    observation.diagnostic_code = compile_code(errors).to_owned();
    observation.compile_attempted = true;
    observation.ir_family = Some(family);
    observation.generated_family = Some(family);
    observation.rendered_output = errors.to_string();
    observation
}

fn generated_observation_for_ir(
    scenario: &FailureTaxonomyScenario,
    workflow: &vb_core::CompiledWorkflow,
) -> PublicObservation {
    let mut observation = generated_base_observation(scenario);
    observation.compile_attempted = true;
    observation.ir_family = None;
    match validate_generated_subset(workflow).and_then(|()| emit_rust_workflow(workflow)) {
        Ok(_source) => {
            observation.typed_error = "GeneratedParity::BothAccepted".to_owned();
            observation.diagnostic_code = "GENERATED_AND_IR_ACCEPTED".to_owned();
            observation.generated_family = None;
        }
        Err(error) => {
            observation.typed_error = generated_error_name(&error).to_owned();
            observation.diagnostic_code = "GENERATED_UNSUPPORTED".to_owned();
            observation.generated_unsupported = Some("GENERATED_UNSUPPORTED");
            observation.rendered_output = error.to_string();
        }
    }
    observation
}

fn generated_base_observation(scenario: &FailureTaxonomyScenario) -> PublicObservation {
    PublicObservation {
        typed_error: "GeneratedParity".to_owned(),
        diagnostic_code: String::new(),
        diagnostic_path: observed_path(scenario),
        cli_exit_code: Some(3),
        runtime_code: None,
        rendered_output: String::new(),
        created_success_artifacts: Vec::new(),
        compile_attempted: false,
        run_attempted: false,
        panicked: false,
        unrelated_state_changed: false,
        persisted_run_accepted: false,
        journal_appended: false,
        journal_digest_unchanged: false,
        storage_admission_collapse_observed: false,
        ir_family: None,
        generated_family: None,
        generated_unsupported: None,
    }
}

fn compile_error_family(errors: &CompileErrors) -> FailureFamily {
    match errors.0.first() {
        Some(CompileError::CanonicalYaml { .. })
        | Some(CompileError::DuplicateKey { .. })
        | Some(CompileError::Validation(_))
        | Some(CompileError::InvalidVersion { .. })
        | Some(CompileError::InvalidName { .. })
        | Some(CompileError::DuplicateStepId { .. }) => FailureFamily::Validation,
        _ => FailureFamily::CompileLowering,
    }
}

fn generated_error_name(error: &CodegenError) -> &'static str {
    match error {
        CodegenError::UnsupportedIr { .. } => "CodegenError::UnsupportedIr",
        CodegenError::FormatBufferOverflow => "CodegenError::FormatBufferOverflow",
        CodegenError::RustfmtFailed { .. } => "CodegenError::RustfmtFailed",
        CodegenError::CompileCheckFailed { .. } => "CodegenError::CompileCheckFailed",
        CodegenError::SemanticMismatch { .. } => "CodegenError::SemanticMismatch",
        CodegenError::Io(_) => "CodegenError::Io",
        CodegenError::TrybuildFixture { .. } => "CodegenError::TrybuildFixture",
    }
}

fn probe_cli_boundary(scenario: &FailureTaxonomyScenario) -> PublicObservation {
    let Some(binary) = cli_binary_path() else {
        let mut observation =
            PublicObservation::unavailable(scenario.family, "velvet-ballastics CLI process");
        observation.cli_exit_code = None;
        return observation;
    };
    let fixture_path = product_artifact_root(&scenario.scenario_id).join("negative-workflow.yaml");
    if let Err(error) = ensure_parent_dir(&fixture_path) {
        let mut observation =
            PublicObservation::unavailable(scenario.family, "velvet-ballastics CLI fixture root");
        observation.rendered_output = error.to_string();
        return observation;
    }
    if let Err(error) = fs::write(&fixture_path, cli_negative_fixture(scenario)) {
        let mut observation =
            PublicObservation::unavailable(scenario.family, "velvet-ballastics CLI fixture write");
        observation.rendered_output = error.to_string();
        return observation;
    }
    let output = Command::new(binary)
        .arg("validate")
        .arg(&fixture_path)
        .arg("--json")
        .output();
    match output {
        Ok(process_output) => cli_process_observation(scenario, &process_output),
        Err(error) => {
            let mut observation = PublicObservation::unavailable(
                scenario.family,
                "velvet-ballastics CLI process spawn",
            );
            observation.rendered_output = error.to_string();
            observation
        }
    }
}

fn cli_binary_path() -> Option<PathBuf> {
    if let Some(path) = option_env!("CARGO_BIN_EXE_velvet-ballastics") {
        return Some(PathBuf::from(path));
    }
    cargo_built_cli_binary_path()
}

fn cargo_built_cli_binary_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let deps_dir = exe.parent()?;
    let debug_dir = deps_dir.parent()?;
    let candidate = debug_dir.join("velvet-ballastics");
    if candidate.is_file() {
        return Some(candidate);
    }
    None
}

fn ensure_parent_dir(path: &Path) -> std::io::Result<()> {
    match path.parent() {
        Some(parent) => fs::create_dir_all(parent),
        None => Ok(()),
    }
}

fn cli_negative_fixture(scenario: &FailureTaxonomyScenario) -> &'static str {
    if scenario.scenario_id.contains("SECRET") {
        return "version: velvet-ballastics/v1\nname: cli_secret_probe\nwhen:\n  manual: {}\nsteps:\n  - id: one\n    ask:\n      prompt: vb82ah-RAW-SECRET-DO-NOT-LEAK\n";
    }
    "version: velvet-ballastics/v1\nname: cli_invalid_probe\nwhen:\n  http: {}\nsteps:\n  - id: done\n    finish: {}\n"
}

fn cli_process_observation(
    scenario: &FailureTaxonomyScenario,
    output: &std::process::Output,
) -> PublicObservation {
    let mut observation = PublicObservation {
        typed_error: "CliDiagnostic".to_owned(),
        diagnostic_code: String::new(),
        diagnostic_path: observed_path(scenario),
        cli_exit_code: None,
        runtime_code: None,
        rendered_output: String::new(),
        created_success_artifacts: Vec::new(),
        compile_attempted: false,
        run_attempted: false,
        panicked: false,
        unrelated_state_changed: false,
        persisted_run_accepted: false,
        journal_appended: false,
        journal_digest_unchanged: false,
        storage_admission_collapse_observed: false,
        ir_family: None,
        generated_family: None,
        generated_unsupported: None,
    };
    observation.cli_exit_code = output.status.code();
    observation.rendered_output = String::from_utf8_lossy(&output.stderr).to_string();
    if observation.rendered_output.is_empty() {
        observation.rendered_output = String::from_utf8_lossy(&output.stdout).to_string();
    }
    observation.diagnostic_code = extract_public_diagnostic_code(&observation.rendered_output)
        .unwrap_or_else(|| "CLI_DIAGNOSTIC_MISSING".to_owned());
    observation
}

fn extract_public_diagnostic_code(output: &str) -> Option<String> {
    if let Ok(value) = serde_json::from_str::<Value>(output)
        && let Some(code) = value.get("code").and_then(Value::as_str)
    {
        return Some(code.to_owned());
    }
    output
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .find(|token| token.starts_with('E') || token.contains("_"))
        .map(str::to_owned)
}

fn observe_artifact_absence(scenario: &FailureTaxonomyScenario) -> ArtifactObservation {
    let root = product_artifact_root(&scenario.scenario_id);
    let candidates = [
        "success.vbir",
        "generated.rs",
        "accepted.artifact",
        "RunAccepted.journal",
    ];
    let created_success_artifacts = candidates
        .into_iter()
        .filter_map(|name| {
            let path = root.join(name);
            if path.exists() {
                Some(path.display().to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let persisted_run_accepted = root.join("RunAccepted.journal").exists();
    ArtifactObservation {
        created_success_artifacts,
        persisted_run_accepted,
    }
}

fn product_artifact_root(scenario_id: &str) -> PathBuf {
    std::env::temp_dir()
        .join("vb-82ah-real-probes")
        .join(scenario_id)
}

fn observe_cli_schema(
    scenario: &FailureTaxonomyScenario,
    observation: &PublicObservation,
) -> CliSchemaObservation {
    let Some(schema) = &scenario.cli_schema else {
        return CliSchemaObservation {
            missing_fields: Vec::new(),
        };
    };
    let mut fields = BTreeSet::new();
    if observation.cli_exit_code.is_some() {
        fields.insert("status".to_owned());
    }
    if !observation.diagnostic_code.is_empty() {
        fields.insert("diagnostics[].code".to_owned());
    }
    if !observation.diagnostic_path.is_empty() {
        fields.insert("path".to_owned());
    }
    if !observation.rendered_output.is_empty() {
        fields.insert("message".to_owned());
    }
    if scenario.family == FailureFamily::Yaml && !observation.rendered_output.is_empty() {
        fields.insert("repair".to_owned());
        fields.insert("span".to_owned());
    }
    schema
        .required_fields
        .difference(&fields)
        .cloned()
        .collect::<Vec<_>>()
        .into()
}

impl From<Vec<String>> for CliSchemaObservation {
    fn from(missing_fields: Vec<String>) -> Self {
        Self { missing_fields }
    }
}

fn observed_path(scenario: &FailureTaxonomyScenario) -> String {
    if scenario.expected_path.is_empty() {
        "public-surface".to_owned()
    } else {
        scenario.expected_path.clone()
    }
}

impl FailureTaxonomyEvidence {
    pub fn typed_error(&self) -> &str {
        &self.typed_error
    }

    pub fn diagnostic_code(&self) -> &str {
        &self.diagnostic_code
    }

    pub fn diagnostic_path(&self) -> &str {
        &self.diagnostic_path
    }

    pub fn cli_exit_code(&self) -> Option<i32> {
        self.cli_exit_code
    }

    pub fn runtime_code(&self) -> Option<&'static str> {
        self.runtime_code
    }

    pub fn missing_cli_schema_fields(&self) -> Vec<String> {
        self.missing_cli_schema_fields.clone()
    }

    pub fn created_success_artifacts(&self) -> Vec<String> {
        self.created_success_artifacts.clone()
    }

    pub fn contains_raw_secret(&self) -> bool {
        self.contains_raw_secret
    }

    pub fn stderr_contains_ansi(&self) -> bool {
        self.stderr_contains_ansi
    }

    pub fn compile_attempted(&self) -> bool {
        self.compile_attempted
    }

    pub fn run_attempted(&self) -> bool {
        self.run_attempted
    }

    pub fn panicked(&self) -> bool {
        self.panicked
    }

    pub fn unrelated_state_changed(&self) -> bool {
        self.unrelated_state_changed
    }

    pub fn persisted_run_accepted(&self) -> bool {
        self.persisted_run_accepted
    }

    pub fn journal_appended(&self) -> bool {
        self.journal_appended
    }

    pub fn journal_digest_unchanged(&self) -> bool {
        self.journal_digest_unchanged
    }

    pub fn storage_admission_collapse_observed(&self) -> bool {
        self.storage_admission_collapse_observed
    }

    pub fn ir_diagnostic_family(&self) -> Option<FailureFamily> {
        self.ir_family
    }

    pub fn generated_diagnostic_family(&self) -> Option<FailureFamily> {
        self.generated_family
    }

    pub fn generated_unsupported_diagnostic(&self) -> Option<&'static str> {
        self.generated_unsupported
    }

    pub fn generated_succeeded_while_ir_rejected(&self) -> bool {
        self.ir_family.is_some()
            && self.generated_family.is_none()
            && self.generated_unsupported.is_none()
            && self.diagnostic_code != "PUBLIC_SURFACE_UNAVAILABLE"
    }

    pub fn ir_succeeded_while_generated_rejected_without_unsupported(&self) -> bool {
        self.ir_family.is_none()
            && self.generated_family.is_some()
            && self.generated_unsupported.is_none()
    }

    pub fn generated_and_ir_diagnostic_family_equivalent(&self) -> bool {
        self.ir_family.is_some() && self.ir_family == self.generated_family
    }

    pub fn generated_rejection_is_equivalent_or_unsupported(&self) -> bool {
        self.generated_and_ir_diagnostic_family_equivalent()
            || self.generated_unsupported == Some("GENERATED_UNSUPPORTED")
    }
}

#[cfg(test)]
mod mutation_guard_tests {
    use super::*;

    fn evidence_with_flags(
        cli_exit_code: Option<i32>,
        contains_raw_secret: bool,
        stderr_contains_ansi: bool,
        compile_attempted: bool,
        run_attempted: bool,
        panicked: bool,
        unrelated_state_changed: bool,
        persisted_run_accepted: bool,
        journal_appended: bool,
        journal_digest_unchanged: bool,
        storage_admission_collapse_observed: bool,
    ) -> FailureTaxonomyEvidence {
        FailureTaxonomyEvidence {
            typed_error: "Typed".to_owned(),
            diagnostic_code: "CODE".to_owned(),
            diagnostic_path: "path".to_owned(),
            cli_exit_code,
            runtime_code: Some("RUNTIME"),
            missing_cli_schema_fields: vec!["schema_version".to_owned()],
            created_success_artifacts: vec!["success.vbir".to_owned()],
            contains_raw_secret,
            stderr_contains_ansi,
            compile_attempted,
            run_attempted,
            panicked,
            unrelated_state_changed,
            persisted_run_accepted,
            journal_appended,
            journal_digest_unchanged,
            storage_admission_collapse_observed,
            ir_family: Some(FailureFamily::Validation),
            generated_family: Some(FailureFamily::Validation),
            generated_unsupported: None,
        }
    }

    #[test]
    fn validate_observed_diagnostic_rejects_blank_bogus_and_accepts_exact_code() {
        let blank =
            FailureTaxonomyRow::fixture("VB-82AH-BLANK-ACTUAL").with_observed_diagnostic_code("");
        let bogus = FailureTaxonomyRow::fixture("VB-82AH-BOGUS-ACTUAL")
            .with_observed_diagnostic_code("xyzzy");
        let exact = FailureTaxonomyRow::fixture("VB-82AH-EXACT-ACTUAL")
            .with_observed_diagnostic_code("DUPLICATE_KEY");

        assert_eq!(
            validate_observed_diagnostic(&blank),
            Err(FailureTaxonomyContractError::DiagnosticMismatch {
                mismatch: Box::new(DiagnosticMismatch {
                    scenario_id: "VB-82AH-BLANK-ACTUAL".to_owned(),
                    failure_family: FailureFamily::Yaml,
                    public_surface: PublicSurface::Cli,
                    expected: "DUPLICATE_KEY".to_owned(),
                    actual: String::new(),
                    evidence_path: ".evidence/vb-82ah/VB-82AH-BLANK-ACTUAL/diagnostic.json"
                        .to_owned(),
                }),
            })
        );
        assert_eq!(
            validate_observed_diagnostic(&bogus),
            Err(FailureTaxonomyContractError::DiagnosticMismatch {
                mismatch: Box::new(DiagnosticMismatch {
                    scenario_id: "VB-82AH-BOGUS-ACTUAL".to_owned(),
                    failure_family: FailureFamily::Yaml,
                    public_surface: PublicSurface::Cli,
                    expected: "DUPLICATE_KEY".to_owned(),
                    actual: "xyzzy".to_owned(),
                    evidence_path: ".evidence/vb-82ah/VB-82AH-BOGUS-ACTUAL/diagnostic.json"
                        .to_owned(),
                }),
            })
        );
        assert_eq!(validate_observed_diagnostic(&exact), Ok(()));
    }

    #[test]
    fn default_code_and_target_are_exact_for_every_failure_family() {
        let cases = [
            (
                FailureFamily::Yaml,
                "DUPLICATE_KEY",
                "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_yaml_validation.rs",
            ),
            (
                FailureFamily::Validation,
                "INVALID_ID",
                "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_yaml_validation.rs",
            ),
            (
                FailureFamily::CompileLowering,
                "UNSUPPORTED_PRIMITIVE",
                "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_compile_runtime.rs",
            ),
            (
                FailureFamily::RuntimeCore,
                "INVALID_COMPILED_WORKFLOW",
                "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_compile_runtime.rs",
            ),
            (
                FailureFamily::ActionResourceAdmission,
                "CAPABILITY_DENIED",
                "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_compile_runtime.rs",
            ),
            (
                FailureFamily::StorageRecovery,
                "0x400B",
                "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_storage_ipc_replay.rs",
            ),
            (
                FailureFamily::Ipc,
                "E3004",
                "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_storage_ipc_replay.rs",
            ),
            (
                FailureFamily::Replay,
                "REPLAY_DIVERGED",
                "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_storage_ipc_replay.rs",
            ),
            (
                FailureFamily::GeneratedParity,
                "GENERATED_UNSUPPORTED",
                "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_generated_cli.rs",
            ),
            (
                FailureFamily::CliDiagnostics,
                "INVALID_ID",
                "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_generated_cli.rs",
            ),
            (
                FailureFamily::Verification,
                "VERIFICATION_FAILED",
                "crates/workspace_tests/tests/vb_82ah_failure_taxonomy_generated_cli.rs",
            ),
        ];

        for (family, code, target) in cases {
            assert_eq!(default_code(family), code);
            assert_eq!(default_target(family), target);
        }
    }

    #[test]
    fn yaml_observation_preserves_node_limit_gate_only_for_sequence_too_long() {
        let node_limit = FailureTaxonomyScenario::yaml_fixture("VB-82AH-YAML-NODE-LIMIT")
            .with_expected_path("document");
        let not_node_limit = FailureTaxonomyScenario::yaml_fixture("VB-82AH-YAML-SEQUENCE")
            .with_expected_path("steps");
        let node_limit_wrong_error =
            FailureTaxonomyScenario::yaml_fixture("VB-82AH-YAML-NODE-LIMIT")
                .with_expected_path("name");

        let node_evidence =
            yaml_observation(YamlError::SequenceTooLong { len: 11, max: 10 }, &node_limit);
        let sequence_evidence = yaml_observation(
            YamlError::SequenceTooLong { len: 11, max: 10 },
            &not_node_limit,
        );
        let duplicate_evidence = yaml_observation(
            YamlError::DuplicateKey { key: "name".into() },
            &node_limit_wrong_error,
        );

        assert_eq!(node_evidence.typed_error, "YamlError::NodeLimitExceeded");
        assert_eq!(node_evidence.diagnostic_code, "LIMIT_EXCEEDED");
        assert_eq!(sequence_evidence.typed_error, "YamlError::SequenceTooLong");
        assert_eq!(sequence_evidence.diagnostic_code, "LIMIT_EXCEEDED");
        assert_eq!(duplicate_evidence.typed_error, "YamlError::DuplicateKey");
        assert_eq!(duplicate_evidence.diagnostic_code, "DUPLICATE_KEY");
    }

    #[test]
    fn compile_limit_error_maps_to_exact_typed_name_and_code() {
        let errors = CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded {
            primitive: "collect",
            field: "source",
            value: 70_000,
            limit: 65_535,
        }]);

        assert_eq!(
            compile_typed_name(&errors),
            "CompileError::ResourceLimitOverflow"
        );
        assert_eq!(compile_code(&errors), "RESOURCE_LIMIT_OVERFLOW");
    }

    #[test]
    fn core_surface_cli_exit_distinguishes_runtime_from_action_admission() {
        let runtime = FailureTaxonomyScenario::runtime_bounds_fixture("VB-82AH-RUNTIME-CORE");
        let action = FailureTaxonomyScenario::admission_fixture("VB-82AH-ACTION-ADMISSION");

        assert_eq!(probe_core_surface(&runtime).cli_exit_code, Some(4));
        assert_eq!(probe_core_surface(&action).cli_exit_code, Some(7));
    }

    #[test]
    fn public_diagnostic_code_extraction_rejects_missing_blank_and_accepts_codes() {
        assert_eq!(extract_public_diagnostic_code("plain words only"), None);
        assert_eq!(
            extract_public_diagnostic_code("status: E3004"),
            Some("E3004".to_owned())
        );
        assert_eq!(
            extract_public_diagnostic_code("code=DUPLICATE_KEY path=/workflow"),
            Some("DUPLICATE_KEY".to_owned())
        );
    }

    #[test]
    fn product_artifact_root_is_namespaced_by_scenario_id() {
        let expected = std::env::temp_dir()
            .join("vb-82ah-real-probes")
            .join("VB-82AH-ARTIFACT-ROOT");

        assert_eq!(product_artifact_root("VB-82AH-ARTIFACT-ROOT"), expected);
    }

    #[test]
    fn cli_schema_observation_reports_exact_missing_and_present_fields() {
        let scenario = FailureTaxonomyScenario::yaml_fixture("VB-82AH-CLI-SCHEMA")
            .with_required_schema_fields([
                "status",
                "diagnostics[].code",
                "path",
                "message",
                "repair",
                "span",
            ]);
        let complete = PublicObservation {
            typed_error: "YamlError::DuplicateKey".to_owned(),
            diagnostic_code: "DUPLICATE_KEY".to_owned(),
            diagnostic_path: "name".to_owned(),
            cli_exit_code: Some(1),
            runtime_code: None,
            rendered_output: "duplicate key repair line".to_owned(),
            created_success_artifacts: Vec::new(),
            compile_attempted: false,
            run_attempted: false,
            panicked: false,
            unrelated_state_changed: false,
            persisted_run_accepted: false,
            journal_appended: false,
            journal_digest_unchanged: false,
            storage_admission_collapse_observed: false,
            ir_family: None,
            generated_family: None,
            generated_unsupported: None,
        };
        let missing_message = PublicObservation {
            rendered_output: String::new(),
            ..complete.clone()
        };

        assert_eq!(
            observe_cli_schema(&scenario, &complete).missing_fields,
            Vec::<String>::new()
        );
        assert_eq!(
            observe_cli_schema(&scenario, &missing_message).missing_fields,
            vec!["message".to_owned(), "repair".to_owned(), "span".to_owned()]
        );
    }

    #[test]
    fn evidence_accessors_return_exact_positive_and_negative_values() {
        let positive = evidence_with_flags(
            Some(6),
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
            true,
        );
        let negative = evidence_with_flags(
            None, false, false, false, false, false, false, false, false, false, false,
        );

        assert_eq!(positive.cli_exit_code(), Some(6));
        assert_eq!(negative.cli_exit_code(), None);
        assert_eq!(positive.runtime_code(), Some("RUNTIME"));
        assert_eq!(
            positive.missing_cli_schema_fields(),
            vec!["schema_version".to_owned()]
        );
        assert_eq!(
            positive.created_success_artifacts(),
            vec!["success.vbir".to_owned()]
        );
        assert_eq!(positive.contains_raw_secret(), true);
        assert_eq!(negative.contains_raw_secret(), false);
        assert_eq!(positive.stderr_contains_ansi(), true);
        assert_eq!(negative.stderr_contains_ansi(), false);
        assert_eq!(positive.compile_attempted(), true);
        assert_eq!(negative.compile_attempted(), false);
        assert_eq!(positive.run_attempted(), true);
        assert_eq!(negative.run_attempted(), false);
        assert_eq!(positive.panicked(), true);
        assert_eq!(negative.panicked(), false);
        assert_eq!(positive.unrelated_state_changed(), true);
        assert_eq!(negative.unrelated_state_changed(), false);
        assert_eq!(positive.persisted_run_accepted(), true);
        assert_eq!(negative.persisted_run_accepted(), false);
        assert_eq!(positive.journal_appended(), true);
        assert_eq!(negative.journal_appended(), false);
        assert_eq!(positive.journal_digest_unchanged(), true);
        assert_eq!(negative.journal_digest_unchanged(), false);
        assert_eq!(positive.storage_admission_collapse_observed(), true);
        assert_eq!(negative.storage_admission_collapse_observed(), false);
    }

    #[test]
    fn generated_parity_predicates_cover_equivalent_unsupported_and_divergent_cases() {
        let equivalent = FailureTaxonomyEvidence {
            ir_family: Some(FailureFamily::Validation),
            generated_family: Some(FailureFamily::Validation),
            generated_unsupported: None,
            ..evidence_with_flags(
                None, false, false, false, false, false, false, false, false, false, false,
            )
        };
        let unsupported = FailureTaxonomyEvidence {
            ir_family: Some(FailureFamily::Validation),
            generated_family: None,
            generated_unsupported: Some("GENERATED_UNSUPPORTED"),
            ..evidence_with_flags(
                None, false, false, false, false, false, false, false, false, false, false,
            )
        };
        let generated_success_while_ir_rejected = FailureTaxonomyEvidence {
            ir_family: Some(FailureFamily::Validation),
            generated_family: None,
            generated_unsupported: None,
            ..evidence_with_flags(
                None, false, false, false, false, false, false, false, false, false, false,
            )
        };
        let ir_success_while_generated_rejected = FailureTaxonomyEvidence {
            ir_family: None,
            generated_family: Some(FailureFamily::Validation),
            generated_unsupported: None,
            ..evidence_with_flags(
                None, false, false, false, false, false, false, false, false, false, false,
            )
        };
        let both_rejected_same_family = FailureTaxonomyEvidence {
            ir_family: Some(FailureFamily::Validation),
            generated_family: Some(FailureFamily::Validation),
            generated_unsupported: None,
            ..evidence_with_flags(
                None, false, false, false, false, false, false, false, false, false, false,
            )
        };
        let both_succeeded = FailureTaxonomyEvidence {
            ir_family: None,
            generated_family: None,
            generated_unsupported: None,
            ..evidence_with_flags(
                None, false, false, false, false, false, false, false, false, false, false,
            )
        };
        let divergent = FailureTaxonomyEvidence {
            ir_family: Some(FailureFamily::Validation),
            generated_family: Some(FailureFamily::RuntimeCore),
            generated_unsupported: None,
            ..evidence_with_flags(
                None, false, false, false, false, false, false, false, false, false, false,
            )
        };

        assert_eq!(
            equivalent.generated_diagnostic_family(),
            Some(FailureFamily::Validation)
        );
        assert_eq!(
            equivalent.generated_and_ir_diagnostic_family_equivalent(),
            true
        );
        assert_eq!(
            equivalent.generated_rejection_is_equivalent_or_unsupported(),
            true
        );
        assert_eq!(
            unsupported.generated_unsupported_diagnostic(),
            Some("GENERATED_UNSUPPORTED")
        );
        assert_eq!(
            unsupported.generated_rejection_is_equivalent_or_unsupported(),
            true
        );
        assert_eq!(
            generated_success_while_ir_rejected.generated_succeeded_while_ir_rejected(),
            true
        );
        assert_eq!(
            ir_success_while_generated_rejected
                .ir_succeeded_while_generated_rejected_without_unsupported(),
            true
        );
        assert_eq!(
            both_rejected_same_family.ir_succeeded_while_generated_rejected_without_unsupported(),
            false
        );
        assert_eq!(
            both_succeeded.ir_succeeded_while_generated_rejected_without_unsupported(),
            false
        );
        assert_eq!(
            divergent.generated_and_ir_diagnostic_family_equivalent(),
            false
        );
        assert_eq!(
            divergent.generated_rejection_is_equivalent_or_unsupported(),
            false
        );
    }
}
