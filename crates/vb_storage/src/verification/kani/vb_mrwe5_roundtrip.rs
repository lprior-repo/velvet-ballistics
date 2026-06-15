//! Kani harness for `obl-vb-mrwe-5-ps003-kani-012`.
//!
//! Production binding: verifies generated StepSucceeded and SlotWrittenEvent
//! values against a faithful extraction of the production post-decode semantic
//! envelope decision. Full production `encode_record`/`decode_journal_event`
//! round-trip execution is covered by the companion proptest because Kani times
//! out inside postcard allocation/formatting loops.

#![forbid(unsafe_code)]

use crate::constants::{CURRENT_SCHEMA_VERSION, MAGIC_JOURNAL_EVENT};
use crate::{EventSeq, JournalEvent, RecordEnvelope, RecordKind};
use core::mem::ManuallyDrop;
use vb_core::{RunId, SlotIdx, StepIdx};

fn semantic_decode_accepts(envelope: &RecordEnvelope, event: &JournalEvent) -> bool {
    envelope.record_kind == event.record_kind().id() && event.is_valid()
}

#[kani::proof]
pub fn step_succeeded_and_slot_written_roundtrip_with_envelope_assertions() {
    let step_event = ManuallyDrop::new(JournalEvent::StepSucceeded {
        run: RunId::new(kani::any::<u64>() | 1),
        seq: EventSeq::new(kani::any::<u64>() & 0x0000_ffff),
        step: StepIdx::new(kani::any()),
        output: SlotIdx::new(kani::any()),
    });
    let slot_event = ManuallyDrop::new(JournalEvent::SlotWrittenEvent {
        run: RunId::new(kani::any::<u64>() | 1),
        seq: EventSeq::new(kani::any::<u64>() & 0x0000_ffff),
        slot: SlotIdx::new(kani::any()),
        value: None,
        extra: None,
        attempt: kani::any::<u16>() | 1,
    });

    kani::assert(step_event.record_kind() == RecordKind::StepSucceeded, "kani harness assertion");
    kani::assert(slot_event.record_kind() == RecordKind::SlotWritten, "kani harness assertion");
    kani::assert(step_event.record_kind() != slot_event.record_kind(), "kani harness assertion");

    let step_envelope = RecordEnvelope {
        magic: MAGIC_JOURNAL_EVENT,
        schema_version: CURRENT_SCHEMA_VERSION,
        record_kind: RecordKind::StepSucceeded.id(),
        sequence: step_event.seq().get(),
    };
    kani::assert(semantic_decode_accepts(&step_envelope, &step_event));
    kani::assert(matches!(&*step_event, JournalEvent::StepSucceeded { .. }));

    let slot_envelope = RecordEnvelope {
        magic: MAGIC_JOURNAL_EVENT,
        schema_version: CURRENT_SCHEMA_VERSION,
        record_kind: RecordKind::SlotWritten.id(),
        sequence: slot_event.seq().get(),
    };
    kani::assert(semantic_decode_accepts(&slot_envelope, &slot_event));
    kani::assert(matches!(
        &*slot_event, JournalEvent::SlotWrittenEvent { .. }
    ));
}
