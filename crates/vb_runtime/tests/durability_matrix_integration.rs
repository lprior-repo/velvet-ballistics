#![forbid(unsafe_code)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::expect_used,
    clippy::get_first,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]
//! Integration tests for durability matrix: handler persistence-before-ack.

use std::sync::Arc;
use vb_core::action::{ActionContract, ActionName, Idempotency, RetrySafety, SideEffect};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};
use vb_runtime::journal::{RuntimeJournalEvent, VolatileRuntimeJournal};
use vb_runtime::shard::{Shard, ShardCommand, ShardConfig};

fn contract_required_capability(action: ActionId) -> Capability {
    Capability::new("__contract_required__".into(), action)
}

fn suspended_action_contracts() -> Box<[ActionContract]> {
    let action = ActionId::new(0);
    Box::from([ActionContract {
        id: action,
        name: ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::None,
        retry_safety: RetrySafety::Safe,
        required_capabilities: Box::from([contract_required_capability(action)]),
    }])
}

fn submit_suspended(shard: &Shard, run: RunId, workflow: vb_core::workflow::CompiledWorkflow) {
    let action = ActionId::new(0);
    shard
        .enqueue(ShardCommand::SubmitWithInputsAndContracts {
            run,
            workflow,
            inputs: Box::from([(SlotIdx::new(0), SlotValue::Bool(false))]),
            caps: CapabilitySet::from_grants(Box::from([contract_required_capability(action)])),
            action_contracts: suspended_action_contracts(),
        })
        .unwrap();
}

// ---------------------------------------------------------------------------
// Workflow fixtures
// ---------------------------------------------------------------------------

