//! Dispatch generic mock derivation tests.
//!
//! Verifies that dispatch_generic routes action names to the correct
//! MockMarker value.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-008, RFO-009, RFO-011, RFO-018, RFO-019, RFO-020
//! Contract clauses: PST-DISPATCH-3 (github, ai, http arms), C-CROSS-2
//!                  (mock derived from contract.name)

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
            mock: MockMarker::GithubIssueCreate, // intentionally wrong — will be overridden
        },
    }
}

#[test]
fn test_dispatch_github_arm() {
    let input = make_input_with_action_name(1, "github.issue.create");
    let contract = make_contract(1, "github.issue.create");

    let outcome = dispatch_generic(&input, &contract).unwrap();

    match outcome {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.mock,
                MockMarker::GithubIssueCreate,
                "github.issue.create must route to GithubIssueCreate mock marker"
            );
        }
        unexpected => panic!("Expected Suspended, got {unexpected:?}"),
    }
}

#[test]
fn test_dispatch_ai_arm() {
    let input = make_input_with_action_name(2, "ai.classify_ticket");
    let contract = make_contract(2, "ai.classify_ticket");

    let outcome = dispatch_generic(&input, &contract).unwrap();

    match outcome {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.mock,
                MockMarker::AiClassifyTicket,
                "ai.classify_ticket must route to AiClassifyTicket mock marker"
            );
        }
        unexpected => panic!("Expected Suspended, got {unexpected:?}"),
    }
}

#[test]
fn test_dispatch_http_arm() {
    let input = make_input_with_action_name(3, "http.request");
    let contract = make_contract(3, "http.request");

    let outcome = dispatch_generic(&input, &contract).unwrap();

    match outcome {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.mock,
                MockMarker::HttpGet,
                "http.request must route to HttpGet mock marker"
            );
        }
        unexpected => panic!("Expected Suspended, got {unexpected:?}"),
    }
}

#[test]
fn test_dispatch_default_arm_http_get() {
    // Any unknown action name → mock == MockMarker::HttpGet (default arm).
    let unknown_names = [
        "custom.action",
        "foo.bar.baz",
        "github.unknown.action",
        "ai.other.task",
    ];

    for name in &unknown_names {
        let id = name.len() as u16;
        let input = make_input_with_action_name(id, name);
        let contract = make_contract(id, name);

        let outcome = dispatch_generic(&input, &contract).unwrap();

        match outcome {
            ActionOutcome::Suspended(ticket) => {
                assert_eq!(
                    ticket.mock,
                    MockMarker::HttpGet,
                    "Unknown action '{name}' must default to HttpGet mock marker"
                );
            }
            unexpected => panic!("Expected Suspended for '{name}', got {unexpected:?}"),
        }
    }
}

#[test]
fn test_dispatch_mock_derived_not_from_input() {
    // Create a ticket with a different mock value than what dispatch would derive.
    let input = make_input_with_action_name(1, "github.issue.create");
    let contract = make_contract(1, "github.issue.create");

    let outcome = dispatch_generic(&input, &contract).unwrap();

    match outcome {
        ActionOutcome::Suspended(ticket) => {
            assert_eq!(
                ticket.mock,
                MockMarker::GithubIssueCreate,
                "mock must be derived from contract.name, not from input.ticket.mock"
            );
            assert_ne!(
                ticket.mock,
                MockMarker::AiClassifyTicket,
                "mock must NOT be the input ticket's mock value"
            );
        }
        unexpected => panic!("Expected Suspended, got {unexpected:?}"),
    }
}
