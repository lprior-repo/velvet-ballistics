#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::approx_constant,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::as_ref_should_use,
    clippy::useless_vec,
    clippy::useless_conversion,
    clippy::let_underscore_must_use
)]
#![forbid(unsafe_code)]

//! Integration tests for vb-mrwe.5: storage record kind parity for StepSucceeded.
//!
//! These tests verify the behavior contract for MRWE5:
//! - B1: StepSucceeded and SlotWrittenEvent use distinct record kinds (29 and 12)
//! - B2: Semantic decode rejects kind/payload mismatches before semantic use
//! - B3: Canonical round-trip preserves envelope kind and variant
//! - B5: ValidatedJournalRecord parity witness construction
//!
//! Gap coverage:
//! - parse_event integration test for mismatch rejection (PS-MRWE5-002)
//! - ValidatedJournalRecord parity witness construction (PS-MRWE5-002)

use vb_core::{RunId, SlotIdx, StepIdx};
use vb_storage::codec::{
    JournalKindCompatibility, JournalSemanticDecodeDecision, classify_journal_semantic_decode,
    decode_journal_event, decode_validated_journal_record, encode_journal_event_record,
    encode_record,
};
use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
use vb_storage::journal::parse_event;
use vb_storage::{EventSeq, JournalError, JournalEvent, RecordKind};

// =============================================================================
// B2 / C2: parse_event rejects kind/payload mismatches (integration test)
// =============================================================================

/// Integration test: parse_event rejects StepSucceeded payload under SlotWritten envelope.
///
/// This is the gap-filling test for PS-MRWE5-002: "Integration test for parse_event
/// reject path". The proptest vb_mrwe5_decode_reject_props covers this with generated
/// inputs; this deterministic test provides a concrete failing-first example.
#[test]
fn parse_event_rejects_step_succeeded_under_slot_written_envelope() {
    // Given: a valid StepSucceeded event
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(42),
        step: StepIdx::new(7),
        output: SlotIdx::new(3),
    };

    // When: we encode the event with a WRONG envelope kind (SlotWritten instead of StepSucceeded)
    // This creates a kind/payload mismatch that should be rejected
    let mismatched_bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::SlotWritten, // WRONG - should be StepSucceeded
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("valid event should encode");

    // Then: parse_event must reject the mismatched record
    let result = parse_event(&mismatched_bytes);
    assert!(
        matches!(result, Err(JournalError::InvalidEvent)),
        "parse_event must reject StepSucceeded under SlotWritten envelope, got {:?}",
        result
    );
}

/// Integration test: parse_event rejects SlotWrittenEvent payload under StepSucceeded envelope.
///
/// Second direction of the kind/payload mismatch rejection for parse_event.
#[test]
fn parse_event_rejects_slot_written_under_step_succeeded_envelope() {
    // Given: a valid SlotWrittenEvent
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(99),
        slot: SlotIdx::new(5),
        value: None,
        extra: None,
        attempt: 1,
    };

    // When: we encode with a WRONG envelope kind (StepSucceeded instead of SlotWritten)
    let mismatched_bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::StepSucceeded, // WRONG - should be SlotWritten
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("valid event should encode");

    // Then: parse_event must reject the mismatched record
    let result = parse_event(&mismatched_bytes);
    assert!(
        matches!(result, Err(JournalError::InvalidEvent)),
        "parse_event must reject SlotWrittenEvent under StepSucceeded envelope, got {:?}",
        result
    );
}

/// Integration test: parse_event accepts canonical StepSucceeded encoding.
#[test]
fn parse_event_accepts_canonical_step_succeeded() {
    // Given: a valid StepSucceeded event
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(13),
        step: StepIdx::new(2),
        output: SlotIdx::new(3),
    };

    // When: encoded with the CORRECT envelope kind
    let canonical_bytes = encode_journal_event_record(&event).expect("valid event should encode");

    // Then: parse_event must accept it and return the correct variant
    let parsed = parse_event(&canonical_bytes).expect("canonical encoding must parse successfully");
    assert!(
        matches!(parsed, JournalEvent::StepSucceeded { run, seq, step, output }
            if run == RunId::new(1)
            && seq == EventSeq::new(13)
            && step == StepIdx::new(2)
            && output == SlotIdx::new(3)
        ),
        "parsed event must be StepSucceeded with correct fields, got {:?}",
        parsed
    );
}

