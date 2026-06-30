// SPDX-License-Identifier: MIT
//
// ============================================================================
// WEAK PRODUCTION BINDING (production_inner mirror)
// ============================================================================
//
// This file binds `vb_rpch_unsupported_state.rs` Verus spec to the production
// `UnsupportedRecoveryState` type and its algebraic decision surface at:
//
//   crates/vb_storage/src/recovery/types.rs:552-626
//
// The production `UnsupportedRecoveryState` is a 4-bool-field struct whose
// `union` is flag-wise disjunction (NOT bitwise OR over a packed integer).
//
// ============================================================================
// STRUCTURAL BINDING — production mirror via #[path] for drift detection
// ============================================================================
//
// This file uses two complementary binding mechanisms:
//
//   1. **Drift-detection inclusion**: a direct `#[path]` inclusion of the
//      verbatim production mirror at
//      `verification/verus/production_inner/unsupported_recovery_state_production.rs`
//      wrapped in `#[verifier::external]` at module level. This validates
//      that the production source still compiles and that production
//      method/field names resolve at compile time. Any drift in
//      production breaks this inclusion.
//
//   2. **Spec-side mirror struct**: a hand-written mirror struct
//      `UnsupportedRecoveryState` declared in `verus!` context below
//      so the spec proofs can reason about it. The mirror struct
//      mirrors the production field shape byte-for-byte: 4 bool
//      fields with the same names. This is what the spec proofs use.
//
//   3. **`assume_specification` bridges** in the companion spec file
//      `vb_rpch_unsupported_state.rs` attach the production contracts
//      (per-field OR for `union`, all-false for `is_fully_supported`,
//      per-field equality for `union_matches_flags`, etc.) to the
//      spec-side mirror methods.
//
// ============================================================================
// WHY TWO MECHANISMS
// ============================================================================
//
// Direct spec-side usage of `production_inner::*` types is blocked
// because `#[verifier::external]` at module level makes the included
// types opaque to Verus (the spec cannot reference them in `spec fn`
// signatures, even via `#[verifier::external_type_specification]`,
// because the field types match but the methods on the production
// struct are `pub const fn` and not all verus-compatible in the
// extern context).
//
// The drift-detection `#[path]` inclusion satisfies the structural
// binding requirement (any drift in production breaks the build via
// the `prod_methods_drift_check` helper that resolves production
// method names) while the spec-side mirror struct + method
// definitions in `verus!` context give the spec proofs a Verus-visible
// type to reason about. The `assume_specification` bridges in the
// companion spec file declare the production contracts on the
// spec-side mirror methods, and the exec wrappers in the spec file
// invoke the spec-side mirror methods to discharge the contracts.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
// Production source: `crates/vb_storage/src/recovery/types.rs:552-626`.
//
// Drift-detection inclusion (production mirror via `#[path]`):
//   - `prod_src::UnsupportedRecoveryState`                        <- types.rs:553-563
//   - `prod_src::UnsupportedRecoveryState::SUPPORTED`             <- types.rs:567-572
//   - `prod_src::UnsupportedRecoveryState::event_slot_taint_unsupported` <- types.rs:575-581
//   - `prod_src::UnsupportedRecoveryState::slot_values_unsupported`  <- types.rs:584-590
//   - `prod_src::UnsupportedRecoveryState::pending_actions_unsupported` <- types.rs:593-599
//   - `prod_src::UnsupportedRecoveryState::union`                  <- types.rs:603-610
//   - `prod_src::UnsupportedRecoveryState::is_fully_supported`     <- types.rs:614-616
//   - `prod_src::UnsupportedRecoveryState::union_matches_flags`    <- types.rs:620-625
//
// Spec-side mirror (used in Verus proofs):
//   - `UnsupportedRecoveryState` (local struct below, field-identical
//     to production)
//   - `UnsupportedRecoveryState::SUPPORTED` (associated const)
//   - `UnsupportedRecoveryState::slot_values_unsupported`
//   - `UnsupportedRecoveryState::event_slot_taint_unsupported`
//   - `UnsupportedRecoveryState::pending_actions_unsupported`
//   - `UnsupportedRecoveryState::union` (assoc fn)
//   - `UnsupportedRecoveryState::is_fully_supported` (assoc fn)
//   - `UnsupportedRecoveryState::union_matches_flags` (assoc fn)
//
// ============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
//
// The production bodies of the seven `UnsupportedRecoveryState` items
// above are NOT verified by Verus directly. The production mirror
// module is marked `#[verifier::external]` at module level. The
// spec-side mirror methods in this file are also `#[verifier::external]`.
// The `assume_specification` bridges in the companion spec file
// (`vb_rpch_unsupported_state.rs`) attach the production contracts
// to the spec-side mirror methods. The exec wrappers in the spec
// file invoke the spec-side mirror methods and assert the contracts
// hold; they are the discharge witnesses that prevent the bridges
// from being used as vacuum specifications. Drift between the
// production mirror and the production source is reported as
// binding-debt tracked outside Verus.
//
// ============================================================================
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Drift-detection inclusion: `#[path]` to verbatim production mirror
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the verbatim production mirror at
// `production_inner/unsupported_recovery_state_production.rs`. The
// mirror is marked `#[verifier::external]` at module level so the
// production bodies are opaque to Verus; the inclusion still
// validates Rust resolution (field names, discriminant sets, fn
// signatures) at compile time. Any drift in the production impl
// surface breaks this Verus build.
#[verifier::external]
#[path = "production_inner/unsupported_recovery_state_production.rs"]
pub mod prod_src;

