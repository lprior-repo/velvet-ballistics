// SPDX-License-Identifier: MIT
//
// Extern surface for `vb_rpch_unsupported_recovery_state` Verus spec.
//
// ============================================================================
// WEAK PRODUCTION BINDING (production_inner mirror)
// ============================================================================
//
// This file is the production-binding surface for the
// `vb_rpch_unsupported_recovery_state.rs` Verus spec. It contains:
//
//   1. A direct `#[path]` inclusion of the verbatim production mirror
//      at
//      `verification/verus/production_inner/unsupported_recovery_state_production.rs`,
//      which is a verbatim copy of
//      `crates/vb_storage/src/recovery/types.rs:553-626`
//      (the `UnsupportedRecoveryState` struct and its `impl` block)
//      with only the `Serialize, Deserialize, PartialEq, Eq, Debug`
//      proc-macro derives dropped (they require proc-macro crate
//      registration unavailable in `verus --crate-type=lib`) and the
//      `#[must_use]` lint hints dropped. This structural binding
//      means any rename, field reorder, or signature change in the
//      production source breaks this Verus build at compile time
//      (the phantom `prod_items_drift_check` below forces Rust to
//      resolve every production item by name). See the drift policy
//      header in
//      `production_inner/unsupported_recovery_state_production.rs`.
//
//   2. A Verus-mode mirror struct `UnsupportedRecoveryState`
//      declared inside the `verus!` block below, with the same field
//      names, same field order, and same field types as the
//      production source. The mirror is NOT marked
//      `#[verifier::external]` so Verus can see the field shape in
//      spec mode; the methods on the mirror ARE marked
//      `#[verifier::external]` so Verus skips body verification.
//      This matches the established pattern in
//      `extern_vb_xi2f_error_mapping.rs` (line 142+): the
//      Verus-mode mirror is a separate declaration in the `verus!`
//      block, and the production `#[path]` mirror exists for
//      drift-detection via Rust resolution at compile time.
//
//   3. The `production::UnsupportedRecoveryState` re-export so the
//      companion spec file can address the Verus-mode mirror type
//      uniformly. The production `prod_src` mirror is referenced by
//      the `prod_items_drift_check` phantom only — the spec fns and
//      proof fns operate on the Verus-mode mirror, not on
//      `prod_src::UnsupportedRecoveryState`.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
// Production source: `crates/vb_storage/src/recovery/types.rs:553-626`.
//
// Production mirror included via `#[path]` (drift-detection only):
//   - `UnsupportedRecoveryState` struct       <- types.rs:553-563
//        (4 bool fields: slot_values, slot_taint, action_payloads,
//         pending_actions — no derives, public fields)
//   - `UnsupportedRecoveryState::SUPPORTED`   <- types.rs:567-572
//        (const init: all four fields false)
//   - `UnsupportedRecoveryState::event_slot_taint_unsupported`
//                                            <- types.rs:575-581
//   - `UnsupportedRecoveryState::slot_values_unsupported`
//                                            <- types.rs:583-590
//   - `UnsupportedRecoveryState::pending_actions_unsupported`
//                                            <- types.rs:592-599
//   - `UnsupportedRecoveryState::union`       <- types.rs:601-610
//        (body: flagwise OR across all 4 fields)
//   - `UnsupportedRecoveryState::is_fully_supported`
//                                            <- types.rs:612-616
//        (body: conjunction of `!flag` for all 4 fields)
//   - `UnsupportedRecoveryState::union_matches_flags`
//                                            <- types.rs:618-625
//        (body: conjunction of `union.f == (a.f || b.f)` for all 4
//         fields)
//
// Verus-mode mirror declared in this file's `verus!` block:
//   - `UnsupportedRecoveryState` struct       <- mirror of types.rs:553-563
//        (4 pub bool fields, `#[derive(Clone, Copy)]` only)
//   - `UnsupportedRecoveryState::SUPPORTED`   <- mirror of types.rs:567-572
//        (const init: all four fields false)
//   - `UnsupportedRecoveryState::event_slot_taint_unsupported`
//                                            <- mirror of types.rs:575-581
//   - `UnsupportedRecoveryState::slot_values_unsupported`
//                                            <- mirror of types.rs:583-590
//   - `UnsupportedRecoveryState::pending_actions_unsupported`
//                                            <- mirror of types.rs:592-599
//   - `UnsupportedRecoveryState::union`       <- mirror of types.rs:601-610
//        (body: flagwise OR across all 4 fields;
//         `#[verifier::external]`)
//   - `UnsupportedRecoveryState::is_fully_supported`
//                                            <- mirror of types.rs:612-616
//        (body: conjunction of `!flag` for all 4 fields;
//         `#[verifier::external]`)
//   - `UnsupportedRecoveryState::union_matches_flags`
//                                            <- mirror of types.rs:618-625
//        (body: conjunction of `union.f == (a.f || b.f)` for all 4
//         fields; `#[verifier::external]`)
//
// The companion spec file `vb_rpch_unsupported_recovery_state.rs`
// attaches `assume_specification` bridges for the mirror's `union`,
// `is_fully_supported`, and `union_matches_flags` methods. The
// `event_slot_taint_unsupported`, `slot_values_unsupported`, and
// `pending_actions_unsupported` constructors are mirrored for
// drift-detection completeness but are NOT bridged because the
// original spec (lines 56-86 of
// `vb_rpch_unsupported_recovery_state.rs`) does not reason about
// them — they are out of scope for the 4 production-bound proof
// obligations (PO-1..PO-4).
//
// ============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
//
// The production bodies of `union`, `is_fully_supported`, and
// `union_matches_flags` are NOT verified by Verus. The mirror
// methods are marked `#[verifier::external]` so Verus skips body
// verification. The `assume_specification` bridges in the companion
// spec file state the production behavior the spec proofs
// discharge. The production mirror module
// (`production_inner/unsupported_recovery_state_production.rs`) is
// also `#[verifier::external]` at module level inside the `#[path]`
// inclusion, so its bodies are opaque to Verus. Drift between the
// production mirror and the production source is reported as
// binding-debt tracked outside Verus.
//
// ============================================================================
// BINDING DEBT
// ============================================================================
//
// D1: The mirror drops the `Serialize, Deserialize, PartialEq, Eq,
//     Debug` proc-macro derives from the production mirror struct.
//     Field shape is preserved exactly; only the derives are
//     dropped. See the header of
//     `production_inner/unsupported_recovery_state_production.rs`
//     for the per-derive rationale. Spec-side struct equality is
//     discharged by the closed spec fn `unsupported_state_eq` in
//     the companion spec file (the spec fn compares the four bool
//     fields directly, which is the production semantics for
//     `#[derive(PartialEq)]` on this all-bool struct).
//
// D2: The `SUPPORTED` const value is duplicated between the
//     production mirror and the Verus-mode mirror. The production
//     mirror holds the canonical value; the Verus-mode mirror
//     declares the same initializers for spec-mode visibility.
//     Drift in the production-side `SUPPORTED` value is not
//     automatically caught — the spec proof asserts the Verus-mode
//     mirror's `SUPPORTED` shape, which is the same as production
//     by transcription. This is acceptable for the current 4 proof
//     obligations because the production
//     `RecoveryFrameSeed::unsupported: UnsupportedRecoveryState`
//     invariant (types.rs:648) and the `ProductionError::UnsupportedRecovery`
//     production test at
//     `crates/vb_runtime/src/recovery/tests.rs:395-453` both
//     anchor the all-false shape; any change to `SUPPORTED` would
//     break those production tests first.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION INCLUSION via #[path] — STRUCTURAL drift detection
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the verbatim production mirror at
// `production_inner/unsupported_recovery_state_production.rs`. The
// mirror is marked `#[verifier::external]` at module level so the
// production bodies are opaque to Verus; the inclusion still
// validates Rust resolution (field names, field types, fn
// signatures, const item presence) at compile time. Any drift in
// the production struct, const, or impl block surface breaks the
// `prod_items_drift_check` helper below.

