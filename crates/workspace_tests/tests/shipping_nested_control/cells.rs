//! Per-cell owned recovery qualification for outer Repeat/Together/Choose.
//!
//! Each legal cell is compiled, admitted, driven to a nonterminal cut inside
//! the nested body, replayed, cold-reopened, resumed to completion, and
//! checked against the documented component oracle.

#![forbid(unsafe_code)]

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::{CompiledWorkflow, SlotValue, StepIdx, Taint};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::recovery::RuntimeRecoveryProduct;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::recovery::replay_owned_for_each;
use vb_storage::{DecodedSlotWrittenExtra, FjallJournal, JournalEvent, decode_slot_written_extra};

use self::writes::{for_slot, for_step};
use crate::support::{Cell, Cut, LEAF, SENTINEL, TestResult, cut_at_leaf, multiplicity};

#[path = "cells/writes.rs"]
mod writes;

/// Qualifies one legal cell end to end, collecting every failed stage.
pub(crate) fn qualify(cell: Cell) -> TestResult {
    let label = cell.label();
    if cell.prohibited() {
        return Err(format!("{label}: prohibited cell must not be qualified"));
    }
    let expected_leaf = multiplicity(cell.outer)
        .checked_mul(multiplicity(cell.inner))
        .ok_or_else(|| format!("{label}: leaf count overflow"))?;
    let expected_tail = multiplicity(cell.outer);
    let cut = cut_at_leaf(cell)?;
    assert_repeat_collect_budget(cell, &cut)?;
    let mut gaps: Vec<String> = Vec::new();
    if let Err(error) = replay_parity(&label, &cut) {
        gaps.push(error);
    }
    let events = match reopen_finish(&label, &cut) {
        Ok(events) => events,
        Err(error) => {
            gaps.push(error);
            return Err(gaps.join("; "));
        }
    };
    if let Err(error) = assert_cut(&label, &cut) {
        gaps.push(error);
    }
    if let Err(error) = assert_writes(&label, &events, &cut, expected_leaf, expected_tail) {
        gaps.push(error);
    }
    if let Err(error) = assert_order(&label, &events, &cut) {
        gaps.push(error);
    }
    match CompiledWorkflow::try_from_parts(cut.workflow.to_parts()) {
        Ok(revalidated) if revalidated == cut.workflow => {}
        Ok(_) => gaps.push(format!("{label}: full IR changed")),
        Err(error) => gaps.push(format!("{label}: full IR: {error:?}")),
    }
    if gaps.is_empty() {
        Ok(())
    } else {
        Err(gaps.join("; "))
    }
}

fn assert_repeat_collect_budget(cell: Cell, cut: &Cut) -> TestResult {
    if cell.outer != "repeat" || cell.inner != "collect" {
        return Ok(());
    }
    let budget = cut.events.iter().find_map(|event| match event {
        JournalEvent::RunAdmissionBudget { budget, .. } => Some(budget),
        _ => None,
    });
    let budget = budget.ok_or_else(|| "repeat/collect admission budget missing".to_owned())?;
    if (budget.max_gather_pages, budget.max_gather_items) != (4, 4) {
        return Err(format!(
            "repeat/collect gather budget {:?}/{:?} != 4/4",
            budget.max_gather_pages, budget.max_gather_items
        ));
    }
    Ok(())
}

fn replay_parity(label: &str, cut: &Cut) -> TestResult {
    let replay = replay_owned_for_each(
        &cut.events,
        &cut.workflow,
        &cut.roots,
        Some(&cut.input_store),
    )
    .map_err(|error| format!("{label}: owned replay: {error:?}"))?
    .ok_or_else(|| format!("{label}: owned replay not selected"))?;
    if replay.frame.checkpoint_v1() != cut.frame {
        return Err(format!("{label}: replay frame diverged"));
    }
    if replay.store != cut.store {
        return Err(format!("{label}: replay store diverged"));
    }
    Ok(())
}

fn reopen_finish(label: &str, cut: &Cut) -> TestResult<Vec<JournalEvent>> {
    let journal = Arc::new(
        FjallJournal::open(cut.directory.path(), None)
            .map_err(|error| format!("{label}: reopen: {error:?}"))?,
    );
    let mut runtime = Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: vb_core::RuntimePolicy::Strict,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(Arc::clone(&journal)),
    );
    match runtime
        .recover_product(cut.run)
        .map_err(|error| format!("{label}: recover_product: {error:?}"))?
    {
        RuntimeRecoveryProduct::Resumable(_) => {}
        other => return Err(format!("{label}: nonterminal cut not resumable: {other:?}")),
    }
    runtime
        .recover_and_resume(cut.run)
        .map_err(|error| format!("{label}: recover_and_resume: {error:?}"))?;
    for _ in 0..8192 {
        runtime
            .tick_all()
            .map_err(|error| format!("{label}: resume tick: {error:?}"))?;
        if runtime.counters_snapshot().runs_failed != 0 {
            return Err(format!("{label}: resumed run failed"));
        }
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    if runtime.counters_snapshot().runs_completed != 1 {
        return Err(format!("{label}: resumed run did not complete"));
    }
    journal
        .events_for_run_full(cut.run)
        .map_err(|error| format!("{label}: final events: {error:?}"))
}

