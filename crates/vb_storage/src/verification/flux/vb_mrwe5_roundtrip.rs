#![forbid(unsafe_code)]

//! Flux bridge artifact for `obl-vb-mrwe-5-ps003-flux-013`.

use crate::{JournalSemanticDecodeDecision, RecordKind, classify_journal_semantic_decode};

pub const SLOT_WRITTEN_KIND_ID: u16 = 12;
pub const RESERVED_STEP_SUCCEEDED_KIND_ID: u16 = 29;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundTripVariant {
    StepSucceeded,
    SlotWrittenEvent,
}

#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(variant: RoundTripVariant) -> u16))]
pub const fn expected_roundtrip_kind(variant: RoundTripVariant) -> u16 {
    match variant {
        RoundTripVariant::StepSucceeded => RecordKind::StepSucceeded.id(),
        RoundTripVariant::SlotWrittenEvent => RecordKind::SlotWritten.id(),
    }
}

#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(variant: RoundTripVariant, envelope_kind: u16, event_valid: bool) -> bool))]
pub const fn production_roundtrip_accepts_exact_variant(
    variant: RoundTripVariant,
    envelope_kind: u16,
    event_valid: bool,
) -> bool {
    matches!(
        classify_journal_semantic_decode(
            envelope_kind,
            expected_roundtrip_kind(variant),
            event_valid
        ),
        JournalSemanticDecodeDecision::SemanticSuccess
    )
}
