use super::*;
use crate::commands_incident::build_incident_report;
use vb_core::{ActionId, ActionTicket, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_storage::{DurableActionOutcome, EventSeq};

#[path = "tests/ticket_phase_tests.rs"]
mod ticket_phase_tests;

fn ticket(attempt: u16, fence: u64) -> ActionTicket {
    ActionTicket {
        run: RunId::new(7),
        step: StepIdx::new(4),
        seq: SeqNo::new(8),
        action: ActionId::new(9),
        run_attempt: 2,
        attempt,
        idempotency_key: u128::MAX,
        capacity: 5,
        fencing_token: fence,
    }
}

fn expected_ticket(ticket: ActionTicket) -> Value {
    json!({
        "run": ticket.run.get(),
        "step": ticket.step.get(),
        "seq": ticket.seq.get(),
        "action": ticket.action.get(),
        "run_attempt": ticket.run_attempt,
        "attempt": ticket.attempt,
        "idempotency_key": ticket.idempotency_key.to_string(),
        "capacity": ticket.capacity,
        "fencing_token": ticket.fencing_token,
    })
}

fn scheduled(ticket: ActionTicket) -> JournalEvent {
    scheduled_at(ticket, EventSeq::new(11))
}

fn scheduled_at(ticket: ActionTicket, seq: EventSeq) -> JournalEvent {
    JournalEvent::ActionScheduledTicket {
        run: ticket.run,
        seq,
        ticket,
        input: SlotIdx::new(1),
        output: SlotIdx::new(2),
        action_abi_digest: WorkflowDigest::from_bytes([0xabu8; 32]),
    }
}

fn completed(ticket: ActionTicket) -> JournalEvent {
    completed_at(ticket, EventSeq::new(12))
}

fn completed_at(ticket: ActionTicket, seq: EventSeq) -> JournalEvent {
    JournalEvent::ActionCompletedEnvelope {
        run: ticket.run,
        seq,
        ticket,
        output: SlotIdx::new(2),
        outcome: DurableActionOutcome::Ready,
        value: b"redaction sentinel".to_vec(),
        encoded_len: 18,
        taint: vb_core::Taint::Secret,
        value_digest: *blake3::hash(b"redaction sentinel").as_bytes(),
        action_abi_digest: WorkflowDigest::from_bytes([0xabu8; 32]),
    }
}

fn abandoned(ticket: ActionTicket) -> JournalEvent {
    abandoned_at(ticket, EventSeq::new(13))
}

fn abandoned_at(ticket: ActionTicket, seq: EventSeq) -> JournalEvent {
    JournalEvent::ActionAbandoned {
        run: ticket.run,
        seq,
        ticket,
    }
}

#[test]
fn full_ticket_rows_retain_exact_identity_and_durable_phase_facts() {
    let issued = ticket(3, 6);
    let changed_fence = ticket(3, 7);
    let events = [
        scheduled(issued),
        completed(issued),
        scheduled(changed_fence),
    ];
    let report = build_incident_report("run-7", &events);

    assert_eq!(report.side_effects.len(), 3);
    let first = &report.side_effects[0];
    assert_eq!(first["certainty"], "scheduled");
    assert_eq!(first["run_id"], 7);
    assert_eq!(first["seq"], 11);
    assert_eq!(first["attempt"], 2);
    assert_eq!(first["ticket"], expected_ticket(issued));
    assert_eq!(first["ticket"]["attempt"], 3);
    assert_eq!(first["ticket"]["idempotency_key"], u128::MAX.to_string());
    assert_eq!(first["ticket"]["fencing_token"], 6);
    assert_eq!(first["action_abi_digest"], "ab".repeat(32));
    assert_eq!(first["scheduled_durable"], true);
    assert_eq!(first["completion_durable"], true);
    assert_eq!(report.side_effects[1]["certainty"], "confirmed");
    assert_eq!(report.side_effects[1]["scheduled_durable"], true);
    assert_eq!(report.side_effects[1]["completion_durable"], true);
    assert_eq!(first["external_effect"], "uncertain");
    assert!(first["safe_to_retry"].is_null());
    assert_eq!(first["reason"], "retry_contract_not_loaded");
    assert_eq!(report.side_effects[2]["completion_durable"], Value::Null);
    assert_eq!(report.journal_tail.len(), 3);
    assert_eq!(
        report.journal_tail[0],
        json!({"seq": 11, "kind": "ActionScheduledTicket", "run_id": 7, "attempt": 2})
    );
    assert!(
        report
            .side_effects
            .iter()
            .all(|effect| effect.get("value").is_none())
    );
    assert!(
        report
            .journal_tail
            .iter()
            .all(|event| event.get("value").is_none())
    );
}
