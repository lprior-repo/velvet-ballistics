#![forbid(unsafe_code)]

#[path = "common/workflow_fixture.rs"]
mod workflow_fixture;

use vb_core::action::{ActionTicket, compute_action_idempotency_key};
use vb_core::{
    ActionId, CompiledWorkflow, RunId, SeqNo, SlotIdx, SlotValue, StepIdx, Taint, ValueStore,
    WorkflowDigest,
};
use vb_storage::recovery::{
    RecoveredSlotEntry, RecoveredStepState, RecoveryCannotResumeState, RecoveryFrameSeedProduct,
    recover_runtime_frame_seed_from_events_with_replay_inputs,
};
use vb_storage::{EventSeq, JournalEvent};

type TestResult = Result<(), String>;
const RUN: RunId = RunId::new(29);

fn accepted(workflow: &CompiledWorkflow) -> JournalEvent {
    JournalEvent::RunAccepted {
        run: RUN,
        seq: EventSeq::ZERO,
        workflow: workflow.digest(),
    }
}

fn started(step: StepIdx, seq: u64) -> JournalEvent {
    JournalEvent::StepStarted {
        run: RUN,
        seq: EventSeq::new(seq),
        step,
        attempt: 1,
    }
}

fn succeeded(output: SlotIdx, seq: u64) -> JournalEvent {
    JournalEvent::StepSucceeded {
        run: RUN,
        seq: EventSeq::new(seq),
        step: StepIdx::ZERO,
        output,
    }
}

fn action_scheduled() -> JournalEvent {
    JournalEvent::ActionScheduledTicket {
        run: RUN,
        seq: EventSeq::new(2),
        ticket: action_ticket(),
        input: SlotIdx::ZERO,
        output: SlotIdx::new(1),
        action_abi_digest: WorkflowDigest::from_bytes([74; 32]),
    }
}

fn action_ticket() -> ActionTicket {
    ActionTicket {
        run: RUN,
        step: StepIdx::ZERO,
        seq: SeqNo::ZERO,
        action: ActionId::new(7),
        run_attempt: 1,
        attempt: 1,
        capacity: 2,
        fencing_token: 0,
        idempotency_key: compute_action_idempotency_key(RUN, 1, SeqNo::ZERO, ActionId::new(7)),
    }
}

fn action_completed() -> Result<JournalEvent, String> {
    let value = postcard::to_allocvec(&SlotValue::I64(9)).map_err(|e| e.to_string())?;
    let encoded_len = u32::try_from(value.len()).map_err(|e| e.to_string())?;
    let value_digest = *blake3::hash(&value).as_bytes();
    Ok(JournalEvent::ActionCompletedEnvelope {
        run: RUN,
        seq: EventSeq::new(3),
        ticket: action_ticket(),
        action_abi_digest: WorkflowDigest::from_bytes([74; 32]),
        output: SlotIdx::new(1),
        outcome: vb_storage::DurableActionOutcome::Ready,
        value,
        encoded_len,
        taint: Taint::Clean,
        value_digest,
    })
}

fn durable_output() -> Result<JournalEvent, String> {
    let value = postcard::to_allocvec(&SlotValue::I64(9)).map_err(|e| e.to_string())?;
    let extra =
        vb_storage::encode_slot_written_extra(Taint::Clean, None).map_err(|e| format!("{e:?}"))?;
    Ok(JournalEvent::SlotWrittenEvent {
        run: RUN,
        seq: EventSeq::new(4),
        slot: SlotIdx::new(1),
        value: Some(value),
        extra: Some(extra),
        attempt: 1,
    })
}

fn completed_do_events(plan: &CompiledWorkflow) -> Result<Vec<JournalEvent>, String> {
    Ok(vec![
        accepted(plan),
        started(StepIdx::ZERO, 1),
        action_scheduled(),
        action_completed()?,
        durable_output()?,
        succeeded(SlotIdx::new(1), 5),
    ])
}

