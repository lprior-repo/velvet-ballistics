//! Dispatch generic exhaustive match test.
//!
//! Verifies that all match arms (3 named + default) are exercised.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-019
//! Contract clauses: I-DISPATCH-1 (exhaustiveness)

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
fn test_dispatch_generic_exhaustive_match() {
    // All 3 named arms plus the default arm are exercised.
    let all_names = [
        ("github.issue.create", MockMarker::GithubIssueCreate),
        ("ai.classify_ticket", MockMarker::AiClassifyTicket),
        ("http.request", MockMarker::HttpGet),
        ("unknown.name", MockMarker::HttpGet), // defaults to HttpGet
    ];

    for (name, expected_mock) in all_names {
        let id = name.len() as u16;
        let input = make_input_with_action_name(id, name);
        let contract = make_contract(id, name);
        let outcome = dispatch_generic(&input, &contract).unwrap();

        match outcome {
            ActionOutcome::Suspended(ticket) => {
                assert_eq!(
                    ticket.mock, expected_mock,
                    "dispatch_generic must handle '{name}' exhaustively (expected {expected_mock:?})"
                );
            }
            unexpected => panic!("Expected Suspended for '{name}', got {unexpected:?}"),
        }
    }

    // Verify that no branch can return an error for valid input.
    // All paths must produce Ok(ActionOutcome::Suspended).
}
