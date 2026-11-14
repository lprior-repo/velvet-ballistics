#![forbid(unsafe_code)]
//! Cold reopen and owned-arena recovery expectations for Together results.

use super::support::*;
use vb_core::{SlotValue, Taint};
use vb_storage::JournalEvent;
use vb_storage::recovery::RecoveryTerminalState;
use vb_storage::recovery::{
    UnsupportedRecoveryState, recover_raw_runtime_frame_seed_from_events_with_workflow,
    summarize_recovery_events,
};

#[test]
fn cold_reopen_reconstructs_together_outcome_from_journal() -> TestResult {
    let live = live(FANOUT, Box::new([]))?;
    let before = events(&live)?;
    let live_pc = live.state.frame.pc();
    let Live {
        journal,
        directory,
        plan,
        ..
    } = live;
    drop(journal);

    let reopened = open_journal(&directory)?;
    let after = reopened.events_for_run_full(RUN)?;
    assert_eq!(
        after, before,
        "cold reopen must preserve the exact evidence"
    );

    let hydration = summarize_recovery_events(&after)?;
    let summary = hydration.summary();
    assert_eq!(
        summary.terminal,
        Some(RecoveryTerminalState::Finished {
            result: after
                .iter()
                .find_map(|event| match event {
                    JournalEvent::RunFinished { result, .. } => Some(*result),
                    _ => None,
                })
                .ok_or("terminal receipt missing")?
        })
    );
    assert_eq!(
        summary.steps_succeeded as usize,
        succeeded(&after).len(),
        "summary counts every recorded Together step success"
    );

    let seed = recover_raw_runtime_frame_seed_from_events_with_workflow(&after, &plan)?;
    assert_eq!(seed.pc, live_pc, "recovered pc equals live pc");
    let join_slot = finish_result(&plan)?;
    let join_entry = seed
        .slots
        .iter()
        .find(|entry| entry.slot == join_slot)
        .ok_or("recovered join output slot missing")?;
    assert!(
        matches!(join_entry.value, SlotValue::List(_)),
        "join outcome is recovered as a list, not a fabricated scalar"
    );
    assert_eq!(
        join_entry.taint,
        Taint::Clean,
        "durable slot extra preserves join taint"
    );
    assert_eq!(
        seed.steps
            .iter()
            .filter(|entry| { entry.state == vb_storage::recovery::RecoveredStepState::Succeeded })
            .count(),
        succeeded(&after).len(),
        "recovered step states mirror durable successes"
    );
    assert_eq!(
        seed.unsupported,
        UnsupportedRecoveryState::slot_values_unsupported(),
        "composite Together results are surfaced as unsupported, never fabricated"
    );
    let recovered_pc = seed.pc;
    assert!(
        seed.steps.iter().any(|entry| entry.step == recovered_pc),
        "recovered pc refers to an observed step"
    );
    Ok(())
}

/// Positive cold-reopen expectation for a Together list result.
#[test]
fn reopened_together_result_is_exactly_reconstructed_when_owned_arena_is_available() -> TestResult {
    let live = live(FANOUT, Box::new([]))?;
    let before = events(&live)?;
    let Live {
        journal,
        directory,
        plan,
        state,
    } = live;
    drop(journal);

    let reopened = open_journal(&directory)?;
    let after = reopened.events_for_run_full(RUN)?;
    assert_eq!(after, before);

    let replay = vb_storage::recovery::replay_owned_for_each(&after, &plan, &[], None)?
        .ok_or("generic owned replay not selected")?;
    assert!(
        replay.product.seed().unsupported.is_fully_supported(),
        "owned arena recovery must not mark list values unsupported: {:?}",
        replay.product.seed()
    );
    let result_slot = finish_result(&plan)?;
    assert_eq!(
        replay.frame.read_slot(result_slot)?,
        state.frame.read_slot(result_slot)?,
        "recovered join slot equals the live join slot"
    );
    let store = vb_core::ValueStore::from_checkpoint_v1(replay.store.clone())?;
    let SlotValue::List(list) = *replay.frame.read_slot(result_slot)? else {
        return Err("together result is not a list".into());
    };
    assert_eq!(
        store.list(list)?,
        [SlotValue::I64(7), SlotValue::I64(11), SlotValue::I64(13)]
    );
    for index in 0..3 {
        assert_eq!(store.list_item_with_taint(list, index)?.1, Taint::Clean);
    }
    Ok(())
}
