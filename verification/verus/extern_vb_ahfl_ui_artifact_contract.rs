// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for `vb_ahfl_ui_artifact_contract` Verus spec.
//
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance) — UI artifact scope
// ============================================================================
//
// This file is the production-binding surface for the
// `vb_ahfl_ui_artifact_contract.rs` Verus spec. It contains a direct
// `#[path]` inclusion of the in-tree mirror at
// `verification/verus/production_inner/vb_ahfl_ui_artifact_contract_inner.rs`,
// which is a verbatim copy of the relevant production surface from
//   crates/vb_cli/src/cli_envelope.rs:18-174
//   crates/vb_storage/src/journal/core.rs:25-48
//   crates/vb_storage/src/preview.rs:58-130
//   crates/vb_cli/src/commands_ai_context.rs:399-413
// with `serde_json`-dependent items abstracted (no installs allowed
// by the task brief).
//
// The mirror is included via `#[path]` from inside `verus!` so the
// type declarations are nameable in spec mode. The companion spec
// file attaches `assume_specification` contracts to the production-
// bound exec methods.
//
// ============================================================================
// BINDING SCOPE — honest disclosure
// ============================================================================
//
// The ORIGINAL spec file declared SIX spec mirror types:
//
//   - ArtifactKind       (4 variants: WorkflowGraph, RunEventTable,
//                         AiContext, VerificationReport)
//   - SecretSensitivity  (3 variants: Public, Secret, Unknown)
//   - UiArtifactMetadata { schema_version, kind, generated_at_present,
//                          source_present, redaction_status_present }
//   - BoundedCollectionFacts { len, limit, truncated,
//                              truncation_metadata_present }
//   - RedactedValueViewFacts { raw_secret_present,
//                              redaction_status_present, digest_present,
//                              summary_len, summary_limit }
//   - GraphEventFacts    { node_count, edge_count, event_count,
//                          max_edge_from_step, max_edge_to_step,
//                          max_event_step, seq_strictly_ordered,
//                          step_identity_stable }
//
// As of this writing, none of these exact types exists in the production
// workspace (the `vb_ui_model` crate has been REMOVED — see
// `crates/vb_cli/Cargo.toml:35`). All six are bound here to their
// closest production analogues:
//
//   - ArtifactKind::WorkflowGraph     -> Kind::WorkflowGraph
//   - ArtifactKind::RunEventTable     -> Kind::RunEvents
//   - ArtifactKind::AiContext         -> Kind::AiContextPacket
//   - ArtifactKind::VerificationReport -> Kind::VerificationReport
//   - SecretSensitivity::Public       -> clean taint (raw value passes through)
//   - SecretSensitivity::Secret       -> taint == 2 (raw secret bytes)
//   - SecretSensitivity::Unknown      -> taint == 1 (Derived, fail-closed)
//   - UiArtifactMetadata  -> SpecEnvelopeProduction
//                            (cli_envelope::build_envelope output)
//   - BoundedCollectionFacts -> SpecBoundedCollectionProduction
//                            (EventReplayLimit + DecodedPreview.truncated)
//   - RedactedValueViewFacts -> SpecRedactedValueViewProduction
//                            (redacted_slot_value return shape)
//   - GraphEventFacts      -> SpecGraphEventFactsProduction
//                            (Kind::WorkflowGraph envelope + journal limits)
//
// The four mappings are the only production surfaces in the current
// workspace that back the original spec types. Re-introducing
// `vb_ui_model` would close the remaining informal-shape gap (the spec
// types are NOT 1:1 with these production surfaces; they are
// math-level projections of the production-derived facts).
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
//
//   * `production::build_envelope_mirror` body is `#[verifier::external]` —
//     Verus does NOT verify it. The `assume_specification` bridge in the
//     companion spec file states the contract.
//   * `production::redacted_slot_value_mirror` body is
//     `#[verifier::external]` — same treatment.
//   * `production::make_bounded_collection_mirror` body is
//     `#[verifier::external]` — same treatment.
//   * `production::make_graph_event_facts_mirror` body is
//     `#[verifier::external]` — same treatment.
//   * `SpecKindProduction::as_str` and `SpecKindProduction::from_str`
//     bodies are plain Rust and Verus-verified (exhaustive enum matches
//     over a fixed set of literal `&'static str`s).
//   * `SpecEnvelopeProduction`, `SpecBoundedCollectionProduction`,
//     `SpecRedactedValueViewProduction`, `SpecGraphEventFactsProduction`
//     field-level accessors are plain Rust and Verus-verified.
//   * The `serde_json::Value` return type of production `build_envelope`
//     and `redacted_slot_value` is abstracted to typed structs because
//     `serde_json` is not in scope in a standalone
//     `verus --crate-type=lib` invocation (no installs allowed).
// ============================================================================

#![allow(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION MIRROR INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the in-tree mirror at
// `production_inner/vb_ahfl_ui_artifact_contract_inner.rs`. The
// mirror is a verbatim copy of the production surface from
//   crates/vb_cli/src/cli_envelope.rs:18-174
//   crates/vb_storage/src/journal/core.rs:25-48
//   crates/vb_storage/src/preview.rs:58-130
//   crates/vb_cli/src/commands_ai_context.rs:399-413
// with `serde_json`-dependent items abstracted. Any drift in field
// NAME or method signature breaks the verification build (the
// `assume_specification` bridge becomes inconsistent).
#[path = "production_inner/vb_ahfl_ui_artifact_contract_inner.rs"]
pub mod production_ui_artifact;

} // verus!

// Re-export the production types so the spec file can reference them
// via `crate::production::production_ui_artifact::*`.
pub use production_ui_artifact::{
    SPEC_SCHEMA_VERSION,
    SPEC_SCHEMA_VERSION_LEN,
    SpecKindProduction,
    SpecEnvelopeProduction,
    SpecBoundedCollectionProduction,
    SpecRedactedValueViewProduction,
    SpecGraphEventFactsProduction,
    build_envelope_mirror,
    make_bounded_collection_mirror,
    redacted_slot_value_mirror,
    make_graph_event_facts_mirror,
    NoProductionSourceArtifactKind,
    NoProductionSourceSecretSensitivity,
};