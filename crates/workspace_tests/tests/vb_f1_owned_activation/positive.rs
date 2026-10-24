use super::support::{Case, TestResult, assert_pristine};
use vb_core::{RuntimePolicy, SlotValue, StepIdx, Taint};
use vb_runtime::RuntimeError;

#[test]
fn actual_public_runtime_preserves_owned_arenas_at_both_real_collect_cuts() -> TestResult {
    for after in [false, true] {
        let case = Case::new(false, RuntimePolicy::Strict)?;
        let journal = case.open()?;
        assert_pristine(&case, &journal)?;
        let mut runtime = case.runtime(journal.clone(), if after { 11 } else { 10 })?;
        runtime.activate_owned_submission(case.run, case.source.digest)?;
        assert_pristine(&case, &journal)?;
        assert!(runtime.tick_all()?);
        super::state::rich_cut(&case, &runtime, after)?;
        super::evidence::execution_prefix(&case, &journal)?;
        assert_eq!(
            journal.events_for_run_full(case.run)?.len(),
            if after { 35 } else { 32 }
        );
        assert_eq!(case.source.source, super::support::SOURCE.as_bytes());
    }
    Ok(())
}

#[test]
fn actual_public_runtime_finishes_with_all_original_items_once() -> TestResult {
    let case = Case::new(false, RuntimePolicy::Strict)?;
    let journal = case.open()?;
    let mut runtime = case.runtime(journal.clone(), 12)?;
    runtime.activate_owned_submission(case.run, case.source.digest)?;
    assert!(runtime.tick_all()?);
    assert!(matches!(
        runtime.inspect_run_state(case.run),
        Err(RuntimeError::RunNotFound)
    ));
    super::evidence::terminal_output(&case, &journal, SlotValue::List(case.root))?;
    let events = journal.events_for_run_full(case.run)?;
    assert_eq!(events.len(), 38);
    assert!(matches!(
        runtime.activate_owned_submission(case.run, case.source.digest),
        Err(RuntimeError::RunAlreadyExists)
    ));
    assert_eq!(journal.events_for_run_full(case.run)?, events);
    let fresh_runtime = case.runtime(journal.clone(), 12)?;
    assert!(
        matches!(fresh_runtime.activate_owned_submission(case.run, case.source.digest),
        Err(RuntimeError::StorageJournalAppend { source }) if matches!(*source, vb_storage::JournalError::TooManyEvents { .. }))
    );
    Ok(())
}

#[test]
fn empty_input_uses_real_empty_start_and_finish_without_extra_finish_allocation() -> TestResult {
    for after in [false, true] {
        let case = Case::new(true, RuntimePolicy::Strict)?;
        let journal = case.open()?;
        let mut runtime = case.runtime(journal.clone(), if after { 2 } else { 1 })?;
        runtime.activate_owned_submission(case.run, case.source.digest)?;
        assert!(runtime.tick_all()?);
        super::state::original_prefix(&case, &runtime)?;
        let state = runtime.inspect_run_state(case.run)?;
        assert_eq!(state.frame.pc(), StepIdx::new(if after { 5 } else { 4 }));
        assert_eq!(state.store.list_count(), 2);
        assert!(state.store.list(case.root)?.is_empty());
        let collector = case.output(0)?;
        let SlotValue::List(empty) = state.frame.read_slot(collector)? else {
            return Err("empty collector is not list".into());
        };
        assert_eq!(*empty, vb_core::ListId::new(1));
        assert!(state.store.list(*empty)?.is_empty());
        assert_eq!(state.frame.read_taint(collector)?, Taint::Secret);
        assert!(state.collect_states.checkpoint_v1().entries.is_empty());
        if after {
            assert_eq!(
                state.frame.read_slot(case.output(4)?)?,
                &SlotValue::List(*empty)
            );
        }
    }
    Ok(())
}

#[test]
fn actual_public_runtime_finishes_empty_collect_as_empty_list() -> TestResult {
    let case = Case::new(true, RuntimePolicy::Strict)?;
    let journal = case.open()?;
    let mut runtime = case.runtime(journal.clone(), 3)?;
    runtime.activate_owned_submission(case.run, case.source.digest)?;
    assert!(runtime.tick_all()?);
    super::evidence::terminal_output(&case, &journal, SlotValue::List(vb_core::ListId::new(1)))?;
    Ok(())
}
