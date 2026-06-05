#![forbid(unsafe_code)]
//! Semantic classification and typestate for journal records.

use crate::{
    error::JournalError,
    events::JournalEvent,
    mrwe5_contract::{
        Mrwe5KindCompatibility, Mrwe5RecordKindFamilyDecision, Mrwe5SemanticDecodeDecision,
        mrwe5_classify_kind_compatibility, mrwe5_classify_record_kind_family,
        mrwe5_classify_semantic_decode, mrwe5_kinds_are_exact_match,
    },
    types::RecordEnvelope,
};

use super::RecordKindFamilyDecision;

/// Semantic decode classification for journal-event payload/envelope parity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum JournalSemanticDecodeDecision {
    /// Envelope kind, payload kind, and payload structural validity all agree.
    SemanticSuccess = 1,
    /// Envelope kind and decoded payload variant disagree.
    KindPayloadMismatch = 2,
    /// Envelope/payload kind agrees, but payload structural validity fails.
    InvalidEvent = 3,
}

impl JournalSemanticDecodeDecision {
    /// Stable verifier-facing code for semantic decode outcomes.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::SemanticSuccess => 1,
            Self::KindPayloadMismatch => 2,
            Self::InvalidEvent => 3,
        }
    }
}

/// Explicit compatibility policy for journal envelope/payload kind pairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum JournalKindCompatibility {
    /// Exact envelope/payload kind match accepted by new writes and semantic decode.
    ExactMatch = 1,
    /// Any mismatch is rejected; no silent legacy promotion is available here.
    RejectedMismatch = 2,
}

impl JournalKindCompatibility {
    /// Stable verifier-facing code for compatibility outcomes.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::ExactMatch => 1,
            Self::RejectedMismatch => 2,
        }
    }
}

/// Proof-carrying witness that an envelope kind equals the decoded payload kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactJournalKindParity {
    envelope_kind: u16,
    payload_kind: u16,
}

impl ExactJournalKindParity {
    /// Builds a parity witness only when the two durable kind identifiers match.
    pub const fn new(envelope_kind: u16, payload_kind: u16) -> Option<Self> {
        if envelope_kind == payload_kind {
            Some(Self {
                envelope_kind,
                payload_kind,
            })
        } else {
            None
        }
    }

    /// Envelope kind proven equal to the payload semantic kind.
    #[must_use]
    pub const fn envelope_kind(self) -> u16 {
        self.envelope_kind
    }

    /// Payload semantic kind proven equal to the envelope kind.
    #[must_use]
    pub const fn payload_kind(self) -> u16 {
        self.payload_kind
    }

    /// Verifier-facing equality predicate carried by this witness.
    #[must_use]
    pub const fn is_exact_match(self) -> bool {
        self.envelope_kind == self.payload_kind
    }
}

/// Journal record whose decoded payload has passed envelope/payload parity and
/// structural event validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedJournalRecord {
    envelope: RecordEnvelope,
    event: JournalEvent,
    parity: ExactJournalKindParity,
}

impl ValidatedJournalRecord {
    /// Validates and seals a decoded journal record before semantic use.
    pub fn try_new(envelope: RecordEnvelope, event: JournalEvent) -> Result<Self, JournalError> {
        let parity = validate_journal_event_semantics(&envelope, &event)?;
        Ok(Self {
            envelope,
            event,
            parity,
        })
    }

    /// Borrow the validated envelope metadata.
    #[must_use]
    pub const fn envelope(&self) -> &RecordEnvelope {
        &self.envelope
    }

    /// Borrow the validated semantic event.
    #[must_use]
    pub const fn event(&self) -> &JournalEvent {
        &self.event
    }

    /// Returns the exact-kind witness produced by validation.
    #[must_use]
    pub const fn parity(&self) -> ExactJournalKindParity {
        self.parity
    }

