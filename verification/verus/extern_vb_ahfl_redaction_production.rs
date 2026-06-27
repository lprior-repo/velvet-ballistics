// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for `vb_ahfl_redaction_production` Verus spec.
//
// WEAK PRODUCTION BINDING (GOD RULE 2 compliance) — redaction scope
// ============================================================================
//
// This file is the production-binding surface for the
// `vb_ahfl_redaction_production.rs` Verus spec. It contains a direct
// `#[path]` inclusion of the in-tree mirror at
// `verification/verus/production_inner/vb_ahfl_redaction_production_inner.rs`,
// which is a verbatim copy of the relevant production surface from
//   crates/vb_cli/src/commands_ai_context.rs:399-422
//   crates/vb_core/src/value.rs:14-25
//   xtask/src/evidence/release_contract.rs:54-64
// with `serde_json` / `postcard` / `vb_storage::RunSnapshot` extern
// type references abstracted (no installs allowed by the task brief).
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
// The ORIGINAL spec file declared FIVE spec mirror types it claimed to
// bind to `vb_ui_model::redact`:
//
//   - SpecSecretSensitivity { Sensitive, NonSensitive, Unknown }
//   - SpecTaint             { Clean, DerivedFromSecret, Secret }
//   - SpecRedactedValueView { is_tainted, taint_marker, digest_present,
//                             summary_len }
//   - spec_summary_bounded(summary_len)              (constant 64)
//   - spec_digest_present_for_sensitive(sens, view)
//   - spec_taint_invariant(sens, taint, view)
//
// The `vb_ui_model` crate has been REMOVED from the current workspace
// (see `crates/vb_cli/Cargo.toml:35`:
//     `# vb_ui_model is removed from the current workspace scope.`).
// None of the original mirror types exist in production source. After
// auditing the workspace, the ACTUAL production redaction surface
// comprises two groups:
//
//   RUNTIME REDACTION (vb_cli::commands_ai_context):
//     - `redacted_slot_value(slot, value, snapshot) -> Value`
//         Returns `Value::String("[REDACTED]")` (10 chars) when
//         `slot_is_secret_or_derived(slot, snapshot)` is true
//         (production lines 404-406). Otherwise returns the decoded
//         slot value as a string, or `Value::Null` if value is None,
//         or `Value::String("[UNDECODED]")` (11 chars) on decode
//         failure (production lines 407-412).
//     - `slot_is_secret_or_derived(slot, snapshot) -> bool`
//         Reads `snapshot.taint.get(slot.as_usize())` and returns
//         `is_some_and(|raw| matches!(*raw, 1 | 2))`. The literals
//         `1` and `2` are `Taint::DerivedFromSecret` and
//         `Taint::Secret` discriminants (production lines 415-422).
//
//   FIXTURE-EVIDENCE REDACTION (xtask::evidence::release_contract):
//     - `REDACTION_CLASSES: [(&str, &str); 6]` — table of 6 secret
//       classes (sentinel, api_key, token, password, idempotency_key,
//       tainted_fixture_value) and their `[REDACTED:CLASS]` markers.
//       Consumed by `release_validators.rs` and `tooling_and_gate_types.rs`
//       during `ai-release` evidence generation.
//
// Per the user's instruction, this extern file binds the FULL runtime
// redaction surface (Taint, slot_is_secret_or_derived, redacted_slot_value)
// plus the REDACTION_CLASSES fixture table. The five spec types in the
// original file are retained as mathematical models in the companion
// spec file with explicit binding debt comments.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
//
//   * `slot_is_secret_or_derived_mirror` body is `#[verifier::external]` —
//     Verus does NOT verify it. The `assume_specification` bridge in
//     the companion spec file states the production contract.
//   * `redacted_slot_value_mirror` body is `#[verifier::external]` — same.
//   * `SpecTaintProduction` discriminants are plain Rust constants; Verus
//     verifies the body.
//   * `SPEC_REDACTION_CLASSES` is a plain Rust constant table; Verus
//     verifies its construction (literal numeric values, not strings).
//   * Plain Rust predicate functions on `SpecRedactedSlotValueProduction`
//     are Verus-verified inside the `verus!` block.
//   * The `serde_json::Value` return type of production `redacted_slot_value`
//     is abstracted to four `bool` flags plus two `usize` lengths because
//     `serde_json` is not in scope. The projection only needs to assert
//     the structural shape of the output (marker present, marker length
//     known, value string bounded).
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
// `production_inner/vb_ahfl_redaction_production_inner.rs`. The mirror
// is a verbatim copy of `crates/vb_cli/src/commands_ai_context.rs:399-422`
// and `crates/vb_core/src/value.rs:14-25` and
// `xtask/src/evidence/release_contract.rs:54-64` with
// `serde_json`/`postcard`/`vb_storage::RunSnapshot` extern type
// references abstracted. Any drift in field NAME or method signature
// breaks the verification build (the `assume_specification` bridge
// becomes inconsistent).
#[path = "production_inner/vb_ahfl_redaction_production_inner.rs"]
pub mod production_redaction;

} // verus!

// Re-export the production types so the spec file can reference them
// via `crate::production::production_redaction::*`.
pub use production_redaction::{
    SpecTaintProduction,
    SpecRedactedSlotValueProduction,
    TAINT_NONE_SENTINEL,
    SPEC_REDACTION_CLASS_COUNT,
    SPEC_REDACTED_MARKER_PREFIX_LEN,
    SPEC_REDACTED_MARKER_SUFFIX_LEN,
    spec_redaction_class_count,
    spec_redacted_marker_len,
    slot_is_secret_or_derived_mirror,
    redacted_slot_value_mirror,
    SPEC_REDACTION_CLASSES,
};