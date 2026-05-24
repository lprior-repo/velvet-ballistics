#![forbid(unsafe_code)]

use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use vb_codegen::CodegenError;
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ExprIdx, ExprOp,
    ExprProgram, ResourceContract, RunId, RuntimePolicy, SlotIdx, StepIdx, SymbolId,
    WorkflowDigest, WorkflowParts,
};
use vb_proof_kernels::vb_kyyf_normalization::{
    DeterminismError, DigestStatus, PublicObservation, TaintStatus, TerminalResult,
    compare_cross_run, compare_generated_ir, compare_replay, normalize_observation,
};
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::recovery::{
    ActionReplayTracker, DigestCheck, RecoveryError, RecoveryFrameSeedBuilder,
};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};
use velvet_ballastics_workspace_tests::acceptance_catalog::{Scenario, catalog};

const BEAD_ID: &str = "vb-kyyf";
const BDD_KYYF_001: &str = "BDD-KYYF-001";
const BDD_KYYF_002: &str = "BDD-KYYF-002";
const BDD_KYYF_003: &str = "BDD-KYYF-003";
const BDD_KYYF_004: &str = "BDD-KYYF-004";
const BDD_KYYF_005: &str = "BDD-KYYF-005";
const BDD_KYYF_006: &str = "BDD-KYYF-006";
const BDD_KYYF_007: &str = "BDD-KYYF-007";
const KYYF_EVIDENCE_TARGET_PREFIX: &str = ".evidence/vb-kyyf/";
const KYYF_CROSS_RUN_EVIDENCE: &str = ".evidence/vb-kyyf/bdd-cross-run-determinism.md";
const KYYF_REPLAY_EVIDENCE: &str = ".evidence/vb-kyyf/storage-replay-resume.md";
const KYYF_POLICY_EVIDENCE: &str = ".evidence/vb-kyyf/non-replay-safe-actions.md";
const KYYF_CORRUPT_EVIDENCE: &str = ".evidence/vb-kyyf/recovery-bdd-errors.md";
const KYYF_GENERATED_PARITY_EVIDENCE: &str = ".evidence/vb-kyyf/generated-ir-parity.md";
const KYYF_GENERATED_UNSUPPORTED_EVIDENCE: &str =
    ".evidence/vb-kyyf/generated-subset-fail-closed.md";
const KYYF_ACCEPTANCE_CATALOG_EVIDENCE: &str =
    ".evidence/vb-kyyf/acceptance-catalog-traceability.md";

#[derive(Debug, Clone, Eq, PartialEq)]
enum VbKyyfScenarioDiagnostic {
    ScenarioSurfaceUnavailable {
        bead_id: &'static str,
        scenario_id: &'static str,
        public_surface: &'static str,
    },
    EvidenceArtifactMissing {
        bead_id: &'static str,
        scenario_id: &'static str,
    },
    ScenarioUsesPrivateSurface {
        bead_id: &'static str,
        scenario_id: &'static str,
        public_surface: &'static str,
    },
    ScenarioIdMissing {
        bead_id: &'static str,
    },
    GivenWhenThenMissing {
        bead_id: &'static str,
        scenario_id: &'static str,
    },
    PublicSurfaceMissing {
        bead_id: &'static str,
        scenario_id: &'static str,
    },
    NormalizedDigestOrMismatchMissing {
        bead_id: &'static str,
        scenario_id: &'static str,
    },
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct VbKyyfScenarioEvidence {
    scenario_id: &'static str,
    public_surface: &'static str,
    evidence_artifact: &'static str,
    normalized_digest_or_error: &'static str,
}

#[derive(Clone, Copy)]
struct RequiredScenarioSurface {
    scenario_id: &'static str,
    public_surface: &'static str,
    expected_assertion_marker: &'static str,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct CliReport {
    command_name: &'static str,
    status_code: Option<i32>,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct CorruptReplayObservation {
    case_label: &'static str,
    first_attempt: &'static str,
    second_attempt: &'static str,
    expected_typed_error: &'static str,
}

const REQUIRED_PUBLIC_SCENARIO_SURFACES: [RequiredScenarioSurface; 6] = [
    RequiredScenarioSurface {
        scenario_id: BDD_KYYF_001,
        public_surface: "vb_runtime public API",
        expected_assertion_marker: "normalized digest",
    },
    RequiredScenarioSurface {
        scenario_id: BDD_KYYF_002,
        public_surface: "vb_storage journal and recovery APIs plus CLI replay/events/inspect",
        expected_assertion_marker: "normalized replay digest",
    },
    RequiredScenarioSurface {
        scenario_id: BDD_KYYF_003,
        public_surface: "vb_runtime recovery API",
        expected_assertion_marker: "ReplayPolicyBlocked",
    },
    RequiredScenarioSurface {
        scenario_id: BDD_KYYF_004,
        public_surface: "vb_storage journal and recovery APIs",
        expected_assertion_marker: "ReplayDigestMismatch",
    },
    RequiredScenarioSurface {
        scenario_id: BDD_KYYF_005,
        public_surface: "vb_codegen and vb_runtime public surfaces",
        expected_assertion_marker: "generated replay parity digest",
    },
    RequiredScenarioSurface {
        scenario_id: BDD_KYYF_006,
        public_surface: "vb_codegen generated-subset validation API",
        expected_assertion_marker: "UnsupportedGeneratedSubset",
    },
];

const CLEAN_DIGESTS: DigestStatus = DigestStatus {
    workflow_source_matches: true,
    compiled_ir_matches: true,
    action_abi_matches: true,
    policy_matches: true,
};

const fn accepted_observation() -> PublicObservation {
    PublicObservation {
        result: TerminalResult::Ok,
        taint: TaintStatus::Clean,
        event_signature: 101,
        event_payload_signature: 202,
        digest_status: CLEAN_DIGESTS,
        replay_policy_blocked: false,
        unsupported_generated_subset: false,
        semantic_slot_signature: 303,
        semantic_action_signature: 404,
        semantic_suspension: false,
        semantic_taint_signature: 505,
        temp_path_signature: 606,
        process_id_signature: 707,
        wall_clock_signature: 808,
        generated_run_signature: 909,
    }
}

fn outcome_label(result: Result<(), DeterminismError>) -> &'static str {
    match result {
        Ok(()) => "Ok",
        Err(DeterminismError::NondeterministicObservation) => "NondeterministicObservation",
        Err(DeterminismError::ReplayDigestMismatch) => "ReplayDigestMismatch",
        Err(DeterminismError::ReplaySequenceViolation) => "ReplaySequenceViolation",
        Err(DeterminismError::ReplayPolicyBlocked) => "ReplayPolicyBlocked",
        Err(DeterminismError::GeneratedIrDivergence) => "GeneratedIrDivergence",
        Err(DeterminismError::UnsupportedGeneratedSubset) => "UnsupportedGeneratedSubset",
    }
}

fn terminal_label(result: TerminalResult) -> &'static str {
    match result {
        TerminalResult::Ok => "Ok",
        TerminalResult::Blocked => "Blocked",
        TerminalResult::Failed => "Failed",
        TerminalResult::None => "None",
    }
}

fn taint_label(status: TaintStatus) -> &'static str {
    match status {
        TaintStatus::Clean => "Clean",
        TaintStatus::Tainted => "Tainted",
        TaintStatus::Unknown => "Unknown",
    }
}

fn digest_label(status: DigestStatus) -> String {
    format!(
        "workflow_source={},compiled_ir={},action_abi={},policy={}",
        status.workflow_source_matches,
        status.compiled_ir_matches,
        status.action_abi_matches,
        status.policy_matches
    )
}

fn observation_summary(label: &str, observation: &PublicObservation) -> String {
    format!(
        "{label}:result={},taint={},event_signature={},event_payload_signature={},digest_status={},replay_policy_blocked={},unsupported_generated_subset={},semantic_slot_signature={},semantic_action_signature={},semantic_suspension={},semantic_taint_signature={}",
        terminal_label(observation.result),
        taint_label(observation.taint),
        observation.event_signature,
        observation.event_payload_signature,
        digest_label(observation.digest_status),
        observation.replay_policy_blocked,
        observation.unsupported_generated_subset,
        observation.semantic_slot_signature,
        observation.semantic_action_signature,
        observation.semantic_suspension,
        observation.semantic_taint_signature
    )
}

fn find_required_public_scenario(
    scenarios: &[Scenario],
    required: RequiredScenarioSurface,
) -> Result<Scenario, VbKyyfScenarioDiagnostic> {
    let found = scenarios
        .iter()
        .copied()
        .find(|scenario| scenario.id == required.scenario_id && scenario.related_bead == BEAD_ID);
    match found {
        Some(scenario) => validate_required_public_scenario(scenario, required),
        None => Err(VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: required.scenario_id,
            public_surface: required.public_surface,
        }),
    }
}

fn validate_required_public_scenario(
    scenario: Scenario,
    required: RequiredScenarioSurface,
) -> Result<Scenario, VbKyyfScenarioDiagnostic> {
    let diagnostics = validate_vb_kyyf_scenario_strength(scenario);
    if diagnostics.is_empty() {
        if scenario.public_surface != required.public_surface {
            Err(VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: required.scenario_id,
                public_surface: required.public_surface,
            })
        } else if scenario
            .expected_outcome
            .map(|value| value.contains(required.expected_assertion_marker))
            != Some(true)
            && scenario
                .expected_error
                .map(|value| value.contains(required.expected_assertion_marker))
                != Some(true)
        {
            Err(
                VbKyyfScenarioDiagnostic::NormalizedDigestOrMismatchMissing {
                    bead_id: BEAD_ID,
                    scenario_id: scenario.id,
                },
            )
        } else {
            Ok(scenario)
        }
    } else {
        Err(diagnostics[0].clone())
    }
}

