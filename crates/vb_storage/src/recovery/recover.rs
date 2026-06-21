#![forbid(unsafe_code)]
//! High-level recovery orchestration.

use crate::recovery::digest::{
    first_action_abi_mismatch, first_policy_mismatch, workflow_digest_bytes_equal,
};
use crate::recovery::{
    DigestCheck, DigestCheckConfig, RecoveryError, RecoveryHydration, RecoveryResult,
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
///
/// SR-002: this function reads the **full** per-run event history, including
/// events that occurred at or before the most recent durable snapshot. The
/// `RunAccepted` event always precedes any snapshot, so a tail-only reader
/// would silently miss it.
pub fn check_workflow_source_digest(
    journal: &FjallJournal,
    run: RunId,
    expected: WorkflowDigest,
) -> RecoveryResult<()> {
    let events = journal.events_for_run_full(run)?;
    for event in &events {
        if let JournalEvent::RunAccepted { workflow, .. } = event {
            if !workflow_digest_bytes_equal(*workflow, expected) {
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
    if workflow_digest_bytes_equal(expected, found) {
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
    if workflow_digest_bytes_equal(expected, found) {
        Ok(())
    } else {
        Err(RecoveryError::ActionAbiMismatch {
            action_id,
            expected,
            found,
        })
    }
}

/// Verifies a runtime policy digest at a recovery boundary.
pub fn check_policy_digest(
    step: StepIdx,
    expected: WorkflowDigest,
    found: WorkflowDigest,
) -> RecoveryResult<()> {
    if workflow_digest_bytes_equal(expected, found) {
        Ok(())
    } else {
        Err(RecoveryError::PolicyDigestMismatch {
            step,
            expected,
            found,
        })
    }
}

/// Verifies all digests at the requested check level.
///
/// POST-003: returns Ok only when ALL digests match (workflow, compiled IR,
/// action ABI, policy).
///
/// For `DigestCheck::Full`, `config` must provide both action ABI and policy
/// entry slices. Empty slices are valid only when the caller has no entries for
/// that digest class; omitted slices fail closed.
pub fn verify_digests(
    journal: &FjallJournal,
    run: RunId,
    workflow_digest: WorkflowDigest,
    ir_digest: WorkflowDigest,
    found_ir_digest: WorkflowDigest,
    level: DigestCheck,
    config: Option<DigestCheckConfig<'_>>,
) -> RecoveryResult<()> {
    check_workflow_and_ir(
        journal,
        run,
        workflow_digest,
        ir_digest,
        found_ir_digest,
        level,
    )?;
    check_full_level(config, level)?;
    Ok(())
}

fn check_workflow_and_ir(
    journal: &FjallJournal,
    run: RunId,
    workflow_digest: WorkflowDigest,
    ir_digest: WorkflowDigest,
    found_ir_digest: WorkflowDigest,
    level: DigestCheck,
) -> RecoveryResult<()> {
    if level.checks_workflow_source() {
        check_workflow_source_digest(journal, run, workflow_digest)?;
    }
    if level.checks_compiled_ir() {
        check_compiled_ir_digest(ir_digest, found_ir_digest)?;
    }
    Ok(())
}

fn check_full_level(
    config: Option<DigestCheckConfig<'_>>,
    level: DigestCheck,
) -> RecoveryResult<()> {
    if !matches!(level, DigestCheck::Full) {
        return Ok(());
    }

    let Some(cfg) = config else {
        return Err(RecoveryError::FullDigestCheckConfigMissing);
    };

    let Some(action_entries) = cfg.action_abi_entries else {
        return Err(RecoveryError::FullDigestCheckConfigMissing);
    };
    let Some(policy_entries) = cfg.policy_entries else {
        return Err(RecoveryError::FullDigestCheckConfigMissing);
    };

    check_action_abi_digests(action_entries)?;
    check_policy_digests(policy_entries)?;

    Ok(())
}

/// Checks action ABI digests against expected values from an external source.
///
/// Each entry provides `(action_id, expected_digest, found_digest)`.
/// Returns `ActionAbiMismatch { action_id, expected, found }` on the first mismatch found.
/// Returns `Ok(())` when all entries match or when no entries are provided.
/// Does not guess mismatches from missing data — only checks explicitly provided inputs.
pub fn check_action_abi_digests(
    entries: &[(ActionId, WorkflowDigest, WorkflowDigest)],
) -> RecoveryResult<()> {
    if let Some((action_id, expected, found)) = first_action_abi_mismatch(entries) {
        return Err(RecoveryError::ActionAbiMismatch {
            action_id,
            expected,
            found,
        });
    }
    Ok(())
}

/// Checks policy digests against expected values from an external source.
///
/// Each entry provides `(step, expected_digest, found_digest)`.
/// Returns `PolicyDigestMismatch { step, expected, found }` on the first mismatch found.
/// Returns `Ok(())` when all entries match or when no entries are provided.
/// Does not guess mismatches from missing data — only checks explicitly provided inputs.
pub fn check_policy_digests(
    entries: &[(StepIdx, WorkflowDigest, WorkflowDigest)],
) -> RecoveryResult<()> {
    if let Some((step, expected, found)) = first_policy_mismatch(entries) {
        return Err(RecoveryError::PolicyDigestMismatch {
            step,
            expected,
            found,
        });
    }
    Ok(())
}

/// Recovers a summary-only runtime hydration product for a run.
///
/// SR-002: this function reads the **full** per-run event history. The
/// `RunAccepted` event that supplies the workflow digest and the
/// `RunAdmission` event that seeds the policy digests both precede any
/// durable snapshot, so a tail-only reader would produce a summary with
/// `workflow = None` and under-counted step/slot totals.
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
/// Compares the recovered terminal state against the expected value via the
/// `PartialEq` derive on [`RecoveryTerminalState`], so structurally distinct
/// values (e.g. `Finished { result: SlotIdx(7) }` vs `Finished { result:
/// SlotIdx(99) }`) cannot silently compare equal. Returns
/// `TerminalStateMismatch` when the recovered terminal state does not match
/// the expected value.
///
/// SR-002: reads the **full** per-run event history (see [`recover_runtime_summary`]).
pub fn recover_runtime_summary_with_expected(
    journal: &FjallJournal,
    run: RunId,
    expected: crate::recovery::RecoveryTerminalState,
) -> RecoveryResult<RecoveryHydration> {
    let events = journal.events_for_run_full(run)?;
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run });
    }
    let hydration = crate::recovery::replay::summary::summarize_recovery_events(&events)?;

    if hydration.summary().terminal != Some(expected) {
        return Err(RecoveryError::TerminalStateMismatch {
            expected: format!("{:?}", Some(expected)),
            found: format!("{:?}", hydration.summary().terminal),
        });
    }

    Ok(hydration)
}

/// Recovers a minimal live-frame seed from durable journal events for a run.
///
/// SR-002: reads the **full** per-run event history so the seed's
/// `step_states` map and slot values reflect every step started before any
/// snapshot, not just the tail.
pub fn recover_runtime_frame_seed(
    journal: &FjallJournal,
    run: RunId,
) -> RecoveryResult<crate::recovery::RecoveryFrameSeed> {
    let events = journal.events_for_run_full(run)?;
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run });
    }
    crate::recovery::replay::summary::recover_runtime_frame_seed_from_events(&events)
}

