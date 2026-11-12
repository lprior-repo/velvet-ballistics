#![forbid(unsafe_code)]
//! Default-implemented trait that enforces envelope kind / payload parity after decoding.
//!
//! `decode_record` requires its `T: DeserializeOwned` bound to additionally
//! implement [`EnforceKindParity`]. The trait's default implementation is a
//! no-op, suitable for record types whose payload does not carry a
//! record-kind discriminant (workflow source, compiled IR, blob, run snapshot,
//! run header, and any external test payload). For [`crate::JournalEvent`],
//! parity is mandatory: the override verifies that the envelope record kind
//! matches the decoded payload variant, except for the named schema-1 shared
//! envelope compatibility path, and that the event passes `JournalEvent::is_valid()`.
//!
//! External types that decode through `decode_record` only need to add a
//! trivial `impl EnforceKindParity for MyType {}` line. Because the default
//! does no validation, this cannot smuggle a JournalEvent past the parity
//! check — the only path that enforces parity is the explicit override on
//! `JournalEvent` itself, and Rust's coherence rules prevent a downstream
//! crate from providing a different impl for `JournalEvent`.

use crate::{
    JournalError,
    records::{BlobRecord, CompiledIrRecord, RunHeaderRecord, WorkflowSourceRecord},
    recovery::RunSnapshot,
    types::RecordEnvelope,
};
use serde::de::DeserializeOwned;

/// Validates envelope/payload parity after a record decode.
///
/// `decode_record` calls this immediately after postcard deserialization.
/// The default implementation is a no-op and is appropriate for record
/// types whose payload does not embed a record-kind discriminant.
///
/// Override the method when the payload carries a variant discriminant
/// that must match the envelope record kind. The only override in this
/// crate is for [`crate::JournalEvent`], which is the only variant-bearing
/// record type the storage contract admits.
pub trait EnforceKindParity: Sized {
    /// Decodes the payload bytes into `Self`.
    ///
    /// Most record payloads use their Serde implementation directly. Journal
    /// events override this hook because schema-1 replay must accept the
    /// pre-stable in-repo ordinal postcard shape as a compatibility fallback
    /// while new writes use stable record-kind tags.
    fn decode_payload<'payload>(
        envelope: &RecordEnvelope,
        payload: &'payload [u8],
    ) -> Result<(Self, &'payload [u8]), JournalError>
    where
        Self: DeserializeOwned,
    {
        let _ = envelope;
        postcard::take_from_bytes(payload).map_err(JournalError::PostcardDecodeFailed)
    }

    /// Verifies that the decoded value is consistent with the envelope
    /// metadata. Returns `Err` if the value must be rejected.
    ///
    /// The default implementation accepts every value, which is correct
    /// for record types that do not carry a record-kind discriminant in
    /// the payload.
    fn enforce_kind_parity(envelope: &RecordEnvelope, value: &Self) -> Result<(), JournalError> {
        let _ = (envelope, value);
        Ok(())
    }
}

impl EnforceKindParity for crate::JournalEvent {
    fn decode_payload<'payload>(
        envelope: &RecordEnvelope,
        payload: &'payload [u8],
    ) -> Result<(Self, &'payload [u8]), JournalError> {
        crate::events::decode_journal_event_payload_for_envelope(envelope, payload)
    }

    fn enforce_kind_parity(envelope: &RecordEnvelope, value: &Self) -> Result<(), JournalError> {
        super::validate_decoded_journal_event_record_kind(envelope, value)?;
        if !value.is_valid() {
            return Err(JournalError::InvalidEvent);
        }
        Ok(())
    }
}

impl EnforceKindParity for WorkflowSourceRecord {}
impl EnforceKindParity for CompiledIrRecord {}
impl EnforceKindParity for BlobRecord {}
impl EnforceKindParity for RunSnapshot {}
impl EnforceKindParity for RunHeaderRecord {}