/// Integration test: parse_event accepts canonical SlotWrittenEvent encoding.
#[test]
fn parse_event_accepts_canonical_slot_written_event() {
    // Given: a valid SlotWrittenEvent
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(42),
        seq: EventSeq::new(7),
        slot: SlotIdx::new(11),
        value: None,
        extra: None,
        attempt: 3,
    };

    // When: encoded with the CORRECT envelope kind
    let canonical_bytes = encode_journal_event_record(&event).expect("valid event should encode");

    // Then: parse_event must accept it
    let parsed = parse_event(&canonical_bytes).expect("canonical encoding must parse successfully");
    assert!(
        matches!(parsed, JournalEvent::SlotWrittenEvent { slot, attempt, .. }
            if slot == SlotIdx::new(11) && attempt == 3
        ),
        "parsed event must be SlotWrittenEvent with correct fields, got {:?}",
        parsed
    );
}

// =============================================================================
// B5 / C5: ValidatedJournalRecord parity witness construction
// =============================================================================

/// Integration test: ValidatedJournalRecord::try_new succeeds when parity holds.
///
/// This is the gap-filling test for PS-MRWE5-002: "ValidatedJournalRecord parity witness
/// construction not exercised in integration tests".
#[test]
fn validated_journal_record_succeeds_when_parity_holds() {
    // Given: a canonical StepSucceeded event encoded correctly
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(42),
        step: StepIdx::new(7),
        output: SlotIdx::new(3),
    };
    let bytes = encode_journal_event_record(&event).expect("valid event should encode");

    // When: decoded through decode_validated_journal_record
    let record = decode_validated_journal_record(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("canonical encoding must decode successfully");

    // Then: ValidatedJournalRecord::parity() returns ExactJournalKindParity
    let parity = record.parity();
    assert!(
        parity.is_exact_match(),
        "parity must be exact match for canonical encoding"
    );
    assert_eq!(
        parity.envelope_kind(),
        RecordKind::StepSucceeded.id(),
        "envelope_kind must be 29 (StepSucceeded)"
    );
    assert_eq!(
        parity.payload_kind(),
        RecordKind::StepSucceeded.id(),
        "payload_kind must be 29 (StepSucceeded)"
    );
}

/// Integration test: ValidatedJournalRecord::try_new succeeds for SlotWrittenEvent.
#[test]
fn validated_journal_record_succeeds_for_slot_written_event() {
    // Given: a canonical SlotWrittenEvent encoded correctly
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(99),
        slot: SlotIdx::new(5),
        value: None,
        extra: None,
        attempt: 1,
    };
    let bytes = encode_journal_event_record(&event).expect("valid event should encode");

    // When: decoded through decode_validated_journal_record
    let record = decode_validated_journal_record(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("canonical encoding must decode successfully");

    // Then: ValidatedJournalRecord::parity() returns ExactJournalKindParity
    let parity = record.parity();
    assert!(
        parity.is_exact_match(),
        "parity must be exact match for canonical SlotWrittenEvent"
    );
    assert_eq!(
        parity.envelope_kind(),
        RecordKind::SlotWritten.id(),
        "envelope_kind must be 12 (SlotWritten)"
    );
    assert_eq!(
        parity.payload_kind(),
        RecordKind::SlotWritten.id(),
        "payload_kind must be 12 (SlotWritten)"
    );
}