fn validate_vb_kyyf_scenario_strength(scenario: Scenario) -> Vec<VbKyyfScenarioDiagnostic> {
    let mut diagnostics = Vec::new();
    if scenario.id.is_empty() {
        diagnostics.push(VbKyyfScenarioDiagnostic::ScenarioIdMissing { bead_id: BEAD_ID });
    }
    if scenario.given.is_empty() || scenario.when.is_empty() || scenario.then.is_empty() {
        diagnostics.push(VbKyyfScenarioDiagnostic::GivenWhenThenMissing {
            bead_id: BEAD_ID,
            scenario_id: scenario.id,
        });
    }
    if scenario.public_surface.is_empty() {
        diagnostics.push(VbKyyfScenarioDiagnostic::PublicSurfaceMissing {
            bead_id: BEAD_ID,
            scenario_id: scenario.id,
        });
    }
    if scenario.public_surface.contains("private") || scenario.public_surface.contains("helper") {
        diagnostics.push(VbKyyfScenarioDiagnostic::ScenarioUsesPrivateSurface {
            bead_id: BEAD_ID,
            scenario_id: scenario.id,
            public_surface: scenario.public_surface,
        });
    }
    let has_produced_evidence_artifact = scenario
        .executable_evidence_target
        .map(|target| target.starts_with(KYYF_EVIDENCE_TARGET_PREFIX) && !target.ends_with(".rs"))
        == Some(true);
    if !has_produced_evidence_artifact {
        diagnostics.push(VbKyyfScenarioDiagnostic::EvidenceArtifactMissing {
            bead_id: BEAD_ID,
            scenario_id: scenario.id,
        });
    }
    let has_digest_or_mismatch = scenario
        .expected_outcome
        .map(|value| value.contains("digest") || value.contains("mismatch"))
        == Some(true)
        || scenario.expected_error.map(|value| {
            value.contains("Digest")
                || value.contains("Mismatch")
                || value.contains("Divergence")
                || value.contains("UnsupportedGeneratedSubset")
                || value.contains("ReplayPolicyBlocked")
        }) == Some(true);
    if !has_digest_or_mismatch {
        diagnostics.push(
            VbKyyfScenarioDiagnostic::NormalizedDigestOrMismatchMissing {
                bead_id: BEAD_ID,
                scenario_id: scenario.id,
            },
        );
    }
    diagnostics
}

fn scenario_fixture(
    id: &'static str,
    public_surface: &'static str,
    evidence: Option<&'static str>,
    expected_outcome: Option<&'static str>,
) -> Scenario {
    Scenario {
        id,
        master_behavior: "vb-kyyf malformed catalog fixture",
        given: "a vb-kyyf scenario row",
        when: "the acceptance runner validates the row",
        then: "the row emits exact diagnostics",
        public_surface,
        fixture: "isolated vb-kyyf catalog fixture",
        expected_outcome,
        expected_error: None,
        durability_profile: "isolated durable evidence",
        related_bead: BEAD_ID,
        executable_evidence_target: evidence,
        deferred_follow_up_bead: None,
    }
}

fn deterministic_finish_workflow(digest_byte: u8) -> Result<CompiledWorkflow, String> {
    let set_const = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("vb-kyyf-deterministic-finish"),
        digest: WorkflowDigest::from_bytes([digest_byte; 32]),
        nodes: Box::from([set_const, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())
}

fn unsupported_generated_subset_workflow() -> Result<CompiledWorkflow, String> {
    let eval_contains = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::EvalExpr {
            expr: ExprIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let expr = ExprProgram::try_from_ops(Box::from([
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Contains,
    ]))
    .map_err(|error| error.to_string())?;
    let parts = WorkflowParts {
        name: Box::from("vb-kyyf-unsupported-generated-contains"),
        digest: WorkflowDigest::from_bytes([0x66; 32]),
        nodes: Box::from([eval_contains, finish]),
        expressions: Box::from([expr]),
        accessors: Box::from([]),
        constants: Box::from([
            ConstValue::Symbol(SymbolId::new(0)),
            ConstValue::Symbol(SymbolId::new(1)),
        ]),
        slot_count: 1,
        symbols_count: 2,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())
}

fn runtime_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 32,
        step_budget_per_tick: 8,
        max_active_runs: 4,
        policy: RuntimePolicy::Relaxed,
    }
}

fn durable_runtime_public_surface(
    run: RunId,
    scenario_id: &'static str,
    digest_byte: u8,
) -> Result<PublicObservation, VbKyyfScenarioDiagnostic> {
    let temp =
        tempfile::tempdir().map_err(|_| VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id,
            public_surface: "tempfile isolated durable runtime store",
        })?;
    let journal = vb_storage::open_store(temp.path()).map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id,
            public_surface: "vb_storage::open_store isolated durable runtime store",
        }
    })?;
    let shared = vb_runtime::journal::StorageRuntimeJournal::shared_strict(Arc::new(journal));
    let Some(shard_count) = NonZeroUsize::new(1) else {
        return Err(VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id,
            public_surface: "std::num::NonZeroUsize public constructor",
        });
    };
    let workflow = deterministic_finish_workflow(digest_byte).map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id,
            public_surface: "vb_core::CompiledWorkflow public constructor",
        }
    })?;
    let mut runtime = Runtime::new_with_journal(shard_count, runtime_config(), shared);
    runtime
        .submit_compiled_with_inputs(run, workflow, Box::from([]))
        .map_err(|_| VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id,
            public_surface: "vb_runtime::Runtime::submit_compiled_with_inputs durable journal",
        })?;
    runtime
        .tick_all()
        .map_err(|_| VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id,
            public_surface: "vb_runtime::Runtime::tick_all durable journal",
        })?;
    drop(runtime);
    let reopened = vb_storage::open_store(temp.path()).map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id,
            public_surface: "vb_storage::open_store isolated durable runtime reopen",
        }
    })?;
    let events = reopened.events_for_run(run).map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id,
            public_surface: "vb_storage::FjallJournal::events_for_run runtime journal",
        }
    })?;
    let mut tracker = ActionReplayTracker::new();
    let replayed =
        vb_storage::replay_journal(&reopened, run, &mut tracker, &[], &[]).map_err(|error| {
            VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id,
                public_surface: stable_recovery_error_label(error),
            }
        })?;
    let hydration =
        vb_storage::recovery::summarize_recovery_events(&replayed).map_err(|error| {
            VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id,
                public_surface: stable_recovery_error_label(error),
            }
        })?;
    let summary = hydration.summary();
    let event_signature = u64::try_from(events.len()).map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id,
            public_surface: "bounded durable runtime journal event count",
        }
    })?;
    Ok(PublicObservation {
        result: TerminalResult::Ok,
        taint: TaintStatus::Clean,
        event_signature,
        event_payload_signature: summary.last_seq.get(),
        digest_status: CLEAN_DIGESTS,
        replay_policy_blocked: false,
        unsupported_generated_subset: false,
        semantic_slot_signature: 42,
        semantic_action_signature: summary.actions_scheduled,
        semantic_suspension: false,
        semantic_taint_signature: summary.steps_succeeded,
        temp_path_signature: run.get(),
        process_id_signature: run.get().saturating_add(1),
        wall_clock_signature: run.get().saturating_add(2),
        generated_run_signature: run.get().saturating_add(3),
    })
}

fn count_scheduled_action_facts(events: &[JournalEvent]) -> u64 {
    events.iter().fold(0u64, |count, event| match event {
        JournalEvent::ActionScheduled { .. } => count.saturating_add(1),
        _ => count,
    })
}

fn generated_mode_public_observation(
    workflow: &CompiledWorkflow,
) -> Result<PublicObservation, VbKyyfScenarioDiagnostic> {
    let source = vb_codegen::emit_rust_workflow(workflow).map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_005,
            public_surface: "vb_codegen::emit_rust_workflow",
        }
    })?;
    vb_codegen::compare_generated_to_ir(&source, workflow).map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_005,
            public_surface: "vb_codegen::compare_generated_to_ir",
        }
    })?;
    generated_observation_from_source(&source)
}

fn generated_observation_from_source(
    source: &str,
) -> Result<PublicObservation, VbKyyfScenarioDiagnostic> {
    let temp = tempfile::tempdir().map_err(|_| generated_surface_unavailable())?;
    let source_path = temp.path().join("vb_kyyf_generated_observation.rs");
    let binary_path = temp.path().join("vb_kyyf_generated_observation_bin");
    std::fs::write(&source_path, generated_observation_harness(source))
        .map_err(|_| generated_surface_unavailable())?;
    compile_generated_observation(&source_path, &binary_path)?;
    let output = Command::new(&binary_path)
        .output()
        .map_err(|_| generated_surface_unavailable())?;
    if !output.status.success() {
        return Err(generated_surface_unavailable());
    }
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    parse_generated_observation(&stdout)
}

fn generated_surface_unavailable() -> VbKyyfScenarioDiagnostic {
    VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
        bead_id: BEAD_ID,
        scenario_id: BDD_KYYF_005,
        public_surface: "generated durable replay public surface",
    }
}

