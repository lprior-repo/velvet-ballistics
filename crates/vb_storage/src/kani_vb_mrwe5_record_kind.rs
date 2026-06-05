// Kani proof: StepSucceeded and SlotWrittenEvent have distinct record_kind values.
// Obligation: vb-mrwe.5 PO-001

#[cfg(kani)]
mod kani_vb_mrwe5_record_kind_injectivity {
    use crate::{EventSeq, JournalEvent, RecordKind};
    use vb_core::{RunId, SlotIdx, StepIdx};

    /// PO-001: record_kind() is injective for StepSucceeded and SlotWrittenEvent.
    #[kani::proof]
    fn vb_mrwe5_record_kind_injectivity() {
        let step_succeeded = JournalEvent::StepSucceeded {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        };

        let slot_written = JournalEvent::SlotWrittenEvent {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            slot: SlotIdx::new(0),
            value: None,
            extra: None,
            attempt: 1,
        };

        let kind_succeeded = step_succeeded.record_kind();
        let kind_written = slot_written.record_kind();

        kani::assert(
            kind_succeeded != kind_written,
            "StepSucceeded and SlotWrittenEvent must have different RecordKind values",
        );

        kani::assert(
            kind_succeeded == RecordKind::StepSucceeded,
            "StepSucceeded must map to RecordKind::StepSucceeded",
        );

        kani::assert(
            kind_written == RecordKind::SlotWritten,
            "SlotWrittenEvent must map to RecordKind::SlotWritten",
        );
    }
}