/// Integration test: ValidatedJournalRecord::try_new fails when kind/payload mismatch.
#[test]
fn validated_journal_record_fails_when_kind_payload_mismatch() {
    // Given: a StepSucceeded event encoded with SlotWritten envelope
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(42),
        step: StepIdx::new(7),
        output: SlotIdx::new(3),
    };
    let mismatched_bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::SlotWritten, // WRONG
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("valid event should encode");

    // When: decoded through decode_validated_journal_record
    // Then: must fail with InvalidEvent
    let result = decode_validated_journal_record(
        &mismatched_bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::InvalidEvent)),
        "mismatched encoding must be rejected, got {:?}",
        result
    );
}

/// Integration test: ValidatedJournalRecord::try_new fails for structurally invalid event.
#[test]
fn validated_journal_record_fails_when_event_structurally_invalid() {
    // Given: a StepSucceeded with run_id = 0 (which is invalid)
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(0), // INVALID - run_id must be non-zero
        seq: EventSeq::new(42),
        step: StepIdx::new(7),
        output: SlotIdx::new(3),
    };
    // Note: is_valid() returns false for run_id = 0
    assert!(!event.is_valid(), "run_id=0 must be invalid");

    // When: encoded with correct envelope and decoded through decode_validated_journal_record
    let bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::StepSucceeded,
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("event should encode");

    // Then: must fail because is_valid() returns false
    let result = decode_validated_journal_record(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    assert!(
        matches!(result, Err(JournalError::InvalidEvent)),
        "invalid event must be rejected, got {:?}",
        result
    );
}

/// Integration test: decode_validated_journal_record and decode_journal_event agree on
/// canonical records but diverge on mismatched records.
#[test]
fn validated_journal_record_vs_generic_decode_parity() {
    // Given: a canonical StepSucceeded event
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(13),
        step: StepIdx::new(2),
        output: SlotIdx::new(3),
    };
    let bytes = encode_journal_event_record(&event).expect("valid event should encode");

    // Both decode paths agree on canonical records
    let validated_result = decode_validated_journal_record(
        &bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    let generic_result =
        decode_journal_event(&bytes, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);

    let validated_record =
        validated_result.expect("canonical encoding must decode via validated path");
    assert_eq!(
        validated_record.parity().envelope_kind(),
        RecordKind::StepSucceeded.id(),
        "validated record must show StepSucceeded envelope kind"
    );
    let (generic_envelope, _generic_event) =
        generic_result.expect("canonical encoding must decode via generic path");
    assert_eq!(
        generic_envelope.record_kind,
        RecordKind::StepSucceeded.id(),
        "generic record must show StepSucceeded record kind"
    );

    // Given: a mismatched encoding
    let mismatched_bytes = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::SlotWritten, // WRONG
        event.seq().get(),
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .expect("valid event should encode");

    // Both decode paths reject mismatched records
    let validated_mismatch = decode_validated_journal_record(
        &mismatched_bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    let generic_mismatch = decode_journal_event(
        &mismatched_bytes,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );

    assert!(
        matches!(validated_mismatch, Err(JournalError::InvalidEvent)),
        "mismatch must be rejected via validated path"
    );
    assert!(
        matches!(generic_mismatch, Err(JournalError::InvalidEvent)),
        "mismatch must be rejected via generic path"
    );
}

// =============================================================================
// B3: SlotWrittenEvent with value=Some(bytes) roundtrip
// This is covered by proptest in vb_mrwe5_roundtrip_props.rs (slot with value).
// Here we provide a deterministic integration test as gap-filling for PS-MRWE5-003.
// =============================================================================

/// Integration test: SlotWrittenEvent with value=Some(bytes) roundtrips correctly.
///
/// This is the gap-filling test for PS-MRWE5-003: "SlotWrittenEvent with
/// value=Some(_) not covered in roundtrip proptest".
#[test]
fn slot_written_event_with_value_some_bytes_roundtrips() {
    // Given: a SlotWrittenEvent with a value
    let original_value = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(77),
        slot: SlotIdx::new(3),
        value: Some(original_value.clone()),
        extra: None,
        attempt: 1,
    };

    // When: encoded and decoded through the full pipeline
    let bytes = encode_journal_event_record(&event).expect("valid event should encode");
    let parsed = parse_event(&bytes).expect("canonical encoding must parse successfully");

    // Then: the parsed event must have the same value
    match parsed {
        JournalEvent::SlotWrittenEvent {
            value: Some(decoded_value),
            ..
        } => {
            assert_eq!(
                decoded_value, original_value,
                "value must survive roundtrip"
            );
        }
        other => panic!("expected SlotWrittenEvent with value, got {:?}", other),
    }
}