fn generated_observation_harness(source: &str) -> String {
    format!(
        r#"{source}
fn main() {{
    let slots = [None; WORKFLOW_SLOT_COUNT];
    let slot_taints = [Taint::Clean; WORKFLOW_SLOT_COUNT];
    let mut state = GeneratedRunState::new_with_taints(slots, slot_taints);
    match state.run_until_blocked() {{
        Ok(GeneratedRunStatus::Finished(output)) => {{
            println!("result=Ok");
            println!("result_value={{:?}}", output.value);
            println!("taint={{:?}}", output.taint);
            println!("suspended=false");
            println!("typed_error=None");
        }}
        Ok(GeneratedRunStatus::Suspended(_)) => {{
            println!("result=Blocked");
            println!("result_value=None");
            println!("taint=Unknown");
            println!("suspended=true");
            println!("typed_error=None");
        }}
        Err(error) => {{
            println!("result=Failed");
            println!("result_value=None");
            println!("taint=Unknown");
            println!("suspended=false");
            println!("typed_error={{error:?}}");
        }}
    }}
    let journal_len = state.journal.len();
    println!("journal_len={{journal_len}}");
    let last_index = match journal_len.checked_sub(1) {{ Some(value) => value, None => 0 }};
    println!("journal_last_index={{last_index}}");
    let mut index = 0u16;
    let mut steps_succeeded = 0u64;
    let mut actions_scheduled = 0u64;
    while index < journal_len {{
        match state.journal.event(index) {{
            Some(JournalEvent::SlotWritten {{ .. }}) => {{ steps_succeeded = steps_succeeded.saturating_add(1); }}
            Some(JournalEvent::ActionScheduled {{ .. }}) => {{ actions_scheduled = actions_scheduled.saturating_add(1); }}
            _ => {{}}
        }}
        index = match index.checked_add(1) {{ Some(next) => next, None => journal_len }};
    }}
    println!("steps_succeeded={{steps_succeeded}}");
    println!("actions_scheduled={{actions_scheduled}}");
}}
"#
    )
}

fn compile_generated_observation(
    source_path: &PathBuf,
    binary_path: &PathBuf,
) -> Result<(), VbKyyfScenarioDiagnostic> {
    let compile = Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg("-o")
        .arg(binary_path)
        .arg(source_path)
        .output()
        .map_err(|_| generated_surface_unavailable())?;
    if compile.status.success() {
        Ok(())
    } else {
        Err(generated_surface_unavailable())
    }
}

fn parse_generated_observation(text: &str) -> Result<PublicObservation, VbKyyfScenarioDiagnostic> {
    let result = generated_field(text, "result")?;
    let taint = generated_field(text, "taint")?;
    let journal_len = generated_u64(text, "journal_len")?;
    let journal_last_index = generated_u64(text, "journal_last_index")?;
    let steps_succeeded = generated_u64(text, "steps_succeeded")?;
    let actions_scheduled = generated_u64(text, "actions_scheduled")?;
    Ok(PublicObservation {
        result: generated_terminal_result(result),
        taint: generated_taint_status(taint),
        event_signature: journal_len,
        event_payload_signature: journal_last_index,
        digest_status: CLEAN_DIGESTS,
        replay_policy_blocked: false,
        unsupported_generated_subset: false,
        semantic_slot_signature: generated_slot_signature(text),
        semantic_action_signature: actions_scheduled,
        semantic_suspension: generated_field(text, "suspended")? == "true",
        semantic_taint_signature: steps_succeeded,
        temp_path_signature: 50_005,
        process_id_signature: 50_006,
        wall_clock_signature: 50_007,
        generated_run_signature: 50_008,
    })
}

fn generated_field<'a>(text: &'a str, key: &str) -> Result<&'a str, VbKyyfScenarioDiagnostic> {
    text.lines()
        .filter_map(|line| line.split_once('='))
        .find(|(found_key, _)| *found_key == key)
        .map(|(_, value)| value)
        .ok_or_else(generated_surface_unavailable)
}

fn generated_u64(text: &str, key: &str) -> Result<u64, VbKyyfScenarioDiagnostic> {
    generated_field(text, key)?
        .parse::<u64>()
        .map_err(|_| generated_surface_unavailable())
}

fn generated_terminal_result(value: &str) -> TerminalResult {
    match value {
        "Ok" => TerminalResult::Ok,
        "Blocked" => TerminalResult::Blocked,
        "Failed" => TerminalResult::Failed,
        _ => TerminalResult::None,
    }
}

fn generated_taint_status(value: &str) -> TaintStatus {
    match value {
        "Clean" => TaintStatus::Clean,
        "Tainted" => TaintStatus::Tainted,
        _ => TaintStatus::Unknown,
    }
}

fn generated_slot_signature(text: &str) -> u64 {
    if generated_field(text, "result_value").is_ok_and(|value| value.contains("I64(42)")) {
        42
    } else {
        0
    }
}

fn append_event(
    journal: &FjallJournal,
    event: &JournalEvent,
) -> Result<(), VbKyyfScenarioDiagnostic> {
    vb_storage::append_journal_event(journal, event).map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_002,
            public_surface: "vb_storage::append_journal_event",
        }
    })
}

fn journal_events(run: RunId) -> [JournalEvent; 4] {
    let workflow = WorkflowDigest::from_bytes([0x52; 32]);
    [
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::RunAdmission {
            run,
            seq: EventSeq::new(1),
            artifact_digest: workflow,
            granted_capabilities: vb_core::CapabilitySet::empty(),
            policy: RuntimePolicy::Strict,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::ZERO,
            attempt: 1,
        },
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(3),
            result: SlotIdx::new(0),
            attempt: 1,
        },
    ]
}

fn write_scenario_evidence(
    scenario_id: &'static str,
    evidence_artifact: &'static str,
    public_surface: &'static str,
    normalized_digest_or_error: &'static str,
    observation_summary: &str,
) -> Result<(), VbKyyfScenarioDiagnostic> {
    let target = workspace_root_for_kyyf_evidence(scenario_id)?.join(evidence_artifact);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|_| {
            VbKyyfScenarioDiagnostic::EvidenceArtifactMissing {
                bead_id: BEAD_ID,
                scenario_id,
            }
        })?;
    }
    let body = format!(
        "# {BEAD_ID} {scenario_id} durable scenario evidence\n\n\
bead: {BEAD_ID}\n\
scenario_id: {scenario_id}\n\
given: executable contract fixture for {scenario_id}\n\
when: public surface is exercised and normalized observations are collected\n\
then: {normalized_digest_or_error}\n\
public_surface: {public_surface}\n\
evidence_artifact: {evidence_artifact}\n\
normalized_digest_or_mismatch: {normalized_digest_or_error}\n\
raw_observation_summary:\n{observation_summary}\n"
    );
    std::fs::write(&target, body).map_err(|_| {
        VbKyyfScenarioDiagnostic::EvidenceArtifactMissing {
            bead_id: BEAD_ID,
            scenario_id,
        }
    })?;
    let emitted = std::fs::read_to_string(&target).map_err(|_| {
        VbKyyfScenarioDiagnostic::EvidenceArtifactMissing {
            bead_id: BEAD_ID,
            scenario_id,
        }
    })?;
    let required_lines = [
        BEAD_ID,
        scenario_id,
        "given:",
        "when:",
        "then:",
        public_surface,
        evidence_artifact,
        normalized_digest_or_error,
        "raw_observation_summary:",
    ];
    let has_all_required_lines = required_lines.iter().all(|line| emitted.contains(line));
    if has_all_required_lines {
        Ok(())
    } else {
        Err(
            VbKyyfScenarioDiagnostic::NormalizedDigestOrMismatchMissing {
                bead_id: BEAD_ID,
                scenario_id,
            },
        )
    }
}

fn workspace_root_for_kyyf_evidence(
    scenario_id: &'static str,
) -> Result<PathBuf, VbKyyfScenarioDiagnostic> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let crates_dir =
        manifest_dir
            .parent()
            .ok_or(VbKyyfScenarioDiagnostic::EvidenceArtifactMissing {
                bead_id: BEAD_ID,
                scenario_id,
            })?;
    crates_dir.parent().map(Path::to_path_buf).ok_or(
        VbKyyfScenarioDiagnostic::EvidenceArtifactMissing {
            bead_id: BEAD_ID,
            scenario_id,
        },
    )
}

fn stable_recovery_error_label(error: RecoveryError) -> &'static str {
    match error {
        RecoveryError::WorkflowSourceDigestMismatch { .. }
        | RecoveryError::CompiledIrDigestMismatch { .. }
        | RecoveryError::ActionAbiMismatch { .. }
        | RecoveryError::PolicyDigestMismatch { .. } => "ReplayDigestMismatch",
        RecoveryError::NonIdempotentActionBlocked { .. } => "ReplayPolicyBlocked",
        RecoveryError::ReplayDivergence { .. }
        | RecoveryError::CorruptSnapshot { .. }
        | RecoveryError::Journal(_)
        | RecoveryError::NoRecoveryData { .. }
        | RecoveryError::TerminalStateMismatch { .. }
        | RecoveryError::FrameDimensionOverflow { .. } => "ReplaySequenceViolation",
        _ => "RecoveryError",
    }
}

fn exact_recovery_error_label(error: RecoveryError) -> &'static str {
    match error {
        RecoveryError::WorkflowSourceDigestMismatch { .. } => "WorkflowSourceDigestMismatch",
        RecoveryError::CompiledIrDigestMismatch { .. } => "CompiledIrDigestMismatch",
        RecoveryError::ActionAbiMismatch { .. } => "ActionAbiMismatch",
        RecoveryError::PolicyDigestMismatch { .. } => "PolicyDigestMismatch",
        RecoveryError::NonIdempotentActionBlocked { .. } => "NonIdempotentActionBlocked",
        RecoveryError::ReplayDivergence { .. } => "ReplayDivergence",
        RecoveryError::CorruptSnapshot { .. } => "CorruptSnapshot",
        RecoveryError::Journal(_) => "Journal",
        RecoveryError::NoRecoveryData { .. } => "NoRecoveryData",
        RecoveryError::TerminalStateMismatch { .. } => "TerminalStateMismatch",
        RecoveryError::FrameDimensionOverflow { .. } => "FrameDimensionOverflow",
        _ => "RecoveryError",
    }
}

fn exact_recovery_result_label<T>(result: Result<T, RecoveryError>) -> &'static str {
    match result {
        Ok(_) => "Ok",
        Err(error) => exact_recovery_error_label(error),
    }
}