fn assert_cut(label: &str, cut: &Cut) -> TestResult {
    let slots = [cut.input_slot, cut.leaf_slot, cut.tail_slot];
    let unique = slots.iter().enumerate().all(|(index, slot)| {
        !slots
            .iter()
            .skip(index.saturating_add(1))
            .any(|other| other == slot)
    });
    if !unique {
        return Err(format!("{label}: lexical slots alias {slots:?}"));
    }
    if !cut
        .events
        .iter()
        .any(|event| matches!(event, JournalEvent::StepSucceeded { step, .. } if *step == cut.leaf))
    {
        return Err(format!("{label}: cut lacks leaf StepSucceeded"));
    }
    if cut
        .events
        .iter()
        .any(|event| matches!(event, JournalEvent::RunFinished { .. }))
    {
        return Err(format!("{label}: cut is terminal"));
    }
    Ok(())
}

fn assert_writes(
    label: &str,
    events: &[JournalEvent],
    cut: &Cut,
    expected_leaf: usize,
    expected_tail: usize,
) -> TestResult {
    let leaf_writes = for_step(events, cut.leaf_slot, cut.leaf)?;
    if leaf_writes.len() != expected_leaf {
        return Err(format!(
            "{label}: leaf writes {} != {expected_leaf}",
            leaf_writes.len()
        ));
    }
    if !leaf_writes
        .iter()
        .all(|(value, taint)| *value == SlotValue::I64(LEAF) && *taint == Taint::Clean)
    {
        return Err(format!("{label}: leaf value/taint mismatch"));
    }
    let tail_writes = for_slot(events, cut.tail_slot)?;
    if tail_writes.len() != expected_tail {
        return Err(format!(
            "{label}: tail writes {} != {expected_tail}",
            tail_writes.len()
        ));
    }
    if !tail_writes
        .iter()
        .all(|(value, taint)| *value == SlotValue::I64(SENTINEL) && *taint == Taint::Clean)
    {
        return Err(format!("{label}: tail value/taint mismatch"));
    }
    if count_succeeded(events, cut.leaf) != expected_leaf {
        return Err(format!("{label}: leaf executions != {expected_leaf}"));
    }
    if count_succeeded(events, cut.tail) != expected_tail {
        return Err(format!("{label}: tail executions != {expected_tail}"));
    }
    let (value, taint) = terminal_result(events)?;
    if value != SlotValue::I64(SENTINEL) || taint != Taint::Clean {
        return Err(format!("{label}: terminal {value:?}/{taint:?}"));
    }
    Ok(())
}

fn assert_order(label: &str, events: &[JournalEvent], cut: &Cut) -> TestResult {
    match (
        first_succeeded(events, cut.leaf),
        first_succeeded(events, cut.tail),
    ) {
        (Some(leaf), Some(tail)) if leaf < tail => {}
        other => return Err(format!("{label}: source order violated {other:?}")),
    }
    if !seq_monotonic(events) {
        return Err(format!("{label}: journal sequence regressed"));
    }
    Ok(())
}

fn count_succeeded(events: &[JournalEvent], step: StepIdx) -> usize {
    events
        .iter()
        .filter(
            |event| matches!(event, JournalEvent::StepSucceeded { step: found, .. } if *found == step),
        )
        .count()
}

fn first_succeeded(events: &[JournalEvent], step: StepIdx) -> Option<u64> {
    events.iter().find_map(|event| match event {
        JournalEvent::StepSucceeded {
            seq, step: found, ..
        } if *found == step => Some(seq.get()),
        _ => None,
    })
}

fn terminal_result(events: &[JournalEvent]) -> TestResult<(SlotValue, Taint)> {
    let result = events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::RunFinished { result, .. } => Some(*result),
            _ => None,
        })
        .ok_or_else(|| "missing RunFinished".to_owned())?;
    events
        .iter()
        .rev()
        .find_map(|event| match event {
            JournalEvent::SlotWrittenEvent {
                slot,
                value: Some(bytes),
                extra: Some(extra),
                ..
            } if *slot == result => {
                let value = postcard::from_bytes::<SlotValue>(bytes).ok()?;
                let taint = match decode_slot_written_extra(extra).ok()? {
                    DecodedSlotWrittenExtra::Envelope(envelope) => envelope.taint,
                    DecodedSlotWrittenExtra::LegacyFrameExtra(_) => return None,
                };
                Some((value, taint))
            }
            _ => None,
        })
        .ok_or_else(|| "missing terminal slot write".to_owned())
}

fn seq_monotonic(events: &[JournalEvent]) -> bool {
    events.windows(2).all(|window| match window {
        [left, right] => left.seq() < right.seq(),
        _ => true,
    })
}
