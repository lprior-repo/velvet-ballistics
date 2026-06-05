#![forbid(unsafe_code)]

//! Executable production bridge for `obl-vb-mrwe-5-ps001-verus-001`,
//! `obl-vb-mrwe-5-ps002-verus-006`, `obl-vb-mrwe-5-ps003-verus-011`,
//! `obl-vb-mrwe-5-ps004-verus-016`, and the parallel Flux obligations.
//! The Verus files remain compact mathematical artifacts; this bridge test is
//! the executable evidence that their constants and predicates match the real
//! `vb_storage` seams used by writes and semantic decode. This file is test-only
//! verification wiring; it intentionally calls the production/source seams that
//! the Verus models name instead of reconstructing the old standalone model.

use crate::{
    EventSeq, JournalEvent, JournalEventKindClass, JournalSemanticDecodeDecision,
    classify_journal_semantic_decode,
};
use vb_core::{RunId, SlotIdx, StepIdx};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeDecision {
    SemanticSuccess,
    KindPayloadMismatch,
}

#[must_use]
pub fn bridge_event_kind_id(event: &JournalEvent) -> u16 {
    event.record_kind_id()
}

#[must_use]
pub fn bridge_event_kind_class(event: &JournalEvent) -> JournalEventKindClass {
    event.kind_class()
}

#[must_use]
pub fn bridge_decode_decision(envelope_kind: u16, event: &JournalEvent) -> BridgeDecision {
    match classify_journal_semantic_decode(envelope_kind, event.record_kind_id(), event.is_valid())
    {
        JournalSemanticDecodeDecision::SemanticSuccess => BridgeDecision::SemanticSuccess,
        JournalSemanticDecodeDecision::KindPayloadMismatch
        | JournalSemanticDecodeDecision::InvalidEvent => BridgeDecision::KindPayloadMismatch,
    }
}

#[must_use]
pub fn bridge_step_succeeded_event() -> JournalEvent {
    JournalEvent::StepSucceeded {
        run: RunId::new(1),
        seq: EventSeq::new(1),
        step: StepIdx::new(2),
        output: SlotIdx::new(3),
    }
}

#[must_use]
pub fn bridge_slot_written_event() -> JournalEvent {
    JournalEvent::SlotWrittenEvent {
        run: RunId::new(1),
        seq: EventSeq::new(2),
        slot: SlotIdx::new(3),
        value: None,
        extra: None,
        attempt: 1,
    }
}

#[cfg(test)]
mod tests {
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
}