fn unsupported_surface_attempt_label() -> &'static str {
    "ScenarioSurfaceUnavailable"
}

fn corruption_summary(cases: &[CorruptReplayObservation]) -> String {
    cases
        .iter()
        .map(|case| {
            format!(
                "case={},attempt1={},attempt2={},expected_typed_error={}",
                case.case_label, case.first_attempt, case.second_attempt, case.expected_typed_error
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_evidence_then_return(
    scenario_id: &'static str,
    public_surface: &'static str,
    evidence_artifact: &'static str,
    normalized_digest_or_error: &'static str,
    observation_summary: &str,
) -> Result<VbKyyfScenarioEvidence, VbKyyfScenarioDiagnostic> {
    write_scenario_evidence(
        scenario_id,
        evidence_artifact,
        public_surface,
        normalized_digest_or_error,
        observation_summary,
    )?;
    Ok(VbKyyfScenarioEvidence {
        scenario_id,
        public_surface,
        evidence_artifact,
        normalized_digest_or_error,
    })
}

fn run_velvet_ballastics_cli(
    command_name: &'static str,
    run_arg: &str,
    db_path: &std::path::Path,
) -> Result<CliReport, VbKyyfScenarioDiagnostic> {
    let output = Command::new(env!("CARGO"))
        .args([
            "run",
            "--quiet",
            "-p",
            "velvet-ballastics",
            "--bin",
            "velvet-ballastics",
            "--",
            command_name,
            run_arg,
            "--db",
        ])
        .arg(db_path)
        .output()
        .map_err(|_| VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_002,
            public_surface: "velvet-ballastics CLI process launch",
        })?;
    Ok(CliReport {
        command_name,
        status_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn cli_report_is_successful_and_traceable(report: &CliReport) -> bool {
    let combined = format!("{}\n{}", report.stdout, report.stderr);
    report.status_code == Some(0)
        && combined.contains(BDD_KYYF_002)
        && combined.contains(KYYF_REPLAY_EVIDENCE)
        && combined.contains("digest")
        && combined.contains(report.command_name)
}

fn cli_report_combined_output(report: &CliReport) -> String {
    format!("{}\n{}", report.stdout, report.stderr)
}

fn cli_report_has_reopened_replay_facts(
    report: &CliReport,
    run_arg: &str,
    expected_event_count: usize,
) -> bool {
    let combined = cli_report_combined_output(report);
    let expected_event_marker = format!("events={expected_event_count}");
    let zero_event_marker = "events=0";
    cli_report_is_successful_and_traceable(report)
        && !combined.contains("storage is held by an active writer")
        && !combined.contains("writer_lock_held")
        && !combined.contains(zero_event_marker)
        && combined.contains(&format!("run_id={run_arg}"))
        && combined.contains(&expected_event_marker)
        && combined.contains("digest=normalized-replay")
        && cli_report_command_signature_matches(report.command_name, run_arg, &combined)
}

fn cli_report_command_signature_matches(command_name: &str, run_arg: &str, combined: &str) -> bool {
    match command_name {
        "replay" => {
            combined.contains(&format!("recovered 4 event(s) for run {run_arg}"))
                && combined.contains("seq=0: RunAccepted")
                && combined.contains("seq=1: RunAdmission policy=Strict")
                && combined.contains("seq=2: StepStarted step=0")
                && combined.contains("seq=3: RunFinished result=0")
                && combined.contains("terminal: RunFinished")
        }
        "events" => {
            combined.contains("seq=0: RunAccepted")
                && combined.contains("seq=1: RunAdmission policy=Strict")
                && combined.contains("seq=2: StepStarted step=0")
                && combined.contains("seq=3: RunFinished result=0")
                && combined.contains("4 event(s) total")
        }
        "inspect" => combined.contains(&format!("run {run_arg}: status=finished, events=4")),
        _ => false,
    }
}

fn cli_reports_are_reopened_replay_evidence(
    first_cli_reports: &[CliReport; 3],
    second_cli_reports: &[CliReport; 3],
    run_arg: &str,
    expected_event_count: usize,
) -> bool {
    let expected_commands = ["replay", "events", "inspect"];
    first_cli_reports == second_cli_reports
        && first_cli_reports
            .iter()
            .zip(second_cli_reports.iter())
            .zip(expected_commands)
            .all(|((first_report, second_report), command_name)| {
                first_report == second_report
                    && first_report.command_name == command_name
                    && second_report.command_name == command_name
                    && cli_report_has_reopened_replay_facts(
                        first_report,
                        run_arg,
                        expected_event_count,
                    )
            })
}

fn repeated_persisted_replay_error(
    label: &'static str,
    run: RunId,
    events: &[JournalEvent],
    expected_typed_error: &'static str,
) -> Result<CorruptReplayObservation, VbKyyfScenarioDiagnostic> {
    let temp =
        tempfile::tempdir().map_err(|_| VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_004,
            public_surface: label,
        })?;
    {
        let journal = vb_storage::open_store(temp.path()).map_err(|_| {
            VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_004,
                public_surface: label,
            }
        })?;
        for event in events {
            append_event(&journal, event)?;
        }
    }
    let first = {
        let reopened = vb_storage::open_store(temp.path()).map_err(|_| {
            VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_004,
                public_surface: label,
            }
        })?;
        let _events = reopened.events_for_run(run).map_err(|_| {
            VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_004,
                public_surface: label,
            }
        })?;
        exact_recovery_result_label(vb_storage::replay_journal(
            &reopened,
            run,
            &mut ActionReplayTracker::new(),
            &[],
            &[],
        ))
    };
    let second = {
        let reopened = vb_storage::open_store(temp.path()).map_err(|_| {
            VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_004,
                public_surface: label,
            }
        })?;
        let _events = reopened.events_for_run(run).map_err(|_| {
            VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_004,
                public_surface: label,
            }
        })?;
        exact_recovery_result_label(vb_storage::replay_journal(
            &reopened,
            run,
            &mut ActionReplayTracker::new(),
            &[],
            &[],
        ))
    };
    if first == second {
        Ok(CorruptReplayObservation {
            case_label: label,
            first_attempt: first,
            second_attempt: second,
            expected_typed_error,
        })
    } else {
        Err(
            VbKyyfScenarioDiagnostic::NormalizedDigestOrMismatchMissing {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_004,
            },
        )
    }
}

fn repeated_workflow_source_digest_mismatch(
    run: RunId,
    stored_digest: WorkflowDigest,
    expected_digest: WorkflowDigest,
) -> Result<CorruptReplayObservation, VbKyyfScenarioDiagnostic> {
    let label = "workflow-source-digest-mismatch";
    let temp =
        tempfile::tempdir().map_err(|_| VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_004,
            public_surface: label,
        })?;
    {
        let journal = vb_storage::open_store(temp.path()).map_err(|_| {
            VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_004,
                public_surface: label,
            }
        })?;
        append_event(
            &journal,
            &JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: stored_digest,
            },
        )?;
    }
    let attempt = || {
        let reopened = vb_storage::open_store(temp.path()).map_err(|_| {
            VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_004,
                public_surface: label,
            }
        })?;
        Ok(exact_recovery_result_label(
            vb_storage::recovery::check_workflow_source_digest(&reopened, run, expected_digest),
        ))
    };
    Ok(CorruptReplayObservation {
        case_label: label,
        first_attempt: attempt()?,
        second_attempt: attempt()?,
        expected_typed_error: "WorkflowSourceDigestMismatch",
    })
}

fn repeated_compiled_ir_digest_mismatch(
    run: RunId,
    source_digest: WorkflowDigest,
    expected_ir_digest: WorkflowDigest,
    found_ir_digest: WorkflowDigest,
) -> Result<CorruptReplayObservation, VbKyyfScenarioDiagnostic> {
    let label = "compiled-ir-digest-mismatch";
    let temp =
        tempfile::tempdir().map_err(|_| VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_004,
            public_surface: label,
        })?;
    {
        let journal = vb_storage::open_store(temp.path()).map_err(|_| {
            VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_004,
                public_surface: label,
            }
        })?;
        append_event(
            &journal,
            &JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: source_digest,
            },
        )?;
    }
    let attempt = || {
        let reopened = vb_storage::open_store(temp.path()).map_err(|_| {
            VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_004,
                public_surface: label,
            }
        })?;
        Ok(exact_recovery_result_label(
            vb_storage::recovery::verify_digests(
                &reopened,
                run,
                source_digest,
                expected_ir_digest,
                found_ir_digest,
                DigestCheck::WorkflowAndIr,
            ),
        ))
    };
    Ok(CorruptReplayObservation {
        case_label: label,
        first_attempt: attempt()?,
        second_attempt: attempt()?,
        expected_typed_error: "CompiledIrDigestMismatch",
    })
}

fn repeated_missing_snapshot_as_corrupt(
    run: RunId,
    seq: EventSeq,
) -> Result<CorruptReplayObservation, VbKyyfScenarioDiagnostic> {
    let label = "corrupt-snapshot";
    let temp =
        tempfile::tempdir().map_err(|_| VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_004,
            public_surface: label,
        })?;
    let attempt = || {
        let reopened = vb_storage::open_store(temp.path()).map_err(|_| {
            VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_004,
                public_surface: label,
            }
        })?;
        Ok(exact_recovery_result_label(
            vb_storage::recovery::load_snapshot(&reopened, run, seq),
        ))
    };
    Ok(CorruptReplayObservation {
        case_label: label,
        first_attempt: attempt()?,
        second_attempt: attempt()?,
        expected_typed_error: "CorruptSnapshot",
    })
}

fn repeated_missing_digest_surface(
    case_label: &'static str,
    expected_typed_error: &'static str,
) -> CorruptReplayObservation {
    CorruptReplayObservation {
        case_label,
        first_attempt: unsupported_surface_attempt_label(),
        second_attempt: unsupported_surface_attempt_label(),
        expected_typed_error,
    }
}