#[verifier::external]
#[path = "production_inner/unsupported_recovery_state_production.rs"]
pub mod prod_src;

// Phantom drift-detection helper. The body is
// `#[verifier::external]` (opaque to Verus), but the
// `prod_src::UnsupportedRecoveryState::*` references force Rust to
// resolve every production item at compile time. A rename of any
// production item — the struct, the `SUPPORTED` const, any of the
// seven const fn methods, or any field name — breaks the lookup
// and fails this Verus build.
#[verifier::external]
fn prod_items_drift_check(a: prod_src::UnsupportedRecoveryState, b: prod_src::UnsupportedRecoveryState) {
    // Const item drift detection: forces Rust to resolve
    // `SUPPORTED` by name and read its type.
    let _supported: prod_src::UnsupportedRecoveryState = prod_src::UnsupportedRecoveryState::SUPPORTED;
    // Constructor const fn drift detection: forces Rust to resolve
    // all three "single-flag" constructors by name and signature.
    let _ev = prod_src::UnsupportedRecoveryState::event_slot_taint_unsupported();
    let _sv = prod_src::UnsupportedRecoveryState::slot_values_unsupported();
    let _pa = prod_src::UnsupportedRecoveryState::pending_actions_unsupported();
    // Method drift detection: forces Rust to resolve `union`,
    // `is_fully_supported`, and `union_matches_flags` by name and
    // signature. Field access (`a.slot_values`) forces Rust to
    // resolve all four production field names.
    let _u = a.union(b);
    let _f = a.is_fully_supported();
    let _m = a.union_matches_flags(b, a.union(b));
    let _sv1 = a.slot_values;
    let _st1 = a.slot_taint;
    let _ap1 = a.action_payloads;
    let _pa1 = a.pending_actions;
}