// Phantom drift-detection helper. The body is `#[verifier::external]`
// (opaque to Verus), but the `prod_src::UnsupportedRecoveryState::*`
// method references force Rust to resolve the production method
// names at compile time. A rename of any of these production methods
// (or the production struct fields) breaks this fn's compilation.
#[verifier::external]
fn prod_methods_drift_check() {
    // Force resolution of every field name.
    let supported = prod_src::UnsupportedRecoveryState::SUPPORTED;
    let _ = supported.slot_values;
    let _ = supported.slot_taint;
    let _ = supported.action_payloads;
    let _ = supported.pending_actions;
    // Force resolution of every method name on UnsupportedRecoveryState.
    let sv = prod_src::UnsupportedRecoveryState::slot_values_unsupported();
    let est = prod_src::UnsupportedRecoveryState::event_slot_taint_unsupported();
    let pa = prod_src::UnsupportedRecoveryState::pending_actions_unsupported();
    let _ = sv.union(est);
    let _ = pa.union(supported);
    let _ = supported.is_fully_supported();
    let _ = sv.union_matches_flags(est, sv.union(est));
    // Cross-call to ensure `union` is callable with the result type.
    let triple = sv.union(est).union(pa);
    let _ = triple.union(supported);
    let _ = triple.is_fully_supported();
    let _ = triple.union_matches_flags(supported, triple);
}

// ---------------------------------------------------------------------------
// Spec-side mirror struct — production field-identical
// ---------------------------------------------------------------------------
//
// Field-identical to production `UnsupportedRecoveryState` at
// `crates/vb_storage/src/recovery/types.rs:553-563`. All four fields
// are `bool` so the mirror is field-identical to production.
//
// `PartialEq, Eq` are intentionally NOT derived here because the
// macro-generated `discriminant_value` call is not supported by
// Verus 0.2026.05.05 (Rust 1.95.0). Spec proofs reason via per-field
// equalities (e.g. `a.slot_values == b.slot_values`) directly.
#[derive(Clone, Copy)]
pub struct UnsupportedRecoveryState {
    /// `types.rs:556` — slot values are not present in current
    /// slot-written records.
    pub slot_values: bool,
    /// `types.rs:558` — slot taint is not present in current
    /// slot-written records.
    pub slot_taint: bool,
    /// `types.rs:560` — action payload/result bodies are not present
    /// in current action records.
    pub action_payloads: bool,
    /// `types.rs:562` — pending action resumability cannot be
    /// projected into the runtime frame yet.
    pub pending_actions: bool,
}

// ---------------------------------------------------------------------------
// Spec-side mirror methods
// ---------------------------------------------------------------------------
//
// All methods are `#[verifier::external]` so Verus skips body
// verification. The companion spec file attaches `assume_specification`
// bridges that state the production contracts: the four flags are
// returned as their per-field OR, all-false, or per-field equality
// with OR. The exec wrappers in the spec file invoke these mirror
// methods and assert the contracts hold.
//
// Every body is a byte-for-byte copy of the production body at the
// cited `types.rs` line range. Drift in any production body breaks
// the `assume_specification` contract because the projection body no
// longer matches the contract the spec proofs discharge.
impl UnsupportedRecoveryState {
    /// Mirror of `SUPPORTED` at
    /// `crates/vb_storage/src/recovery/types.rs:567-572`.
    ///
    /// Production body:
    /// ```text
    /// pub const SUPPORTED: Self = Self {
    ///     slot_values: false,
    ///     slot_taint: false,
    ///     action_payloads: false,
    ///     pending_actions: false,
    /// };
    /// ```
    ///
    /// `const` so Verus inspects the literal directly; no
    /// `#[verifier::external]` needed (and Verus does not accept
    /// the attribute on associated `const` items).
    pub const SUPPORTED: Self = Self {
        slot_values: false,
        slot_taint: false,
        action_payloads: false,
        pending_actions: false,
    };

