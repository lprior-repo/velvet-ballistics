use super::{
    abandoned, abandoned_at, completed, completed_at, expected_ticket, scheduled, scheduled_at,
    ticket,
};
use crate::commands_incident::build_incident_report;
use serde_json::{Value, json};
use vb_core::{ActionId, ActionTicket, RunId, SeqNo, StepIdx};
use vb_storage::{EventSeq, JournalEvent};

#[test]
fn every_action_ticket_field_controls_phase_matching() {
    let issued = ticket(3, 6);
    let variants = [
        (
            "run",
            ActionTicket {
                run: RunId::new(8),
                ..issued
            },
        ),
        (
            "step",
            ActionTicket {
                step: StepIdx::new(5),
                ..issued
            },
        ),
        (
            "seq",
            ActionTicket {
                seq: SeqNo::new(9),
                ..issued
            },
        ),
        (
            "action",
            ActionTicket {
                action: ActionId::new(10),
                ..issued
            },
        ),
        (
            "run_attempt",
            ActionTicket {
                run_attempt: 3,
                ..issued
            },
        ),
        (
            "attempt",
            ActionTicket {
                attempt: 4,
                ..issued
            },
        ),
        (
            "idempotency_key",
            ActionTicket {
                idempotency_key: 0,
                ..issued
            },
        ),
        (
            "capacity",
            ActionTicket {
                capacity: 6,
                ..issued
            },
        ),
        (
            "fencing_token",
            ActionTicket {
                fencing_token: 7,
                ..issued
            },
        ),
    ];

    for (field, changed) in variants {
        let report = build_incident_report("run-7", &[scheduled(issued), completed(changed)]);
        assert_eq!(report.side_effects.len(), 2, "{field}");
        assert_eq!(
            report.side_effects[0]["ticket"],
            expected_ticket(issued),
            "{field}"
        );
        assert_eq!(report.side_effects[0]["scheduled_durable"], true, "{field}");
        assert_eq!(
            report.side_effects[0]["completion_durable"],
            Value::Null,
            "{field}"
        );
        assert_eq!(
            report.side_effects[1]["ticket"],
            expected_ticket(changed),
            "{field}"
        );
        assert_eq!(
            report.side_effects[1]["scheduled_durable"],
            Value::Null,
            "{field}"
        );
        assert_eq!(
            report.side_effects[1]["completion_durable"], true,
            "{field}"
        );
    }
}

#[test]
fn reverse_order_keeps_only_each_event_own_phase() {
    let issued = ticket(3, 6);
    let report = build_incident_report(
        "run-7",
        &[
            completed_at(issued, EventSeq::new(10)),
            scheduled_at(issued, EventSeq::new(11)),
        ],
    );

    assert_eq!(report.side_effects.len(), 2);
    assert_eq!(report.side_effects[0]["certainty"], "confirmed");
    assert_eq!(report.side_effects[0]["scheduled_durable"], Value::Null);
    assert_eq!(report.side_effects[0]["completion_durable"], true);
    assert_eq!(report.side_effects[1]["certainty"], "scheduled");
    assert_eq!(report.side_effects[1]["scheduled_durable"], true);
    assert_eq!(report.side_effects[1]["completion_durable"], Value::Null);
}

#[test]
fn abandoned_fact_keeps_prior_schedule_without_claiming_completion() {
    let issued = ticket(3, 6);
    let report = build_incident_report(
        "run-7",
        &[
            scheduled_at(issued, EventSeq::new(10)),
            abandoned_at(issued, EventSeq::new(11)),
            completed_at(issued, EventSeq::new(12)),
        ],
    );

    let abandoned = &report.side_effects[1];
    assert_eq!(abandoned["certainty"], "abandoned");
    assert_eq!(abandoned["scheduled_durable"], true);
    assert_eq!(abandoned["completion_durable"], Value::Null);
    let completion = &report.side_effects[2];
    assert_eq!(completion["certainty"], "confirmed");
    assert_eq!(completion["scheduled_durable"], true);
    assert_eq!(completion["completion_durable"], true);
}

