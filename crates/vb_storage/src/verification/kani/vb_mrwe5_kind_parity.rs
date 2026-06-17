//! Kani harness for `obl-vb-mrwe-5-ps001-kani-002`.
//!
//! Production binding: calls `JournalEvent::record_kind` and checks that the
//! kind selected for new durable writes is congruent with the generated payload
//! variant. The generated StepSucceeded and SlotWrittenEvent shapes are not a
//! fixed dummy shape.

#![forbid(unsafe_code)]

use crate::{EventSeq, JournalEvent, RecordKind};
use core::mem::ManuallyDrop;
use vb_core::{RunId, SlotIdx, StepIdx};

fn generated_step_succeeded() -> JournalEvent {
    let run_raw = kani::any::<u64>() | 1;
    let seq_raw = kani::any::<u64>() & 0x0000_0000_0000_ffff;
    JournalEvent::StepSucceeded {
        run: RunId::new(run_raw),
        seq: EventSeq::new(seq_raw),
        step: StepIdx::new(kani::any()),
        output: SlotIdx::new(kani::any()),
    }
}

fn generated_slot_written() -> JournalEvent {
    let run_raw = kani::any::<u64>() | 1;
    let seq_raw = kani::any::<u64>() & 0x0000_0000_0000_ffff;
    let attempt = kani::any::<u16>() | 1;
    JournalEvent::SlotWrittenEvent {
        run: RunId::new(run_raw),
        seq: EventSeq::new(seq_raw),
        slot: SlotIdx::new(kani::any()),
        value: None,
        extra: None,
        attempt,
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
            kani::assert(event.record_kind() != RecordKind::SlotWritten, "kani harness assertion");
            kani::assert(event.record_kind() == RecordKind::StepSucceeded, "kani harness assertion");
        }
        JournalEvent::SlotWrittenEvent { .. } => {
            kani::assert(event.record_kind() == RecordKind::SlotWritten, "kani harness assertion");
            kani::assert(event.record_kind() != RecordKind::StepSucceeded, "kani harness assertion");
        }
        _ => {
             != RecordKind::StepSucceeded, "kani harness assertion");
        }
        _ => {
            kani::assert(false, "kani harness assertion");
        }
    }
}
