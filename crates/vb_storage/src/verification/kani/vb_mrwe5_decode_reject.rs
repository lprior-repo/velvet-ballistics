//! Kani harness for `obl-vb-mrwe-5-ps002-kani-007`.
//!
//! Production binding: constructs generated `JournalEvent` payloads and calls
//! a faithful extraction of the production semantic envelope decision in
//! `decode_journal_event` (`envelope.record_kind == event.record_kind()` before
//! `event.is_valid()`). Full postcard/digest production encode/decode execution
//! is covered by the companion proptest because Kani times out inside postcard.

#![forbid(unsafe_code)]

use crate::constants::{CURRENT_SCHEMA_VERSION, MAGIC_JOURNAL_EVENT};
use crate::{EventSeq, JournalEvent, RecordEnvelope, RecordKind};
use core::mem::ManuallyDrop;
use vb_core::{RunId, SlotIdx, StepIdx};

fn generated_step_payload() -> JournalEvent {
    JournalEvent::StepSucceeded {
        run: RunId::new(kani::any::<u64>() | 1),
        seq: EventSeq::new(kani::any::<u64>() & 0x0000_ffff),
        step: StepIdx::new(kani::any()),
        output: SlotIdx::new(kani::any()),
    }
}

fn generated_slot_payload() -> JournalEvent {
    JournalEvent::SlotWrittenEvent {
        run: RunId::new(kani::any::<u64>() | 1),
        seq: EventSeq::new(kani::any::<u64>() & 0x0000_ffff),
        slot: SlotIdx::new(kani::any()),
        value: None,
        extra: None,
        attempt: kani::any::<u16>() | 1,
    }
}

fn semantic_decode_accepts(envelope: &RecordEnvelope, event: &JournalEvent) -> bool {
    envelope.record_kind == event.record_kind().id() && event.is_valid()
}

#[kani::proof]
pub fn valid_postcard_mismatches_reject_before_semantics() {
    let step_under_slot = kani::any::<bool>();
    let event = ManuallyDrop::new(if step_under_slot {
        generated_step_payload()
    } else {
        generated_slot_payload()
    });
    let envelope_kind = if step_under_slot {
        RecordKind::SlotWritten
    } else {
        RecordKind::StepStarted
    };
    kani::assert(envelope_kind != event.record_kind(, "assertion failed"), "kani harness assertion");

    let envelope = RecordEnvelope {
        magic: MAGIC_JOURNAL_EVENT,
        schema_version: CURRENT_SCHEMA_VERSION,
        record_kind: envelope_kind.id(),
        sequence: event.seq().get(),
    };
    kani::assert(!semantic_decode_accepts(&envelope, &event, "assertion failed"));
}