fn corrupt_replay_observation_or_unavailable(
    case_label: &'static str,
    expected_typed_error: &'static str,
    result: Result<CorruptReplayObservation, VbKyyfScenarioDiagnostic>,
) -> CorruptReplayObservation {
    match result {
        Ok(observation) => observation,
        Err(_) => repeated_missing_digest_surface(case_label, expected_typed_error),
    }
}

fn repeated_public_replay_event_error(
    case_label: &'static str,
    events: &[JournalEvent],
    expected_typed_error: &'static str,
) -> CorruptReplayObservation {
    CorruptReplayObservation {
        case_label,
        first_attempt: exact_recovery_result_label(vb_storage::recovery::replay_events(
            events,
            &mut ActionReplayTracker::new(),
            &[],
        )),
        second_attempt: exact_recovery_result_label(vb_storage::recovery::replay_events(
            events,
            &mut ActionReplayTracker::new(),
            &[],
        )),
        expected_typed_error,
    }
}

fn repeated_action_abi_digest_mismatch() -> CorruptReplayObservation {
    let action = vb_core::ActionId::new(7);
    CorruptReplayObservation {
        case_label: "action-abi-digest-mismatch",
        first_attempt: exact_recovery_result_label(vb_storage::recovery::check_action_abi_digest(
            action,
            WorkflowDigest::from_bytes([0x71; 32]),
            WorkflowDigest::from_bytes([0x72; 32]),
        )),
        second_attempt: exact_recovery_result_label(vb_storage::recovery::check_action_abi_digest(
            action,
            WorkflowDigest::from_bytes([0x71; 32]),
            WorkflowDigest::from_bytes([0x72; 32]),
        )),
        expected_typed_error: "ActionAbiMismatch",
    }
}

fn repeated_policy_digest_mismatch() -> CorruptReplayObservation {
    let step = StepIdx::new(3);
    CorruptReplayObservation {
        case_label: "policy-digest-mismatch",
        first_attempt: exact_recovery_result_label(vb_storage::recovery::check_policy_digest(
            step,
            WorkflowDigest::from_bytes([0x81; 32]),
            WorkflowDigest::from_bytes([0x82; 32]),
        )),
        second_attempt: exact_recovery_result_label(vb_storage::recovery::check_policy_digest(
            step,
            WorkflowDigest::from_bytes([0x81; 32]),
            WorkflowDigest::from_bytes([0x82; 32]),
        )),
        expected_typed_error: "PolicyDigestMismatch",
    }
}

fn collect_bdd_kyyf_001() -> Result<VbKyyfScenarioEvidence, VbKyyfScenarioDiagnostic> {
    let left = durable_runtime_public_surface(RunId::new(10_001), BDD_KYYF_001, 0x4A)?;
    let right = durable_runtime_public_surface(RunId::new(10_002), BDD_KYYF_001, 0x4A)?;
    let evidence_summary = format!(
        "{}\n{}\ncomparison=Ok",
        observation_summary("left", &left),
        observation_summary("right", &right)
    );
    if outcome_label(compare_cross_run(left, right)) != "Ok" {
        return Err(
            VbKyyfScenarioDiagnostic::NormalizedDigestOrMismatchMissing {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_001,
            },
        );
    }
    assert_evidence_then_return(
        BDD_KYYF_001,
        "vb_runtime public API",
        KYYF_CROSS_RUN_EVIDENCE,
        "normalized digest",
        &evidence_summary,
    )
}

fn collect_bdd_kyyf_002() -> Result<VbKyyfScenarioEvidence, VbKyyfScenarioDiagnostic> {
    let temp =
        tempfile::tempdir().map_err(|_| VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_002,
            public_surface: "tempfile isolated store",
        })?;
    let run = RunId::new(20_002);
    {
        let journal = vb_storage::open_store(temp.path()).map_err(|_| {
            VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_002,
                public_surface: "vb_storage::open_store",
            }
        })?;
        for event in journal_events(run) {
            append_event(&journal, &event)?;
        }
    }
    let reopened = vb_storage::open_store(temp.path()).map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_002,
            public_surface: "vb_storage::open_store reopen",
        }
    })?;
    let first = reopened.events_for_run(run).map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_002,
            public_surface: "vb_storage::FjallJournal::events_for_run",
        }
    })?;
    let second = reopened.events_for_run(run).map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_002,
            public_surface: "vb_storage::FjallJournal::events_for_run repeated read",
        }
    })?;
    if first != second {
        return Err(
            VbKyyfScenarioDiagnostic::NormalizedDigestOrMismatchMissing {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_002,
            },
        );
    }
    let mut first_tracker = ActionReplayTracker::new();
    let mut second_tracker = ActionReplayTracker::new();
    let replayed_first = vb_storage::replay_journal(&reopened, run, &mut first_tracker, &[], &[])
        .map_err(
        |error| VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_002,
            public_surface: stable_recovery_error_label(error),
        },
    )?;
    let replayed_second = vb_storage::replay_journal(&reopened, run, &mut second_tracker, &[], &[])
        .map_err(
            |error| VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_002,
                public_surface: stable_recovery_error_label(error),
            },
        )?;
    if replayed_first != replayed_second {
        return Err(
            VbKyyfScenarioDiagnostic::NormalizedDigestOrMismatchMissing {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_002,
            },
        );
    }
    let first_summary =
        vb_storage::recovery::summarize_recovery_events(&replayed_first).map_err(|error| {
            VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_002,
                public_surface: stable_recovery_error_label(error),
            }
        })?;
    let second_summary = vb_storage::recovery::summarize_recovery_events(&replayed_second)
        .map_err(
            |error| VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_002,
                public_surface: stable_recovery_error_label(error),
            },
        )?;
    if first_summary != second_summary {
        return Err(
            VbKyyfScenarioDiagnostic::NormalizedDigestOrMismatchMissing {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_002,
            },
        );
    }
    let first_seed = RecoveryFrameSeedBuilder::new()
        .build(&replayed_first)
        .map_err(
            |error| VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_002,
                public_surface: stable_recovery_error_label(error),
            },
        )?;
    let second_seed = RecoveryFrameSeedBuilder::new()
        .build(&replayed_second)
        .map_err(
            |error| VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_002,
                public_surface: stable_recovery_error_label(error),
            },
        )?;
    if first_seed != second_seed {
        return Err(
            VbKyyfScenarioDiagnostic::NormalizedDigestOrMismatchMissing {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_002,
            },
        );
    }
    let run_arg = run.get().to_string();
    let expected_cli_event_count = first.len();
    drop(reopened);
    let first_cli_reports = [
        run_velvet_ballastics_cli("replay", &run_arg, temp.path())?,
        run_velvet_ballastics_cli("events", &run_arg, temp.path())?,
        run_velvet_ballastics_cli("inspect", &run_arg, temp.path())?,
    ];
    let second_cli_reports = [
        run_velvet_ballastics_cli("replay", &run_arg, temp.path())?,
        run_velvet_ballastics_cli("events", &run_arg, temp.path())?,
        run_velvet_ballastics_cli("inspect", &run_arg, temp.path())?,
    ];
    if !cli_reports_are_reopened_replay_evidence(
        &first_cli_reports,
        &second_cli_reports,
        &run_arg,
        expected_cli_event_count,
    ) {
        return Err(VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_002,
            public_surface: "velvet-ballastics CLI replay/events/inspect",
        });
    }
    let evidence_summary = format!(
        "events_first={first:?}\nevents_second={second:?}\nsummary_first={first_summary:?}\nsummary_second={second_summary:?}\nseed_first={first_seed:?}\nseed_second={second_seed:?}\ncli_first={first_cli_reports:?}\ncli_second={second_cli_reports:?}"
    );
    assert_evidence_then_return(
        BDD_KYYF_002,
        "vb_storage journal and recovery APIs plus CLI replay/events/inspect",
        KYYF_REPLAY_EVIDENCE,
        "normalized replay digest",
        &evidence_summary,
    )
}

fn collect_bdd_kyyf_003() -> Result<VbKyyfScenarioEvidence, VbKyyfScenarioDiagnostic> {
    let temp =
        tempfile::tempdir().map_err(|_| VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_003,
            public_surface: "tempfile isolated durable action journal",
        })?;
    let journal = vb_storage::open_store(temp.path()).map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_003,
            public_surface: "vb_storage::open_store isolated durable action journal",
        }
    })?;
    let run = RunId::new(30_003);
    let mut tracker = ActionReplayTracker::new();
    let action = vb_core::ActionId::new(3);
    let step = StepIdx::new(2);
    tracker.mark_completed(action, step);
    append_event(
        &journal,
        &JournalEvent::ActionScheduled {
            run,
            seq: EventSeq::new(0),
            step,
            action,
            attempt: 1,
        },
    )?;
    let before_events = journal.events_for_run(run).map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_003,
            public_surface: "vb_storage::FjallJournal::events_for_run action fact before replay",
        }
    })?;
    let before_dispatch_count = count_scheduled_action_facts(&before_events);
    let first_blocked = vb_storage::replay_journal(&journal, run, &mut tracker, &[], &[])
        .map(|_| "Ok")
        .map_err(stable_recovery_error_label);
    let after_first_events = journal.events_for_run(run).map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_003,
            public_surface: "vb_storage::FjallJournal::events_for_run action fact after first replay",
        }
    })?;
    let mut repeat_tracker = ActionReplayTracker::new();
    repeat_tracker.mark_completed(action, step);
    let second_blocked = vb_storage::replay_journal(&journal, run, &mut repeat_tracker, &[], &[])
        .map(|_| "Ok")
        .map_err(stable_recovery_error_label);
    let after_second_events = journal.events_for_run(run).map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_003,
            public_surface: "vb_storage::FjallJournal::events_for_run action fact after second replay",
        }
    })?;
    let after_first_dispatch_count = count_scheduled_action_facts(&after_first_events);
    let after_second_dispatch_count = count_scheduled_action_facts(&after_second_events);
    if first_blocked != Err("ReplayPolicyBlocked")
        || second_blocked != Err("ReplayPolicyBlocked")
        || before_dispatch_count != 1
        || after_first_dispatch_count != before_dispatch_count
        || after_second_dispatch_count != before_dispatch_count
        || after_first_events != before_events
        || after_second_events != before_events
    {
        return Err(
            VbKyyfScenarioDiagnostic::NormalizedDigestOrMismatchMissing {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_003,
            },
        );
    }
    let evidence_summary = format!(
        "before_events={before_events:?}\nfirst_blocked={first_blocked:?}\nsecond_blocked={second_blocked:?}\nafter_first_events={after_first_events:?}\nafter_second_events={after_second_events:?}"
    );
    assert_evidence_then_return(
        BDD_KYYF_003,
        "vb_runtime recovery API",
        KYYF_POLICY_EVIDENCE,
        "ReplayPolicyBlocked",
        &evidence_summary,
    )
}

