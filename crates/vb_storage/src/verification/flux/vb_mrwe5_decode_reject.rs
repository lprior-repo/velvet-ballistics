#![forbid(unsafe_code)]

//! Flux bridge artifact for `obl-vb-mrwe-5-ps002-flux-008`.

use crate::{JournalSemanticDecodeDecision, classify_journal_semantic_decode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeState {
    ValidatedSemantic,
    RejectedMismatch,
}

#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(envelope_kind: u16, payload_kind: u16) -> DecodeState))]
pub const fn semantic_decode_state(envelope_kind: u16, payload_kind: u16) -> DecodeState {
    if envelope_kind == payload_kind {
        DecodeState::ValidatedSemantic
    } else {
        DecodeState::RejectedMismatch
    }
}

#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(envelope_kind: u16, payload_kind: u16, event_valid: bool) -> u8))]
pub const fn production_semantic_decode_code(
    envelope_kind: u16,
    payload_kind: u16,
    event_valid: bool,
) -> u8 {
    classify_journal_semantic_decode(envelope_kind, payload_kind, event_valid).code()
}

#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(envelope_kind: u16, payload_kind: u16, event_valid: bool) -> DecodeState))]
pub const fn production_semantic_decode_state(
    envelope_kind: u16,
    payload_kind: u16,
    event_valid: bool,
) -> DecodeState {
    match classify_journal_semantic_decode(envelope_kind, payload_kind, event_valid) {
        JournalSemanticDecodeDecision::SemanticSuccess => DecodeState::ValidatedSemantic,
        JournalSemanticDecodeDecision::KindPayloadMismatch
        | JournalSemanticDecodeDecision::InvalidEvent => DecodeState::RejectedMismatch,
    }
}
