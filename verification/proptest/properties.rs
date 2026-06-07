//! Proptest properties for CLI Postcard typed envelope codec (STUB)
//!
//! These properties would verify the roundtrip bijectivity, error
//! handling, and invariants of the Postcard encoding/decoding functions
//! in `vb_cli::cli_postcard`.
//!
//! # Visibility Blocker
//!
//! The production types and helper functions are declared `pub(crate)`:
//!
//! - `vb_cli::cli_postcard::encode_postcard` / `decode_postcard`
//! - `vb_cli::cli_postcard::PostcardError`
//! - `vb_cli::cli_postcard::PostcardHeader`
//! - `vb_cli::cli_postcard::HEADER_SIZE` / `CLI_MAGIC` /
//!   `CLI_SCHEMA_VERSION` / `CLI_POSTCARD_KIND` / `MAX_PAYLOAD`
//! - `vb_cli::cli_postcard::CliPostcardPayload` / `CliPostcardKind` /
//!   `DiagnosticReport` / `ValidateReport` / `EnvelopeSchemaVersion`
//! - `vb_cli::exit_code::CliExitCode`
//!
//! External proptest suites cannot import them.
//!
//! The previous version of this file contained a typed-payload
//! variant-discrimination smoke test that referenced the legacy
//! `CliPostcardPayload` API removed by the vb-k8ut.5 typed-domain
//! envelope refactor: the JSON-string content type discriminator, the
//! kind-value and JSON-envelope constructors, the secondary tree-shaped
//! variant, and the legacy `DiagnosticReport` shape. None of those
//! symbols exist any longer.
//!
//! # Re-enabling These Properties
//!
//! 1. Move this file to
//!    `crates/vb_cli/src/cli_postcard/proptests.rs` and gate it with
//!    `#[cfg(test)]` so it lives inside the crate's test boundary; OR
//! 2. Re-export the relevant types and helpers with `pub` (not
//!    `pub(crate)`) visibility from `vb_cli::cli_postcard::mod` — not in
//!    scope for vb-k8ut.5.
//!
//! # Hypothetical Proptest Suite (sketched, not compiled)
//!
//! ```ignore
//! use proptest::prelude::*;
//! use vb_cli::cli_postcard::{
//!     CliPostcardKind, CliPostcardPayload, DiagnosticReport,
//!     EnvelopeSchemaVersion, ValidateReport,
//! };
//! use vb_cli::exit_code::CliExitCode;
//!
//! proptest! {
//!     // Property: each per-command typed variant round-trips through
//!     // postcard and decodes back to the same variant tag. The two
//!     // distinct variants must not collapse into the same tag, proving
//!     // variant discrimination.
//!     #[test]
//!     fn properties_typed_payload_variant_discrimination(_ in 0u8..1) {
//!         // Diagnostic variant
//!         let diagnostic = CliPostcardPayload::from_diagnostic(
//!             DiagnosticReport::from_code(
//!                 "p1".to_string(),
//!                 CliExitCode::ValidationFailed,
//!             ),
//!         );
//!         let diag_bytes = postcard::to_allocvec(&diagnostic)
//!             .expect("diagnostic encodes");
//!         let diag_back: CliPostcardPayload = postcard::from_bytes(&diag_bytes)
//!             .expect("diagnostic decodes");
//!         prop_assert!(matches!(diag_back, CliPostcardPayload::Diagnostic(_)));
//!
//!         // Validate variant
//!         let validate = CliPostcardPayload::Validate(ValidateReport {
//!             schema_version: EnvelopeSchemaVersion::current(),
//!             kind: "validate_report".to_string(),
//!             success: true,
//!             status: "ok".to_string(),
//!             exit_code: 0,
//!             repair_hints: Vec::new(),
//!         });
//!         let val_bytes = postcard::to_allocvec(&validate)
//!             .expect("validate encodes");
//!         let val_back: CliPostcardPayload = postcard::from_bytes(&val_bytes)
//!             .expect("validate decodes");
//!         prop_assert!(matches!(val_back, CliPostcardPayload::Validate(_)));
//!     }
//! }
//! ```
//!
//! This file is intentionally left as documentation-only. Future refactors
//! that promote `cli_postcard` types to `pub` visibility (or move these
//! proptests inside the crate's `#[cfg(test)]` boundary) can re-enable
//! the sketch above as live test code.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(unused_imports)]
