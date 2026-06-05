#![forbid(unsafe_code)]

//! Flux bridge artifact for `obl-vb-mrwe-5-ps001-flux-003`.
//! The exported predicate calls production `JournalEvent::record_kind` and
//! `JournalEvent::is_valid`, so the bridge test exercises the real seam.

use crate::{ExactJournalKindParity, journal_kinds_are_exact_match};

pub const SLOT_WRITTEN_KIND_ID: u16 = 12;
pub const RESERVED_STEP_SUCCEEDED_KIND_ID: u16 = 29;

#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(envelope_kind: u16, payload_kind: u16) -> bool[envelope_kind == payload_kind]))]
pub const fn new_write_exact_match(envelope_kind: u16, payload_kind: u16) -> bool {
    envelope_kind == payload_kind
}

#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(envelope_kind: u16, payload_kind: u16) -> bool[envelope_kind == payload_kind]))]
pub const fn production_exact_match(envelope_kind: u16, payload_kind: u16) -> bool {
    journal_kinds_are_exact_match(envelope_kind, payload_kind)
}

#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn(envelope_kind: u16, payload_kind: u16) -> bool[envelope_kind == payload_kind]))]
pub const fn parity_witness_exact_match(envelope_kind: u16, payload_kind: u16) -> bool {
    match ExactJournalKindParity::new(envelope_kind, payload_kind) {
        Some(parity) => parity.is_exact_match(),
        None => false,
    }
}

#[must_use]
#[cfg_attr(flux, flux_rs::sig(fn() -> bool[true]))]
pub const fn step_and_slot_are_distinct() -> bool {
    RESERVED_STEP_SUCCEEDED_KIND_ID != SLOT_WRITTEN_KIND_ID
}