fn finished_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_const = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: vb_core::ids::ConstIdx::new(0),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("finished"),
        digest: WorkflowDigest::from_bytes([2; 32]),
        nodes: Box::from([set_const, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn suspended_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: vb_core::ids::ActionId::new(0),
            input: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("suspended"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn timed_wait_then_finish_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_deadline = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: vb_core::ids::ConstIdx::new(0),
        },
    };
    let wait = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: Some(StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::WaitUntil {
            deadline_slot: SlotIdx::ZERO,
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(2),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let parts = WorkflowParts {
        name: Box::from("timed_wait_then_finish"),
        digest: WorkflowDigest::from_bytes([4; 32]),
        nodes: Box::from([set_deadline, wait, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([vb_core::value::ConstValue::I64(10)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn timed_ask_without_answer_workflow() -> Option<vb_core::workflow::CompiledWorkflow> {
    let set_prompt = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: vb_core::ids::ConstIdx::new(0),
        },
    };
    let set_timeout = CompiledNode {
        id: StepIdx::new(1),
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(2)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: vb_core::ids::ConstIdx::new(1),
        },
    };
    let ask = CompiledNode {
        id: StepIdx::new(2),
        output: None,
        next: Some(StepIdx::new(3)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Ask {
            prompt: SlotIdx::ZERO,
            timeout_slot: Some(SlotIdx::new(1)),
        },
    };
    let resume = CompiledNode {
        id: StepIdx::new(3),
        output: None,
        next: Some(StepIdx::new(4)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::AskResume {
            answer: SlotIdx::new(2),
        },
    };
    let finish = CompiledNode {
        id: StepIdx::new(4),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(2),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("ask_then_finish"),
        digest: WorkflowDigest::from_bytes([7; 32]),
        nodes: Box::from([set_prompt, set_timeout, ask, resume, finish]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([
            vb_core::value::ConstValue::Symbol(vb_core::ids::SymbolId::new(1)),
            vb_core::value::ConstValue::I64(10),
        ]),
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        step_names: Box::from([]),
        resource_contract: ResourceContract::DEFAULT,
    };
    vb_core::workflow::CompiledWorkflow::try_from_parts(parts).ok()
}

fn small_config() -> ShardConfig {
    ShardConfig {
        command_queue_capacity: 16,
        trace_capacity: 16,
        step_budget_per_tick: 4,
        max_active_runs: 4,
        policy: vb_core::policy::RuntimePolicy::Relaxed,
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Persistence-before-ack tests
// ---------------------------------------------------------------------------

#[test]
fn submit_handler_persists_before_ack() {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(small_config(), journal.clone());
    let Some(workflow) = finished_workflow() else {
        return;
    };
    let run = RunId::new(1);

    submit_suspended(&shard, run, workflow);
    shard.tick().unwrap();

    let events = journal.snapshot().unwrap();
    let has_run_submitted = events
        .iter()
        .any(|e| matches!(e, RuntimeJournalEvent::RunSubmitted { run: r, .. } if *r == run));
    assert!(
        has_run_submitted,
        "RunSubmitted must be persisted before ack"
    );
}

#[test]
fn action_completed_persists_before_ack() {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(small_config(), journal.clone());
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(1);

    submit_suspended(&shard, run, workflow);
    shard.tick().unwrap();

    // Clear journal from submit phase
    let _ = journal.snapshot().unwrap();

    let seq = vb_core::ids::SeqNo::ZERO;
    let action = vb_core::ids::ActionId::new(0);
    let ticket = vb_core::action::ActionTicket {
        run,
        step: StepIdx::ZERO,
        seq,
        action,
        attempt: 1,
        idempotency_key: vb_core::action::compute_action_idempotency_key(run, seq, action),
        capacity: 1,
    };
    let output = vb_core::action::ActionOutputReady {
        output_slot: SlotIdx::new(0),
        value: SlotValue::Bool(true),
        taint: Taint::Clean,
        encoded_len: 2,
    };
    shard
        .enqueue(ShardCommand::ActionCompleted { ticket, output })
        .unwrap();
    shard.tick().unwrap();

    let events = journal.snapshot().unwrap();
    let has_slot_written = events
        .iter()
        .any(|e| matches!(e, RuntimeJournalEvent::SlotWritten { run: r, .. } if *r == run));
    let has_action_completed_envelope = events.iter().any(|e| {
        matches!(e, RuntimeJournalEvent::ActionCompletedEnvelope { ticket, .. } if ticket.run == run)
    });

    assert!(has_slot_written, "SlotWritten must be persisted before ack");
    assert!(
        has_action_completed_envelope,
        "ActionCompletedEnvelope must be persisted before ack"
    );
}

#[test]
fn action_failed_persists_before_ack() {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(small_config(), journal.clone());
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(1);

    submit_suspended(&shard, run, workflow);
    shard.tick().unwrap();

    let _ = journal.snapshot().unwrap();

    let seq = vb_core::ids::SeqNo::ZERO;
    let action = vb_core::ids::ActionId::new(0);
    let ticket = vb_core::action::ActionTicket {
        run,
        step: StepIdx::ZERO,
        seq,
        action,
        attempt: 1,
        idempotency_key: vb_core::action::compute_action_idempotency_key(run, seq, action),
        capacity: 1,
    };
    let failure = vb_core::action::ActionFailure {
        code: vb_core::ActionFailureCode::Timeout,
        retry_policy: vb_core::action::RetryPolicy::NonRetryable,
        taint: Taint::Clean,
        detail: None,
        encoded_len: 0,
    };
    shard
        .enqueue(ShardCommand::ActionFailed { ticket, failure })
        .unwrap();
    shard.tick().unwrap();

    let events = journal.snapshot().unwrap();
    let has_action_failed = events
        .iter()
        .any(|e| matches!(e, RuntimeJournalEvent::ActionFailed { run: r, .. } if *r == run));
    assert!(
        has_action_failed,
        "ActionFailed must be persisted before ack"
    );
}

#[test]
fn ask_answered_persists_before_ack() {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(small_config(), journal.clone());
    let Some(workflow) = timed_ask_without_answer_workflow() else {
        return;
    };
    let run = RunId::new(1);

    submit_suspended(&shard, run, workflow);
    shard.tick().unwrap();

    let _ = journal.snapshot().unwrap();

    let answer = vb_runtime::shard::AskAnswer {
        ticket: vb_runtime::shard::AskTicket {
            run,
            ask_step: StepIdx::new(2),
            resume_step: StepIdx::new(3),
        },
        answer_slot: SlotIdx::new(2),
        value: SlotValue::I64(99),
        taint: Taint::Clean,
        encoded_len: 0,
    };
    shard.enqueue(ShardCommand::AskAnswered { answer }).unwrap();
    shard.tick().unwrap();

    let events = journal.snapshot().unwrap();
    let has_ask_answered = events
        .iter()
        .any(|e| matches!(e, RuntimeJournalEvent::AskAnswered { run: r, .. } if *r == run));
    let has_slot_written = events
        .iter()
        .any(|e| matches!(e, RuntimeJournalEvent::SlotWritten { run: r, .. } if *r == run));
    let has_step_succeeded = events
        .iter()
        .any(|e| matches!(e, RuntimeJournalEvent::StepSucceeded { run: r, .. } if *r == run));

    assert!(has_ask_answered, "AskAnswered must be persisted before ack");
    assert!(has_slot_written, "SlotWritten must be persisted before ack");
    assert!(
        has_step_succeeded,
        "StepSucceeded must be persisted before ack"
    );
}

#[test]
fn cancel_persists_before_ack() {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(small_config(), journal.clone());
    let Some(workflow) = suspended_workflow() else {
        return;
    };
    let run = RunId::new(1);

    submit_suspended(&shard, run, workflow);
    shard.tick().unwrap();

    let _ = journal.snapshot().unwrap();

    shard
        .enqueue(ShardCommand::Cancel { run, reason: None })
        .unwrap();
    shard.tick().unwrap();

    let events = journal.snapshot().unwrap();
    let has_run_cancelled = events
        .iter()
        .any(|e| matches!(e, RuntimeJournalEvent::RunCancelled { run: r, reason: _ } if *r == run));
    assert!(
        has_run_cancelled,
        "RunCancelled must be persisted before ack"
    );
}

#[test]
fn timer_fired_persists_before_ack() {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let mut shard = Shard::new_with_journal(small_config(), journal.clone());
    let Some(workflow) = timed_wait_then_finish_workflow() else {
        return;
    };
    let run = RunId::new(1);

    shard
        .enqueue(ShardCommand::Submit {
            run,
            workflow,
            caps: CapabilitySet::empty(),
        })
        .unwrap();
    shard.tick().unwrap();

    let _ = journal.snapshot().unwrap();

    let entry = shard.timer_entry(run).unwrap();
    shard
        .enqueue(ShardCommand::TimerFired {
            run: entry.run,
            generation: entry.generation,
            deadline: entry.deadline,
            kind: entry.kind,
            logical_deadline: None,
        })
        .unwrap();
    shard.tick().unwrap();

    let events = journal.snapshot().unwrap();
    let has_wait_resolved = events
        .iter()
        .any(|e| matches!(e, RuntimeJournalEvent::WaitResolved { run: r, .. } if *r == run));
    assert!(
        has_wait_resolved,
        "WaitResolved must be persisted before ack"
    );
}

// ---------------------------------------------------------------------------
// Gate tests for missing evidence
// ---------------------------------------------------------------------------

use vb_runtime::durability_matrix::{
    verify_ack_after_persist, verify_matrix_completeness, verify_matrix_replay_proofs,
};

#[test]
fn gate_fails_when_primitive_row_is_missing() {
    let result = verify_matrix_completeness();
    assert!(
        result.is_ok(),
        "Matrix should be complete; if this fails, the missing primitive is: {:?}",
        result
    );
}

#[test]
fn gate_fails_when_row_omits_replay_evidence() {
    let result = verify_matrix_replay_proofs();
    assert!(
        result.is_ok(),
        "All rows should have replay proof; if this fails: {:?}",
        result
    );
}

#[test]
fn gate_fails_when_row_claims_ack_before_persist() {
    let result = verify_ack_after_persist();
    assert!(
        result.is_ok(),
        "No row should claim ack-before-persist; if this fails: {:?}",
        result
    );
}
