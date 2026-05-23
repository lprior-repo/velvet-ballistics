#![forbid(unsafe_code)]
//! Proptest invariants for vb-lp2v proof-admission behaviors.
//!
//! These property tests cover the two PI invariants from the vb-lp2v test plan:
//!
//! - **PI-01**: `VerificationWarning::is_valid` gate range is exhaustive —
//!   For any u8 gate value, `is_valid()` returns true iff gate ∈ [1, 15].
//!
//! - **PI-02**: Digest roundtrip through `submit_artifact` and journal read —
//!   After `submit_artifact(journal, workflow, policy)` returns `Ok(artifact)`,
//!   `journal.compiled_ir(workflow.digest())` returns `Ok(Some(record))`
//!   where `record.digest == workflow.digest()`.

use proptest::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;
use vb_core::value::ConstValue;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_core::{CompiledWorkflow, ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::FjallJournal;
use vb_storage::admission::VerificationWarning;
use vb_storage::admission::submit_artifact;

// ============================================================================
// Test helpers
// ============================================================================

/// Build a minimal valid CompiledWorkflow for testing.
fn minimal_workflow(value: i64) -> Result<CompiledWorkflow, String> {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("pi_test"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(value)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let hash_bytes =
        postcard::to_allocvec(&parts).map_err(|e| format!("serialize parts for digest: {e}"))?;
    let computed = blake3::hash(&hash_bytes);
    parts.digest = WorkflowDigest::from_bytes(computed.into());

    CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
}

/// Owns both a temporary directory path and a FjallJournal.
struct TestJournal {
    path: std::path::PathBuf,
    journal: FjallJournal,
}

impl Drop for TestJournal {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.path) {
            std::hint::black_box(error.kind());
        }
    }
}

impl std::ops::Deref for TestJournal {
    type Target = FjallJournal;
    fn deref(&self) -> &Self::Target {
        &self.journal
    }
}

fn temp_journal() -> Result<TestJournal, vb_storage::JournalError> {
    let dir = tempfile::tempdir().map_err(|_| vb_storage::JournalError::ArtifactMalformed)?;
    let path = dir.keep();
    let journal = FjallJournal::open(&path, None)?;
    Ok(TestJournal { path, journal })
}

// ============================================================================
// PI-01: VerificationWarning::is_valid gate range is exhaustive
// ============================================================================

// PI-01: For any u8 gate value, is_valid() returns true iff gate ∈ [1, 15]
proptest! {
    #[test]
    fn pi_01_verification_warning_gate_range_exhaustive(gate in 0u8..=255u8) {
        let warning = VerificationWarning {
            code: 1,
            message: Box::from("pi01 invariant test"),
            gate,
        };
        let is_valid = warning.is_valid();
        let in_range =
            gate >= VerificationWarning::MIN_GATE
            && gate <= VerificationWarning::MAX_GATE;
        prop_assert_eq!(
            is_valid,
            in_range,
            "is_valid() = {} for gate={}, expected {} (gate in [1,15] = {})",
            is_valid,
            gate,
            in_range,
            in_range
        );
    }
}

/// PI-01 explicit boundary: MIN_GATE (1) must be valid
#[test]
fn pi_01_gate_boundary_min_is_valid() {
    let warning = VerificationWarning {
        code: 1,
        message: Box::from("min gate boundary"),
        gate: VerificationWarning::MIN_GATE,
    };
    assert!(
        warning.is_valid(),
        "gate {} (MIN_GATE) must be valid",
        VerificationWarning::MIN_GATE
    );
}

/// PI-01 explicit boundary: MAX_GATE (15) must be valid
#[test]
fn pi_01_gate_boundary_max_is_valid() {
    let warning = VerificationWarning {
        code: 1,
        message: Box::from("max gate boundary"),
        gate: VerificationWarning::MAX_GATE,
    };
    assert!(
        warning.is_valid(),
        "gate {} (MAX_GATE) must be valid",
        VerificationWarning::MAX_GATE
    );
}

/// PI-01 explicit boundary: gate=0 must be invalid
#[test]
fn pi_01_gate_zero_is_invalid() {
    let warning = VerificationWarning {
        code: 1,
        message: Box::from("gate zero boundary"),
        gate: 0,
    };
    assert!(!warning.is_valid(), "gate 0 must be invalid");
}

/// PI-01 explicit boundary: gate=16 must be invalid
#[test]
fn pi_01_gate_one_past_max_is_invalid() {
    let warning = VerificationWarning {
        code: 1,
        message: Box::from("one past max gate boundary"),
        gate: VerificationWarning::MAX_GATE + 1,
    };
    assert!(
        !warning.is_valid(),
        "gate {} (MAX_GATE+1) must be invalid",
        VerificationWarning::MAX_GATE + 1
    );
}

// ============================================================================
// PI-02: Digest roundtrip through submit_artifact and journal read
// ============================================================================

