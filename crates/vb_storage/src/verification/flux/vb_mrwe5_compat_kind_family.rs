#![forbid(unsafe_code)]

//! Flux bridge artifact for `obl-vb-mrwe-5-ps004-flux-018`.

use crate::{JournalKindCompatibility, classify_journal_kind_compatibility};

pub const SLOT_WRITTEN_KIND_ID: u16 = 12;
pub const RESERVED_STEP_SUCCEEDED_KIND_ID: u16 = 29;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityClass {
    ExactMatch,
    RejectedMismatch,
}

#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(envelope_kind: u16, payload_kind: u16) -> CompatibilityClass))]
pub const fn classify(envelope_kind: u16, payload_kind: u16) -> CompatibilityClass {
    if envelope_kind == payload_kind {
        CompatibilityClass::ExactMatch
    } else {
        CompatibilityClass::RejectedMismatch
    }
}

#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(envelope_kind: u16, payload_kind: u16) -> u8))]
pub const fn production_compatibility_code(envelope_kind: u16, payload_kind: u16) -> u8 {
    classify_journal_kind_compatibility(envelope_kind, payload_kind).code()
}

#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(envelope_kind: u16, payload_kind: u16) -> CompatibilityClass))]
pub const fn production_classify(envelope_kind: u16, payload_kind: u16) -> CompatibilityClass {
    match classify_journal_kind_compatibility(envelope_kind, payload_kind) {
        JournalKindCompatibility::ExactMatch => CompatibilityClass::ExactMatch,
        JournalKindCompatibility::RejectedMismatch => CompatibilityClass::RejectedMismatch,
    }
}
