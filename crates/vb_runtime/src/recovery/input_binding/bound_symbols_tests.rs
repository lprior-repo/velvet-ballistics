use super::*;
use vb_core::value_store::InputSymbolBindingError;
use vb_core::{CoreError, SymbolId};

mod fixtures;
use fixtures::*;

pub(super) fn assert_durable_unchanged(
    case: &DurableCase,
    journal: &FjallJournal,
    events: &[JournalEvent],
) -> TestResult {
    assert_eq!(journal.events_for_run_full(RUN)?, events);
    let record = journal
        .owned_run_inputs(RUN, case.artifact.digest, source().digest, &case.workflow)?
        .ok_or("retained owned record missing after recovery")?;
    assert_eq!(
        postcard::to_allocvec(&record)?,
        postcard::to_allocvec(&case.record)?
    );
    Ok(())
}

fn assert_reopened(key: &str, key_id: SymbolId) -> TestResult {
    let case = durable_case(Some(&["key"]), key, 8)?;
    let journal = FjallJournal::open(case.directory.path(), None)?;
    let events = journal.events_for_run_full(RUN)?;
    let restored = load(&case, &journal)?;
    assert_eq!(restored.initial_inputs(), roots().as_ref());
    assert_eq!(restored.store_checkpoint(), Some(expected(key, key_id, 8)));
    let again = load(&case, &journal)?;
    assert_eq!(again.initial_inputs(), restored.initial_inputs());
    assert_eq!(again.store_checkpoint(), restored.store_checkpoint());
    assert_durable_unchanged(&case, &journal, &events)
}

#[test]
fn cold_retained_symbols_reopen_present_key_remaps_complete_store() -> TestResult {
    assert_reopened("key", SymbolId::new(0))
}

#[test]
fn cold_retained_symbols_reopen_missing_key_retains_distinct_id() -> TestResult {
    assert_reopened("absent", SymbolId::new(2))
}

#[test]
fn cold_retained_symbols_reopen_capacity_failure_preserves_durable_inputs() -> TestResult {
    let case = durable_case(Some(&["key", "prefix-only"]), "key", 3)?;
    let journal = FjallJournal::open(case.directory.path(), None)?;
    let events = journal.events_for_run_full(RUN)?;
    let cause = InputSymbolBindingError::Core(CoreError::BudgetExceeded {
        budget: "max_slots",
        limit: 3,
    });
    assert_typed_capacity(&case, &cause)?;
    match load(&case, &journal) {
        Err(error) => assert_eq!(
            error,
            RuntimeError::Recovery {
                error: format!("bound input symbol composition failed: {cause:?}"),
            }
        ),
        Ok(_) => return Err("cold capacity rejection was lost".into()),
    }
    assert_durable_unchanged(&case, &journal, &events)
}

fn assert_typed_capacity(case: &DurableCase, cause: &InputSymbolBindingError) -> TestResult {
    assert_eq!(
        crate::prepared_inputs::prepare_bound_runtime_inputs(
            &case.workflow,
            case.record.checkpoint(),
            case.record.roots(),
        ),
        Err(cause.clone())
    );
    Ok(())
}

#[test]
fn cold_retained_symbols_reopen_v1_preserves_duplicate_identity() -> TestResult {
    let case = durable_case(None, "noise", 8)?;
    let journal = FjallJournal::open(case.directory.path(), None)?;
    let events = journal.events_for_run_full(RUN)?;
    let restored = load(&case, &journal)?;
    assert_eq!(restored.initial_inputs(), roots().as_ref());
    assert_eq!(restored.store_checkpoint(), Some(checkpoint("noise", 8)));
    assert_durable_unchanged(&case, &journal, &events)
}

#[test]
fn cold_retained_symbols_scalar_prefix_keeps_prepared_store() -> TestResult {
    let workflow = workflow(Some(&["key"]), 8)?;
    let roots = vec![(SlotIdx::ZERO, SlotValue::I64(7), Taint::Clean)];
    let restored = preparation::prepare_scalar_roots(&workflow, roots.clone())?;
    let mut expected =
        ValueStore::with_max_slots(workflow.resource_contract().max_slots).checkpoint_v1();
    expected.symbols = vec![Box::from("key")];
    assert_eq!(restored.initial_inputs(), roots);
    assert_eq!(restored.store_checkpoint(), Some(expected));
    Ok(())
}

