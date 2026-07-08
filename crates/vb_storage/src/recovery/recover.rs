#![forbid(unsafe_code)]
//! High-level recovery orchestration.
//!
//! Provides:
//! - Runtime summary recovery
//! - Frame seed recovery
//! - Incomplete run discovery
//! - Digest verification
//!
//! Master §18 (Fjall Persistence Behavior), persistence invariant 3:
//! "Recovery replays snapshots plus tail journal or full journal
//! deterministically." All public entry points in this module use
//! `events_for_run_full` so that pre-snapshot events (RunAccepted,
//! RunAdmission, step / action lifecycle prior to the durable snapshot)
//! are visible to recovery; the snapshot+tail reader (`events_for_run`)
//! skips them by design and is reserved for hot-path replay.

use crate::recovery::types::{
    ActionAbiDigestComparison, DigestVerificationRequest, PolicyDigestComparison, RecoveryError,
    RecoveryFrameSeed, RecoveryFrameSeedProduct, RecoveryHydration, RecoveryResult,
};
use crate::{FjallJournal, JournalEvent};
use vb_core::{ActionId, RunId, StepIdx, WorkflowDigest};

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
    let events = journal.events_for_run_full(run)?;
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

/// Verifies an action ABI digest at a recovery boundary.
pub fn check_action_abi_digest(
    action_id: ActionId,
    expected: WorkflowDigest,
    found: WorkflowDigest,
) -> RecoveryResult<()> {
    if expected == found {
        Ok(())
    } else {
        Err(RecoveryError::ActionAbiMismatch { action_id })
    }
}

/// Verifies a runtime policy digest at a recovery boundary.
pub fn check_policy_digest(
    step: StepIdx,
    expected: WorkflowDigest,
    found: WorkflowDigest,
) -> RecoveryResult<()> {
    if expected == found {
        Ok(())
    } else {
        Err(RecoveryError::PolicyDigestMismatch { step })
    }
}

/// Verifies digests described by a typed request.
///
/// The request shape prevents workflow-only and workflow+IR callers from
/// passing meaningless action/policy placeholders. Full verification must carry
/// [`crate::recovery::FullDigestEvidence`] so action ABI and policy subjects are
/// explicit at the call site.
pub fn verify_digests(
    journal: &FjallJournal,
    run: RunId,
    request: DigestVerificationRequest<'_>,
) -> RecoveryResult<()> {
    match request {
        DigestVerificationRequest::WorkflowSourceOnly {
            expected_workflow_digest,
        } => check_workflow_source_digest(journal, run, expected_workflow_digest),
        DigestVerificationRequest::WorkflowAndIr {
            expected_workflow_digest,
            expected_ir_digest,
            found_ir_digest,
        } => {
            check_workflow_source_digest(journal, run, expected_workflow_digest)?;
            check_compiled_ir_digest(expected_ir_digest, found_ir_digest)
        }
        DigestVerificationRequest::Full {
            expected_workflow_digest,
            expected_ir_digest,
            found_ir_digest,
            evidence,
        } => {
            check_workflow_source_digest(journal, run, expected_workflow_digest)?;
            check_compiled_ir_digest(expected_ir_digest, found_ir_digest)?;
            check_action_abi_comparisons(evidence.action_abi())?;
            check_policy_comparisons(evidence.policy())
        }
    }
}

fn check_action_abi_comparisons(entries: &[ActionAbiDigestComparison]) -> RecoveryResult<()> {
    for entry in entries {
        check_action_abi_digest(entry.action_id, entry.digest.expected, entry.digest.found)?;
    }
    Ok(())
}

fn check_policy_comparisons(entries: &[PolicyDigestComparison]) -> RecoveryResult<()> {
    for entry in entries {
        check_policy_digest(entry.step, entry.digest.expected, entry.digest.found)?;
    }
    Ok(())
}

