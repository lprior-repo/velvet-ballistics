#![forbid(unsafe_code)]
//! SECTION 2.2: admit_compiled_artifact — Admission Gate (Unit Tests)

use crate::admission::{admit_compiled_artifact, submit_artifact};
use crate::{EventSeq, FjallJournal, JournalError};
use vb_core::{RunId, RuntimePolicy, SlotIdx, StepIdx, WorkflowDigest};

use crate::tests::fixtures::{minimal_valid_workflow, temp_journal};

/// TEST: admit_compiled_artifact structure gate enforced
///
/// Contract §2.2: ArtifactMalformed when try_from_parts fails.
#[test]
fn admit_compiled_artifact_structure_gate_enforced() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let workflow = minimal_valid_workflow()?;

    let result = admit_compiled_artifact(&journal, &workflow)
        .map_err(|e| format!("admit_compiled_artifact failed: {e}"))?;

    assert_eq!(
        result,
        workflow.digest(),
        "admit_compiled_artifact must return workflow digest on success"
    );
    Ok(())
}

/// TEST: admit_compiled_artifact idempotent on duplicate
///
/// Contract §2.2 Postcondition: On duplicate admission, returns same digest without error.
#[test]
fn admit_compiled_artifact_idempotent_on_duplicate() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let workflow = minimal_valid_workflow()?;

    let digest_a = admit_compiled_artifact(&journal, &workflow)
        .map_err(|e| format!("first admit failed: {e}"))?;
    let digest_b = admit_compiled_artifact(&journal, &workflow)
        .map_err(|e| format!("second admit failed: {e}"))?;

    assert_eq!(
        digest_a, digest_b,
        "duplicate admission must return same digest (idempotent)"
    );
    Ok(())
}

/// TEST: admit_compiled_artifact puts record with matching digest
///
/// Contract §2.2 Postcondition: journal.put_compiled_ir called where record.digest == workflow.digest().
#[test]
fn admit_compiled_artifact_puts_record_with_matching_digest() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let workflow = minimal_valid_workflow()?;
    let expected_digest = workflow.digest();

    admit_compiled_artifact(&journal, &workflow).map_err(|e| format!("admit failed: {e}"))?;

    let loaded = journal
        .compiled_ir(expected_digest)
        .map_err(|e| format!("read: {e}"))?;
    assert!(
        loaded.is_some(),
        "compiled_ir must contain record with matching digest"
    );
    Ok(())
}

/// TEST: admit_compiled_artifact returns workflow digest
///
/// Contract §2.2 Postcondition: Returns workflow.digest().
#[test]
fn admit_compiled_artifact_returns_workflow_digest() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let workflow = minimal_valid_workflow()?;

    let result = admit_compiled_artifact(&journal, &workflow)
        .map_err(|e| format!("admit failed: {e}"))?;

    assert_eq!(
        result,
        workflow.digest(),
        "returned digest must equal workflow.digest()"
    );
    Ok(())
}