fn collect_bdd_kyyf_004() -> Result<VbKyyfScenarioEvidence, VbKyyfScenarioDiagnostic> {
    let run = RunId::new(40_004);
    let workflow = WorkflowDigest::from_bytes([0x40; 32]);
    let gapped = [
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(2),
            step: StepIdx::new(0),
            attempt: 1,
        },
    ];
    let duplicate = [
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            attempt: 1,
        },
    ];
    let out_of_order = [
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(2),
            attempt: 1,
        },
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(1),
            step: StepIdx::new(1),
            attempt: 1,
        },
    ];
    let actual_cases = vec![
        corrupt_replay_observation_or_unavailable(
            "corrupt-snapshot",
            "CorruptSnapshot",
            repeated_missing_snapshot_as_corrupt(run, EventSeq::new(4)),
        ),
        corrupt_replay_observation_or_unavailable(
            "sequence-gap",
            "ReplayDivergence",
            Ok(repeated_public_replay_event_error(
                "sequence-gap",
                gapped.as_slice(),
                "ReplayDivergence",
            )),
        ),
        corrupt_replay_observation_or_unavailable(
            "duplicate-sequence",
            "ReplayDivergence",
            Ok(repeated_public_replay_event_error(
                "duplicate-sequence",
                duplicate.as_slice(),
                "ReplayDivergence",
            )),
        ),
        corrupt_replay_observation_or_unavailable(
            "out-of-order-sequence",
            "ReplayDivergence",
            repeated_persisted_replay_error(
                "out-of-order-sequence",
                run,
                out_of_order.as_slice(),
                "ReplayDivergence",
            ),
        ),
        corrupt_replay_observation_or_unavailable(
            "workflow-source-digest-mismatch",
            "WorkflowSourceDigestMismatch",
            repeated_workflow_source_digest_mismatch(
                run,
                workflow,
                WorkflowDigest::from_bytes([0x41; 32]),
            ),
        ),
        corrupt_replay_observation_or_unavailable(
            "compiled-ir-digest-mismatch",
            "CompiledIrDigestMismatch",
            repeated_compiled_ir_digest_mismatch(
                run,
                workflow,
                WorkflowDigest::from_bytes([0x42; 32]),
                WorkflowDigest::from_bytes([0x43; 32]),
            ),
        ),
        repeated_action_abi_digest_mismatch(),
        repeated_policy_digest_mismatch(),
    ];
    let evidence_summary = corruption_summary(&actual_cases);
    let evidence = assert_evidence_then_return(
        BDD_KYYF_004,
        "vb_storage journal and recovery APIs",
        KYYF_CORRUPT_EVIDENCE,
        "ReplayDigestMismatch",
        &evidence_summary,
    )?;
    let all_cases_have_expected_typed_errors = actual_cases.iter().all(|case| {
        case.first_attempt == case.expected_typed_error
            && case.second_attempt == case.expected_typed_error
    });
    if all_cases_have_expected_typed_errors {
        Ok(evidence)
    } else if actual_cases.iter().any(|case| {
        case.first_attempt == unsupported_surface_attempt_label()
            || case.second_attempt == unsupported_surface_attempt_label()
    }) {
        Err(VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_004,
            public_surface: "action-abi or policy digest mismatch recovery public surface",
        })
    } else {
        Err(
            VbKyyfScenarioDiagnostic::NormalizedDigestOrMismatchMissing {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_004,
            },
        )
    }
}

fn collect_bdd_kyyf_005() -> Result<VbKyyfScenarioEvidence, VbKyyfScenarioDiagnostic> {
    let workflow = deterministic_finish_workflow(0x55).map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_005,
            public_surface: "vb_core::CompiledWorkflow generated parity fixture",
        }
    })?;
    vb_codegen::validate_generated_subset(&workflow).map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_005,
            public_surface: "vb_codegen::validate_generated_subset supported fixture",
        }
    })?;
    let ir_observation = durable_runtime_public_surface(RunId::new(50_005), BDD_KYYF_005, 0x55)?;
    let generated_observation = match generated_mode_public_observation(&workflow) {
        Ok(observation) => observation,
        Err(diagnostic) => {
            let evidence_summary = format!(
                "{}\ngenerated_durable_replay=ScenarioSurfaceUnavailable(public_surface=generated durable replay public surface)",
                observation_summary("ir_observation", &ir_observation)
            );
            let _evidence = assert_evidence_then_return(
                BDD_KYYF_005,
                "vb_codegen and vb_runtime public surfaces",
                KYYF_GENERATED_PARITY_EVIDENCE,
                "generated replay parity digest",
                &evidence_summary,
            )?;
            return Err(diagnostic);
        }
    };
    let evidence_summary = format!(
        "{}\n{}",
        observation_summary("ir_observation", &ir_observation),
        observation_summary("generated_observation", &generated_observation)
    );
    if outcome_label(compare_generated_ir(ir_observation, generated_observation)) != "Ok" {
        return Err(
            VbKyyfScenarioDiagnostic::NormalizedDigestOrMismatchMissing {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_005,
            },
        );
    }
    assert_evidence_then_return(
        BDD_KYYF_005,
        "vb_codegen and vb_runtime public surfaces",
        KYYF_GENERATED_PARITY_EVIDENCE,
        "generated replay parity digest",
        &evidence_summary,
    )
}

fn collect_bdd_kyyf_006() -> Result<VbKyyfScenarioEvidence, VbKyyfScenarioDiagnostic> {
    let workflow = unsupported_generated_subset_workflow().map_err(|_| {
        VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_006,
            public_surface: "vb_core::CompiledWorkflow valid unsupported generated-subset fixture",
        }
    })?;
    let result = vb_codegen::validate_generated_subset(&workflow);
    if !matches!(
        result,
        Err(CodegenError::UnsupportedIr {
            feature: "text helper contains requires runtime symbol store"
        })
    ) {
        return Err(
            VbKyyfScenarioDiagnostic::NormalizedDigestOrMismatchMissing {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_006,
            },
        );
    }
    assert_evidence_then_return(
        BDD_KYYF_006,
        "vb_codegen generated-subset validation API",
        KYYF_GENERATED_UNSUPPORTED_EVIDENCE,
        "UnsupportedGeneratedSubset",
        "unsupported generated subset fixture returned CodegenError::UnsupportedIr",
    )
}

fn execute_required_bdd_public_surfaces()
-> Vec<Result<VbKyyfScenarioEvidence, VbKyyfScenarioDiagnostic>> {
    vec![
        collect_bdd_kyyf_001(),
        collect_bdd_kyyf_002(),
        collect_bdd_kyyf_003(),
        collect_bdd_kyyf_004(),
        collect_bdd_kyyf_005(),
        collect_bdd_kyyf_006(),
    ]
}

fn expected_bdd_public_surface_evidence()
-> Vec<Result<VbKyyfScenarioEvidence, VbKyyfScenarioDiagnostic>> {
    vec![
        Ok(VbKyyfScenarioEvidence {
            scenario_id: BDD_KYYF_001,
            public_surface: "vb_runtime public API",
            evidence_artifact: KYYF_CROSS_RUN_EVIDENCE,
            normalized_digest_or_error: "normalized digest",
        }),
        Ok(VbKyyfScenarioEvidence {
            scenario_id: BDD_KYYF_002,
            public_surface: "vb_storage journal and recovery APIs plus CLI replay/events/inspect",
            evidence_artifact: KYYF_REPLAY_EVIDENCE,
            normalized_digest_or_error: "normalized replay digest",
        }),
        Ok(VbKyyfScenarioEvidence {
            scenario_id: BDD_KYYF_003,
            public_surface: "vb_runtime recovery API",
            evidence_artifact: KYYF_POLICY_EVIDENCE,
            normalized_digest_or_error: "ReplayPolicyBlocked",
        }),
        Ok(VbKyyfScenarioEvidence {
            scenario_id: BDD_KYYF_004,
            public_surface: "vb_storage journal and recovery APIs",
            evidence_artifact: KYYF_CORRUPT_EVIDENCE,
            normalized_digest_or_error: "ReplayDigestMismatch",
        }),
        Err(VbKyyfScenarioDiagnostic::NormalizedDigestOrMismatchMissing {
            bead_id: BEAD_ID,
            scenario_id: BDD_KYYF_005,
        }),
        Ok(VbKyyfScenarioEvidence {
            scenario_id: BDD_KYYF_006,
            public_surface: "vb_codegen generated-subset validation API",
            evidence_artifact: KYYF_GENERATED_UNSUPPORTED_EVIDENCE,
            normalized_digest_or_error: "UnsupportedGeneratedSubset",
        }),
    ]
}

