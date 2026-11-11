use super::{goldens, legacy_authority, support};
use support::{Case, TestResult};
use vb_core::{BlobId, CapabilitySet, CoreError, ListId, ObjectId, SlotIdx, SlotValue};
use vb_core::{SymbolId, WorkflowDigest};
use vb_runtime::RuntimeError;
use vb_runtime::runtime::Runtime;
use vb_storage::{FjallJournal, JournalError, OwnedInputError};

#[derive(Debug, PartialEq, Eq)]
struct Publication {
    boundary: vb_runtime::runtime::RuntimePendingBoundarySnapshot,
    counters: vb_runtime::counters::CounterSnapshot,
    journal: support::JournalSnapshot,
    retained: Vec<u8>,
}

fn snapshot(case: &Case, actor: &Runtime, journal: &FjallJournal) -> TestResult<Publication> {
    Ok(Publication {
        boundary: actor.pending_boundary_snapshot(16),
        counters: actor.counters_snapshot(),
        journal: support::journal_snapshot(journal, case.run)?,
        retained: case.retained_bytes(journal)?,
    })
}

#[test]
fn bound_symbol_runtime_composition_failure_has_no_publication() -> TestResult {
    let case = legacy_authority::owned_case(&["key", "prefix-only"], 3)?;
    let journal = case.open()?;
    let mut actor = support::runtime(journal.clone(), support::config(1))?;
    let before = snapshot(&case, &actor, &journal)?;
    assert_eq!(
        actor.activate_owned_submission(case.run, case.source.digest),
        Ok(())
    );
    assert_eq!(actor.tick_all(), Err(binding_capacity_error()));
    assert_eq!(snapshot(&case, &actor, &journal)?, before);
    assert_eq!(
        actor.inspect_run_state(case.run).map(|_| ()),
        Err(RuntimeError::RunNotFound)
    );
    Ok(())
}

fn binding_capacity_error() -> RuntimeError {
    vb_core::value_store::InputSymbolBindingError::Core(CoreError::BudgetExceeded {
        budget: "max_slots",
        limit: 3,
    })
    .into()
}

#[test]
fn bound_symbol_runtime_queued_activation_rechecks_state() -> TestResult {
    let case = goldens::fresh_case("key")?;
    let journal = case.open()?;
    let mut actor = support::runtime(journal.clone(), support::config(1))?;
    assert_eq!(
        actor.activate_owned_submission(case.run, case.source.digest),
        Ok(())
    );
    assert_eq!(
        actor.activate_owned_submission(case.run, case.source.digest),
        Ok(())
    );
    assert_eq!(actor.tick_all(), Ok(true));
    let state = actor.inspect_run_state(case.run)?.clone();
    let counters = actor.counters_snapshot();
    let events = support::journal_snapshot(&journal, case.run)?;
    let retained = case.retained_bytes(&journal)?;
    assert_eq!(actor.tick_all(), Err(RuntimeError::RunAlreadyExists));
    assert_eq!(actor.inspect_run_state(case.run)?, &state);
    assert_eq!(actor.counters_snapshot(), counters);
    assert_eq!(support::journal_snapshot(&journal, case.run)?, events);
    assert_eq!(case.retained_bytes(&journal)?, retained);
    Ok(())
}

#[test]
fn bound_symbol_runtime_existing_failure_precedence() -> TestResult {
    let case = legacy_authority::owned_case(&["key", "prefix-only"], 3)?;
    let journal = case.open()?;
    let mut config = support::config(1);
    config.max_active_runs = 0;
    let actor = support::runtime(journal.clone(), config)?;
    let before = snapshot(&case, &actor, &journal)?;
    assert_eq!(
        actor.activate_owned_submission(case.run, case.source.digest),
        Err(RuntimeError::ActiveRunCapacityExceeded { capacity: 0 })
    );
    assert_eq!(snapshot(&case, &actor, &journal)?, before);
    duplicate_before_bad_source()
}

fn duplicate_before_bad_source() -> TestResult {
    let case = goldens::fresh_case("key")?;
    let journal = case.open()?;
    let mut actor = support::runtime(journal.clone(), support::config(1))?;
    assert_eq!(
        actor.activate_owned_submission(case.run, case.source.digest),
        Ok(())
    );
    assert_eq!(actor.tick_all(), Ok(true));
    let state = actor.inspect_run_state(case.run)?.clone();
    let before = snapshot(&case, &actor, &journal)?;
    assert_eq!(
        actor.activate_owned_submission(case.run, WorkflowDigest::from_bytes([99; 32])),
        Err(RuntimeError::RunAlreadyExists)
    );
    assert_eq!(snapshot(&case, &actor, &journal)?, before);
    assert_eq!(actor.inspect_run_state(case.run)?, &state);
    Ok(())
}

