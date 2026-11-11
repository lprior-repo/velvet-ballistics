use super::support::{Case, TestResult, compile, config, runtime};
use std::sync::Arc;
use vb_core::action::{
    ActionContract, ActionFailure, ActionFailureCode, ActionName, ActionOutputReady, ActionTicket,
    Idempotency, RetryPolicy, RetrySafety, SideEffect,
};
use vb_core::root_bundle::OwnedRootBundle;
use vb_core::{
    ActionId, CompiledWorkflow, ObjectField, RunId, SlotIdx, SlotValue, StepIdx, SymbolId, Taint,
    ValueStore,
};
use vb_runtime::run_checkpoint::{RunCheckpointV1, decode_checkpoint_payload};
use vb_runtime::runtime::Runtime;
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

pub(super) const ACTION: StepIdx = StepIdx::new(1);
pub(super) const CHECK: StepIdx = StepIdx::new(2);
const INPUT: SlotIdx = SlotIdx::new(3);
const SOURCE: &str = r#"version: velvet-ballistics/v1
name: retry_has
when:
  manual: {}
inputs:
  obj: object
steps:
  - id: before
    set: { output: seed, value: '0' }
  - id: act
    do: { action: '0', input: '$before' }
  - id: check
    set: { output: result, value: 'has($input.obj, "key")' }
  - id: done
    finish: { result: '$check' }
"#;

pub(super) struct RetryEvidence {
    pub(super) checkpoint: RunCheckpointV1,
    pub(super) next_seq: EventSeq,
    pub(super) events: Vec<JournalEvent>,
}

pub(super) fn case(present: bool) -> TestResult<Case> {
    let workflow = compile(SOURCE)?;
    assert_source_shape(&workflow)?;
    let original = original(&workflow, present)?;
    Case::persist(
        workflow,
        SOURCE,
        original,
        RunId::new(425_401),
        &[contract()?],
    )
}

fn assert_source_shape(workflow: &CompiledWorkflow) -> TestResult {
    let expected: Box<[Box<str>]> = Box::from([Box::from("key")]);
    assert_eq!(workflow.symbol_payloads(), Some(expected.as_ref()));
    assert_eq!(workflow.input_slots().len(), 1);
    assert_eq!(
        workflow.input_slots().first().ok_or("input missing")?.slot,
        INPUT
    );
    assert!(matches!(workflow.node(ACTION).ok_or("Do missing")?.kind,
        vb_core::CompiledNodeKind::Do { action, input }
        if action == ActionId::new(0) && input == SlotIdx::ZERO));
    Ok(())
}

fn original(workflow: &CompiledWorkflow, present: bool) -> TestResult<OwnedRootBundle> {
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    let noise = store.insert_symbol("noise")?;
    let key = store.insert_symbol(if present { "key" } else { "absent" })?;
    let object = store.insert_object(Box::from([ObjectField::clean(
        key,
        SlotValue::Symbol(noise),
    )]))?;
    OwnedRootBundle::new(
        Box::from([(INPUT, SlotValue::Object(object), Taint::Clean)]),
        store.checkpoint_v1(),
        workflow,
    )
    .map_err(Into::into)
}

fn contract() -> TestResult<ActionContract> {
    Ok(ActionContract {
        id: ActionId::new(0),
        name: ActionName::new("test-action")?,
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::default(),
    })
}

fn failure() -> ActionFailure {
    ActionFailure {
        code: ActionFailureCode::Rejected,
        retry_policy: RetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    }
}

pub(super) fn strict_runtime(journal: Arc<FjallJournal>) -> TestResult<Runtime> {
    let mut settings = config(64);
    settings.policy = vb_core::RuntimePolicy::Strict;
    runtime(journal, settings)
}

pub(super) fn pending_ticket(runtime: &Runtime, run: RunId) -> TestResult<ActionTicket> {
    runtime
        .lookup_pending_action_ticket(run, u64::from(ACTION.get()))
        .ok_or_else(|| "canonical retry action ticket missing".into())
}