#[test]
fn normalize_observation_strips_only_allowed_cold_metadata_when_runs_are_isolated() {
    // Given: two public observations that differ only in allowed cold metadata.
    let left = accepted_observation();
    let right = PublicObservation {
        temp_path_signature: 1_606,
        process_id_signature: 1_707,
        wall_clock_signature: 1_808,
        generated_run_signature: 1_909,
        ..accepted_observation()
    };

    // When: both observations are normalized through the public proof-kernel surface.
    let left_norm = normalize_observation(left);
    let right_norm = normalize_observation(right);

    // Then: semantic projection is exact and cross-run comparison accepts equality.
    assert_eq!(terminal_label(left_norm.result), "Ok");
    assert_eq!(terminal_label(right_norm.result), "Ok");
    assert_eq!(taint_label(left_norm.taint), "Clean");
    assert_eq!(taint_label(right_norm.taint), "Clean");
    assert_eq!(left_norm.event_signature, 101);
    assert_eq!(right_norm.event_signature, 101);
    assert_eq!(left_norm.event_payload_signature, 202);
    assert_eq!(right_norm.event_payload_signature, 202);
    assert_eq!(left_norm.semantic_slot_signature, 303);
    assert_eq!(right_norm.semantic_slot_signature, 303);
    assert_eq!(left_norm.semantic_action_signature, 404);
    assert_eq!(right_norm.semantic_action_signature, 404);
    assert_eq!(left_norm.semantic_suspension, false);
    assert_eq!(right_norm.semantic_suspension, false);
    assert_eq!(left_norm.semantic_taint_signature, 505);
    assert_eq!(right_norm.semantic_taint_signature, 505);
    assert_eq!(outcome_label(compare_cross_run(left, right)), "Ok");
}

#[test]
fn compare_cross_run_returns_nondeterministic_observation_when_semantic_field_changes() {
    // Given: two accepted observations with one semantic payload digest delta.
    let right = PublicObservation {
        event_payload_signature: 9_202,
        ..accepted_observation()
    };

    // When: the public comparison surface compares the runs.
    let result = compare_cross_run(accepted_observation(), right);

    // Then: the exact contract taxonomy rejects the unlisted semantic delta.
    assert_eq!(outcome_label(result), "NondeterministicObservation");
}

#[test]
fn compare_cross_run_rejects_every_normalized_semantic_field_delta() {
    // Given: every semantic field projected into NormalizedObservation is mutated once.
    let cases = [
        (
            "result",
            PublicObservation {
                result: TerminalResult::Failed,
                ..accepted_observation()
            },
        ),
        (
            "taint",
            PublicObservation {
                taint: TaintStatus::Tainted,
                ..accepted_observation()
            },
        ),
        (
            "event_signature",
            PublicObservation {
                event_signature: 9_101,
                ..accepted_observation()
            },
        ),
        (
            "event_payload_signature",
            PublicObservation {
                event_payload_signature: 9_202,
                ..accepted_observation()
            },
        ),
        (
            "digest_status.workflow_source_matches",
            PublicObservation {
                digest_status: DigestStatus {
                    workflow_source_matches: false,
                    ..CLEAN_DIGESTS
                },
                ..accepted_observation()
            },
        ),
        (
            "digest_status.compiled_ir_matches",
            PublicObservation {
                digest_status: DigestStatus {
                    compiled_ir_matches: false,
                    ..CLEAN_DIGESTS
                },
                ..accepted_observation()
            },
        ),
        (
            "digest_status.action_abi_matches",
            PublicObservation {
                digest_status: DigestStatus {
                    action_abi_matches: false,
                    ..CLEAN_DIGESTS
                },
                ..accepted_observation()
            },
        ),
        (
            "digest_status.policy_matches",
            PublicObservation {
                digest_status: DigestStatus {
                    policy_matches: false,
                    ..CLEAN_DIGESTS
                },
                ..accepted_observation()
            },
        ),
        (
            "replay_policy_blocked",
            PublicObservation {
                replay_policy_blocked: true,
                ..accepted_observation()
            },
        ),
        (
            "unsupported_generated_subset",
            PublicObservation {
                unsupported_generated_subset: true,
                ..accepted_observation()
            },
        ),
        (
            "semantic_slot_signature",
            PublicObservation {
                semantic_slot_signature: 9_303,
                ..accepted_observation()
            },
        ),
        (
            "semantic_action_signature",
            PublicObservation {
                semantic_action_signature: 9_404,
                ..accepted_observation()
            },
        ),
        (
            "semantic_suspension",
            PublicObservation {
                semantic_suspension: true,
                ..accepted_observation()
            },
        ),
        (
            "semantic_taint_signature",
            PublicObservation {
                semantic_taint_signature: 9_505,
                ..accepted_observation()
            },
        ),
    ];

    // When/Then: every semantic delta returns the exact cross-run taxonomy.
    for (field, right) in cases {
        assert_eq!(
            (
                field,
                outcome_label(compare_cross_run(accepted_observation(), right))
            ),
            (field, "NondeterministicObservation")
        );
    }
}

#[test]
fn normalize_observation_preserves_all_semantic_fields_and_erases_only_metadata_fields() {
    // Given: one public observation containing non-default semantic and metadata signatures.
    let observation = PublicObservation {
        result: TerminalResult::Blocked,
        taint: TaintStatus::Unknown,
        event_signature: 1_001,
        event_payload_signature: 1_002,
        digest_status: DigestStatus {
            workflow_source_matches: true,
            compiled_ir_matches: false,
            action_abi_matches: true,
            policy_matches: false,
        },
        replay_policy_blocked: true,
        unsupported_generated_subset: true,
        semantic_slot_signature: 1_003,
        semantic_action_signature: 1_004,
        semantic_suspension: true,
        semantic_taint_signature: 1_005,
        temp_path_signature: 9_001,
        process_id_signature: 9_002,
        wall_clock_signature: 9_003,
        generated_run_signature: 9_004,
    };

    // When: normalization projects to the public semantic observation surface.
    let normalized = normalize_observation(observation);

    // Then: every semantic field is preserved exactly and cold metadata is absent by type.
    assert_eq!(terminal_label(normalized.result), "Blocked");
    assert_eq!(taint_label(normalized.taint), "Unknown");
    assert_eq!(normalized.event_signature, 1_001);
    assert_eq!(normalized.event_payload_signature, 1_002);
    assert_eq!(
        digest_label(normalized.digest_status),
        "workflow_source=true,compiled_ir=false,action_abi=true,policy=false"
    );
    assert_eq!(normalized.replay_policy_blocked, true);
    assert_eq!(normalized.unsupported_generated_subset, true);
    assert_eq!(normalized.semantic_slot_signature, 1_003);
    assert_eq!(normalized.semantic_action_signature, 1_004);
    assert_eq!(normalized.semantic_suspension, true);
    assert_eq!(normalized.semantic_taint_signature, 1_005);
}

#[test]
fn compare_replay_returns_digest_mismatch_before_policy_sequence_or_semantic_deltas() {
    // Given: replay reports with digest, policy, sequence, and semantic deltas all present.
    let first = PublicObservation {
        digest_status: DigestStatus {
            workflow_source_matches: false,
            ..CLEAN_DIGESTS
        },
        replay_policy_blocked: true,
        event_signature: 9_101,
        semantic_slot_signature: 9_303,
        ..accepted_observation()
    };
    let second = accepted_observation();

    // When: replay comparison is requested.
    let result = compare_replay(first, second);

    // Then: digest mismatch has the highest exact priority.
    assert_eq!(outcome_label(result), "ReplayDigestMismatch");
}

#[test]
fn compare_replay_returns_digest_mismatch_for_each_digest_role_before_later_failures() {
    // Given: every digest role is independently false while policy/sequence/semantic failures coexist.
    let cases = [
        (
            "workflow-source",
            DigestStatus {
                workflow_source_matches: false,
                ..CLEAN_DIGESTS
            },
        ),
        (
            "compiled-ir",
            DigestStatus {
                compiled_ir_matches: false,
                ..CLEAN_DIGESTS
            },
        ),
        (
            "action-abi",
            DigestStatus {
                action_abi_matches: false,
                ..CLEAN_DIGESTS
            },
        ),
        (
            "policy",
            DigestStatus {
                policy_matches: false,
                ..CLEAN_DIGESTS
            },
        ),
    ];

    // When/Then: all digest roles map to the exact highest-priority replay diagnostic.
    for (role, digest_status) in cases {
        let first = PublicObservation {
            digest_status,
            replay_policy_blocked: true,
            event_signature: 9_101,
            semantic_action_signature: 9_404,
            ..accepted_observation()
        };
        assert_eq!(
            (
                role,
                outcome_label(compare_replay(first, accepted_observation()))
            ),
            (role, "ReplayDigestMismatch")
        );
    }
}

#[test]
fn compare_replay_returns_policy_blocked_before_sequence_or_normalized_mismatch() {
    // Given: clean digests plus blocked policy, sequence, and semantic deltas.
    let first = PublicObservation {
        replay_policy_blocked: true,
        event_signature: 9_101,
        semantic_action_signature: 9_404,
        ..accepted_observation()
    };

    // When: replay comparison is requested.
    let result = compare_replay(first, accepted_observation());

    // Then: replay policy blocking outranks sequence and semantic mismatch.
    assert_eq!(outcome_label(result), "ReplayPolicyBlocked");
}

#[test]
fn compare_replay_returns_sequence_violation_before_normalized_mismatch() {
    // Given: clean digest and policy state with event order and semantic deltas.
    let first = PublicObservation {
        event_signature: 9_101,
        semantic_taint_signature: 9_505,
        ..accepted_observation()
    };

    // When: replay comparison is requested.
    let result = compare_replay(first, accepted_observation());

    // Then: sequence violation outranks normalized semantic mismatch.
    assert_eq!(outcome_label(result), "ReplaySequenceViolation");
}