fn recover(
    plan: &CompiledWorkflow,
    events: &[JournalEvent],
    roots: &[(SlotIdx, SlotValue, Taint)],
    store: &ValueStore,
) -> Result<(RecoveryFrameSeedProduct, Option<(StepIdx, u64)>), String> {
    recover_runtime_frame_seed_from_events_with_replay_inputs(events, plan, roots, Some(store))
        .map_err(|e| e.to_string())
}

#[test]
fn completed_do_before_target_fallback_has_no_successor_cursor() -> TestResult {
    let plan =
        workflow_fixture::action_then_finish_workflow("completed_do", 71, ActionId::new(7), None)?;
    let events = completed_do_events(&plan)?;
    let roots = [(SlotIdx::ZERO, SlotValue::I64(7), Taint::Secret)];
    let store = ValueStore::new();
    let before = store.checkpoint_v1();
    let (product, cursor) = recover(&plan, &events, &roots, &store)?;
    assert_eq!(cursor, None);
    assert_storage_authority(&product);
    assert_eq!(product.seed().pc, StepIdx::ZERO);
    assert_eq!(product.seed().summary.actions_scheduled, 1);
    assert_eq!(product.seed().summary.actions_resolved, 1);
    assert_completed_slots(&product.seed().slots);
    assert_eq!(store.checkpoint_v1(), before);
    assert_eq!(roots, [(SlotIdx::ZERO, SlotValue::I64(7), Taint::Secret)]);
    Ok(())
}

fn assert_storage_authority(product: &RecoveryFrameSeedProduct) {
    assert_eq!(
        product.cannot_resume_state(),
        RecoveryCannotResumeState {
            workflow_missing: true,
            store_missing: true,
            action_attempts_missing: true,
            admission_missing: true,
            collect_states_missing: true,
            action_contracts_missing: true,
            action_abi_digests_missing: true,
            ..RecoveryCannotResumeState::RESUMABLE
        }
    );
}

fn assert_completed_slots(actual: &[RecoveredSlotEntry]) {
    assert_eq!(
        actual,
        [
            RecoveredSlotEntry {
                slot: SlotIdx::ZERO,
                value: SlotValue::I64(7),
                taint: Taint::Secret
            },
            RecoveredSlotEntry {
                slot: SlotIdx::new(1),
                value: SlotValue::I64(9),
                taint: Taint::Clean
            },
        ]
    );
}

fn deterministic_events(plan: &CompiledWorkflow) -> Vec<JournalEvent> {
    vec![
        accepted(plan),
        started(StepIdx::ZERO, 1),
        succeeded(SlotIdx::ZERO, 2),
    ]
}

#[test]
fn deterministic_through_target_exports_exact_successor_cursor() -> TestResult {
    let plan = workflow_fixture::finished_workflow("deterministic_cursor", 72)?;
    let events = deterministic_events(&plan);
    let roots = [(SlotIdx::ZERO, SlotValue::I64(7), Taint::Secret)];
    let store = ValueStore::new();
    let before = store.checkpoint_v1();
    let (product, cursor) = recover(&plan, &events, &roots, &store)?;
    assert_eq!(cursor, Some((StepIdx::new(1), 1)));
    assert_eq!(product.seed().pc, StepIdx::new(1));
    assert_eq!(
        product.seed().slots,
        [RecoveredSlotEntry {
            slot: SlotIdx::ZERO,
            value: SlotValue::Bool(true),
            taint: Taint::Clean,
        }]
    );
    assert_eq!(store.checkpoint_v1(), before);
    Ok(())
}

#[test]
fn later_journal_boundary_withholds_deterministic_replay_cursor() -> TestResult {
    let plan = workflow_fixture::finished_workflow("later_boundary", 73)?;
    let mut events = deterministic_events(&plan);
    events.push(started(StepIdx::new(1), 3));
    let roots = [(SlotIdx::ZERO, SlotValue::I64(7), Taint::Secret)];
    let store = ValueStore::new();
    let (product, cursor) = recover(&plan, &events, &roots, &store)?;
    assert_eq!(cursor, None);
    assert_eq!(product.seed().pc, StepIdx::new(1));
    assert_eq!(
        product.seed().steps.last().map(|s| s.state),
        Some(RecoveredStepState::Running)
    );
    Ok(())
}