/// Recovers the latest run admission metadata for a run from durable events.
///
/// SR-002: reads the **full** per-run event history. The `RunAdmission` event
/// always precedes any durable snapshot, so a tail-only reader would
/// unconditionally return `None` once a snapshot has been written.
pub fn recover_run_admission(
    journal: &FjallJournal,
    run: RunId,
) -> RecoveryResult<Option<crate::recovery::RecoveredRunAdmission>> {
    let events = journal.events_for_run_full(run)?;
    if events.is_empty() {
        return Err(RecoveryError::NoRecoveryData { run });
    }
    Ok(crate::recovery::replay::summary::recover_run_admission_from_events(&events))
}

/// Recovers summary hydration for every durable run header whose journal has no
/// terminal event. The run header scan supplies candidates; journal events define
/// incompleteness because the status byte/index has no stable terminal mapping.
///
/// SR-002: reads the **full** per-run event history for each candidate run so
/// terminal events that occurred at or before the snapshot are still detected.
/// A tail-only reader would re-enqueue completed runs whose terminal event was
/// pre-snapshot, causing duplicate recovery.
pub fn recover_all_incomplete_runs(
    journal: &FjallJournal,
) -> RecoveryResult<Vec<RecoveryHydration>> {
    let headers = journal.run_headers()?;
    let mut recovered = Vec::with_capacity(headers.len());

    for header in headers {
        let events = journal.events_for_run_full(header.run)?;
        if events.is_empty() {
            return Err(RecoveryError::NoRecoveryData { run: header.run });
        }
        if crate::recovery::replay::extract_terminal(&events).is_none() {
            let seed =
                crate::recovery::replay::summary::recover_runtime_frame_seed_from_events(&events)?;
            recovered.push(RecoveryHydration::FrameSeed(seed));
        }
    }

    Ok(recovered)
}
