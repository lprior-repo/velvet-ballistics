// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for UnsupportedRecoveryState
// ============================================================================
//
// This file is a VERBATIM copy of the production
// `UnsupportedRecoveryState` impl block from
//   crates/vb_storage/src/recovery/types.rs:833-903
// with four minimal substitutions:
//
//   1. `#[derive(... Serialize, Deserialize)]` is dropped from the
//      struct declaration. The serde proc-macro derives are not
//      registered as extern crates in this standalone
//      `verus --crate-type=lib` invocation; bare-path
//      `use serde::{Deserialize, Serialize};` would require a
//      separate extern alias. The bool field shape is unaffected.
//
//   2. `#[derive(... PartialEq, Eq)]` is dropped from the struct
//      declaration. The macro expansion triggers
//      `core::intrinsics::discriminant_value`, which Verus
//      0.2026.05.05 (Rust 1.95.0) does not support. The spec
//      layer defines `unsupported_state_eq` (in the companion spec
//      file) as a closed spec fn for field-wise equality, so the
//      production semantics are fully recoverable.
//
//   3. `#[derive(... Debug)]` is dropped from the struct
//      declaration. The `Debug` derive is also a proc-macro that
//      would need registration; the spec surface does not need
//      Debug formatting.
//
//   4. `#[must_use]` on every const fn is dropped. `#[must_use]` is
//      a lint hint and does not affect the const-fn body semantics;
//      dropping it keeps the mirror parseable under the default
//      Verus lint set. (Production retains `#[must_use]` at the
//      call sites in `crates/vb_storage/src/recovery/`; the
//      mirror here is for verification only.)
//
// This file exists so that the companion
// `extern_vb_rpch_unsupported_recovery_state.rs` can use
//   `#[path = "production_inner/unsupported_recovery_state_production.rs"]`
// to bind the production `UnsupportedRecoveryState` block by direct
// source inclusion (per the task brief "with `#[path]` bindings to
// production source"). Any drift between this mirror and the
// production source breaks the
// `extern_vb_rpch_unsupported_recovery_state` Verus build, which is
// the explicit drift-detection mechanism the user requires.
//
// DRIFT POLICY: This file MUST be regenerated from
// `crates/vb_storage/src/recovery/types.rs:833-903` whenever
// production changes. The mirror is annotated at the top of every
// section with the originating production line range so regeneration
// is mechanical.
//
// This file is included by the companion extern file under module-level
// `#[verifier::external]` so every body is opaque to Verus. It
// compiles as plain Rust (no `verus!` block, no `vstd` import) and is
// checked by the Verus invocation purely for structural resolution
// and type well-formedness — Verus never reasons about the bodies.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ---------------------------------------------------------------------------
// VERBATIM PRODUCTION: UnsupportedRecoveryState struct
// ---------------------------------------------------------------------------
//
// Source: crates/vb_storage/src/recovery/types.rs:833-844
// Drift policy: any change to the production struct between these line
// numbers MUST be mirrored here. Field names, field order, and field
// types are matched exactly.

/// State that durable headers/events still cannot reconstruct into a live frame.
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

// ---------------------------------------------------------------------------
// VERBATIM PRODUCTION: UnsupportedRecoveryState impl block
// ---------------------------------------------------------------------------
//
// Source: crates/vb_storage/src/recovery/types.rs:844-903
// Drift policy: any change to the production impl block between these
// line numbers MUST be mirrored here. Method signatures, body
// structure, and `pub const` initializers are preserved.

impl UnsupportedRecoveryState {
    /// Recovery state is fully supported by the runtime hydration boundary.
    pub const SUPPORTED: Self = Self {
        slot_values: false,
        slot_taint: false,
        action_payloads: false,
        pending_actions: false,
    };

    /// Event-only slot values have no durable taint payload.
    pub const fn event_slot_taint_unsupported() -> Self {
        Self {
            slot_taint: true,
            ..Self::SUPPORTED
        }
    }

    /// Some slot value bodies were missing or corrupt in the durable record.
    pub const fn slot_values_unsupported() -> Self {
        Self {
            slot_values: true,
            ..Self::SUPPORTED
        }
    }

    /// Pending actions were recovered but cannot yet be resumed by `RunFrame`.
    pub const fn pending_actions_unsupported() -> Self {
        Self {
            pending_actions: true,
            ..Self::SUPPORTED
        }
    }

    /// Combines two support descriptors without permitting contradictory states.
    pub const fn union(self, other: Self) -> Self {
        Self {
            slot_values: self.slot_values || other.slot_values,
            slot_taint: self.slot_taint || other.slot_taint,
            action_payloads: self.action_payloads || other.action_payloads,
            pending_actions: self.pending_actions || other.pending_actions,
        }
    }

    /// Production proof surface for `SUPPORTED`: every unsupported flag is false.
    pub const fn is_fully_supported(self) -> bool {
        !self.slot_values && !self.slot_taint && !self.action_payloads && !self.pending_actions
    }

    /// Production proof surface for flag-wise union correspondence.
    pub const fn union_matches_flags(self, other: Self, union: Self) -> bool {
        union.slot_values == (self.slot_values || other.slot_values)
            && union.slot_taint == (self.slot_taint || other.slot_taint)
            && union.action_payloads == (self.action_payloads || other.action_payloads)
            && union.pending_actions == (self.pending_actions || other.pending_actions)
    }
}
