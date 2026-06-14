//! Dispatch generic capacity bound test (with mock field).
//!
//! Verifies that dispatch_generic always sets capacity to 1 regardless
//! of input capacity.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-011, RFO-020
//! Contract clauses: PST-DISPATCH-2

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
fn test_dispatch_capacity_bounded() {
    let input = make_input_with_action_name(1, "github.issue.create");
    let mut modified_input = input.clone();
    modified_input.ticket.capacity = 9999;
    let contract = make_contract(1, "github.issue.create");

    let outcome = dispatch_generic(&modified_input, &contract).unwrap();

    match outcome {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.capacity, 1,
                "dispatch_generic must always set capacity to 1, not from input"
            );
            assert_ne!(
                ticket.capacity, 9999,
                "capacity must NOT be derived from input.ticket.capacity"
            );
        }
        unexpected => panic!("Expected Suspended, got {unexpected:?}"),
    }
}
