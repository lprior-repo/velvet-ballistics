use super::{SideEffectCertainty, analyze_incident_events};
use crate::{EventSeq, JournalEvent};
use vb_core::{ActionId, ActionTicket, RunId, SeqNo, SlotIdx, StepIdx, WorkflowDigest};

fn ticket(fence: u64) -> ActionTicket {
    ActionTicket {
        run: RunId::new(7),
        step: StepIdx::new(4),
        seq: SeqNo::new(8),
        action: ActionId::new(9),
        run_attempt: 2,
        attempt: 3,
        idempotency_key: u128::MAX,
        capacity: 5,
        fencing_token: fence,
    }
}

fn scheduled(ticket: ActionTicket) -> JournalEvent {
    JournalEvent::ActionScheduledTicket {
        run: ticket.run,
        seq: EventSeq::new(11),
        ticket,
        input: SlotIdx::new(1),
        output: SlotIdx::new(2),
        action_abi_digest: WorkflowDigest::from_bytes([0xabu8; 32]),
    }
}

fn completed(ticket: ActionTicket) -> JournalEvent {
    JournalEvent::ActionCompletedEnvelope {
        run: ticket.run,
        seq: EventSeq::new(12),
        ticket,
        output: SlotIdx::new(2),
        outcome: crate::DurableActionOutcome::Ready,
        value: Vec::new(),
        encoded_len: 0,
        taint: vb_core::Taint::Clean,
        value_digest: *blake3::hash(&[]).as_bytes(),
        action_abi_digest: WorkflowDigest::from_bytes([0xabu8; 32]),
    }
}

fn abandoned(ticket: ActionTicket) -> JournalEvent {
    JournalEvent::ActionAbandoned {
        run: ticket.run,
        seq: EventSeq::new(13),
        ticket,
    }
}

#[test]
fn scheduled_completion_and_abandonment_preserve_order_and_distinct_facts() {
    let issued = ticket(6);
    let changed_fence = ticket(7);
    let events = [
        scheduled(issued),
        completed(issued),
        abandoned(changed_fence),
    ];
    let original = events.clone();
    let analysis = analyze_incident_events(&events);

    assert_eq!(analysis.side_effects.len(), 3);
    assert_eq!(
        analysis
            .side_effects
            .iter()
            .map(|item| item.certainty)
            .collect::<Vec<_>>(),
        [
            SideEffectCertainty::Scheduled,
            SideEffectCertainty::Confirmed,
            SideEffectCertainty::Abandoned,
        ]
    );
    assert_eq!(analysis.side_effects[0].ticket, Some(issued));
    assert_eq!(analysis.side_effects[0].run, issued.run);
    assert_eq!(analysis.side_effects[0].seq, EventSeq::new(11));
    assert_eq!(analysis.side_effects[0].attempt, issued.run_attempt);
    assert_eq!(
        analysis.side_effects[0].action_abi_digest,
        Some(WorkflowDigest::from_bytes([0xabu8; 32]))
    );
    assert_eq!(analysis.side_effects[2].ticket, Some(changed_fence));
    assert_eq!(events, original);
}

#[test]
fn sparse_action_history_keeps_missing_full_identity_absent() {
    let event = JournalEvent::ActionCompletedEvent {
        run: RunId::new(2),
        seq: EventSeq::new(5),
        step: StepIdx::new(3),
        action: ActionId::new(4),
        attempt: 1,
    };
    let analysis = analyze_incident_events(&[event]);

    assert_eq!(analysis.side_effects.len(), 1);
    assert_eq!(analysis.side_effects[0].ticket, None);
    assert_eq!(analysis.side_effects[0].action_abi_digest, None);
    assert_eq!(
        analysis.side_effects[0].certainty,
        SideEffectCertainty::Confirmed
    );
}
