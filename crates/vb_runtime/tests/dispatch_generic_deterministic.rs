//! Dispatch generic determinism tests (with mock field).
//!
//! Verifies that dispatch is deterministic: same input always produces
//! the same output.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-011, RFO-020
//! Contract clauses: I-DISPATCH-3 (idempotence + determinism)

#![forbid(unsafe_code)]
#![cfg(feature = "vb-rxru0-mock-marker")]

use vb_core::action::{
    ActionContract, ActionInput, ActionName, ActionOutcome, ActionTicket, Idempotency,
    RetrySafety, SideEffect,
};
use vb_core::ids::{ActionId, RunId, SeqNo, SlotIdx, StepIdx};
use vb_core::action::MockMarker;
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

fn make_input_with_action_name(action_id: u16, name: &str) -> ActionInput {
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
fn test_dispatch_deterministic() {
    let names = [
        "github.issue.create",
        "ai.classify_ticket",
        "http.request",
        "random.action.name",
    ];

    for name in &names {
        let id = name.len() as u16;
        let results: Vec<ActionOutcome> = (0..10)
            .map(|_| {
                let input = make_input_with_action_name(id, name);
                let contract = make_contract(id, name);
                dispatch_generic(&input, &contract).unwrap()
            })
            .collect();

        // All 10 executions produce identical outcomes.
        for result in &results[1..] {
            assert_eq!(
                result,
                &results[0],
                "dispatch_generic must be deterministic for '{name}'"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Proptest: property-based determinism
// ---------------------------------------------------------------------------

#[cfg(feature = "proptest")]
proptest! {
    /// Property-based determinism test for dispatch_generic.
    #[test]
    fn test_dispatch_determinism(name in "[a-z\\.]{1,64}") {
        let id = name.len() as u16;
        let input = make_input_with_action_name(id, &name);
        let contract = make_contract(id, &name);
        let result1 = dispatch_generic(&input, &contract).unwrap();
        let result2 = dispatch_generic(&input, &contract).unwrap();
        prop_assert_eq!(result1, result2, "dispatch must be deterministic");
    }

    /// Property-based non-mock names default to HttpGet.
    #[test]
    fn test_dispatch_non_mock_defaults_to_http_get(name in "[a-z]{3,20}") {
        let known_names = ["github.issue.create", "ai.classify_ticket", "http.request"];
        if !known_names.contains(&name.as_str()) {
            let id = name.len() as u16;
            let input = make_input_with_action_name(id, &name);
            let contract = make_contract(id, &name);
            let outcome = dispatch_generic(&input, &contract).unwrap();
            match outcome {
                ActionOutcome::Suspended(ticket) => {
                    prop_assert_eq!(
                        ticket.mock,
                        MockMarker::HttpGet,
                        "Non-mock names must default to HttpGet"
                    );
                }
                _ => panic!("Expected Suspended"),
            }
        }
    }
}
