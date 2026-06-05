#![forbid(unsafe_code)]

//! Proptest artifact for `obl-vb-mrwe-5-ps001-proptest-004`.

use proptest::prelude::*;
use vb_core::{RunId, SlotIdx, StepIdx};
use vb_storage::{EventSeq, JournalEvent, RecordKind};

fn run_id_strategy() -> impl Strategy<Value = RunId> {
    (1_u64..=u64::from(u16::MAX)).prop_map(RunId::new)
}

fn event_seq_strategy() -> impl Strategy<Value = EventSeq> {
    (0_u64..=u64::from(u16::MAX)).prop_map(EventSeq::new)
}

proptest! {
    #[test]
    fn vb_mrwe5_new_writes_are_kind_congruent(
        run in run_id_strategy(),
        seq in event_seq_strategy(),
        step in any::<u16>(),
        output in any::<u16>(),
        slot in any::<u16>(),
        attempt in 1_u16..=u16::MAX,
    ) {
        let step_event = JournalEvent::StepSucceeded {
            run,
            seq,
            step: StepIdx::new(step),
            output: SlotIdx::new(output),
        };
        let slot_event = JournalEvent::SlotWrittenEvent {
            run,
            seq,
            slot: SlotIdx::new(slot),
            value: None,
            extra: None,
            attempt,
        };

        prop_assert_eq!(step_event.record_kind(), RecordKind::StepSucceeded);
        prop_assert_eq!(slot_event.record_kind(), RecordKind::SlotWritten);
        prop_assert_ne!(step_event.record_kind(), slot_event.record_kind());
    }
}
