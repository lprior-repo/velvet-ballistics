#![forbid(unsafe_code)]
//! Source-includable MRWE5 journal kind contract kernel.
//!
//! This module is intentionally limited to primitive scalar inputs and outputs so
//! production code can delegate MRWE5 decisions to the same source file that a
//! Verus artifact can include with `#[path = ...]`.  It avoids serde, chrono,
//! postcard, Fjall, and crate-local rich domain types by design.

/// Magic value used by journal-event records.
pub const MRWE5_MAGIC_JOURNAL_EVENT: u32 = 0x5642_4A45;

/// Durable kind id for `JournalEvent::SlotWrittenEvent`.
pub const MRWE5_SLOT_WRITTEN_KIND_ID: u16 = 12;

/// Durable kind id for `JournalEvent::StepSucceeded`.
pub const MRWE5_STEP_SUCCEEDED_KIND_ID: u16 = 29;

/// First durable kind id in the journal-event family.
pub const MRWE5_JOURNAL_MIN_KIND_ID: u16 = 10;

/// Last currently accepted durable kind id in the journal-event family.
pub const MRWE5_JOURNAL_MAX_KIND_ID: u16 = 29;

/// Primitive payload class for the MRWE5 kind-separation contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Mrwe5PayloadClass {
    /// `JournalEvent::StepSucceeded` payloads.
    StepSucceeded = 1,
    /// `JournalEvent::SlotWrittenEvent` payloads.
    SlotWrittenEvent = 2,
    /// Payloads outside the MRWE5 separation pair.
    Other = 3,
}

/// Primitive compatibility policy for envelope/payload kind pairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Mrwe5KindCompatibility {
    /// Envelope and payload kind ids match exactly.
    ExactMatch = 1,
    /// Mismatches are rejected; no silent legacy promotion is available.
    RejectedMismatch = 2,
}

/// Primitive semantic decode decision before a `JournalEvent` is exposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Mrwe5SemanticDecodeDecision {
    /// Envelope kind, payload kind, and payload validity all agree.
    SemanticSuccess = 1,
    /// Envelope kind and decoded payload semantic kind disagree.
    KindPayloadMismatch = 2,
    /// Kind parity holds, but payload structural validity fails.
    InvalidEvent = 3,
}

/// Primitive family decision for storage record kind validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Mrwe5RecordKindFamilyDecision {
    /// The kind belongs to the selected magic family.
    Accepted = 1,
    /// The kind is not valid for the selected magic family.
    Rejected = 2,
}

/// Canonical durable kind id for the MRWE5 payload classes.
#[must_use]
pub const fn mrwe5_canonical_kind_id(class: Mrwe5PayloadClass) -> Option<u16> {
    match class {
        Mrwe5PayloadClass::StepSucceeded => Some(MRWE5_STEP_SUCCEEDED_KIND_ID),
        Mrwe5PayloadClass::SlotWrittenEvent => Some(MRWE5_SLOT_WRITTEN_KIND_ID),
        Mrwe5PayloadClass::Other => None,
    }
}

/// Exact envelope/payload kind equality predicate.
#[must_use]
pub const fn mrwe5_kinds_are_exact_match(envelope_kind: u16, payload_kind: u16) -> bool {
    envelope_kind == payload_kind
}

/// Classifies the compatibility policy for a journal envelope/payload pair.
#[must_use]
pub const fn mrwe5_classify_kind_compatibility(
    envelope_kind: u16,
    payload_kind: u16,
) -> Mrwe5KindCompatibility {
    if mrwe5_kinds_are_exact_match(envelope_kind, payload_kind) {
        Mrwe5KindCompatibility::ExactMatch
    } else {
        Mrwe5KindCompatibility::RejectedMismatch
    }
}

/// Classifies semantic decode before a journal payload can be used.
#[must_use]
pub const fn mrwe5_classify_semantic_decode(
    envelope_kind: u16,
    payload_kind: u16,
    event_valid: bool,
) -> Mrwe5SemanticDecodeDecision {
    match mrwe5_classify_kind_compatibility(envelope_kind, payload_kind) {
        Mrwe5KindCompatibility::ExactMatch if event_valid => {
            Mrwe5SemanticDecodeDecision::SemanticSuccess
        }
        Mrwe5KindCompatibility::ExactMatch => Mrwe5SemanticDecodeDecision::InvalidEvent,
        Mrwe5KindCompatibility::RejectedMismatch => {
            Mrwe5SemanticDecodeDecision::KindPayloadMismatch
        }
    }
}

/// Returns whether a raw durable kind id is in the journal-event family.
#[must_use]
pub const fn mrwe5_is_journal_record_kind(kind: u16) -> bool {
    MRWE5_JOURNAL_MIN_KIND_ID <= kind && kind <= MRWE5_JOURNAL_MAX_KIND_ID
}

/// Classifies whether a raw kind id belongs to the selected record family.
#[must_use]
pub const fn mrwe5_classify_record_kind_family(
    magic: u32,
    kind: u16,
) -> Mrwe5RecordKindFamilyDecision {
    if magic == MRWE5_MAGIC_JOURNAL_EVENT && mrwe5_is_journal_record_kind(kind) {
        Mrwe5RecordKindFamilyDecision::Accepted
    } else {
        Mrwe5RecordKindFamilyDecision::Rejected
    }
}