#[test]
fn cold_retained_symbols_scalar_empty_binding_is_prepared() -> TestResult {
    let workflow = workflow(Some(&[]), 8)?;
    let restored = preparation::prepare_scalar_roots(&workflow, Vec::new())?;
    assert_eq!(restored.initial_inputs(), &[]);
    assert_eq!(
        restored.store_checkpoint(),
        Some(ValueStore::with_max_slots(workflow.resource_contract().max_slots).checkpoint_v1(),)
    );
    Ok(())
}

#[test]
fn cold_retained_symbols_scalar_v1_keeps_absent_store() -> TestResult {
    let workflow = workflow(None, 8)?;
    let roots = vec![(SlotIdx::ZERO, SlotValue::I64(7), Taint::Clean)];
    let restored = preparation::prepare_scalar_roots(&workflow, roots.clone())?;
    assert_eq!(restored.initial_inputs(), roots);
    assert_eq!(restored.store_checkpoint(), None);
    Ok(())
}

#[test]
fn cold_retained_symbols_reopen_keeps_nested_arena_ids_and_taints() -> TestResult {
    let case = durable_checkpoint_case(Some(&["key"]), rich_input())?;
    let journal = FjallJournal::open(case.directory.path(), None)?;
    let events = journal.events_for_run_full(RUN)?;
    let restored = load(&case, &journal)?;
    assert_eq!(restored.initial_inputs(), roots().as_ref());
    assert_eq!(restored.store_checkpoint(), Some(rich_expected()));
    assert_eq!(case.record.checkpoint(), &rich_input());
    assert_durable_unchanged(&case, &journal, &events)
}

#[test]
fn cold_retained_symbols_hydration_keeps_runtime_journal_symbol_id() -> TestResult {
    let case = durable_case(Some(&["key"]), "key", 8)?;
    persist_runtime_write(case.directory.path())?;
    let journal = FjallJournal::open(case.directory.path(), None)?;
    let events = journal.events_for_run_full(RUN)?;
    let restored = load(&case, &journal)?;
    let seed = vb_storage::recovery::recover_runtime_frame_seed_from_events_with_workflow(
        &events,
        &case.workflow,
    )?;
    let frame = crate::recovery::frame::hydrate_shape_checked_run_frame_with_inputs(
        seed.seed(),
        restored.initial_inputs().to_vec(),
    )?;
    assert_eq!(
        frame.read_slot(SlotIdx::ZERO)?,
        &SlotValue::Symbol(SymbolId::new(1))
    );
    assert_eq!(frame.read_taint(SlotIdx::ZERO)?, Taint::DerivedFromSecret);
    assert_eq!(
        restored.store_checkpoint(),
        Some(expected("key", SymbolId::new(0), 8))
    );
    assert_durable_unchanged(&case, &journal, &events)
}

#[test]
fn cold_retained_symbols_run_mismatch_precedes_capacity_composition() -> TestResult {
    let case = durable_case(Some(&["key", "prefix-only"]), "key", 3)?;
    let journal = FjallJournal::open(case.directory.path(), None)?;
    let events = journal.events_for_run_full(RUN)?;
    let admission = events
        .iter()
        .find(|event| matches!(event, JournalEvent::RunAdmission { .. }))
        .ok_or("persisted admission missing")?;
    let wrong = RunId::new(42_523);
    match owned_retained_inputs(&case.record, &case.workflow, wrong, admission) {
        Err(error) => assert_eq!(
            error,
            RuntimeError::Recovery {
                error: format!(
                    "recovery: retained owned input run mismatch: expected {wrong:?}, got {RUN:?}"
                ),
            }
        ),
        Ok(_) => return Err("run mismatch was lost".into()),
    }
    assert_durable_unchanged(&case, &journal, &events)
}