// ---------------------------------------------------------------------------
// Verus-mode mirror — STRUCTURAL mirror visible to Verus
// ---------------------------------------------------------------------------
//
// The Verus-mode mirror is a separate declaration (not a re-export
// from `prod_src`) because the production mirror module is
// `#[verifier::external]` at module level and therefore opaque to
// Verus. Without a Verus-visible mirror, spec fns in the companion
// spec file would fail to type-check (Verus rejects references to
// opaque types in spec mode). The mirror's field shape is byte-
// identical to the production source: same field names, same field
// order, same `bool` field types. The mirror is annotated at the
// top of every section with the originating production line range
// so regeneration against `crates/vb_storage/src/recovery/types.rs`
// is mechanical.
//
// Method bodies on the mirror are `#[verifier::external]` so
// Verus does not attempt to verify them. The companion spec file
// attaches `assume_specification` bridges to the mirror's
// `is_fully_supported`, `union`, and `union_matches_flags` methods.

/// Mirror of `UnsupportedRecoveryState` at
/// `crates/vb_storage/src/recovery/types.rs:553-563`. All four
/// fields are `bool`; the mirror is bit-identical to production
/// modulo the dropped proc-macro derives (see BINDING DEBT D1).
///
/// `PartialEq, Eq, Debug, Serialize, Deserialize` are intentionally
/// NOT derived: the macro-generated
/// `core::intrinsics::discriminant_value` and the proc-macro
/// derives are not supported by Verus 0.2026.05.05 (Rust 1.95.0)
/// standalone. Spec proofs compare via the closed spec fn
/// `unsupported_state_eq` in the companion spec file.
#[derive(Clone, Copy)]
pub struct UnsupportedRecoveryState {
    /// Slot values are not present in current slot-written records.
    pub slot_values: bool,
    /// Slot taint is not present in current slot-written records.
    pub slot_taint: bool,
    /// Action payload/result bodies are not present in current action records.
    pub action_payloads: bool,
    /// Pending action resumability cannot be projected into the runtime frame yet.
    pub pending_actions: bool,
}

impl UnsupportedRecoveryState {
    /// Mirror of `UnsupportedRecoveryState::SUPPORTED` at
    /// `crates/vb_storage/src/recovery/types.rs:567-572`.
    pub const SUPPORTED: Self = Self {
        slot_values: false,
        slot_taint: false,
        action_payloads: false,
        pending_actions: false,
    };

    /// Mirror of `UnsupportedRecoveryState::event_slot_taint_unsupported`
    /// at `crates/vb_storage/src/recovery/types.rs:575-581`.
    pub const fn event_slot_taint_unsupported() -> Self {
        Self {
            slot_taint: true,
            ..Self::SUPPORTED
        }
    }

    /// Mirror of `UnsupportedRecoveryState::slot_values_unsupported`
    /// at `crates/vb_storage/src/recovery/types.rs:583-590`.
    pub const fn slot_values_unsupported() -> Self {
        Self {
            slot_values: true,
            ..Self::SUPPORTED
        }
    }

    /// Mirror of `UnsupportedRecoveryState::pending_actions_unsupported`
    /// at `crates/vb_storage/src/recovery/types.rs:592-599`.
    pub const fn pending_actions_unsupported() -> Self {
        Self {
            pending_actions: true,
            ..Self::SUPPORTED
        }
    }

    /// Mirror of `UnsupportedRecoveryState::union` at
    /// `crates/vb_storage/src/recovery/types.rs:601-610`. Body:
    /// flagwise OR across all 4 fields.
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`). The `assume_specification`
    /// bridge in the companion spec file attaches the production
    /// contract.
    #[verifier::external]
    pub const fn union(self, other: Self) -> Self {
        Self {
            slot_values: self.slot_values || other.slot_values,
            slot_taint: self.slot_taint || other.slot_taint,
            action_payloads: self.action_payloads || other.action_payloads,
            pending_actions: self.pending_actions || other.pending_actions,
        }
    }

    /// Mirror of `UnsupportedRecoveryState::is_fully_supported` at
    /// `crates/vb_storage/src/recovery/types.rs:612-616`. Body:
    /// conjunction of `!flag` for all 4 fields.
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`).
    #[verifier::external]
    pub const fn is_fully_supported(self) -> bool {
        !self.slot_values && !self.slot_taint && !self.action_payloads && !self.pending_actions
    }

    /// Mirror of `UnsupportedRecoveryState::union_matches_flags` at
    /// `crates/vb_storage/src/recovery/types.rs:618-625`. Body:
    /// conjunction of `union.f == (a.f || b.f)` for all 4 fields.
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`).
    #[verifier::external]
    pub const fn union_matches_flags(self, other: Self, union: Self) -> bool {
        union.slot_values == (self.slot_values || other.slot_values)
            && union.slot_taint == (self.slot_taint || other.slot_taint)
            && union.action_payloads == (self.action_payloads || other.action_payloads)
            && union.pending_actions == (self.pending_actions || other.pending_actions)
    }
}

} // verus!
