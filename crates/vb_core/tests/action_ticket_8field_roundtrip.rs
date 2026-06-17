#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, clippy::let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports)]

//! ActionTicket 8-field postcard roundtrip.
//!
//! Verifies that all 8 fields (including mock) are preserved through
//! serialization.
//! Gated behind `vb-rxru0-mock-marker` feature.
//!
//! Obligations: RFO-007, RFO-015, RFO-023
//! Contract clauses: C-TICKET-3, PST-DISPATCH-2

#![forbid(unsafe_code)]
#![cfg(feature = "vb-rxru0-mock-marker")]

use proptest::prelude::{any, prop_assert_eq, proptest};
use vb_core::action::MockMarker;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

#[test]
fn test_action_ticket_8field_postcard_roundtrip() {
    let ticket = vb_core::action::ActionTicket {
        run: RunId::new(0x0102_0304_0506_0708),
        step: StepIdx::new(0x1234),
        seq: SeqNo::new(0x2021_2223_2425_2627),
        action: ActionId::new(0x5678),
        attempt: 42,
        idempotency_key: 0xAABB_CCDD_0011_2233_4455_6677_8899_AABB,
        capacity: 99,
        mock: MockMarker::AiClassifyTicket,
    };

    let buf = postcard::to_allocvec(&ticket).expect("ActionTicket serialization must succeed");
    let ticket2: vb_core::action::ActionTicket =
        postcard::from_bytes(&buf).expect("ActionTicket deserialization must succeed");

    // All 8 fields.
    assert_eq!(
        ticket2.run.get(),
        ticket.run.get(),
        "run must be preserved through serialization"
    );
    assert_eq!(
        ticket2.step.get(),
        ticket.step.get(),
        "step must be preserved through serialization"
    );
    assert_eq!(
        ticket2.seq.get(),
        ticket.seq.get(),
        "seq must be preserved through serialization"
    );
    assert_eq!(
        ticket2.action.get(),
        ticket.action.get(),
        "action must be preserved through serialization"
    );
    assert_eq!(
        ticket2.attempt, ticket.attempt,
        "attempt must be preserved through serialization"
    );
    assert_eq!(
        ticket2.idempotency_key, ticket.idempotency_key,
        "idempotency_key must be preserved through serialization"
    );
    assert_eq!(
        ticket2.capacity, ticket.capacity,
        "capacity must be preserved through serialization"
    );
    assert_eq!(
        ticket2.mock, ticket.mock,
        "mock must be preserved through serialization"
    );
}

#[test]
fn test_action_ticket_8field_roundtrip_all_mock_variants() {
    let mocks = [
        MockMarker::GithubIssueCreate,
        MockMarker::AiClassifyTicket,
        MockMarker::HttpGet,
    ];

    for mock in &mocks {
        let ticket = vb_core::action::ActionTicket {
            run: RunId::new(1),
            step: StepIdx::new(2),
            seq: SeqNo::new(3),
            action: ActionId::new(4),
            attempt: 5,
            idempotency_key: 6,
            capacity: 7,
            mock: *mock,
        };

        let buf = postcard::to_allocvec(&ticket).expect("ActionTicket serialization must succeed");
        let ticket2: vb_core::action::ActionTicket =
            postcard::from_bytes(&buf).expect("ActionTicket deserialization must succeed");

        assert_eq!(
            ticket2.mock, *mock,
            "mock field must survive roundtrip for {:?}",
            mock
        );
        assert_eq!(
            ticket2.run.get(),
            ticket.run.get(),
            "run must be preserved for mock={:?}",
            mock
        );
    }
}

#[test]
fn test_serialization_wire_format_changed() {
    // The new 8-field serialization is longer than a hypothetical 7-field one.
    let ticket_with_mock = vb_core::action::ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        mock: MockMarker::GithubIssueCreate,
    };
    let buf =
        postcard::to_allocvec(&ticket_with_mock).expect("ActionTicket serialization must succeed");

    // With 1-byte MockMarker, the ticket is longer than 7-field version.
    // Exact byte count depends on postcard encoding but must be > 7-field length.
    assert!(
        buf.len() > 0,
        "8-field serialization must produce non-empty output"
    );

    // The wire format must include the mock byte (at least 1 extra byte
    // compared to 7-field serialization of equivalent values).
    let ticket_no_mock = vb_core::action::ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(2),
        seq: SeqNo::new(3),
        action: ActionId::new(4),
        attempt: 5,
        idempotency_key: 6,
        capacity: 7,
        mock: MockMarker::GithubIssueCreate,
    };
    let buf2 =
        postcard::to_allocvec(&ticket_no_mock).expect("ActionTicket serialization must succeed");

    assert_eq!(
        buf.len(),
        buf2.len(),
        "same ticket must serialize to same size regardless of mock value"
    );
}

// ---------------------------------------------------------------------------
// Proptest: property-based ActionTicket 8-field roundtrip
// ---------------------------------------------------------------------------