/// Integration test: SlotWrittenEvent with large value bytes roundtrips correctly.
#[test]
fn slot_written_event_with_large_value_roundtrips() {
    // Given: a SlotWrittenEvent with a large value (1KB)
    let large_value = vec![0xAB_u8; 1024];
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(88),
        slot: SlotIdx::new(u16::MAX.into()),
        value: Some(large_value.clone()),
        extra: None,
        attempt: u16::MAX,
    };

    // When: roundtripped
    let bytes = encode_journal_event_record(&event).expect("valid event should encode");
    let parsed = parse_event(&bytes).expect("canonical encoding must parse successfully");

    // Then: the large value must survive intact
    match parsed {
        JournalEvent::SlotWrittenEvent {
            value: Some(decoded_value),
            ..
        } => {
            assert_eq!(
                decoded_value.len(),
                large_value.len(),
                "large value length must be preserved"
            );
            assert_eq!(
                decoded_value, large_value,
                "large value content must be preserved"
            );
        }
        other => panic!(
            "expected SlotWrittenEvent with large value, got {:?}",
            other
        ),
    }
}

// =============================================================================
// B1: Kind parity assertions (deterministic)
// =============================================================================

/// Integration test: StepSucceeded encodes with RecordKind::StepSucceeded (id=29).
#[test]
fn step_succeeded_encodes_with_record_kind_step_succeeded_id_29() {
    let event = JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        output: SlotIdx::new(0),
    };
    assert_eq!(
        event.record_kind(),
        RecordKind::StepSucceeded,
        "StepSucceeded must use RecordKind::StepSucceeded"
    );
    assert_eq!(
        event.record_kind().id(),
        29,
        "RecordKind::StepSucceeded.id() must be 29"
    );
}

/// Integration test: SlotWrittenEvent encodes with RecordKind::SlotWritten (id=12).
#[test]
fn slot_written_event_encodes_with_record_kind_slot_written_id_12() {
    let event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        slot: SlotIdx::new(0),
        value: None,
        extra: None,
        attempt: 1,
    };
    assert_eq!(
        event.record_kind(),
        RecordKind::SlotWritten,
        "SlotWrittenEvent must use RecordKind::SlotWritten"
    );
    assert_eq!(
        event.record_kind().id(),
        12,
        "RecordKind::SlotWritten.id() must be 12"
    );
}

/// Integration test: StepSucceeded and SlotWrittenEvent kinds are never equal.
#[test]
fn step_succeeded_and_slot_written_record_kinds_are_never_equal() {
    let step_event = JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        step: StepIdx::new(0),
        output: SlotIdx::new(0),
    };
    let slot_event = JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        slot: SlotIdx::new(0),
        value: None,
        extra: None,
        attempt: 1,
    };
    assert_ne!(
        step_event.record_kind(),
        slot_event.record_kind(),
        "StepSucceeded and SlotWrittenEvent must use different record kinds"
    );
}

// =============================================================================
// B4: classify_journal_semantic_decode correctness
// =============================================================================

