use super::{
    BridgeDecision, bridge_decode_decision, bridge_event_kind_class, bridge_event_kind_id,
    bridge_slot_written_event, bridge_step_succeeded_event,
};
use crate::{
    JournalEventKindClass, JournalKindCompatibility, JournalSemanticDecodeDecision, RecordKind,
    RecordKindFamilyDecision, classify_journal_kind_compatibility,
    classify_journal_semantic_decode, classify_record_kind_family,
    constants::MAGIC_JOURNAL_EVENT, is_journal_record_kind,
};

#[test]
fn mrwe5_verus_bridge_binds_to_production_seams() {
    let step = bridge_step_succeeded_event();
    let slot = bridge_slot_written_event();

    assert_eq!(bridge_event_kind_id(&step), RecordKind::StepSucceeded.id());
    assert_eq!(bridge_event_kind_id(&slot), RecordKind::SlotWritten.id());
    assert_eq!(
        bridge_event_kind_class(&step),
        JournalEventKindClass::StepSucceeded
    );
    assert_eq!(
        bridge_event_kind_class(&slot),
        JournalEventKindClass::SlotWrittenEvent
    );
    assert_eq!(
        step.kind_class().canonical_record_kind_id(),
        Some(RecordKind::StepSucceeded.id())
    );
    assert_eq!(
        slot.kind_class().canonical_record_kind_id(),
        Some(RecordKind::SlotWritten.id())
    );
    assert!(step.has_canonical_envelope_kind(RecordKind::StepSucceeded.id()));
    assert!(slot.has_canonical_envelope_kind(RecordKind::SlotWritten.id()));
    assert!(!step.has_canonical_envelope_kind(RecordKind::SlotWritten.id()));
    assert_ne!(RecordKind::StepSucceeded.id(), RecordKind::SlotWritten.id());

    assert_eq!(
        classify_journal_kind_compatibility(
            RecordKind::StepSucceeded.id(),
            step.record_kind_id(),
        ),
        JournalKindCompatibility::ExactMatch
    );
    assert_eq!(
        classify_journal_kind_compatibility(
            RecordKind::SlotWritten.id(),
            step.record_kind_id()
        ),
        JournalKindCompatibility::RejectedMismatch
    );
    assert_eq!(
        classify_journal_semantic_decode(
            RecordKind::SlotWritten.id(),
            step.record_kind_id(),
            step.is_valid(),
        ),
        JournalSemanticDecodeDecision::KindPayloadMismatch
    );

    assert_eq!(
        bridge_decode_decision(RecordKind::StepSucceeded.id(), &step),
        BridgeDecision::SemanticSuccess
    );
    assert_eq!(
        bridge_decode_decision(RecordKind::SlotWritten.id(), &step),
        BridgeDecision::KindPayloadMismatch
    );
    assert_eq!(
        bridge_decode_decision(RecordKind::SlotWritten.id(), &slot),
        BridgeDecision::SemanticSuccess
    );
    assert_eq!(
        bridge_decode_decision(RecordKind::StepSucceeded.id(), &slot),
        BridgeDecision::KindPayloadMismatch
    );

    assert!(is_journal_record_kind(RecordKind::StepSucceeded.id()));
    assert!(is_journal_record_kind(RecordKind::SlotWritten.id()));
    assert_eq!(
        classify_record_kind_family(MAGIC_JOURNAL_EVENT, RecordKind::StepSucceeded.id()),
        RecordKindFamilyDecision::Accepted
    );
    assert_eq!(
        classify_record_kind_family(MAGIC_JOURNAL_EVENT, RecordKind::SlotWritten.id()),
        RecordKindFamilyDecision::Accepted
    );
}
