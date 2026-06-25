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

use crate::recovery::types::{DigestCheck, RecoveryError, RecoveryHydration, RecoveryResult};
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

/// Verifies all digests at the requested check level.
///
/// Master §18.8 mandates that replay check workflow source, compiled IR, action ABI,
/// and policy digests. `DigestCheck::Full` therefore consults every category.
/// `action_abi_digests` and `policy_digests` are each `(subject, expected, found)`
/// triples; an empty slice skips that category without raising an error. A non-empty
/// slice whose entries do not all match returns the matching typed `RecoveryError`.
/// POST-003: returns Ok only when ALL requested digests match.
pub fn verify_digests(
    journal: &FjallJournal,
    run: RunId,
    workflow_digest: WorkflowDigest,
    ir_digest: WorkflowDigest,
    found_ir_digest: WorkflowDigest,
    level: DigestCheck,
    action_abi_digests: &[(ActionId, WorkflowDigest, WorkflowDigest)],
    policy_digests: &[(StepIdx, WorkflowDigest, WorkflowDigest)],
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
    if matches!(level, DigestCheck::Full) {
        check_action_abi_digests(action_abi_digests)?;
        check_policy_digests(policy_digests)?;
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

/// Recovers a minimal live-frame seed from durable journal events for a run.
pub fn recover_runtime_frame_seed(
    journal: &FjallJournal,
    run: RunId,
) -> RecoveryResult<crate::recovery::types::RecoveryFrameSeed> {
    let events = journal.events_for_run_full(run)?;
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
