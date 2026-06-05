#![forbid(unsafe_code)]

pub mod vb_mrwe5_compat_kind_family;
pub mod vb_mrwe5_decode_reject;
pub mod vb_mrwe5_kind_parity;
pub mod vb_mrwe5_roundtrip;

#[cfg(test)]
mod tests {
    use super::{
        vb_mrwe5_compat_kind_family::{
            CompatibilityClass, production_classify, production_compatibility_code,
        },
        vb_mrwe5_decode_reject::{
            DecodeState, production_semantic_decode_code, production_semantic_decode_state,
        },
        vb_mrwe5_kind_parity::{
            parity_witness_exact_match, production_exact_match, step_and_slot_are_distinct,
        },
        vb_mrwe5_roundtrip::{
            RoundTripVariant, expected_roundtrip_kind, production_roundtrip_accepts_exact_variant,
        },
    };
    use crate::{EventSeq, JournalEvent, RecordKind};
    use vb_core::{RunId, SlotIdx, StepIdx};

    fn step_event() -> JournalEvent {
        JournalEvent::StepSucceeded {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            step: StepIdx::new(2),
            output: SlotIdx::new(3),
        }
    }

    fn slot_event() -> JournalEvent {
        JournalEvent::SlotWrittenEvent {
            run: RunId::new(1),
            seq: EventSeq::new(2),
            slot: SlotIdx::new(3),
            value: None,
            extra: None,
            attempt: 1,
        }
    }

    #[test]
    fn mrwe5_flux_bridge_binds_to_production_event_helpers() {
        let step = step_event();
        let slot = slot_event();

        assert!(step_and_slot_are_distinct());
        assert!(production_exact_match(
            RecordKind::StepSucceeded.id(),
            step.record_kind_id()
        ));
        assert!(production_exact_match(
            RecordKind::SlotWritten.id(),
            slot.record_kind_id()
        ));
        assert!(!production_exact_match(
            RecordKind::SlotWritten.id(),
            step.record_kind_id()
        ));
        assert!(parity_witness_exact_match(
            RecordKind::StepSucceeded.id(),
            step.record_kind_id()
        ));

        assert_eq!(
            production_semantic_decode_state(
                RecordKind::StepSucceeded.id(),
                step.record_kind_id(),
                step.is_valid()
            ),
            DecodeState::ValidatedSemantic
        );
        assert_eq!(
            production_semantic_decode_state(
                RecordKind::SlotWritten.id(),
                step.record_kind_id(),
                step.is_valid()
            ),
            DecodeState::RejectedMismatch
        );
        assert_eq!(
            production_semantic_decode_code(
                RecordKind::StepSucceeded.id(),
                step.record_kind_id(),
                step.is_valid()
            ),
            1
        );

        assert_eq!(
            expected_roundtrip_kind(RoundTripVariant::StepSucceeded),
            RecordKind::StepSucceeded.id()
        );
        assert_eq!(
            expected_roundtrip_kind(RoundTripVariant::SlotWrittenEvent),
            RecordKind::SlotWritten.id()
        );
        assert!(production_roundtrip_accepts_exact_variant(
            RoundTripVariant::StepSucceeded,
            RecordKind::StepSucceeded.id(),
            true
        ));

        assert_eq!(
            production_classify(RecordKind::StepSucceeded.id(), step.record_kind_id()),
            CompatibilityClass::ExactMatch
        );
        assert_eq!(
            production_classify(RecordKind::SlotWritten.id(), step.record_kind_id()),
            CompatibilityClass::RejectedMismatch
        );
        assert_eq!(
            production_compatibility_code(RecordKind::StepSucceeded.id(), step.record_kind_id()),
            1
        );
    }
}