/// Checks action ABI digests against expected values from an external source.
///
/// Each entry provides `(action_id, expected_digest, found_digest)`.
/// Returns `ActionAbiMismatch { action_id }` on the first mismatch found.
/// Returns `Ok(())` when all entries match or when no entries are provided.
/// Does not guess mismatches from missing data — only checks explicitly provided inputs.
pub fn check_action_abi_digests(
    entries: &[(ActionId, WorkflowDigest, WorkflowDigest)],
) -> RecoveryResult<()> {
    for (action_id, expected, found) in entries {
        if *expected != *found {
            return Err(RecoveryError::ActionAbiMismatch {
                action_id: *action_id,
            });
        }
    }
    Ok(())
}

/// Checks policy digests against expected values from an external source.
///
/// Each entry provides `(step, expected_digest, found_digest)`.
/// Returns `PolicyDigestMismatch { step }` on the first mismatch found.
/// Returns `Ok(())` when all entries match or when no entries are provided.
/// Does not guess mismatches from missing data — only checks explicitly provided inputs.
pub fn check_policy_digests(
    entries: &[(StepIdx, WorkflowDigest, WorkflowDigest)],
) -> RecoveryResult<()> {
    for (step, expected, found) in entries {
        if *expected != *found {
            return Err(RecoveryError::PolicyDigestMismatch { step: *step });
        }
    }
    Ok(())
}

/// Recovers a summary-only runtime hydration product for a run.
pub fn recover_runtime_summary(
    journal: &FjallJournal,
    run: RunId,
) -> RecoveryResult<RecoveryHydration> {
    let events = journal.events_for_run_full(run)?;
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run });
    }
    crate::recovery::replay::summary::summarize_recovery_events(&events)
}

/// Recovers a summary-only runtime hydration product and verifies terminal state.
///
/// Returns `TerminalStateMismatch` when the recovered terminal state does not match
/// the expected value.
pub fn recover_runtime_summary_with_expected(
    journal: &FjallJournal,
    run: RunId,
    expected: crate::recovery::types::RecoveryTerminalState,
) -> RecoveryResult<RecoveryHydration> {
    let events = journal.events_for_run_full(run)?;
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run });
    }
    let hydration = crate::recovery::replay::summary::summarize_recovery_events(&events)?;

    let found_str = terminal_state_to_string(hydration.summary().terminal);
    let expected_str = terminal_state_to_string(Some(expected));

    if found_str != expected_str {
        return Err(RecoveryError::TerminalStateMismatch {
            expected: expected_str,
            found: found_str,
        });
    }

    Ok(hydration)
}

/// Converts a `RecoveryTerminalState` to its string representation.
fn terminal_state_to_string(
    terminal: Option<crate::recovery::types::RecoveryTerminalState>,
) -> String {
    match terminal {
        None => "NoTerminal".to_owned(),
        Some(crate::recovery::types::RecoveryTerminalState::Cancelled) => "Cancelled".to_owned(),
        Some(crate::recovery::types::RecoveryTerminalState::Killed) => "Killed".to_owned(),
        Some(crate::recovery::types::RecoveryTerminalState::Failed) => "Failed".to_owned(),
        Some(crate::recovery::types::RecoveryTerminalState::Finished { .. }) => {
            "Finished".to_owned()
        }
    }
}

/// Recovers a typed frame-seed product from durable journal events for a run.
pub fn recover_runtime_frame_seed(
    journal: &FjallJournal,
    run: RunId,
) -> RecoveryResult<RecoveryFrameSeedProduct> {
    recover_raw_runtime_frame_seed(journal, run).map(RecoveryFrameSeedProduct::from_seed)
}

/// Compatibility/raw replay DTO recovery for low-level verifier and tests.
pub fn recover_raw_runtime_frame_seed(
    journal: &FjallJournal,
    run: RunId,
) -> RecoveryResult<RecoveryFrameSeed> {
    let events = journal.events_for_run_full(run)?;
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run });
    }
    crate::recovery::replay::summary::recover_raw_runtime_frame_seed_from_events(&events)
}

/// Recovers the latest run admission metadata for a run from durable events.
pub fn recover_run_admission(
    journal: &FjallJournal,
    run: RunId,
) -> RecoveryResult<Option<crate::recovery::types::RecoveredRunAdmission>> {
    let events = journal.events_for_run_full(run)?;
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
        let events = journal.events_for_run_full(header.run)?;
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
