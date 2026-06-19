#![forbid(unsafe_code)]

//! HVR-PO-STORAGE-006: generated executable classification properties.

use proptest::prelude::*;
use vb_storage::JournalError;
use vb_storage::codec::{
    JournalKindCompatibility, JournalSemanticDecodeDecision, RecordKindFamilyDecision,
    classify_journal_kind_compatibility, classify_journal_semantic_decode,
    classify_record_kind_family, validate_known_record_kind, validate_record_kind_family,
};

fn expected_known_kind(kind: u16) -> bool {
    matches!(kind, 1 | 2 | 3 | 7 | 10..=29 | 30 | 40 | 50)
}

fn expected_journal_family_kind(kind: u16) -> bool {
    (10..=29).contains(&kind)
}

fn expected_family_accepts(magic: u32, kind: u16) -> bool {
    match magic {
        vb_storage::MAGIC_WORKFLOW_SOURCE => kind == vb_storage::RecordKind::WorkflowSource.id(),
        vb_storage::MAGIC_COMPILED_ARTIFACT => kind == vb_storage::RecordKind::CompiledIr.id(),
        vb_storage::MAGIC_JOURNAL_EVENT => expected_journal_family_kind(kind),
        vb_storage::MAGIC_SNAPSHOT => kind == vb_storage::RecordKind::Snapshot.id(),
        vb_storage::MAGIC_BLOB => kind == vb_storage::RecordKind::Blob.id(),
        vb_storage::MAGIC_INDEX_RECORD => matches!(kind, 3 | 50),
        vb_storage::MAGIC_RECOVERY_STAMP => kind == vb_storage::RecordKind::RecoveryStamp.id(),
        _ => false,
    }
}

proptest! {
    #[test]
    fn vb_god2f_storage_classification_properties(
        magic in any::<u32>(),
        kind in any::<u16>(),
        payload_kind in any::<u16>(),
        event_valid in any::<bool>(),
    ) {
        let known = validate_known_record_kind(kind);
        if expected_known_kind(kind) {
            prop_assert!(known.is_ok(), "known kind {kind} should validate");
        } else {
            prop_assert!(
                matches!(known, Err(JournalError::UnknownRecordKind { kind: observed }) if observed == kind),
                "unknown kind {kind} should produce typed UnknownRecordKind, got {known:?}"
            );
        }

        let family = classify_record_kind_family(magic, kind);
        let family_result = validate_record_kind_family(magic, kind);
        if expected_family_accepts(magic, kind) {
            prop_assert_eq!(family, RecordKindFamilyDecision::Accepted);
            prop_assert!(family_result.is_ok(), "expected family acceptance for {magic:#010x}/{kind}");
        } else {
            prop_assert_eq!(family, RecordKindFamilyDecision::Rejected);
            prop_assert!(
                matches!(family_result, Err(JournalError::RecordKindFamilyMismatch { magic: observed_magic, kind: observed_kind }) if observed_magic == magic && observed_kind == kind),
                "expected typed family mismatch for {magic:#010x}/{kind}, got {family_result:?}"
            );
        }

        let compatibility = classify_journal_kind_compatibility(kind, payload_kind);
        let expected_compatibility = if kind == payload_kind {
            JournalKindCompatibility::ExactMatch
        } else {
            JournalKindCompatibility::RejectedMismatch
        };
        prop_assert_eq!(compatibility, expected_compatibility);

        let semantic = classify_journal_semantic_decode(kind, payload_kind, event_valid);
        let expected_semantic = if kind != payload_kind {
            JournalSemanticDecodeDecision::KindPayloadMismatch
        } else if event_valid {
            JournalSemanticDecodeDecision::SemanticSuccess
        } else {
            JournalSemanticDecodeDecision::InvalidEvent
        };
        prop_assert_eq!(semantic, expected_semantic);
    }
}

#[test]
fn vb_god2f_storage_classification_matrix_covers_documented_boundaries() {
    let accepted = [10u16, 12, 28, 29];
    for kind in accepted {
        assert_eq!(
            classify_record_kind_family(vb_storage::MAGIC_JOURNAL_EVENT, kind),
            RecordKindFamilyDecision::Accepted
        );
    }
    let rejected = [0u16, 9, 30, u16::MAX];
    for kind in rejected {
        assert_eq!(
            classify_record_kind_family(vb_storage::MAGIC_JOURNAL_EVENT, kind),
            RecordKindFamilyDecision::Rejected
        );
    }
}