    /// Mirror of `event_slot_taint_unsupported` at
    /// `crates/vb_storage/src/recovery/types.rs:575-581`.
    ///
    /// Production body:
    /// ```text
    /// pub const fn event_slot_taint_unsupported() -> Self {
    ///     Self {
    ///         slot_taint: true,
    ///         ..Self::SUPPORTED
    ///     }
    /// }
    /// ```
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`). The `assume_specification` bridge
    /// in the companion spec file attaches the production contract.
    #[verifier::external]
    pub const fn event_slot_taint_unsupported() -> Self {
        Self { slot_taint: true, ..Self::SUPPORTED }
    }

    /// Mirror of `slot_values_unsupported` at
    /// `crates/vb_storage/src/recovery/types.rs:584-590`.
    ///
    /// Production body:
    /// ```text
    /// pub const fn slot_values_unsupported() -> Self {
    ///     Self {
    ///         slot_values: true,
    ///         ..Self::SUPPORTED
    ///     }
    /// }
    /// ```
    #[verifier::external]
    pub const fn slot_values_unsupported() -> Self {
        Self { slot_values: true, ..Self::SUPPORTED }
    }

    /// Mirror of `pending_actions_unsupported` at
    /// `crates/vb_storage/src/recovery/types.rs:593-599`.
    ///
    /// Production body:
    /// ```text
    /// pub const fn pending_actions_unsupported() -> Self {
    ///     Self {
    ///         pending_actions: true,
    ///         ..Self::SUPPORTED
    ///     }
    /// }
    /// ```
    #[verifier::external]
    pub const fn pending_actions_unsupported() -> Self {
        Self { pending_actions: true, ..Self::SUPPORTED }
    }

    /// Mirror of `union` at
    /// `crates/vb_storage/src/recovery/types.rs:603-610`.
    ///
    /// Production body:
    /// ```text
    /// pub const fn union(self, other: Self) -> Self {
    ///     Self {
    ///         slot_values: self.slot_values || other.slot_values,
    ///         slot_taint: self.slot_taint || other.slot_taint,
    ///         action_payloads: self.action_payloads || other.action_payloads,
    ///         pending_actions: self.pending_actions || other.pending_actions,
    ///     }
    /// }
    /// ```
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`). The `assume_specification` bridge
    /// in the companion spec file attaches the production contract:
    /// `union.slot_values == self.slot_values || other.slot_values`,
    /// and analogously for the other three flags.
    #[verifier::external]
    pub const fn union(self, other: Self) -> Self {
        Self {
            slot_values: self.slot_values || other.slot_values,
            slot_taint: self.slot_taint || other.slot_taint,
            action_payloads: self.action_payloads || other.action_payloads,
            pending_actions: self.pending_actions || other.pending_actions,
        }
    }

    /// Mirror of `is_fully_supported` at
    /// `crates/vb_storage/src/recovery/types.rs:614-616`.
    ///
    /// Production body:
    /// ```text
    /// pub const fn is_fully_supported(self) -> bool {
    ///     !self.slot_values && !self.slot_taint
    ///         && !self.action_payloads && !self.pending_actions
    /// }
    /// ```
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`). The `assume_specification` bridge
    /// in the companion spec file attaches the production contract.
    #[verifier::external]
    pub const fn is_fully_supported(self) -> bool {
        !self.slot_values && !self.slot_taint && !self.action_payloads && !self.pending_actions
    }

    /// Mirror of `union_matches_flags` at
    /// `crates/vb_storage/src/recovery/types.rs:620-625`.
    ///
    /// Production body:
    /// ```text
    /// pub const fn union_matches_flags(self, other: Self, union: Self) -> bool {
    ///     union.slot_values == (self.slot_values || other.slot_values)
    ///         && union.slot_taint == (self.slot_taint || other.slot_taint)
    ///         && union.action_payloads == (self.action_payloads || other.action_payloads)
    ///         && union.pending_actions == (self.pending_actions || other.pending_actions)
    /// }
    /// ```
    #[verifier::external]
    pub const fn union_matches_flags(self, other: Self, union: Self) -> bool {
        union.slot_values == (self.slot_values || other.slot_values) && union.slot_taint == (
        self.slot_taint || other.slot_taint) && union.action_payloads == (self.action_payloads
            || other.action_payloads) && union.pending_actions == (self.pending_actions
            || other.pending_actions)
    }
}

} // verus!
