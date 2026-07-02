// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for `vb_ahfl_metadata_envelope_production` Verus spec.
//
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance) — cli_envelope scope
// ============================================================================
//
// This file is the production-binding surface for the
// `vb_ahfl_metadata_envelope_production.rs` Verus spec. It contains
// a direct `#[path]` inclusion of the in-tree mirror at
// `verification/verus/production_inner/vb_ahfl_metadata_envelope_production_inner.rs`,
// which is a verbatim copy of the relevant production surface from
// `crates/vb_cli/src/cli_envelope.rs:18-174` with `serde_json`-dependent
// items abstracted (no installs allowed by the task brief).
//
// The mirror is included via `#[path]` from inside `verus!` (WITHOUT
// module-level `#[verifier::external]`) so the type declarations are
// nameable in spec mode. The companion spec file attaches
// `assume_specification` contracts to the production-bound exec
// methods.
//
// ============================================================================
// BINDING SCOPE — honest disclosure
// ============================================================================
//
// The ORIGINAL spec file declared TWO mirror types:
//
//   - SpecEnvelopeKind      (6 variants: Success, Error, DiagnosticReport,
//                            Status, Event, Workflow)
//   - SpecMetadataEnvelope  { run_id, command, timestamp }
//
// The ORIGINAL spec header claims these mirror
// `vb_ui_model::envelope::types::MetadataEnvelope` and
// `vb_ui_model::envelope::types::EnvelopeKind`. As of this writing,
// the `vb_ui_model` crate has been REMOVED from the workspace (see
// `crates/vb_cli/Cargo.toml`:
//     `# vb_ui_model is removed from the current workspace scope.`).
// A repo-wide grep for `MetadataEnvelope`, `EnvelopeKind`,
// `canonicalize_ui_artifact` returns ONLY references inside the
// verus spec files themselves — there is no production Rust source
// for those exact types.
//
// The CLOSEST production surface in the current workspace is the
// `vb_cli::cli_envelope` module (different name, different shape):
//
//   - Production `Kind` has 17 variants (cli_envelope.rs:45-63); the
//     original 6-variant `SpecEnvelopeKind` is a STRICT SUBSET only
//     by informal name overlap (DiagnosticReport, CliStatus/SystemStatus
//     vs Status, WorkflowGraph/WorkflowExplanation vs Workflow). Three
//     original variants have no production equivalent at all:
//     Success, Error, Event.
//   - Production envelopes have shape `{ schema_version: String, kind:
//     String, data: Value }`. Production has NO `run_id`, `command`,
//     or `timestamp` fields on the envelope itself (timestamps live
//     inside individual JournalEvents, not on the envelope struct).
//   - The production constructor `build_envelope(data, kind)` returns
//     a `serde_json::Value` (a JSON object), not a typed struct. There
//     is NO production `canonicalize_ui_artifact` function.
//
// Per the user's instruction, this extern file binds the FULL
// production cli_envelope surface (Kind, SCHEMA_VERSION,
// build_envelope, serialize_with_version, EnvelopeError, from_str,
// as_str) so the spec file can re-state its obligations against
// production-realistic data shapes.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
//
//   * `build_envelope_mirror` body is `#[verifier::external]` — Verus
//     does NOT verify it. The `assume_specification` bridge in the
//     companion spec file states the projection contract.
//   * `serialize_with_version_mirror` body is `#[verifier::external]` —
//     same treatment.
//   * `SpecKindProduction::as_str` and `SpecKindProduction::from_str`
//     bodies are plain Rust and Verus-verified (the bodies are
//     exhaustive enum matches over a fixed set of literal `&'static str`s).
//   * `SpecEnvelopeProduction` field-level accessors (the trivial
//     `.is_valid()` and `.schema_version_nonempty()` predicates below)
//     are plain Rust and Verus-verified.
//   * `SPEC_SCHEMA_VERSION` is a compile-time constant — Verus treats
//     it as opaque.
//   * The `serde_json::Value` return type of production
//     `build_envelope` is abstracted to `data_present: bool` because
//     `serde_json` is not in scope in a standalone
//     `verus --crate-type=lib` invocation (no installs allowed by task
//     brief). The projection only ever needs to assert the three keys
//     are present and that `data` is non-null, both of which are
//     captured by `data_present: bool`.
// ============================================================================

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unsafe_code)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION MIRROR INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the in-tree mirror at
// `production_inner/vb_ahfl_metadata_envelope_production_inner.rs`.
// The mirror is a verbatim copy of `crates/vb_cli/src/cli_envelope.rs:18-174`
// with `serde_json`-dependent items abstracted: the `Value` return
// types of `build_envelope` and `serialize_with_version` are replaced
// by typed `SpecEnvelopeProduction` structs. Any drift in field NAME
// or method signature breaks the verification build (the
// `assume_specification` bridge becomes inconsistent).
#[path = "production_inner/vb_ahfl_metadata_envelope_production_inner.rs"]
pub mod production_envelope;

} // verus!

// Re-export the production types so the spec file can reference them
// via `crate::production::production_envelope::*`.
pub use production_envelope::{
    SPEC_SCHEMA_VERSION,
    SPEC_SCHEMA_VERSION_LEN,
    SpecKindProduction,
    SpecEnvelopeProduction,
    SpecEnvelopeErrorProduction,
    build_envelope_mirror,
    serialize_with_version_mirror,
    NoProductionSourceEnvelopeKind,
    NoProductionSourceMetadataEnvelope,
};