    /// Decomposes the validated record for legacy tuple-shaped APIs.
    #[must_use]
    pub fn into_parts(self) -> (RecordEnvelope, JournalEvent) {
        (self.envelope, self.event)
    }
}

/// Pure seam for MRWE5 compatibility policy: new semantic records require exact
/// envelope/payload kind equality.
#[must_use]
pub const fn classify_journal_kind_compatibility(
    envelope_kind: u16,
    payload_kind: u16,
) -> JournalKindCompatibility {
    if envelope_kind == payload_kind {
        return JournalKindCompatibility::ExactMatch;
    }
    match mrwe5_classify_kind_compatibility(envelope_kind, payload_kind) {
        Mrwe5KindCompatibility::ExactMatch => JournalKindCompatibility::ExactMatch,
        Mrwe5KindCompatibility::RejectedMismatch => JournalKindCompatibility::RejectedMismatch,
    }
}

/// Pure boolean seam for implementation-bound Verus/Flux closure.
#[must_use]
pub const fn journal_kinds_are_exact_match(envelope_kind: u16, payload_kind: u16) -> bool {
    mrwe5_kinds_are_exact_match(envelope_kind, payload_kind)
}

/// Pure seam used by semantic journal decode before returning a `JournalEvent`.
#[must_use]
pub const fn classify_journal_semantic_decode(
    envelope_kind: u16,
    payload_kind: u16,
    event_valid: bool,
) -> JournalSemanticDecodeDecision {
    match mrwe5_classify_semantic_decode(envelope_kind, payload_kind, event_valid) {
        Mrwe5SemanticDecodeDecision::SemanticSuccess => {
            JournalSemanticDecodeDecision::SemanticSuccess
        }
        Mrwe5SemanticDecodeDecision::InvalidEvent => JournalSemanticDecodeDecision::InvalidEvent,
        Mrwe5SemanticDecodeDecision::KindPayloadMismatch => {
            JournalSemanticDecodeDecision::KindPayloadMismatch
        }
    }
}

/// Returns whether a raw wire record-kind value is recognized by storage.
pub const fn is_known_record_kind(kind: u16) -> bool {
    super::validation::is_known_record_kind(kind)
}

/// Returns whether a raw wire record-kind value belongs to the journal family.
pub const fn is_journal_record_kind(kind: u16) -> bool {
    super::validation::is_journal_record_kind(kind)
}

/// Pure family-classification seam for proof artifacts and source bridges.
pub const fn classify_record_kind_family(magic: u32, kind: u16) -> RecordKindFamilyDecision {
    if matches!(
        mrwe5_classify_record_kind_family(magic, kind),
        Mrwe5RecordKindFamilyDecision::Accepted
    ) {
        return RecordKindFamilyDecision::Accepted;
    }
    super::validation::classify_kind_family(magic, kind)
}

/// Validates a raw wire record-kind value before semantic family checks.
pub fn validate_known_record_kind(kind: u16) -> Result<(), JournalError> {
    super::validation::validate_known_kind(kind)
}

/// Validates that a raw wire record-kind value belongs to the magic family.
pub fn validate_record_kind_family(magic: u32, kind: u16) -> Result<(), JournalError> {
    super::validation::validate_kind_family(magic, kind)
}

/// Validates semantic journal event use and returns an exact-kind proof witness.
pub fn validate_journal_event_semantics(
    envelope: &RecordEnvelope,
    event: &JournalEvent,
) -> Result<ExactJournalKindParity, JournalError> {
    match classify_journal_semantic_decode(
        envelope.record_kind,
        event.record_kind_id(),
        event.is_valid(),
    ) {
        JournalSemanticDecodeDecision::SemanticSuccess => {
            ExactJournalKindParity::new(envelope.record_kind, event.record_kind_id())
                .ok_or(JournalError::InvalidEvent)
        }
        JournalSemanticDecodeDecision::KindPayloadMismatch
        | JournalSemanticDecodeDecision::InvalidEvent => Err(JournalError::InvalidEvent),
    }
}
