use super::{goldens, support};
use support::{Case, TestResult};
use vb_core::{ObjectId, SlotIdx, SlotValue, SymbolId, Taint};
use vb_runtime::recovery::RuntimeRecoveryProduct;
use vb_runtime::runtime::Runtime;
use vb_storage::FjallJournal;

#[test]
fn bound_symbol_runtime_cold_owned_input_reopen_executes_has() -> TestResult {
    run("key", &["key", "noise"], 0, true)?;
    run("absent", &["key", "noise", "absent"], 2, false)
}

#[test]
fn bound_symbol_runtime_cold_owned_input_missing_key_reopens_independently() -> TestResult {
    run("absent", &["key", "noise", "absent"], 2, false)
}

fn run(key: &str, symbols: &[&str], expected_key: u32, expected: bool) -> TestResult {
    cut(key, symbols, expected_key, expected, 1)?;
    cut(key, symbols, expected_key, expected, 2)
}

fn cut(key: &str, symbols: &[&str], expected_key: u32, expected: bool, budget: u64) -> TestResult {
    let case = goldens::cold_case(key)?;
    let store = goldens::store(&case, symbols, expected_key);
    let retained = suspend(&case, &store)?;
    let journal = case.open()?;
    let before = support::journal_snapshot(&journal, case.run)?;
    let mut actor = support::runtime(journal.clone(), support::config(budget))?;
    assert_product(&actor, &case)?;
    assert_eq!(support::journal_snapshot(&journal, case.run)?, before);
    assert_eq!(actor.recover_and_resume(case.run), Ok(()));
    assert_eq!(support::journal_snapshot(&journal, case.run)?, before);
    assert_eq!(actor.tick_all(), Ok(true));
    if budget == 1 {
        after_has(&case, &actor, &store, expected)?;
    } else {
        support::finished_has(&case, &mut actor, &journal, expected)?;
    }
    assert_eq!(case.retained_bytes(&journal)?, retained);
    Ok(())
}

fn assert_product(actor: &Runtime, case: &Case) -> TestResult {
    let product = actor.recover_product(case.run)?;
    let RuntimeRecoveryProduct::Resumable(resumable) = product else {
        return Err(format!("expected resumable cold product, found {product:?}").into());
    };
    assert_eq!(resumable.run_id(), case.run);
    Ok(())
}

fn suspend(
    case: &Case,
    store: &vb_core::value_store::ValueStoreCheckpointV1,
) -> TestResult<Vec<u8>> {
    let journal = case.open()?;
    let retained = case.retained_bytes(&journal)?;
    let before = support::journal_snapshot(&journal, case.run)?;
    let mut actor = support::runtime(journal.clone(), support::config(1))?;
    assert_eq!(
        actor.activate_owned_submission(case.run, case.source.digest),
        Ok(())
    );
    assert_eq!(support::journal_snapshot(&journal, case.run)?, before);
    assert_eq!(actor.tick_all(), Ok(true));
    let slots = cold_slots(None);
    goldens::assert_state(case, &actor, store, goldens::frame(case, 1, 1, slots))?;
    symbol_write(&journal, case)?;
    assert_eq!(case.retained_bytes(&journal)?, retained);
    drop(actor);
    drop(journal);
    Ok(retained)
}

fn symbol_write(journal: &FjallJournal, case: &Case) -> TestResult {
    let events = journal.events_for_run_full(case.run)?;
    support::assert_write(
        &events,
        SlotIdx::new(0),
        SlotValue::Symbol(SymbolId::new(1)),
        Taint::Clean,
    )?;
    assert_eq!(
        case.original.roots().last().ok_or("token root missing")?.1,
        SlotValue::Symbol(SymbolId::new(0))
    );
    assert_ne!(
        SlotValue::Symbol(SymbolId::new(1)),
        SlotValue::Symbol(SymbolId::new(0))
    );
    Ok(())
}

fn after_has(
    case: &Case,
    actor: &Runtime,
    store: &vb_core::value_store::ValueStoreCheckpointV1,
    expected: bool,
) -> TestResult {
    let slots = cold_slots(Some(expected));
    goldens::assert_state(case, actor, store, goldens::frame(case, 2, 1, slots))
}

fn cold_slots(value: Option<bool>) -> Vec<Option<SlotValue>> {
    vec![
        Some(SlotValue::Symbol(SymbolId::new(1))),
        value.map(SlotValue::Bool),
        Some(SlotValue::Object(ObjectId::new(0))),
        Some(SlotValue::Symbol(SymbolId::new(1))),
    ]
}

pub(super) fn legacy_roundtrip(case: Case, prompt: SlotValue) -> TestResult {
    let (frame, retained) = legacy_suspend(&case, prompt)?;
    let journal = case.open()?;
    let before = support::journal_snapshot(&journal, case.run)?;
    let mut actor = support::runtime(journal.clone(), support::config(8))?;
    assert_product(&actor, &case)?;
    assert_eq!(support::journal_snapshot(&journal, case.run)?, before);
    assert_eq!(actor.recover_and_resume(case.run), Ok(()));
    assert_eq!(support::journal_snapshot(&journal, case.run)?, before);
    assert_eq!(actor.tick_all(), Ok(true));
    let state = actor.inspect_run_state(case.run)?;
    assert_eq!(state.frame.checkpoint_v1(), legacy_restored_frame(frame));
    assert_eq!(&state.store.checkpoint_v1(), case.original.checkpoint());
    assert_eq!(state.workflow, case.workflow);
    assert_eq!(
        state
            .admission
            .as_ref()
            .ok_or("missing legacy admission")?
            .policy_pin(),
        vb_storage::RunPolicyPin::Pinned(case.artifact.policy_digest)
    );
    assert_eq!(case.retained_bytes(&journal)?, retained);
    legacy_answer(&case, &mut actor, &journal, &retained)
}