#[test]
fn only_observed_phases_are_reported_for_sparse_and_ticketed_history() {
    let issued = ticket(3, 6);
    let digest = Value::String("ab".repeat(32));
    let scheduled_only = build_incident_report("run-7", &[scheduled(issued)]);
    assert_eq!(
        scheduled_only.side_effects,
        [json!({
            "run_id": 7, "seq": 11, "attempt": 2, "step": 4, "action": 9,
            "certainty": "scheduled", "ticket": expected_ticket(issued),
            "action_abi_digest": digest, "scheduled_durable": true,
            "completion_durable": null, "external_effect": "uncertain",
            "safe_to_retry": null, "reason": "retry_contract_not_loaded"
        })]
    );
    let completion_only = build_incident_report("run-7", &[completed(issued)]);
    assert_eq!(
        completion_only.side_effects,
        [json!({
            "run_id": 7, "seq": 12, "attempt": 2, "step": 4, "action": 9,
            "certainty": "confirmed", "ticket": expected_ticket(issued),
            "action_abi_digest": digest, "scheduled_durable": null,
            "completion_durable": true, "external_effect": "uncertain",
            "safe_to_retry": null, "reason": "retry_contract_not_loaded"
        })]
    );

    let sparse_completion = JournalEvent::ActionCompletedEvent {
        run: RunId::new(3),
        seq: EventSeq::new(5),
        step: StepIdx::new(2),
        action: ActionId::new(5),
        attempt: 1,
    };
    let sparse = build_incident_report("run-3", &[sparse_completion.clone()]);
    assert_eq!(
        sparse.side_effects,
        [json!({
            "run_id": 3, "seq": 5, "attempt": 1, "step": 2, "action": 5,
            "certainty": "confirmed", "ticket": null, "action_abi_digest": null,
            "scheduled_durable": null, "completion_durable": true,
            "external_effect": "uncertain", "safe_to_retry": null,
            "reason": "retry_contract_not_loaded"
        })]
    );
    let sparse_schedule = JournalEvent::ActionScheduled {
        run: RunId::new(3),
        seq: EventSeq::new(4),
        step: StepIdx::new(2),
        action: ActionId::new(5),
        attempt: 1,
    };
    let sparse_pair = build_incident_report("run-3", &[sparse_schedule, sparse_completion]);
    assert_eq!(sparse_pair.side_effects[0]["scheduled_durable"], true);
    assert_eq!(
        sparse_pair.side_effects[0]["completion_durable"],
        Value::Null
    );
    assert_eq!(
        sparse_pair.side_effects[1]["scheduled_durable"],
        Value::Null
    );
    assert_eq!(sparse_pair.side_effects[1]["completion_durable"], true);

    let failure = build_incident_report(
        "run-3",
        &[JournalEvent::ActionFailedEvent {
            run: RunId::new(3),
            seq: EventSeq::new(6),
            step: StepIdx::new(2),
            action: ActionId::new(5),
            attempt: 1,
        }],
    );
    assert_eq!(
        failure.side_effects,
        [json!({
            "run_id": 3, "seq": 6, "attempt": 1, "step": 2, "action": 5,
            "certainty": "failed", "ticket": null, "action_abi_digest": null,
            "scheduled_durable": null, "completion_durable": null,
            "external_effect": "uncertain", "safe_to_retry": null,
            "reason": "retry_contract_not_loaded"
        })]
    );
    let abandoned = build_incident_report("run-7", &[abandoned(issued)]);
    assert_eq!(
        abandoned.side_effects,
        [json!({
            "run_id": 7, "seq": 13, "attempt": 2, "step": 4, "action": 9,
            "certainty": "abandoned", "ticket": expected_ticket(issued),
            "action_abi_digest": null, "scheduled_durable": null,
            "completion_durable": null, "external_effect": "uncertain",
            "safe_to_retry": null, "reason": "retry_contract_not_loaded"
        })]
    );
}
