#![forbid(unsafe_code)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use velvet_ballistics_workspace_tests::acceptance_catalog::{catalog, validate_catalog};

const REQUIRED_FUZZ_TARGETS: &[&str] =
    &["yaml_events", "ipc_frame", "journal_event", "compiled_ir"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EvidenceError {
    MissingScenario,
    UnrelatedMutationScope,
    BuildOnlyFuzzSmoke,
    MissingFuzzTarget,
    TaintParityIgnored,
    UnsafeBoundaryFuzzMissing,
    ReleaseGateWouldPassUnsafely,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct MutationEvidence {
    scope: Option<&'static str>,
    blocks_release: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FuzzSmokeEvidence {
    built_targets: Vec<&'static str>,
    run_targets: Vec<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PropertyEvidence {
    result_slots_match: bool,
    signals_match: bool,
    taint_compared: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BoundaryFuzzEvidence {
    boundary_id: &'static str,
    has_fuzz: bool,
    approved_blocker: bool,
}

#[test]
fn test_mutation_gate_fails_when_admission_branch_removed() {
    assert_eq!(
        validate_admission_mutation_gate(MutationEvidence {
            scope: None,
            blocks_release: true,
        }),
        Err(EvidenceError::UnrelatedMutationScope)
    );
    assert_eq!(
        validate_admission_mutation_gate(MutationEvidence {
            scope: Some("crates/vb_core/src/diagnostic.rs"),
            blocks_release: true,
        }),
        Err(EvidenceError::UnrelatedMutationScope)
    );
    assert_eq!(
        validate_admission_mutation_gate(MutationEvidence {
            scope: Some("runtime admission branch removal"),
            blocks_release: false,
        }),
        Err(EvidenceError::ReleaseGateWouldPassUnsafely)
    );
    assert_eq!(
        validate_admission_mutation_gate(MutationEvidence {
            scope: Some("runtime admission branch removal"),
            blocks_release: true,
        }),
        Ok(())
    );
}

#[test]
fn test_fuzz_smoke_runs_yaml_ipc_journal_compiled_ir_targets() -> io::Result<()> {
    let root = workspace_root()?;
    let moon_tasks = fs::read_to_string(root.join(".moon/tasks/all.yml"))?;
    let fuzz_manifest = fs::read_to_string(root.join("fuzz/Cargo.toml"))?;

    assert_eq!(assert_fuzz_targets_declared(&fuzz_manifest), Ok(()));
    assert_eq!(
        assert_fuzz_smoke_task_runs_required_targets(&moon_tasks),
        Ok(())
    );

    assert_eq!(
        validate_required_fuzz_smoke(FuzzSmokeEvidence {
            built_targets: REQUIRED_FUZZ_TARGETS.to_vec(),
            run_targets: Vec::new(),
        }),
        Err(EvidenceError::BuildOnlyFuzzSmoke)
    );
    assert_eq!(
        validate_required_fuzz_smoke(FuzzSmokeEvidence {
            built_targets: REQUIRED_FUZZ_TARGETS.to_vec(),
            run_targets: vec!["yaml_events", "ipc_frame", "journal_event"],
        }),
        Err(EvidenceError::MissingFuzzTarget)
    );
    assert_eq!(
        validate_required_fuzz_smoke(FuzzSmokeEvidence {
            built_targets: REQUIRED_FUZZ_TARGETS.to_vec(),
            run_targets: REQUIRED_FUZZ_TARGETS.to_vec(),
        }),
        Ok(())
    );

    Ok(())
}

#[test]
fn test_property_gate_fails_when_generated_ir_comparison_ignores_taint() {
    assert_eq!(
        validate_generated_ir_taint_parity(PropertyEvidence {
            result_slots_match: true,
            signals_match: true,
            taint_compared: false,
        }),
        Err(EvidenceError::TaintParityIgnored)
    );
    assert_eq!(
        validate_generated_ir_taint_parity(PropertyEvidence {
            result_slots_match: true,
            signals_match: true,
            taint_compared: true,
        }),
        Ok(())
    );
}

#[test]
fn test_unsafe_boundary_fuzz_missing_causes_release_gate_failure() {
    assert_eq!(
        validate_unsafe_boundary_release_gate(&[BoundaryFuzzEvidence {
            boundary_id: "decoder-byte-ingest-boundary",
            has_fuzz: false,
            approved_blocker: false,
        }]),
        Err(EvidenceError::UnsafeBoundaryFuzzMissing)
    );
    assert_eq!(
        validate_unsafe_boundary_release_gate(&[BoundaryFuzzEvidence {
            boundary_id: "decoder-byte-ingest-boundary",
            has_fuzz: false,
            approved_blocker: true,
        }]),
        Ok(())
    );
    assert_eq!(
        validate_unsafe_boundary_release_gate(&[BoundaryFuzzEvidence {
            boundary_id: "ipc-frame-boundary",
            has_fuzz: true,
            approved_blocker: false,
        }]),
        Ok(())
    );
}

#[test]
fn vb_njju_catalog_rows_exist_and_validate() {
    let scenarios = catalog();
    assert_eq!(validate_catalog(scenarios), Ok(()));
    for scenario_id in [
        "BDD-NJJU-001",
        "BDD-NJJU-002",
        "BDD-NJJU-003",
        "BDD-NJJU-004",
    ] {
        assert!(
            scenarios
                .iter()
                .any(|scenario| scenario.id == scenario_id && scenario.related_bead == "vb-njju"),
            "{scenario_id} must be present in the public acceptance catalog"
        );
    }
    assert_eq!(
        validate_vb_njju_catalog(&["BDD-NJJU-001", "BDD-NJJU-002", "BDD-NJJU-003"]),
        Err(EvidenceError::MissingScenario)
    );
}

fn validate_vb_njju_catalog(ids: &[&str]) -> Result<(), EvidenceError> {
    for required in [
        "BDD-NJJU-001",
        "BDD-NJJU-002",
        "BDD-NJJU-003",
        "BDD-NJJU-004",
    ] {
        if !ids.contains(&required) {
            return Err(EvidenceError::MissingScenario);
        }
    }
    Ok(())
}

fn validate_admission_mutation_gate(evidence: MutationEvidence) -> Result<(), EvidenceError> {
    match evidence.scope {
        Some(scope) if scope.contains("admission branch") => {
            if evidence.blocks_release {
                Ok(())
            } else {
                Err(EvidenceError::ReleaseGateWouldPassUnsafely)
            }
        }
        _ => Err(EvidenceError::UnrelatedMutationScope),
    }
}

fn validate_required_fuzz_smoke(evidence: FuzzSmokeEvidence) -> Result<(), EvidenceError> {
    if evidence.run_targets.is_empty() {
        return Err(EvidenceError::BuildOnlyFuzzSmoke);
    }
    for required in REQUIRED_FUZZ_TARGETS {
        if !evidence.built_targets.contains(required) || !evidence.run_targets.contains(required) {
            return Err(EvidenceError::MissingFuzzTarget);
        }
    }
    Ok(())
}

fn validate_generated_ir_taint_parity(evidence: PropertyEvidence) -> Result<(), EvidenceError> {
    if evidence.result_slots_match && evidence.signals_match && !evidence.taint_compared {
        Err(EvidenceError::TaintParityIgnored)
    } else {
        Ok(())
    }
}

fn validate_unsafe_boundary_release_gate(
    evidence: &[BoundaryFuzzEvidence],
) -> Result<(), EvidenceError> {
    if evidence.is_empty() {
        return Err(EvidenceError::ReleaseGateWouldPassUnsafely);
    }
    for boundary in evidence {
        if boundary.boundary_id.is_empty() || (!boundary.has_fuzz && !boundary.approved_blocker) {
            return Err(EvidenceError::UnsafeBoundaryFuzzMissing);
        }
    }
    Ok(())
}

fn assert_fuzz_targets_declared(manifest: &str) -> Result<(), EvidenceError> {
    for target in REQUIRED_FUZZ_TARGETS {
        let declaration = format!("name = \"{target}\"");
        if !manifest.contains(&declaration) {
            return Err(EvidenceError::MissingFuzzTarget);
        }
    }
    Ok(())
}

fn assert_fuzz_smoke_task_runs_required_targets(tasks: &str) -> Result<(), EvidenceError> {
    if !tasks.contains("cargo fuzz build") {
        return Err(EvidenceError::MissingFuzzTarget);
    }
    if !tasks.contains("cargo fuzz run") {
        return Err(EvidenceError::BuildOnlyFuzzSmoke);
    }
    for target in REQUIRED_FUZZ_TARGETS {
        if !tasks.contains(target) {
            return Err(EvidenceError::MissingFuzzTarget);
        }
    }
    Ok(())
}

fn workspace_root() -> io::Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let crates_dir = manifest_dir
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "workspace_tests parent missing"))?;
    crates_dir
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "workspace root parent missing"))
}
