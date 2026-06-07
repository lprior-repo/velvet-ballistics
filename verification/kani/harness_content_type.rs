//! Kani harness for typed CLI postcard payload discrimination proof (STUB)
//!
//! # Proof Obligation
//!
//! po-vbqa2g-014: `CliPostcardPayload` variants discriminate cleanly between
//! typed `Diagnostic`, `Validate`, and other per-command typed envelopes
//! (vb-k8ut.5).
//!
//! # Visibility Blocker
//!
//! The production types are declared `pub(crate)` inside
//! `vb_cli::cli_postcard` (see `crates/vb_cli/src/cli_postcard/mod.rs`).
//! External verification scripts cannot import them:
//!
//! - `vb_cli::cli_postcard::CliPostcardPayload` (enum)
//! - `vb_cli::cli_postcard::CliPostcardKind` (enum, derives
//!   `Serialize`/`Deserialize`)
//! - `vb_cli::cli_postcard::DiagnosticReport` (struct)
//! - `vb_cli::cli_postcard::ValidateReport` (struct)
//! - `vb_cli::cli_postcard::EnvelopeSchemaVersion` (newtype)
//! - `vb_cli::cli_postcard::PostcardError` (enum)
//! - `vb_cli::exit_code::CliExitCode` (enum, derives
//!   `Serialize`/`Deserialize`)
//!
//! The previous version of this harness referenced the legacy
//! `CliPostcardPayload` API that the vb-k8ut.5 typed-domain envelope
//! refactor removed: the JSON-string content type discriminator, the
//! kind-value and JSON-envelope constructors, the secondary tree-shaped
//! variant, the legacy `DiagnosticReport` shape, and the legacy
//! envelope-kind normalization that silently coerced unknown strings to
//! the diagnostic variant.
//!
//! # Re-enabling This Harness
//!
//! 1. Move this file to
//!    `crates/vb_cli/src/cli_postcard/kani_harnesses.rs` and gate it with
//!    `#[cfg(kani)]` so it lives inside the crate's `#[cfg(test)]`
//!    boundary; OR
//! 2. Re-export the relevant types with `pub` (not `pub(crate)`) visibility
//!    from `vb_cli::cli_postcard::mod` — not in scope for vb-k8ut.5.
//!
//! # Hypothetical Harness (sketched, not compiled)
//!
//! ```ignore
//! use vb_cli::cli_postcard::{
//!     CliPostcardKind, CliPostcardPayload, DiagnosticReport,
//!     EnvelopeSchemaVersion, ValidateReport,
//! };
//! use vb_cli::exit_code::CliExitCode;
//!
//! // Property 1: a Diagnostic payload postcard-encodes, postcard-decodes,
//! // and decodes back into the `Diagnostic` variant tag.
//! #[kani::proof]
//! fn harness_typed_payload_round_trips_diagnostic() {
//!     let report = DiagnosticReport::from_code(
//!         "kani-test".to_string(),
//!         CliExitCode::ValidationFailed,
//!     );
//!     let payload = CliPostcardPayload::from_diagnostic(report);
//!     let bytes = postcard::to_allocvec(&payload)
//!         .expect("typed payload encodes");
//!     let decoded: CliPostcardPayload = postcard::from_bytes(&bytes)
//!         .expect("typed payload decodes");
//!     kani::assert(
//!         matches!(decoded, CliPostcardPayload::Diagnostic(_)),
//!         "decoded must carry the Diagnostic variant tag",
//!     );
//! }
//!
//! // Property 2: a Validate payload postcard-encodes, postcard-decodes,
//! // and decodes back into the `Validate` variant tag.
//! #[kani::proof]
//! fn harness_typed_payload_round_trips_validate() {
//!     let report = ValidateReport {
//!         schema_version: EnvelopeSchemaVersion::current(),
//!         kind: "validate_report".to_string(),
//!         success: true,
//!         status: "ok".to_string(),
//!         exit_code: 0,
//!         repair_hints: Vec::new(),
//!     };
//!     let payload = CliPostcardPayload::Validate(report);
//!     let bytes = postcard::to_allocvec(&payload)
//!         .expect("typed payload encodes");
//!     let decoded: CliPostcardPayload = postcard::from_bytes(&bytes)
//!         .expect("typed payload decodes");
//!     kani::assert(
//!         matches!(decoded, CliPostcardPayload::Validate(_)),
//!         "decoded must carry the Validate variant tag",
//!     );
//! }
//!
//! // Property 3: `from_envelope_kind` returns `None` for unknown strings
//! // and `Some(CliPostcardKind::ValidateReport)` for the per-command JSON
//! // kind string `"validate_report"`. The vb-k8ut.5 contract rejects
//! // silent coercion to the diagnostic kind.
//! #[kani::proof]
//! fn harness_envelope_kind_normalization_returns_none_for_unknown() {
//!     kani::assert(
//!         CliPostcardKind::from_envelope_kind("unknown") == None,
//!         "unknown envelope kind must return None",
//!     );
//!     kani::assert(
//!         CliPostcardKind::from_envelope_kind("validate_report")
//!             == Some(CliPostcardKind::ValidateReport),
//!         "validate_report must resolve to ValidateReport",
//!     );
//! }
//! ```
//!
//! This file is intentionally left as documentation-only. Future refactors
//! that promote `cli_postcard` types to `pub` visibility (or move this
//! harness inside the crate's `#[cfg(test)]` boundary) can re-enable the
//! sketch above as live proof code.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(unused_imports)]