/// Unit-style integration test: classify_journal_semantic_decode returns
/// KindPayloadMismatch for known mismatch.
#[test]
fn classify_journal_semantic_decode_returns_kind_payload_mismatch_for_known_mismatch() {
    // StepSucceeded envelope (29) with SlotWritten payload (12)
    let decision = classify_journal_semantic_decode(
        29,   // envelope_kind
        12,   // payload_kind (SlotWritten)
        true, // event_valid
    );
    assert_eq!(
        decision,
        JournalSemanticDecodeDecision::KindPayloadMismatch,
        "envelope=29, payload=12 must be KindPayloadMismatch"
    );

    // SlotWritten envelope (12) with StepSucceeded payload (29)
    let decision = classify_journal_semantic_decode(
        12,   // envelope_kind
        29,   // payload_kind (StepSucceeded)
        true, // event_valid
    );
    assert_eq!(
        decision,
        JournalSemanticDecodeDecision::KindPayloadMismatch,
        "envelope=12, payload=29 must be KindPayloadMismatch"
    );
}

/// Unit-style integration test: classify_journal_semantic_decode returns
/// SemanticSuccess for exact match.
#[test]
fn classify_journal_semantic_decode_returns_semantic_success_for_exact_match() {
    let decision = classify_journal_semantic_decode(
        29,   // envelope_kind
        29,   // payload_kind
        true, // event_valid
    );
    assert_eq!(
        decision,
        JournalSemanticDecodeDecision::SemanticSuccess,
        "exact match must be SemanticSuccess"
    );

    let decision = classify_journal_semantic_decode(
        12,   // envelope_kind
        12,   // payload_kind
        true, // event_valid
    );
    assert_eq!(
        decision,
        JournalSemanticDecodeDecision::SemanticSuccess,
        "exact match must be SemanticSuccess"
    );
}

/// Unit-style integration test: classify_journal_semantic_decode returns
/// InvalidEvent when event_valid is false.
#[test]
fn classify_journal_semantic_decode_returns_invalid_event_when_event_valid_false() {
    let decision = classify_journal_semantic_decode(
        29,    // envelope_kind
        29,    // payload_kind
        false, // event_valid = false
    );
    assert_eq!(
        decision,
        JournalSemanticDecodeDecision::InvalidEvent,
        "invalid event must yield InvalidEvent even on exact match"
    );
}

// =============================================================================
// B4 / C4: classify_journal_kind_compatibility
// =============================================================================

/// Integration test: classify_journal_kind_compatibility returns ExactMatch for same ids.
#[test]
fn classify_journal_kind_compatibility_exact_match_for_same_ids() {
    assert_eq!(
        vb_storage::codec::classify_journal_kind_compatibility(29, 29),
        JournalKindCompatibility::ExactMatch,
        "same ids must be ExactMatch"
    );
    assert_eq!(
        vb_storage::codec::classify_journal_kind_compatibility(12, 12),
        JournalKindCompatibility::ExactMatch,
        "same ids must be ExactMatch"
    );
}

/// Integration test: classify_journal_kind_compatibility returns RejectedMismatch for different ids.
#[test]
fn classify_journal_kind_compatibility_rejected_mismatch_for_different_ids() {
    assert_eq!(
        vb_storage::codec::classify_journal_kind_compatibility(29, 12),
        JournalKindCompatibility::RejectedMismatch,
        "29 vs 12 must be RejectedMismatch"
    );
    assert_eq!(
        vb_storage::codec::classify_journal_kind_compatibility(12, 29),
        JournalKindCompatibility::RejectedMismatch,
        "12 vs 29 must be RejectedMismatch"
    );
}

/// Integration test: mismatch matrix - all non-equal pairs return RejectedMismatch.
#[test]
fn mismatch_matrix_all_non_equal_pairs_return_rejected_mismatch() {
    // Test a representative sample of non-equal pairs
    let test_pairs = [
        (10, 12),
        (10, 29),
        (12, 10),
        (12, 29),
        (29, 10),
        (29, 12),
        (10, 20),
        (20, 29),
        (15, 25),
    ];

    for (a, b) in test_pairs {
        if a != b {
            let result = vb_storage::codec::classify_journal_kind_compatibility(a, b);
            assert_eq!(
                result,
                JournalKindCompatibility::RejectedMismatch,
                "({}, {}) must be RejectedMismatch",
                a,
                b
            );
        }
    }
}
