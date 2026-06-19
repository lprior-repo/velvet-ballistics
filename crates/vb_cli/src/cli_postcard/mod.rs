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
mod constants;
mod deserial;
mod error;
mod serial;
mod types;
mod types_more;
mod validation;

pub(crate) use classify::{ClassifyError, classify_envelope};
pub(crate) use codec::{decode_cli_payload, decode_postcard_payload, encode_postcard};
pub(crate) use constants::{
    CLI_MAGIC, CLI_POSTCARD_KIND, CLI_SCHEMA_VERSION, HEADER_SIZE, HEADER_SIZE_U32, MAX_PAYLOAD,
    MAX_PAYLOAD_U32,
};
pub(crate) use deserial::{CliPostcardKind, UnknownCliPostcardKind};
pub(crate) use error::PostcardError;
pub(crate) use serial::PostcardHeader;
pub(crate) use types::{
    CliPostcardPayload, DiagnosticReport, DiffEntry, DiffReport, EnvelopeSchemaVersion, EventEntry,
    EventsReport, ExplainErrorEntry, ExplainReport, GenericPayload, ReplayReport, TraceEntry,
    TraceReport, ValidateReport, VerifyArtifactSection, VerifyDurabilitySection,
    VerifyReplaySection, VerifyReport,
};
pub(crate) use types_more::{AiContextPacketReport, SystemStatusReport, WorkflowDiffReport};
pub(crate) use validation::{decode_postcard, payload_digest};

#[cfg(test)]
pub(crate) use classify::GenericEnvelopeRepr;

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
