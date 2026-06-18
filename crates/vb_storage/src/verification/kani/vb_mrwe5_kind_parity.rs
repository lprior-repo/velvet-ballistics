#![forbid(unsafe_code)]
//! Kani harness for `obl-vb-mrwe-5-ps001-kani-002`.
//!
//! Production binding: calls `JournalEvent::record_kind` on symbolic
//! StepSucceeded and SlotWrittenEvent payloads and checks that new durable writes
//! select the canonical record kind for the generated payload variant.

use crate::{EventSeq, JournalEvent, RecordKind};
use core::mem::ManuallyDrop;
use vb_core::{RunId, SlotIdx, StepIdx};

fn generated_step_succeeded() -> JournalEvent {
    JournalEvent::StepSucceeded {
        run: RunId::new(kani::any()),
        seq: EventSeq::new(kani::any()),
        step: StepIdx::new(kani::any()),
        output: SlotIdx::new(kani::any()),
    }
}

fn generated_slot_written() -> JournalEvent {
    JournalEvent::SlotWrittenEvent {
        run: RunId::new(kani::any()),
        seq: EventSeq::new(kani::any()),
        slot: SlotIdx::new(kani::any()),
        value: None,
        extra: None,
        attempt: kani::any(),
    }
}

#[kani::proof]
pub fn new_writes_use_canonical_record_kind() {
    let choose_slot = kani::any::<bool>();
    let event = ManuallyDrop::new(if choose_slot {
        generated_slot_written()
    } else {
        generated_step_succeeded()
    });

    match &*event {
        JournalEvent::StepSucceeded { .. } => {
            kani::assert(
                event.record_kind() == RecordKind::StepSucceeded,
                "StepSucceeded writes canonical StepSucceeded kind",
            );
            kani::assert(
                event.record_kind() != RecordKind::SlotWritten,
                "StepSucceeded does not write SlotWritten kind",
            );
        }
        JournalEvent::SlotWrittenEvent { .. } => {
            kani::assert(
                event.record_kind() == RecordKind::SlotWritten,
                "SlotWrittenEvent writes canonical SlotWritten kind",
            );
            kani::assert(
                event.record_kind() != RecordKind::StepSucceeded,
                "SlotWrittenEvent does not write StepSucceeded kind",
            );
        }
        _ => kani::assert(false, "generator only creates MRWE5 event pair"),
    }
}
