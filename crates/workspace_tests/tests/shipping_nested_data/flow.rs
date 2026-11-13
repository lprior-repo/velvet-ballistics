//! Live owned admission, cold cut, reopen, recovery, resume, and replay.

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::{RuntimePolicy, SlotValue, StepState, Taint, ValueStore};
use vb_runtime::journal::StorageRuntimeJournal;
use vb_runtime::recovery::RuntimeRecoveryProduct;
use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;
use vb_storage::FjallJournal;
use vb_storage::recovery::replay_owned_for_each;

use crate::support::{
    Observed, Prepared, RUN, TestResult, leaf_step, nested_source, prepare, succeeded, terminal,
};

fn runtime(journal: Arc<FjallJournal>) -> Runtime {
    Runtime::new(
        NonZeroUsize::MIN,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            step_budget_per_tick: 1,
            ..ShardConfig::default()
        },
        StorageRuntimeJournal::shared_strict(journal),
    )
}

/// Runs one owned cell through a nonterminal inner-progress cut, drop, reopen,
/// typed recovery, resume, and owned replay.
pub(crate) fn run_cell(outer: &str, inner: &str) -> TestResult<Observed> {
    let source = nested_source(outer, inner)?;
    run_cell_source(outer, inner, &source)
}

pub(crate) fn run_cell_source(outer: &str, inner: &str, source: &str) -> TestResult<Observed> {
    let prepared = prepare(
        source,
        &[SlotValue::I64(3), SlotValue::I64(5)],
        &[Taint::Secret, Taint::DerivedFromSecret],
    )?;
    let Prepared {
        workflow,
        roots,
        input_store,
        submission,
        source_digest,
        input_slot,
    } = prepared;
    let leaf = leaf_step(&workflow)?;
    let directory = tempfile::tempdir()?;
    let (cut_frame, cut_store, cut_events, cut_completed) = {
        let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
        journal.commit_run_submission(&submission, RuntimePolicy::Strict)?;
        let mut runtime = runtime(journal.clone());
        runtime.activate_owned_submission(RUN, source_digest)?;
        let mut live = None;
        for _ in 0..512 {
            runtime.tick_all()?;
            if runtime.counters_snapshot().runs_failed != 0 {
                return Err(format!("{outer}/{inner}: live run failed").into());
            }
            let state = runtime.inspect_run_state(RUN)?;
            if state.frame.step_state(leaf)? == StepState::Succeeded {
                let completed = runtime.counters_snapshot().runs_completed;
                live = Some((
                    state.frame.checkpoint_v1(),
                    state.store.checkpoint_v1(),
                    completed,
                ));
                break;
            }
        }
        let (frame, store, completed) = live.ok_or("leaf progress never reached")?;
        (frame, store, journal.events_for_run_full(RUN)?, completed)
    };
    if succeeded(&cut_events, leaf) == 0 {
        return Err("cut journal has no inner progress".into());
    }
    let cut_replay = replay_owned_for_each(&cut_events, &workflow, &roots, Some(&input_store))?;
    let cut_replay_selected = cut_replay.is_some();
    if let Some(replay) = cut_replay.as_ref() {
        if replay.frame.checkpoint_v1() != cut_frame {
            return Err(format!("{outer}/{inner}: cut replay frame divergence").into());
        }
        if replay.store != cut_store {
            return Err(format!("{outer}/{inner}: cut replay store divergence").into());
        }
    }
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let mut runtime = runtime(journal.clone());
    match runtime.recover_product(RUN)? {
        RuntimeRecoveryProduct::Resumable(_) => {}
        other => return Err(format!("{outer}/{inner}: recovery not resumable: {other:?}").into()),
    }
    runtime.recover_and_resume(RUN)?;
    for _ in 0..1024 {
        runtime.tick_all()?;
        if runtime.counters_snapshot().runs_failed != 0 {
            return Err(format!("{outer}/{inner}: resumed run failed").into());
        }
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
    }
    if runtime.counters_snapshot().runs_completed != 1 {
        return Err(format!("{outer}/{inner}: resumed run did not complete").into());
    }
    let events = journal.events_for_run_full(RUN)?;
    let (terminal_value, terminal_taint) = terminal(&events)?;
    let full_replay = replay_owned_for_each(&events, &workflow, &roots, Some(&input_store))?;
    let (terminal_items, leaf_terminal) = match full_replay.as_ref() {
        Some(replay) => {
            let store = ValueStore::from_checkpoint_v1(replay.store.clone())?;
            let items = match &terminal_value {
                SlotValue::List(list) => Some(store.list(*list)?.to_vec()),
                _ => None,
            };
            let leaf_slot = workflow.node(leaf).and_then(|node| node.output.slot());
            let leaf_value = leaf_slot
                .map(|slot| {
                    replay
                        .frame
                        .read_slot(slot)
                        .and_then(|value| Ok((*value, replay.frame.read_taint(slot)?)))
                })
                .transpose()?;
            let source = *replay.frame.read_slot(input_slot)?;
            let source_taint = replay.frame.read_taint(input_slot)?;
            let SlotValue::List(source_list) = source else {
                return Err(format!("{outer}/{inner}: source root is not a list").into());
            };
            if store.list(source_list)? != [SlotValue::I64(3), SlotValue::I64(5)] {
                return Err(format!("{outer}/{inner}: source handle mutated").into());
            }
            if source_taint != Taint::Secret {
                return Err(format!("{outer}/{inner}: source taint {source_taint:?}").into());
            }
            if store.list_item_with_taint(source_list, 0)? != (SlotValue::I64(3), Taint::Secret)
                || store.list_item_with_taint(source_list, 1)?
                    != (SlotValue::I64(5), Taint::DerivedFromSecret)
            {
                return Err(format!("{outer}/{inner}: source item taints changed").into());
            }
            (items, leaf_value)
        }
        None => (None, None),
    };
    Ok(Observed {
        terminal_value,
        terminal_taint,
        terminal_items,
        leaf_successes: succeeded(&events, leaf),
        cut_replay_selected,
        full_replay_selected: full_replay.is_some(),
        cut_completed,
        leaf_terminal,
    })
}

/// Attempts a one-over input list and reports the refusing stage, if any.
pub(crate) fn one_over_refusal(outer: &str, inner: &str) -> TestResult<Option<String>> {
    let source = nested_source(outer, inner)?;
    let prepared = match prepare(
        &source,
        &[SlotValue::I64(3), SlotValue::I64(5), SlotValue::I64(7)],
        &[Taint::Clean, Taint::Clean, Taint::Clean],
    ) {
        Ok(prepared) => prepared,
        Err(error) => return Ok(Some(format!("bundle:{error}"))),
    };
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    if let Err(error) = journal.commit_run_submission(&prepared.submission, RuntimePolicy::Strict) {
        return Ok(Some(format!("commit:{error}")));
    }
    let mut runtime = runtime(journal);
    if let Err(error) = runtime.activate_owned_submission(RUN, prepared.source_digest) {
        return Ok(Some(format!("activate:{error}")));
    }
    for _ in 0..16 {
        runtime.tick_all()?;
        if runtime.counters_snapshot().runs_failed != 0 {
            return Ok(Some(String::from("runtime-failed")));
        }
        if runtime.counters_snapshot().runs_completed != 0 {
            break;
        }
    }
    Ok(None)
}
