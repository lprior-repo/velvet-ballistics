use std::sync::Arc;

use vb_core::{RuntimePolicy, SlotValue, Taint};
use vb_storage::FjallJournal;

use super::support::{
    Corruption, collect_extra_count, expected_error, has_collect_start_capture,
    mutate_empty_collect_capture, mutate_first_collect_capture, runtime,
};
use crate::support::{RUN, TestResult, nested_source, prepare};

#[test]
fn public_cold_recovery_rejects_collect_extras_that_diverge_from_replay() -> TestResult {
    let source = nested_source("collect", "choose")?;
    let prepared = prepare(
        &source,
        &[SlotValue::I64(3), SlotValue::I64(5)],
        &[Taint::Secret, Taint::DerivedFromSecret],
    )?;
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    journal.commit_run_submission(&prepared.submission, RuntimePolicy::Strict)?;
    let initial_events = journal.events_for_run_full(RUN)?;
    let cut_events = {
        let mut runtime = runtime(journal.clone());
        runtime.activate_owned_submission(RUN, prepared.source_digest)?;
        let mut cut = None;
        for _ in 0..512 {
            runtime.tick_all()?;
            if runtime.counters_snapshot().runs_failed != 0 {
                return Err("valid Collect trace failed before cut".into());
            }
            let events = journal.events_for_run_full(RUN)?;
            if collect_extra_count(&events) >= 2 {
                cut = Some(events);
                break;
            }
        }
        if runtime.counters_snapshot().runs_completed != 0 {
            return Err("Collect cut unexpectedly completed".into());
        }
        cut.ok_or("two paginated Collect captures were not reached")?
    };
    drop(journal);
    let suffix = cut_events
        .get(initial_events.len()..)
        .ok_or("submission prefix longer than the captured event list")?;
    let corruptions = [
        Corruption::Cursor,
        Corruption::PageSize,
        Corruption::ItemCount,
        Corruption::Source,
        Corruption::Taint,
        Corruption::MissingFrameExtra,
    ];
    for corruption in corruptions {
        let corrupt_dir = tempfile::tempdir()?;
        let corrupt_journal = Arc::new(FjallJournal::open(corrupt_dir.path(), None)?);
        corrupt_journal.commit_run_submission(&prepared.submission, RuntimePolicy::Strict)?;
        let mut corrupt_suffix = suffix.to_vec();
        if !mutate_first_collect_capture(&mut corrupt_suffix, corruption)? {
            return Err("no Collect frame extra was mutated".into());
        }
        corrupt_journal.append_strict_batch(&corrupt_suffix)?;
        let durable_before = corrupt_journal.events_for_run_full(RUN)?;
        drop(corrupt_journal);
        let reopened = Arc::new(FjallJournal::open(corrupt_dir.path(), None)?);
        let runtime = runtime(reopened.clone());
        match runtime.recover_product(RUN) {
            Err(vb_runtime::RuntimeError::Recovery { error })
                if expected_error(corruption, &error) => {}
            other => {
                return Err(format!("{corruption:?}: unexpected recovery result {other:?}").into());
            }
        }
        if runtime.inspect_run_state(RUN).is_ok() {
            return Err(format!("{corruption:?}: rejected recovery installed run state").into());
        }
        if reopened.events_for_run_full(RUN)? != durable_before {
            return Err(format!("{corruption:?}: rejected recovery mutated journal").into());
        }
    }
    Ok(())
}

#[test]
fn public_cold_recovery_rejects_extra_when_collect_replay_has_no_state() -> TestResult {
    let source = nested_source("collect", "choose")?;
    let prepared = prepare(&source, &[], &[])?;
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    journal.commit_run_submission(&prepared.submission, RuntimePolicy::Strict)?;
    let initial_events = journal.events_for_run_full(RUN)?;
    let cut_events = {
        let mut runtime = runtime(journal.clone());
        runtime.activate_owned_submission(RUN, prepared.source_digest)?;
        let mut cut = None;
        for _ in 0..64 {
            runtime.tick_all()?;
            let events = journal.events_for_run_full(RUN)?;
            if has_collect_start_capture(&events, &prepared.workflow) {
                cut = Some(events);
                break;
            }
        }
        if runtime.counters_snapshot().runs_completed != 0 {
            return Err("empty Collect cut unexpectedly completed".into());
        }
        cut.ok_or("empty CollectStart capture was not reached")?
    };
    drop(journal);
    let mut suffix = cut_events
        .get(initial_events.len()..)
        .ok_or("submission prefix longer than the captured event list")?
        .to_vec();
    if !mutate_empty_collect_capture(&mut suffix, &prepared.workflow)? {
        return Err("empty Collect frame extra was not added".into());
    }
    let corrupt_dir = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(corrupt_dir.path(), None)?);
    journal.commit_run_submission(&prepared.submission, RuntimePolicy::Strict)?;
    journal.append_strict_batch(&suffix)?;
    let before = journal.events_for_run_full(RUN)?;
    drop(journal);
    let reopened = Arc::new(FjallJournal::open(corrupt_dir.path(), None)?);
    let runtime = runtime(reopened.clone());
    match runtime.recover_product(RUN) {
        Err(vb_runtime::RuntimeError::Recovery { error })
            if error == "Collect frame extra differs from replay" => {}
        other => return Err(format!("unexpected recovery result {other:?}").into()),
    }
    if runtime.inspect_run_state(RUN).is_ok() || reopened.events_for_run_full(RUN)? != before {
        return Err("rejected empty Collect extra mutated recovery state".into());
    }
    Ok(())
}