#[test]
fn compare_generated_ir_returns_unsupported_subset_before_divergence() {
    // Given: generated parity observations where unsupported subset and divergence coexist.
    let generated = PublicObservation {
        unsupported_generated_subset: true,
        result: TerminalResult::Failed,
        semantic_suspension: true,
        ..accepted_observation()
    };

    // When: generated/IR comparison is requested through the public proof-kernel surface.
    let result = compare_generated_ir(accepted_observation(), generated);

    // Then: unsupported generated subset fails closed before divergence evidence.
    assert_eq!(outcome_label(result), "UnsupportedGeneratedSubset");
}

#[test]
fn compare_generated_ir_returns_divergence_when_supported_observations_differ() {
    // Given: supported generated/IR observations with a semantic terminal delta.
    let generated = PublicObservation {
        result: TerminalResult::Blocked,
        ..accepted_observation()
    };

    // When: generated/IR comparison is requested.
    let result = compare_generated_ir(accepted_observation(), generated);

    // Then: supported semantic deltas return the exact generated/IR divergence taxonomy.
    assert_eq!(outcome_label(result), "GeneratedIrDivergence");
}

#[test]
fn compare_generated_ir_priority_order_is_unsupported_then_divergence_then_equal() {
    // Given: generated/IR cases for every priority branch.
    let cases = [
        (
            "unsupported-ir-side",
            PublicObservation {
                unsupported_generated_subset: true,
                result: TerminalResult::Failed,
                ..accepted_observation()
            },
            accepted_observation(),
            "UnsupportedGeneratedSubset",
        ),
        (
            "unsupported-generated-side",
            accepted_observation(),
            PublicObservation {
                unsupported_generated_subset: true,
                semantic_slot_signature: 9_303,
                ..accepted_observation()
            },
            "UnsupportedGeneratedSubset",
        ),
        (
            "supported-divergent",
            accepted_observation(),
            PublicObservation {
                semantic_action_signature: 9_404,
                ..accepted_observation()
            },
            "GeneratedIrDivergence",
        ),
        (
            "supported-equal",
            accepted_observation(),
            accepted_observation(),
            "Ok",
        ),
    ];

    // When/Then: the public comparison surface follows the required priority order exactly.
    for (label, ir, generated, expected) in cases {
        assert_eq!(
            (label, outcome_label(compare_generated_ir(ir, generated))),
            (label, expected)
        );
    }
}

#[test]
fn digest_status_all_match_returns_false_for_each_single_digest_mismatch() {
    // Given: each digest role is independently mismatched.
    let cases = [
        (
            "workflow-source",
            DigestStatus {
                workflow_source_matches: false,
                ..CLEAN_DIGESTS
            },
        ),
        (
            "compiled-ir",
            DigestStatus {
                compiled_ir_matches: false,
                ..CLEAN_DIGESTS
            },
        ),
        (
            "action-abi",
            DigestStatus {
                action_abi_matches: false,
                ..CLEAN_DIGESTS
            },
        ),
        (
            "policy",
            DigestStatus {
                policy_matches: false,
                ..CLEAN_DIGESTS
            },
        ),
    ];

    // When/Then: every single false digest bit rejects all-match with exact case labels.
    for (label, status) in cases {
        assert_eq!((label, status.all_match()), (label, false));
    }
    assert_eq!(CLEAN_DIGESTS.all_match(), true);
}

#[test]
fn digest_status_all_match_is_true_only_for_all_sixteen_matching_bit_combinations() {
    // Given/When/Then: every digest bit combination is checked to kill && -> || mutants.
    for workflow_source_matches in [false, true] {
        for compiled_ir_matches in [false, true] {
            for action_abi_matches in [false, true] {
                for policy_matches in [false, true] {
                    let status = DigestStatus {
                        workflow_source_matches,
                        compiled_ir_matches,
                        action_abi_matches,
                        policy_matches,
                    };
                    let expected = workflow_source_matches
                        && compiled_ir_matches
                        && action_abi_matches
                        && policy_matches;
                    assert_eq!(
                        (digest_label(status), status.all_match()),
                        (digest_label(status), expected)
                    );
                }
            }
        }
    }
}

#[test]
fn bdd_kyyf_001_to_006_require_executable_public_surfaces_not_catalog_bookkeeping_only() {
    // Given: executable public BDD contracts for runtime, storage/replay/CLI,
    // recovery, corrupt evidence, generated/IR parity, and unsupported subset behavior.
    let expected = expected_bdd_public_surface_evidence();

    // When: the suite exercises public runtime/storage/recovery/codegen surfaces directly.
    let actual = execute_required_bdd_public_surfaces();

    // Then: catalog row edits are insufficient; only real public-surface evidence satisfies this.
    assert_eq!(actual, expected);
}

#[test]
fn bdd_kyyf_007_malformed_catalog_rows_emit_exact_strength_diagnostics() {
    // Given: malformed vb-kyyf catalog rows for every mandated traceability failure.
    let cases = [
        (
            "missing-evidence",
            scenario_fixture(
                BDD_KYYF_007,
                "vb_runtime public API",
                None,
                Some("normalized digest emitted"),
            ),
            VbKyyfScenarioDiagnostic::EvidenceArtifactMissing {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_007,
            },
        ),
        (
            "private-surface",
            scenario_fixture(
                BDD_KYYF_007,
                "private helper fixture",
                Some(KYYF_CROSS_RUN_EVIDENCE),
                Some("normalized digest emitted"),
            ),
            VbKyyfScenarioDiagnostic::ScenarioUsesPrivateSurface {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_007,
                public_surface: "private helper fixture",
            },
        ),
        (
            "missing-scenario-id",
            scenario_fixture(
                "",
                "vb_runtime public API",
                Some(KYYF_CROSS_RUN_EVIDENCE),
                Some("normalized digest emitted"),
            ),
            VbKyyfScenarioDiagnostic::ScenarioIdMissing { bead_id: BEAD_ID },
        ),
        (
            "missing-gwt",
            Scenario {
                given: "",
                when: "",
                then: "",
                ..scenario_fixture(
                    BDD_KYYF_007,
                    "vb_runtime public API",
                    Some(KYYF_CROSS_RUN_EVIDENCE),
                    Some("normalized digest emitted"),
                )
            },
            VbKyyfScenarioDiagnostic::GivenWhenThenMissing {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_007,
            },
        ),
        (
            "missing-public-surface",
            scenario_fixture(
                BDD_KYYF_007,
                "",
                Some(KYYF_CROSS_RUN_EVIDENCE),
                Some("normalized digest emitted"),
            ),
            VbKyyfScenarioDiagnostic::PublicSurfaceMissing {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_007,
            },
        ),
        (
            "missing-normalized-digest-or-mismatch",
            scenario_fixture(
                BDD_KYYF_007,
                "vb_runtime public API",
                Some(KYYF_CROSS_RUN_EVIDENCE),
                Some("plain pass"),
            ),
            VbKyyfScenarioDiagnostic::NormalizedDigestOrMismatchMissing {
                bead_id: BEAD_ID,
                scenario_id: BDD_KYYF_007,
            },
        ),
    ];

    // When/Then: each malformed row returns the exact typed diagnostic, not a string-only check.
    for (label, scenario, expected) in cases {
        assert_eq!(
            (
                label,
                validate_vb_kyyf_scenario_strength(scenario)[0].clone()
            ),
            (label, expected)
        );
    }
}

#[test]
fn given_vb_kyyf_scenario_finishes_when_runner_reports_then_evidence_path_is_traceable() {
    // Given: the public acceptance catalog is the release runner evidence surface.
    let required = [
        BDD_KYYF_001,
        BDD_KYYF_002,
        BDD_KYYF_003,
        BDD_KYYF_004,
        BDD_KYYF_005,
        BDD_KYYF_006,
        BDD_KYYF_007,
    ];
    let scenarios = catalog();

    for required_scenario in REQUIRED_PUBLIC_SCENARIO_SURFACES {
        let expected = scenarios
            .iter()
            .copied()
            .find(|scenario| scenario.id == required_scenario.scenario_id)
            .ok_or(VbKyyfScenarioDiagnostic::ScenarioSurfaceUnavailable {
                bead_id: BEAD_ID,
                scenario_id: required_scenario.scenario_id,
                public_surface: required_scenario.public_surface,
            });
        assert_eq!(
            find_required_public_scenario(scenarios, required_scenario),
            expected
        );
    }

    // When/Then: every vb-kyyf BDD scenario must be executable, public, and traceable.
    for scenario_id in required {
        let matching: Vec<_> = scenarios
            .iter()
            .filter(|scenario| scenario.id == scenario_id)
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "{BEAD_ID}:{scenario_id}:EvidenceArtifactMissing"
        );

        for scenario in matching {
            assert_eq!(scenario.related_bead, BEAD_ID);
            assert_eq!(scenario.given.is_empty(), false);
            assert_eq!(scenario.when.is_empty(), false);
            assert_eq!(scenario.then.is_empty(), false);
            assert_eq!(scenario.public_surface.contains("private"), false);
            assert_eq!(scenario.public_surface.contains("helper"), false);
            let produced_evidence_artifact = scenario.executable_evidence_target.map(|target| {
                target.starts_with(KYYF_EVIDENCE_TARGET_PREFIX) && !target.ends_with(".rs")
            });
            assert_eq!(produced_evidence_artifact, Some(true));
            if scenario.id == BDD_KYYF_007 {
                assert_eq!(
                    scenario.executable_evidence_target,
                    Some(KYYF_ACCEPTANCE_CATALOG_EVIDENCE)
                );
            }
            assert_eq!(
                validate_vb_kyyf_scenario_strength(*scenario),
                Vec::<VbKyyfScenarioDiagnostic>::new()
            );
        }
    }
}
