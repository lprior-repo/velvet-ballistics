use std::sync::Arc;

use vb_core::{RuntimePolicy, SlotValue, Taint};
use vb_storage::FjallJournal;

use super::support::{
    Corruption, collect_extra_count, expected_error, has_collect_start_capture,
    has_none_collect_start_extra, mutate_empty_collect_capture, mutate_first_collect_capture,
    runtime,
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
        Corruption::CurrentPage,
        Corruption::Limit,
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
        let boundary_before = runtime.pending_boundary_snapshot(64);
        match runtime.recover_product(RUN) {
            Err(vb_runtime::RuntimeError::Recovery { error })
                if expected_error(corruption, &error) => {}
            other => {
                return Err(format!("{corruption:?}: unexpected recovery result {other:?}").into());
            }
        }
        if !matches!(
            runtime.inspect_run_state(RUN),
            Err(vb_runtime::RuntimeError::RunNotFound)
        ) {
            return Err(
                format!("{corruption:?}: rejected recovery did not return RunNotFound").into(),
            );
        }
        if runtime.pending_boundary_snapshot(64) != boundary_before {
            return Err(format!("{corruption:?}: rejected recovery changed pending state").into());
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
    let boundary_before = runtime.pending_boundary_snapshot(64);
    match runtime.recover_product(RUN) {
        Err(vb_runtime::RuntimeError::Recovery { error })
            if error == "Collect frame extra differs from replay" => {}
        other => return Err(format!("unexpected recovery result {other:?}").into()),
    }
    if !matches!(
        runtime.inspect_run_state(RUN),
        Err(vb_runtime::RuntimeError::RunNotFound)
    ) {
        return Err("rejected empty Collect extra did not return RunNotFound".into());
    }
    if runtime.pending_boundary_snapshot(64) != boundary_before {
        return Err("rejected empty Collect extra changed pending state".into());
    }
    if reopened.events_for_run_full(RUN)? != before {
        return Err("rejected empty Collect extra mutated the journal".into());
    }
    Ok(())
}

#[test]
fn valid_empty_collect_none_capture_recovers_and_completes() -> TestResult {
    let source = nested_source("collect", "choose")?;
    let prepared = prepare(&source, &[], &[])?;
    let directory = tempfile::tempdir()?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    journal.commit_run_submission(&prepared.submission, RuntimePolicy::Strict)?;
    {
        let mut runtime = runtime(journal.clone());
        runtime.activate_owned_submission(RUN, prepared.source_digest)?;
        for _ in 0..64 {
            runtime.tick_all()?;
            let events = journal.events_for_run_full(RUN)?;
            if has_collect_start_capture(&events, &prepared.workflow) {
                if runtime.counters_snapshot().runs_completed != 0 {
                    return Err("Collect completed before the cold cut".into());
                }
                break;
            }
        }
        if !has_collect_start_capture(&journal.events_for_run_full(RUN)?, &prepared.workflow) {
            return Err("valid empty CollectStart capture was not reached".into());
        }
        if !has_none_collect_start_extra(&journal.events_for_run_full(RUN)?, &prepared.workflow)? {
            return Err("valid empty CollectStart did not record expected None".into());
        }
    }
    drop(journal);

    let reopened = Arc::new(FjallJournal::open(directory.path(), None)?);
    let mut runtime = runtime(reopened.clone());
    match runtime.recover_product(RUN)? {
        vb_runtime::recovery::RuntimeRecoveryProduct::Resumable(_) => {}
        other => return Err(format!("empty Collect recovery was not resumable: {other:?}").into()),
    }
    runtime.recover_and_resume(RUN)?;
    for _ in 0..512 {
        runtime.tick_all()?;
        if runtime.counters_snapshot().runs_completed == 1 {
            break;
        }
        if runtime.counters_snapshot().runs_failed != 0 {
            return Err("valid empty Collect failed after cold recovery".into());
        }
    }
    if runtime.counters_snapshot().runs_completed != 1 {
        return Err("valid empty Collect did not complete after cold recovery".into());
    }
    Ok(())
}
