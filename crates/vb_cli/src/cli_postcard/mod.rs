//! CLI Postcard Module
//!
//! vb-k8ut.5: typed-domain CLI postcard envelopes. Every supported payload
//! deserializes into a per-command typed Rust variant of
//! `CliPostcardPayload` (Validate, Verify, Explain, Events, Trace, Replay,
//! Diff, Diagnostic) carrying typed Rust structs with typed fields. There is
//! no JSON-in-postcard bridge. Migration-fallback shapes use the typed
//! `Generic(GenericPayload)` variant which carries a typed
//! `CliPostcardKind` discriminant plus an opaque postcard-encoded body —
//! never raw UTF-8 JSON bytes and never `serde_json::Value`.
//!
//! ## Contract Clauses
//! - INV-005: Postcard payloads respect bounded allocation (header_len + payload_len validated before decode)
//! - POST-007: Postcard output validates magic + header length before payload decode

#![forbid(unsafe_code)]
#![allow(dead_code)]

mod classify;
mod codec;
mod error;
mod types;
mod validation;

pub(crate) use classify::{ClassifyError, classify_envelope};
pub(crate) use codec::{decode_cli_payload, decode_postcard_payload, encode_postcard};
pub(crate) use error::PostcardError;
pub(crate) use types::{
    CLI_MAGIC, CLI_POSTCARD_KIND, CLI_SCHEMA_VERSION, CliPostcardKind, CliPostcardPayload,
    DiagnosticReport, DiffEntry, DiffReport, EnvelopeSchemaVersion, EventEntry, EventsReport,
    ExplainErrorEntry, ExplainReport, GenericPayload, HEADER_SIZE, HEADER_SIZE_U32, MAX_PAYLOAD,
    MAX_PAYLOAD_U32, PostcardHeader, ReplayReport, TraceEntry, TraceReport, ValidateReport,
    VerifyArtifactSection, VerifyDurabilitySection, VerifyReplaySection, VerifyReport,
};
pub(crate) use validation::{decode_postcard, payload_digest};

#[cfg(test)]
pub(crate) use classify::GenericEnvelopeRepr;

fn read_array<const N: usize>(data: &[u8], start: usize) -> Result<[u8; N], PostcardError> {
    let end = start.checked_add(N).ok_or(PostcardError::DecodeFailed)?;
    let bytes = data.get(start..end).ok_or(PostcardError::DecodeFailed)?;
    <[u8; N]>::try_from(bytes).map_err(|_| PostcardError::DecodeFailed)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
