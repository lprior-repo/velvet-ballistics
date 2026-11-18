#![forbid(unsafe_code)]
//! Recovery tests for velvet-ballistics journal.
use crate::recovery::{
    ActionAbiDigestComparison, ActionReplayTracker, DigestVerificationRequest, FullDigestEvidence,
    PolicyDigestComparison, RecoveredStepState, RecoveryError, RecoveryHydration,
    RecoveryTerminalState, RunSnapshot, UnsupportedRecoveryState, check_compiled_ir_digest,
    check_workflow_source_digest, extract_terminal, is_terminal_event, recover_all_incomplete_runs,
    recover_full_journal, recover_runtime_frame_seed, recover_runtime_frame_seed_from_events,
    recover_runtime_frame_seed_from_events_with_workflow, recover_runtime_summary,
    recover_snapshot_plus_tail, replay_events, summarize_recovery_events, verify_digests,
};
use crate::{
    DurableActionOutcome, EventSeq, FjallJournal, JournalEvent, RecordKind, RunHeaderRecord,
};
use vb_core::action::{ActionTicket, compute_action_idempotency_key};
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts, node,
};
use vb_core::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

fn sample_digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

fn deterministic_plan() -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    CompiledWorkflow::try_from_parts(deterministic_parts())
        .map_err(Box::<dyn std::error::Error>::from)
}

fn deterministic_parts() -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        name: "recovery_replay".into(),
        input_slots: Box::new([]),
        digest: sample_digest(44),
        nodes: deterministic_nodes().into(),
        expressions: Vec::new().into(),
        accessors: Vec::new().into(),
        constants: vec![ConstValue::I64(42)].into(),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn deterministic_nodes() -> Vec<CompiledNode> {
    vec![set_const_zero(), copy_zero_to_one(), finish_one()]
}

fn set_const_zero() -> CompiledNode {
    node(
        StepIdx::ZERO,
        CompiledNodeKind::SetConst {
            value: vb_core::ConstIdx::new(0),
        },
        Some(SlotIdx::new(0)),
        Some(StepIdx::new(1)),
    )
}

fn copy_zero_to_one() -> CompiledNode {
    node(
        StepIdx::new(1),
        CompiledNodeKind::Copy {
            source: SlotIdx::new(0),
        },
        Some(SlotIdx::new(1)),
        Some(StepIdx::new(2)),
    )
}

fn finish_one() -> CompiledNode {
    node(
        StepIdx::new(2),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
        None,
        None,
    )
}

fn deterministic_replay_events(run: RunId, workflow: WorkflowDigest) -> Vec<JournalEvent> {
    vec![
        accepted_event(run, EventSeq::new(0), workflow),
        started_event(run, EventSeq::new(1), StepIdx::ZERO),
        succeeded_event(run, EventSeq::new(2), StepIdx::ZERO, SlotIdx::new(0)),
        started_event(run, EventSeq::new(3), StepIdx::new(1)),
        succeeded_event(run, EventSeq::new(4), StepIdx::new(1), SlotIdx::new(1)),
    ]
}

fn step_succeeded_events(run: RunId, workflow: WorkflowDigest, step: StepIdx) -> Vec<JournalEvent> {
    vec![
        accepted_event(run, EventSeq::new(0), workflow),
        succeeded_event(run, EventSeq::new(1), step, SlotIdx::new(0)),
    ]
}

fn accepted_event(run: RunId, seq: EventSeq, workflow: WorkflowDigest) -> JournalEvent {
    JournalEvent::RunAccepted { run, seq, workflow }
}

fn recovery_action_ticket(run: RunId, step: StepIdx, action: ActionId) -> ActionTicket {
    let seq = SeqNo::ZERO;
    ActionTicket {
        run,
        step,
        seq,
        action,
        run_attempt: 1,
        attempt: 1,
        idempotency_key: compute_action_idempotency_key(run, 1, seq, action),
        capacity: 1,
        fencing_token: 0,
    }
}

fn recovery_action_scheduled_ticket_event(
    run: RunId,
    seq: EventSeq,
    ticket: ActionTicket,
    input: SlotIdx,
    output: SlotIdx,
) -> JournalEvent {
    JournalEvent::ActionScheduledTicket {
        run,
        seq,
        ticket,
        action_abi_digest: sample_digest(0xA1),
        input,
        output,
    }
}

fn recovery_action_completed_envelope_event(
    run: RunId,
    seq: EventSeq,
    ticket: ActionTicket,
    output: SlotIdx,
    value: SlotValue,
    taint: Taint,
) -> JournalEvent {
    let encoded = postcard::to_allocvec(&value).expect("slot value encodes");
    let encoded_len = u32::try_from(encoded.len()).expect("encoded length fits u32");
    let value_digest = *blake3::hash(&encoded).as_bytes();
    JournalEvent::ActionCompletedEnvelope {
        run,
        seq,
        ticket,
        action_abi_digest: sample_digest(0xA1),
        output,
        outcome: DurableActionOutcome::Ready,
        value: encoded,
        encoded_len,
        taint,
        value_digest,
    }
}

fn started_event(run: RunId, seq: EventSeq, step: StepIdx) -> JournalEvent {
    JournalEvent::StepStarted {
        run,
        seq,
        step,
        attempt: 1,
    }
}

fn succeeded_event(run: RunId, seq: EventSeq, step: StepIdx, output: SlotIdx) -> JournalEvent {
    JournalEvent::StepSucceeded {
        run,
        seq,
        step,
        output,
    }
}

fn assert_recovered_i64_slot(seed: &crate::recovery::RecoveryFrameSeed, slot: SlotIdx) {
    assert!(seed.slots.iter().any(|entry| {
        entry.slot == slot && entry.value == SlotValue::I64(42) && entry.taint == Taint::Clean
    }));
}

fn assert_compiled_digest_mismatch(
    result: Result<crate::recovery::RecoveryFrameSeedProduct, RecoveryError>,
    expected: WorkflowDigest,
    found: WorkflowDigest,
) {
    assert!(matches!(
        result,
        Err(RecoveryError::CompiledIrDigestMismatch { expected: e, found: f })
            if e == expected && f == found
    ));
}

fn assert_replay_divergence_step(
    result: Result<crate::recovery::RecoveryFrameSeedProduct, RecoveryError>,
    expected_step: StepIdx,
    expected_detail: &str,
) {
    assert!(matches!(
        result,
        Err(RecoveryError::ReplayDivergence { step, detail })
            if step == expected_step && detail == expected_detail
    ));
}

include!("tests/summaries_and_replay.rs");
include!("tests/frame_seed_and_tracker.rs");
include!("tests/digests_and_terminal.rs");
mod hydrate_run_frame_tests;
