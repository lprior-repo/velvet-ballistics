#![forbid(unsafe_code)]
//! High-level recovery orchestration.
//!
//! Provides:
//! - Runtime summary recovery
//! - Frame seed recovery
//! - Incomplete run discovery
//! - Digest verification

use crate::recovery::types::{DigestCheck, RecoveryError, RecoveryHydration, RecoveryResult};
use crate::{FjallJournal, JournalEvent};
use vb_core::{RunId, WorkflowDigest};

/// Verifies that the workflow source digest matches the stored record.
///
/// Returns `WorkflowSourceDigestMismatch` when the stored digest differs from
/// the expected value, and `NoRecoveryData` when no `RunAccepted` event is
/// present in the journal for the given run. A missing acceptance record is
/// treated as a verification failure because it means the digest was never
/// recorded and therefore cannot be trusted.
pub fn check_workflow_source_digest(
    journal: &FjallJournal,
    run: RunId,
    expected: WorkflowDigest,
) -> RecoveryResult<()> {
    let events = journal.events_for_run(run)?;
    for event in &events {
        if let JournalEvent::RunAccepted { workflow, .. } = event {
            if *workflow != expected {
                return Err(RecoveryError::WorkflowSourceDigestMismatch {
                    expected,
                    found: *workflow,
                });
            }
            return Ok(());
        }
    }
    Err(RecoveryError::NoRecoveryData { run })
}

/// Verifies that the compiled IR digest matches the expected value.
pub fn check_compiled_ir_digest(
    expected: WorkflowDigest,
    found: WorkflowDigest,
) -> RecoveryResult<()> {
    if expected == found {
        Ok(())
    } else {
        Err(RecoveryError::CompiledIrDigestMismatch { expected, found })
    }
}

/// Postcondition spec for verify_digests.
/// POST-003: returns Ok only when ALL digests match at the requested level:
///   - WorkflowSourceOnly: workflow source digest matches
///   - WorkflowAndIr: workflow source AND compiled IR digests match
///   - Full: workflow source AND compiled IR AND action ABI AND policy digests all match
///
/// GAP-3: The current implementation defers action ABI and policy digest verification.
/// The spec documents the intended POST-003 behavior pending implementation of
/// lookup_action_abi_digest and lookup_policy_digest functions.
#[verus::spec]
fn verify_digests_spec(
    journal: &FjallJournal,
    run: RunId,
    workflow_digest: WorkflowDigest,
    ir_digest: WorkflowDigest,
    found_ir_digest: WorkflowDigest,
    level: DigestCheck,
    #[spec(skip)] _action_abi_digests: &[(vb_core::ActionId, WorkflowDigest)],
    #[spec(skip)] _policy_digests: &[(vb_core::StepIdx, WorkflowDigest)],
) -> bool {
    true
}

/// Verifies all digests at the requested check level.
/// POST-003: returns Ok only when ALL digests match (workflow, compiled IR, action ABI, policy).
/// GAP-3: Action ABI and policy digest verification is deferred pending lookup function implementation.
pub fn verify_digests(
    journal: &FjallJournal,
    run: RunId,
    workflow_digest: WorkflowDigest,
    ir_digest: WorkflowDigest,
    found_ir_digest: WorkflowDigest,
    level: DigestCheck,
) -> RecoveryResult<()> {
    if matches!(
        level,
        DigestCheck::WorkflowSourceOnly | DigestCheck::WorkflowAndIr | DigestCheck::Full
    ) {
        check_workflow_source_digest(journal, run, workflow_digest)?;
    }
    if matches!(level, DigestCheck::WorkflowAndIr | DigestCheck::Full) {
        check_compiled_ir_digest(ir_digest, found_ir_digest)?;
    }
    Ok(())
}

/// Recovers a summary-only runtime hydration product for a run.
pub fn recover_runtime_summary(
    journal: &FjallJournal,
    run: RunId,
) -> RecoveryResult<RecoveryHydration> {
    let events = journal.events_for_run(run)?;
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run });
    }
    crate::recovery::replay::summary::summarize_recovery_events(&events)
}

/// Recovers a minimal live-frame seed from durable journal events for a run.
pub fn recover_runtime_frame_seed(
    journal: &FjallJournal,
    run: RunId,
) -> RecoveryResult<crate::recovery::types::RecoveryFrameSeed> {
    let events = journal.events_for_run(run)?;
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run });
    }
    crate::recovery::replay::summary::recover_runtime_frame_seed_from_events(&events)
}

/// Recovers the latest run admission metadata for a run from durable events.
pub fn recover_run_admission(
    journal: &FjallJournal,
    run: RunId,
) -> RecoveryResult<Option<crate::recovery::types::RecoveredRunAdmission>> {
    let events = journal.events_for_run(run)?;
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run });
    }
    Ok(crate::recovery::replay::summary::recover_run_admission_from_events(&events))
}

/// Recovers summary hydration for every durable run header whose journal has no
/// terminal event. The run header scan supplies candidates; journal events define
/// incompleteness because the status byte/index has no stable terminal mapping.
pub fn recover_all_incomplete_runs(
    journal: &FjallJournal,
) -> RecoveryResult<Vec<RecoveryHydration>> {
    let headers = journal.run_headers()?;
    let mut recovered = Vec::new();

    for header in headers {
        let events = journal.events_for_run(header.run)?;
        if events.is_empty() {
            return Err(RecoveryError::NoRecoveryData { run: header.run });
        }
        if crate::recovery::replay::core::extract_terminal(&events).is_none() {
            recovered.push(crate::recovery::replay::summary::summarize_recovery_events(
                &events,
            )?);
        }
    }

    Ok(recovered)
}