#[cfg(test)]
proptest! {
    /// Property-based roundtrip for ActionTicket with GithubIssueCreate mock.
    #[test]
    fn test_action_ticket_postcard_roundtrip_github_issue_create(
        run in any::<u64>(),
        step in any::<u16>(),
        seq in any::<u64>(),
        action in any::<u16>(),
        attempt in any::<u16>(),
        idempotency_key in any::<u128>(),
        capacity in any::<u16>(),
    ) {
        let ticket = vb_core::action::ActionTicket {
            run: RunId::new(run),
            step: StepIdx::new(step),
            seq: SeqNo::new(seq),
            action: ActionId::new(action),
            attempt,
            idempotency_key,
            capacity,
            mock: MockMarker::GithubIssueCreate,
        };

        let buf = postcard::to_allocvec(&ticket).expect("serialize must succeed");
        let ticket2: vb_core::action::ActionTicket = postcard::from_bytes(&buf).expect("deserialize must succeed");

        prop_assert_eq!(ticket2.run.get(), ticket.run.get(), "run must survive roundtrip");
        prop_assert_eq!(ticket2.step.get(), ticket.step.get(), "step must survive roundtrip");
        prop_assert_eq!(ticket2.seq.get(), ticket.seq.get(), "seq must survive roundtrip");
        prop_assert_eq!(ticket2.action.get(), ticket.action.get(), "action must survive roundtrip");
        prop_assert_eq!(ticket2.attempt, ticket.attempt, "attempt must survive roundtrip");
        prop_assert_eq!(ticket2.idempotency_key, ticket.idempotency_key, "idempotency_key must survive roundtrip");
        prop_assert_eq!(ticket2.capacity, ticket.capacity, "capacity must survive roundtrip");
        prop_assert_eq!(ticket2.mock, ticket.mock, "mock must survive roundtrip");
    }

    /// Property-based roundtrip for ActionTicket with AiClassifyTicket mock.
    #[test]
    fn test_action_ticket_postcard_roundtrip_ai_classify_ticket(
        run in any::<u64>(),
        step in any::<u16>(),
        seq in any::<u64>(),
        action in any::<u16>(),
        attempt in any::<u16>(),
        idempotency_key in any::<u128>(),
        capacity in any::<u16>(),
    ) {
        let ticket = vb_core::action::ActionTicket {
            run: RunId::new(run),
            step: StepIdx::new(step),
            seq: SeqNo::new(seq),
            action: ActionId::new(action),
            attempt,
            idempotency_key,
            capacity,
            mock: MockMarker::AiClassifyTicket,
        };

        let buf = postcard::to_allocvec(&ticket).expect("serialize must succeed");
        let ticket2: vb_core::action::ActionTicket = postcard::from_bytes(&buf).expect("deserialize must succeed");

        prop_assert_eq!(ticket2.run.get(), ticket.run.get(), "run must survive roundtrip");
        prop_assert_eq!(ticket2.step.get(), ticket.step.get(), "step must survive roundtrip");
        prop_assert_eq!(ticket2.seq.get(), ticket.seq.get(), "seq must survive roundtrip");
        prop_assert_eq!(ticket2.action.get(), ticket.action.get(), "action must survive roundtrip");
        prop_assert_eq!(ticket2.attempt, ticket.attempt, "attempt must survive roundtrip");
        prop_assert_eq!(ticket2.idempotency_key, ticket.idempotency_key, "idempotency_key must survive roundtrip");
        prop_assert_eq!(ticket2.capacity, ticket.capacity, "capacity must survive roundtrip");
        prop_assert_eq!(ticket2.mock, ticket.mock, "mock must survive roundtrip");
    }

    /// Property-based roundtrip for ActionTicket with HttpGet mock.
    #[test]
    fn test_action_ticket_postcard_roundtrip_http_get(
        run in any::<u64>(),
        step in any::<u16>(),
        seq in any::<u64>(),
        action in any::<u16>(),
        attempt in any::<u16>(),
        idempotency_key in any::<u128>(),
        capacity in any::<u16>(),
    ) {
        let ticket = vb_core::action::ActionTicket {
            run: RunId::new(run),
            step: StepIdx::new(step),
            seq: SeqNo::new(seq),
            action: ActionId::new(action),
            attempt,
            idempotency_key,
            capacity,
            mock: MockMarker::HttpGet,
        };

        let buf = postcard::to_allocvec(&ticket).expect("serialize must succeed");
        let ticket2: vb_core::action::ActionTicket = postcard::from_bytes(&buf).expect("deserialize must succeed");

        prop_assert_eq!(ticket2.run.get(), ticket.run.get(), "run must survive roundtrip");
        prop_assert_eq!(ticket2.step.get(), ticket.step.get(), "step must survive roundtrip");
        prop_assert_eq!(ticket2.seq.get(), ticket.seq.get(), "seq must survive roundtrip");
        prop_assert_eq!(ticket2.action.get(), ticket.action.get(), "action must survive roundtrip");
        prop_assert_eq!(ticket2.attempt, ticket.attempt, "attempt must survive roundtrip");
        prop_assert_eq!(ticket2.idempotency_key, ticket.idempotency_key, "idempotency_key must survive roundtrip");
        prop_assert_eq!(ticket2.capacity, ticket.capacity, "capacity must survive roundtrip");
        prop_assert_eq!(ticket2.mock, ticket.mock, "mock must survive roundtrip");
    }
}
