use super::common::*;

/// Unit-style integration test: classify_journal_semantic_decode returns
/// KindPayloadMismatch for known mismatch.
#[test]
fn classify_journal_semantic_decode_returns_kind_payload_mismatch_for_known_mismatch() {
    let decision = classify_journal_semantic_decode(29, 12, true);
    assert_eq!(
        decision,
        JournalSemanticDecodeDecision::KindPayloadMismatch,
        "envelope=29, payload=12 must be KindPayloadMismatch"
    );

    let decision = classify_journal_semantic_decode(12, 29, true);
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
    let decision = classify_journal_semantic_decode(29, 29, true);
    assert_eq!(
        decision,
        JournalSemanticDecodeDecision::SemanticSuccess,
        "exact match must be SemanticSuccess"
    );

    let decision = classify_journal_semantic_decode(12, 12, true);
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
    let decision = classify_journal_semantic_decode(29, 29, false);
    assert_eq!(
        decision,
        JournalSemanticDecodeDecision::InvalidEvent,
        "invalid event must yield InvalidEvent even on exact match"
    );
}

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