#[test]
fn bound_symbol_runtime_source_binding_has_no_reparse_fallback() -> TestResult {
    let case = goldens::fresh_case("key")?;
    let journal = case.open()?;
    let actor = support::runtime(journal.clone(), support::config(1))?;
    let before = snapshot(&case, &actor, &journal)?;
    assert_eq!(
        actor.activate_owned_submission(case.run, WorkflowDigest::from_bytes([99; 32])),
        Err(RuntimeError::RunNotFound)
    );
    assert_eq!(snapshot(&case, &actor, &journal)?, before);
    let mut header = journal.run_header(case.run)?.ok_or("missing header")?;
    header.compiled_digest = WorkflowDigest::from_bytes([98; 32]);
    journal.put_run_header(&header)?;
    let poisoned = snapshot(&case, &actor, &journal)?;
    assert_eq!(
        actor.activate_owned_submission(case.run, case.source.digest),
        Err(RuntimeError::from(JournalError::OwnedInputs(
            OwnedInputError::IdentityMismatch
        )))
    );
    assert_eq!(snapshot(&case, &actor, &journal)?, poisoned);
    assert_eq!(journal.run_header(case.run)?, Some(header));
    Ok(())
}

#[test]
fn bound_symbol_runtime_storeless_v2_handles_rejected() -> TestResult {
    for value in [
        SlotValue::Symbol(SymbolId::new(0)),
        SlotValue::List(ListId::new(0)),
        SlotValue::Object(ObjectId::new(0)),
        SlotValue::Blob(BlobId::new(0)),
    ] {
        storeless(value)?;
    }
    super::fresh::direct_scalar(&[])
}

fn storeless(value: SlotValue) -> TestResult {
    let case = legacy_authority::owned_case(&[], 8)?;
    let journal = case.open()?;
    let mut actor = support::runtime(journal.clone(), support::config(1))?;
    let before = snapshot(&case, &actor, &journal)?;
    submit_storeless(&actor, &case, value)?;
    assert_eq!(
        actor.tick_all(),
        Err(RuntimeError::UnsupportedOperation {
            operation: "bound_input_requires_owned_store",
        })
    );
    assert_eq!(snapshot(&case, &actor, &journal)?, before);
    assert_eq!(
        support::journal_snapshot(&journal, vb_core::RunId::new(19))?,
        (vec![], vec![0])
    );
    Ok(())
}

fn submit_storeless(actor: &Runtime, case: &Case, value: SlotValue) -> TestResult {
    assert_eq!(
        actor.submit_direct_with_inputs_grants_and_contracts(
            vb_core::RunId::new(19),
            case.workflow.clone(),
            Box::from([(SlotIdx::new(1), value)]),
            CapabilitySet::empty(),
            Box::default()
        ),
        Ok(())
    );
    Ok(())
}

pub(super) fn raw_scalar_authority(
    boundary: vb_runtime::recovery::RecoveredRunBoundary,
) -> TestResult {
    let case = legacy_authority::owned_case(&[], 8)?;
    let journal = case.open()?;
    let backend = vb_runtime::journal::StorageRuntimeJournal::shared_strict(journal.clone());
    let mut shard = vb_runtime::shard::Shard::new_with_journal(support::config(1), backend);
    let command = raw_scalar_command(&case, boundary)?;
    legacy_authority::enqueue_recovery(&shard, command)?;
    assert_eq!(shard.tick(), Ok(true));
    assert_raw_scalar_state(&shard, &case)
}

fn raw_scalar_command(
    case: &Case,
    boundary: vb_runtime::recovery::RecoveredRunBoundary,
) -> TestResult<vb_runtime::shard::types::RecoverRunCommand> {
    let mut frame = vb_core::RunFrame::new(case.run, vb_core::StepIdx::new(0), 2, 2)?;
    frame.write_slot(SlotIdx::new(1), SlotValue::I64(7))?;
    Ok(vb_runtime::shard::types::RecoverRunCommand::new(
        case.run,
        frame,
        case.artifact.digest,
        case.workflow.digest(),
        vb_storage::EventSeq::new(3),
        vb_runtime::primitives::collect::CollectStates::new(),
        boundary,
    ))
}

fn assert_raw_scalar_state(shard: &vb_runtime::shard::Shard, case: &Case) -> TestResult {
    let state = shard
        .run_state_get(case.run)
        .ok_or("raw scalar run missing")?;
    assert_eq!(
        state
            .admission
            .as_ref()
            .ok_or("raw admission missing")?
            .policy_pin(),
        vb_storage::RunPolicyPin::Unverified
    );
    assert_eq!(
        state.frame.read_slot(SlotIdx::new(1)),
        Ok(&SlotValue::I64(7))
    );
    assert_eq!(
        state.store.checkpoint_v1(),
        vb_core::ValueStore::with_max_slots(8).checkpoint_v1()
    );
    Ok(())
}
