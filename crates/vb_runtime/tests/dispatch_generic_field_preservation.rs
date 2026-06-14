//! Dispatch generic field preservation tests (with mock field).
//!
//! Verifies that dispatch_generic preserves all input fields and that
//! calling it twice with the same input produces identical results.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-010, RFO-013, RFO-014
//! Contract clauses: PST-DISPATCH-2 (field preservation),
//!                   I-DISPATCH-3 (idempotence)

#![forbid(unsafe_code)]
#![cfg(feature = "vb-rxru0-mock-marker")]

use vb_core::action::MockMarker;
use vb_core::action::{
    ActionContract, ActionInput, ActionName, ActionOutcome, ActionTicket, Idempotency, RetrySafety,
    SideEffect,
};
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use vb_runtime::action::dispatch_generic;

fn make_contract(id: u16, name: &str) -> ActionContract {
    ActionContract {
        id: ActionId::new(id),
        name: ActionName::new(name).unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 1024,
        timeout_ms: 5000,
        idempotency: Idempotency::DeterministicPure,
        side_effect: SideEffect::Pure,
        retry_safety: RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    }
}

fn make_input_with_action_name(action_id: u16, _name: &str) -> ActionInput {
    ActionInput {
        run: RunId::new(1),
        step: StepIdx::new(0),
        action: ActionId::new(action_id),
        input: SlotIdx::new(0),
        ticket: ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(0),
            seq: SeqNo::new(42),
            action: ActionId::new(action_id),
            attempt: 3,
            idempotency_key: 0xBEEF,
            capacity: 999,
            mock: MockMarker::GithubIssueCreate,
        },
    }
}

#[test]
fn test_dispatch_preserves_all_fields() {
    let input = make_input_with_action_name(1, "github.issue.create");
    let contract = make_contract(1, "github.issue.create");
    let outcome = dispatch_generic(&input, &contract).unwrap();

    match outcome {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(ticket.run, input.run, "run must be preserved from input");
            assert_eq!(ticket.step, input.step, "step must be preserved from input");
            assert_eq!(
                ticket.seq, input.ticket.seq,
                "seq must be preserved from input ticket"
            );
            assert_eq!(
                ticket.action, input.action,
                "action must be preserved from input"
            );
            assert_eq!(
                ticket.attempt, input.ticket.attempt,
                "attempt must be preserved from input ticket"
            );
            assert_eq!(
                ticket.idempotency_key, input.ticket.idempotency_key,
                "idempotency_key must be preserved from input ticket"
            );
            assert_eq!(ticket.capacity, 1, "capacity must be set to 1");
            assert_eq!(
                ticket.mock,
                MockMarker::GithubIssueCreate,
                "mock must be derived from contract.name"
            );
        }
        unexpected => panic!("Expected Suspended, got {unexpected:?}"),
    }
}

#[test]
fn test_dispatch_is_pure() {
    let input = make_input_with_action_name(1, "ai.classify_ticket");
    let contract = make_contract(1, "ai.classify_ticket");

    let result1 = dispatch_generic(&input, &contract).unwrap();
    let result2 = dispatch_generic(&input, &contract).unwrap();

    match (result1, result2) {
        (ActionOutcome::Suspended(t1), ActionOutcome::Suspended(t2)) => {
            assert_eq!(t1, t2, "dispatch_generic must be idempotent");
        }
        _ => panic!("Expected two Suspended outcomes"),
    }
}
