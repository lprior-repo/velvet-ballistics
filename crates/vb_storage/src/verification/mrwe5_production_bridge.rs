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
mod tests;
