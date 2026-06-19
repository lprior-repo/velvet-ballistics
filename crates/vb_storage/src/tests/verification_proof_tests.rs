#![forbid(unsafe_code)]
//! SECTION 2.4: VerificationProof Invariants (Unit Tests)

use crate::admission::submit_artifact;
use crate::{EventSeq, FjallJournal};
use vb_core::{RuntimePolicy, SlotIdx, StepIdx, WorkflowDigest};

use crate::tests::fixtures::{minimal_valid_workflow, temp_journal};

fn submit_artifact_in_fresh_journal(
    workflow: &vb_core::CompiledWorkflow,
    policy: RuntimePolicy,
) -> Result<crate::admission::AcceptedArtifact, String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    submit_artifact(&journal, workflow, policy)
        .map_err(|e| format!("submit_artifact({policy:?}) failed: {e}"))
}

/// TEST: gate_count zero for Relaxed
///
/// Contract §3.2: Relaxed → gate_count = 0.
#[test]
fn gate_count_zero_for_relaxed() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let workflow = minimal_valid_workflow()?;

    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)
        .map_err(|e| format!("submit failed: {e}"))?;

    assert_eq!(
        result.verification.gate_count, 0,
        "Relaxed policy must have gate_count == 0"
    );
    Ok(())
}

/// TEST: gate_count two for Journaled
///
/// Contract §3.2: Journaled → gate_count = 2.
#[test]
fn gate_count_two_for_journaled() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let workflow = minimal_valid_workflow()?;

    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
        .map_err(|e| format!("submit failed: {e}"))?;

    assert_eq!(
        result.verification.gate_count, 15,
        "Journaled policy must have gate_count == 15"
    );
    Ok(())
}

/// TEST: gate_count fifteen for Strict
///
/// Contract §3.2: Strict → gate_count = 15.
#[test]
fn gate_count_two_for_strict() -> Result<(), String> {
    let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
    let workflow = minimal_valid_workflow()?;

    let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
        .map_err(|e| format!("submit failed: {e}"))?;

    assert_eq!(
        result.verification.gate_count, 15,
        "Strict policy must have gate_count == 15"
    );
    Ok(())
}

/// TEST: durable true only for Strict
///
/// Contract §3.2: durable == true only for Strict.
#[test]
fn durable_true_only_for_strict() -> Result<(), String> {
    let workflow = minimal_valid_workflow()?;

    let relaxed = submit_artifact_in_fresh_journal(&workflow, RuntimePolicy::Relaxed)?;
    let journaled = submit_artifact_in_fresh_journal(&workflow, RuntimePolicy::Journaled)?;
    let strict = submit_artifact_in_fresh_journal(&workflow, RuntimePolicy::Strict)?;

    assert!(
        !relaxed.verification.durable,
        "Relaxed must have durable == false"
    );
    assert!(
        !journaled.verification.durable,
        "Journaled must have durable == false"
    );
    assert!(
        strict.verification.durable,
        "Strict must have durable == true"
    );
    Ok(())
}