pub(super) fn capture_failure(case: &Case) -> TestResult<RetryEvidence> {
    let journal = case.open()?;
    let mut runtime = strict_runtime(Arc::clone(&journal))?;
    runtime.activate_owned_submission(case.run, case.source.digest)?;
    assert_eq!(runtime.tick_all()?, true);
    let frame = runtime.inspect_run_state(case.run)?.frame.checkpoint_v1();
    runtime.fail_action(pending_ticket(&runtime, case.run)?, failure())?;
    assert_eq!(runtime.tick_all()?, true);
    assert_eq!(runtime.counters_snapshot().runs_failed, 1);
    let events = journal.events_for_run_full(case.run)?;
    let checkpoint = latest_checkpoint(case, &events)?;
    super::retry::assert_failure_frame(&checkpoint, frame)?;
    let next_seq = next_sequence(&events)?;
    Ok(RetryEvidence {
        checkpoint,
        next_seq,
        events,
    })
}

fn latest_checkpoint(case: &Case, events: &[JournalEvent]) -> TestResult<RunCheckpointV1> {
    let JournalEvent::RetryCheckpoint {
        run,
        attempt,
        step,
        payload,
        payload_digest,
        ..
    } = events.last().ok_or("journal has no checkpoint head")?
    else {
        return Err("journal head is not RetryCheckpoint".into());
    };
    assert_eq!(*run, case.run);
    assert_eq!(*attempt, 2);
    assert_eq!(*step, ACTION);
    assert_eq!(*payload_digest, *blake3::hash(payload).as_bytes());
    let checkpoint = decode_checkpoint_payload(payload)?;
    assert_eq!(checkpoint.run_attempt.get(), 2);
    assert_eq!(checkpoint.retry_step, ACTION);
    assert_eq!(checkpoint.artifact_digest, case.artifact.digest);
    assert_eq!(checkpoint.workflow_digest, case.workflow.digest());
    assert_preceding_failure(case, events, &checkpoint)?;
    Ok(checkpoint)
}

fn assert_preceding_failure(
    case: &Case,
    events: &[JournalEvent],
    checkpoint: &RunCheckpointV1,
) -> TestResult {
    let prior = events
        .iter()
        .rev()
        .nth(1)
        .ok_or("preceding failure missing")?;
    assert!(
        matches!(prior, JournalEvent::RunFailedEvent { run, attempt: 1, .. } if *run == case.run)
    );
    assert_eq!(prior.seq().get(), checkpoint.prior_seq);
    let head = events.last().ok_or("checkpoint head missing")?.seq().get();
    assert_eq!(checkpoint.prior_seq.checked_add(1), Some(head));
    Ok(())
}

fn next_sequence(events: &[JournalEvent]) -> TestResult<EventSeq> {
    let last = events.last().ok_or("retry journal empty")?.seq().get();
    Ok(EventSeq::new(
        last.checked_add(1).ok_or("journal sequence overflow")?,
    ))
}

pub(super) fn action_output(case: &Case) -> TestResult<ActionOutputReady> {
    let output = case
        .workflow
        .node(ACTION)
        .ok_or("Do node missing")?
        .output
        .slot()
        .ok_or("Do output missing")?;
    let value = SlotValue::I64(42);
    Ok(ActionOutputReady {
        output_slot: output,
        value,
        taint: Taint::Clean,
        encoded_len: u32::try_from(postcard::to_allocvec(&value)?.len())?,
    })
}

pub(super) fn expected_store(
    case: &Case,
    present: bool,
) -> vb_core::value_store::ValueStoreCheckpointV1 {
    let mut expected = case.original.checkpoint().clone();
    expected.symbols = if present {
        vec![Box::from("key"), Box::from("noise")]
    } else {
        vec![Box::from("key"), Box::from("noise"), Box::from("absent")]
    };
    let key = if present {
        SymbolId::new(0)
    } else {
        SymbolId::new(2)
    };
    expected.objects = vec![Box::from([ObjectField::clean(
        key,
        SlotValue::Symbol(SymbolId::new(1)),
    )])];
    expected
}

pub(super) fn duplicate_tail(
    case: &Case,
    original: &RunCheckpointV1,
) -> TestResult<RunCheckpointV1> {
    let mut changed = original.clone();
    let tail = SymbolId::new(u32::try_from(changed.value_store.symbols.len())?);
    changed.value_store.symbols.push(Box::from("key"));
    changed
        .value_store
        .objects
        .first_mut()
        .ok_or("object missing")?
        .first_mut()
        .ok_or("field missing")?
        .key = tail;
    assert_eq!(
        changed.validate_against(
            &case.workflow,
            case.artifact.digest,
            &case.artifact.action_contracts,
        ),
        Ok(())
    );
    Ok(changed)
}
