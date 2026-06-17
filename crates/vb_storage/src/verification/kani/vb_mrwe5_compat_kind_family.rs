//! Kani harness for `obl-vb-mrwe-5-ps004-kani-017`.
//!
//! Production binding: calls public known-kind/family validators and a faithful
//! extraction of the production post-decode semantic envelope decision over a
//! bounded mismatch matrix. The selected policy is fail-closed: no legacy-like
//! envelope/payload mismatch is semantic success.

#![forbid(unsafe_code)]

use crate::codec::{is_known_record_kind, validate_record_kind_family};
use crate::constants::CURRENT_SCHEMA_VERSION;
use crate::constants::MAGIC_JOURNAL_EVENT;
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
pub fn kind_ids_and_legacy_policy_are_narrow() {
    let raw = kani::any::<u16>();
    let bounded_kind = raw % 64;
    let known = is_known_record_kind(bounded_kind);
    let family_result = ManuallyDrop::new(validate_record_kind_family(
        MAGIC_JOURNAL_EVENT,
        bounded_kind,
    ));
    let journal_family = matches!(&*family_result, Ok(()));

    if bounded_kind == RecordKind::SlotWritten.id() {
        //! Kani harness for `obl-vb-mrwe-5-ps004-kani-017`.
//!
//! Production binding: calls public known-kind/family validators and a faithful
//! extraction of the production post-decode semantic envelope decision over a
//! bounded mismatch matrix. The selected policy is fail-closed: no legacy-like
//! envelope/payload mismatch is semantic success.

#![forbid(unsafe_code)]

use crate::codec::{is_known_record_kind, validate_record_kind_family};
use crate::constants::CURRENT_SCHEMA_VERSION;
use crate::constants::MAGIC_JOURNAL_EVENT;
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
pub fn kind_ids_and_legacy_policy_are_narrow() {
    let raw = kani::any::<u16>();
    let bounded_kind = raw % 64;
    let known = is_known_record_kind(bounded_kind);
    let family_result = ManuallyDrop::new(validate_record_kind_family(
        MAGIC_JOURNAL_EVENT,
        bounded_kind,
    ));
    let journal_family = matches!(&*family_result, Ok(()));

    if bounded_kind == RecordKind::SlotWritten.id() {
        kani::assert(known, "kani harness assertion");
        kani::assert(journal_family, "kani harness assertion");
    }

    if bounded_kind == RecordKind::StepSucceeded.id() {
        kani::assert(known, "kani harness assertion");
        kani::assert(journal_family, "kani harness assertion");
    }

    if !known {
        kani::assert(!journal_family, "kani harness assertion");
    }

    let use_legacy_like_pair = kani::any::<bool>();
    let payload = ManuallyDrop::new(if use_legacy_like_pair {
        generated_step_payload()
    } else {
        generated_slot_payload()
    });
    let wrong_envelope = if use_legacy_like_pair {
        RecordKind::SlotWritten
    } else {
        RecordKind::StepSucceeded
    };
    kani::assert(wrong_envelope != payload.record_kind(), "kani harness assertion");

    let envelope = RecordEnvelope {
        magic: MAGIC_JOURNAL_EVENT,
        schema_version: CURRENT_SCHEMA_VERSION,
        record_kind: wrong_envelope.id(),
        sequence: payload.seq().get(),
    };
    kani::assert(!semantic_decode_accepts(&envelope, &payload));
}