fn legacy_restored_frame(
    mut before: vb_core::frame::RunFrameCheckpointV1,
) -> vb_core::frame::RunFrameCheckpointV1 {
    assert_eq!(before.executed, 2);
    assert_eq!(before.max_parallel_in_flight, 0);
    before.executed = 0;
    before.max_parallel_in_flight = u16::MAX;
    before
}

fn legacy_suspend(
    case: &Case,
    prompt: SlotValue,
) -> TestResult<(vb_core::frame::RunFrameCheckpointV1, Vec<u8>)> {
    let journal = case.open()?;
    let retained = case.retained_bytes(&journal)?;
    let mut actor = support::runtime(journal.clone(), support::config(8))?;
    assert_eq!(
        actor.activate_owned_submission(case.run, case.source.digest),
        Ok(())
    );
    assert_eq!(actor.tick_all(), Ok(true));
    let frame = legacy_suspended_frame(&actor, case, prompt)?;
    assert_eq!(case.retained_bytes(&journal)?, retained);
    drop(actor);
    drop(journal);
    Ok((frame, retained))
}

fn legacy_suspended_frame(
    actor: &Runtime,
    case: &Case,
    prompt: SlotValue,
) -> TestResult<vb_core::frame::RunFrameCheckpointV1> {
    let state = actor.inspect_run_state(case.run)?;
    assert_eq!(state.frame.read_slot(SlotIdx::new(0)), Ok(&prompt));
    assert_eq!(&state.store.checkpoint_v1(), case.original.checkpoint());
    assert_eq!(state.workflow.symbol_payloads(), None);
    let frame = state.frame.checkpoint_v1();
    assert_eq!(frame.pc, vb_core::StepIdx::new(1));
    assert_eq!(
        legacy_ticket(&actor, case)?.ask_step(),
        vb_core::StepIdx::new(1)
    );
    Ok(frame)
}

fn legacy_answer(
    case: &Case,
    actor: &mut Runtime,
    journal: &FjallJournal,
    retained: &[u8],
) -> TestResult {
    let ticket = legacy_ticket(actor, case)?;
    let answer = vb_runtime::shard::AskAnswer::new(
        ticket,
        SlotIdx::new(1),
        SlotValue::I64(11),
        Taint::Clean,
    );
    assert_eq!(actor.answer_ask(answer), Ok(()));
    assert_eq!(actor.tick_all(), Ok(true));
    legacy_terminal(journal, case.run)?;
    let before = support::journal_snapshot(journal, case.run)?;
    assert_eq!(actor.tick_all(), Ok(true));
    assert_eq!(support::journal_snapshot(journal, case.run)?, before);
    assert_eq!(case.retained_bytes(journal)?, retained);
    Ok(())
}

fn legacy_terminal(journal: &FjallJournal, run: vb_core::RunId) -> TestResult {
    let events = journal.events_for_run_full(run)?;
    assert_legacy_finish(&events, run);
    let writes: Vec<_> = events
        .iter()
        .filter(|event| {
            matches!(event,
        vb_storage::JournalEvent::SlotWrittenEvent { slot, .. } if *slot == SlotIdx::new(1))
        })
        .collect();
    assert_eq!(writes.len(), 2);
    for event in writes {
        support::assert_write(
            std::slice::from_ref(event),
            SlotIdx::new(1),
            SlotValue::I64(11),
            Taint::Clean,
        )?;
    }
    Ok(())
}

fn assert_legacy_finish(events: &[vb_storage::JournalEvent], run: vb_core::RunId) {
    let terminal: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            vb_storage::JournalEvent::RunFinished {
                run,
                result,
                attempt,
                ..
            } => Some((*run, *result, *attempt)),
            _ => None,
        })
        .collect();
    assert_eq!(terminal, [(run, SlotIdx::new(1), 1)]);
}

fn legacy_ticket(actor: &Runtime, case: &Case) -> TestResult<vb_runtime::shard::AskTicket> {
    actor
        .lookup_pending_ask_ticket(case.run, vb_core::StepIdx::new(1))
        .ok_or_else(|| "legacy Ask ticket missing".into())
}

pub(super) fn raw_boundary() -> TestResult<vb_runtime::recovery::RecoveredRunBoundary> {
    let case = super::retry_support::case(true)?;
    let evidence = super::retry_support::capture_failure(&case)?;
    let command = vb_runtime::shard::types::RecoverRunCommand::for_retry(
        evidence.checkpoint,
        evidence.next_seq,
    )?;
    assert_eq!(
        command.boundary.kind(),
        vb_runtime::recovery::RecoveredRunBoundaryKind::None
    );
    Ok(command.boundary)
}