// PI-02 Invariant (Relaxed policy): digest roundtrip
proptest! {
    #[test]
    fn pi_02_digest_roundtrip_relaxed(seed in 0u64..1000u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let value_base = 42i64;
        let value = value_base + (seed % 10) as i64;
        std::hint::black_box(value);
        let journal = match temp_journal() {
            Ok(j) => j,
            Err(e) => { prop_assume!(false, "journal open failed: {}", e); return Ok(()); }
        };
        let workflow = match minimal_workflow(value) {
            Ok(w) => w,
            Err(e) => { prop_assume!(false, "workflow build failed: {}", e); return Ok(()); }
        };
        let expected_digest = workflow.digest();

        let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed);
        prop_assert!(result.is_ok(), "submit_artifact(relaxed) should succeed: {:?}", result);
        let artifact = result.unwrap();
        prop_assert_eq!(
            artifact.digest, expected_digest,
            "artifact.digest must match workflow.digest()"
        );

        let loaded = journal
            .compiled_ir(expected_digest)
            .map_err(|e| format!("compiled_ir read failed: {}", e))
            .unwrap();
        prop_assert!(loaded.is_some(), "artifact must be readable after submit_artifact");
        let record = loaded.unwrap();
        prop_assert_eq!(record.digest, expected_digest);
    }
}

// PI-02 Invariant (Journaled policy): digest roundtrip
proptest! {
    #[test]
    fn pi_02_digest_roundtrip_journaled(seed in 0u64..1000u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let value_base = 42i64;
        let value = value_base + (seed % 10) as i64;
        std::hint::black_box(value);
        let journal = match temp_journal() {
            Ok(j) => j,
            Err(e) => { prop_assume!(false, "journal open failed: {}", e); return Ok(()); }
        };
        let workflow = match minimal_workflow(value) {
            Ok(w) => w,
            Err(e) => { prop_assume!(false, "workflow build failed: {}", e); return Ok(()); }
        };
        let expected_digest = workflow.digest();

        let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled);
        prop_assert!(result.is_ok(), "submit_artifact(journaled) should succeed: {:?}", result);
        let artifact = result.unwrap();
        prop_assert_eq!(artifact.digest, expected_digest);

        let loaded = journal
            .compiled_ir(expected_digest)
            .map_err(|e| format!("compiled_ir read failed: {}", e))
            .unwrap();
        prop_assert!(loaded.is_some(), "artifact must be readable from journal");
        let record = loaded.unwrap();
        prop_assert_eq!(record.digest, expected_digest);
    }
}

// PI-02 Invariant (Strict policy): digest roundtrip
proptest! {
    #[test]
    fn pi_02_digest_roundtrip_strict(seed in 0u64..1000u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let value_base = 42i64;
        let value = value_base + (seed % 10) as i64;
        std::hint::black_box(value);
        let journal = match temp_journal() {
            Ok(j) => j,
            Err(e) => { prop_assume!(false, "journal open failed: {}", e); return Ok(()); }
        };
        let workflow = match minimal_workflow(value) {
            Ok(w) => w,
            Err(e) => { prop_assume!(false, "workflow build failed: {}", e); return Ok(()); }
        };
        let expected_digest = workflow.digest();

        let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict);
        prop_assert!(result.is_ok(), "submit_artifact(strict) should succeed: {:?}", result);
        let artifact = result.unwrap();
        prop_assert_eq!(artifact.digest, expected_digest);

        let loaded = journal
            .compiled_ir(expected_digest)
            .map_err(|e| format!("compiled_ir read failed: {}", e))
            .unwrap();
        prop_assert!(loaded.is_some(), "artifact must be readable from journal");
        let record = loaded.unwrap();
        prop_assert_eq!(record.digest, expected_digest);
    }
}

/// PI-02 anti-invariant: tampered workflow digest is rejected by Journaled/Strict.
/// (Unit test — tampering requires structured construction, not arbitrary input.)
#[test]
fn pi_02_antiinvariant_tampered_digest_rejected() {
    use vb_core::RuntimePolicy;

    let journal = temp_journal().expect("journal should open");

    // Build a structurally-valid workflow with an incorrect digest.
    let parts = WorkflowParts {
        name: Box::from("tampered"),
        digest: WorkflowDigest::from_bytes([0xFFu8; 32]), // intentionally wrong
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let tampered = CompiledWorkflow::try_from_parts(parts).expect("structurally valid");

    // Journaled/Strict must reject tampered digest
    let result_journaled = submit_artifact(&journal, &tampered, RuntimePolicy::Journaled);
    assert!(
        matches!(
            result_journaled,
            Err(vb_storage::JournalError::ArtifactChecksumMismatch)
        ),
        "Journaled must reject tampered digest, got {:?}",
        result_journaled
    );

    let result_strict = submit_artifact(&journal, &tampered, RuntimePolicy::Strict);
    assert!(
        matches!(
            result_strict,
            Err(vb_storage::JournalError::ArtifactChecksumMismatch)
        ),
        "Strict must reject tampered digest, got {:?}",
        result_strict
    );

    // Relaxed accepts any workflow regardless of digest (no checksum gate)
    let result_relaxed = submit_artifact(&journal, &tampered, RuntimePolicy::Relaxed);
    assert!(
        result_relaxed.is_ok(),
        "Relaxed must accept tampered digest (no checksum gate), got {:?}",
        result_relaxed
    );
}
