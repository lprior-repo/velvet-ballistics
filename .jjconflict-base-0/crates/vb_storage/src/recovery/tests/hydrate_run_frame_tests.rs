// ============================================================================
// Hydrate RunFrame from snapshot and journal — TDD Red Phase tests
// ============================================================================

use crate::recovery::hydrate_support::apply_tail_events;
use crate::recovery::{
    ActionReplayTracker, RecoveryError, RunSnapshot, hydrate_run_frame,
    hydrate_run_frame_from_events,
};
use crate::{DurableActionOutcome, EventSeq, JournalEvent};
use vb_core::action::{ActionTicket, compute_action_idempotency_key};
use vb_core::value::{SlotValue, Taint};
use vb_core::{ActionId, RunId, SeqNo, SlotIdx, StepIdx, StepState, WorkflowDigest};

fn sample_digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

fn corrupt_slot_taint_envelope() -> Vec<u8> {
    let mut bytes = crate::SLOT_WRITTEN_EXTRA_PREFIX.to_vec();
    bytes.extend_from_slice(&[255, 255, 255]);
    bytes
}

fn empty_snapshot(run: RunId, seq: EventSeq) -> RunSnapshot {
    RunSnapshot {
        run,
        seq,
        workflow: sample_digest(1),
        slots: Vec::new(),
        taint: Vec::new(),
    }
}

fn action_ticket(run: RunId, step: StepIdx, action: ActionId) -> ActionTicket {
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

fn encoded_slot(value: &SlotValue) -> Vec<u8> {
    postcard::to_allocvec(value).expect("slot value encodes")
}

fn action_scheduled_ticket_event(
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

fn action_completed_envelope_event(
    run: RunId,
    seq: EventSeq,
    ticket: ActionTicket,
    output: SlotIdx,
    value: SlotValue,
    taint: Taint,
) -> JournalEvent {
    let encoded = encoded_slot(&value);
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

fn snapshot_with_slots(
    run: RunId,
    seq: EventSeq,
    slot_entries: &[(SlotIdx, SlotValue, Taint)],
) -> RunSnapshot {
    let slots: Vec<(SlotIdx, SlotValue, Taint)> = slot_entries.to_vec();
    let slots_bytes = postcard::to_allocvec(&slots).expect("postcard encode slots");
    let taint_bytes = postcard::to_allocvec(&slots).expect("postcard encode taint");
    RunSnapshot {
        run,
        seq,
        workflow: sample_digest(1),
        slots: slots_bytes,
        taint: taint_bytes,
    }
}

// --- Happy path: snapshot + tail events ---

include!("hydrate_a.rs");
include!("hydrate_b.rs");
