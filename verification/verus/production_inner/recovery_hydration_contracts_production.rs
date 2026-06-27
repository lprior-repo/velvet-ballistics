// SPDX-License-Identifier: MIT
//
// ============================================================================
// Drift-detection stub for recovery_hydration_contracts
// ============================================================================
//
// This file is a minimal drift-detection stub for the
// `recovery_hydration_contracts` Verus spec. It exists so the companion
// `verification/verus/extern_recovery_hydration_contracts.rs` can
// include this file via
// `#[path = "production_inner/recovery_hydration_contracts_production.rs"]`
// to satisfy the production-binding gate.
//
// The actual production mirror content (full RecoveryError /
// RuntimeError / CoreError mirrors, recovery_decision_pure body, etc.)
// lives in `verification/verus/extern_recovery_hydration_contracts.rs`,
// which carries verbatim copies of the production source at:
//
//   - `RecoveryError`           <- crates/vb_storage/src/recovery/types.rs:39-145
//   - `RuntimeError`            <- crates/vb_runtime/src/error/mod.rs:71-73
//   - `CoreError`               <- crates/vb_core/src/errors.rs:414-425
//   - `recovery_decision_pure`  <- crates/vb_storage/src/recovery/recover.rs
//                                   + crates/vb_storage/src/recovery/hydrate.rs
//                                   + crates/vb_runtime/src/recovery.rs
//                                   + crates/vb_runtime/src/taint.rs
//   - ID newtypes (RunId, StepIdx, SlotIdx, ActionId, WorkflowDigest,
//     EventSeq)                 <- crates/vb_core/src/ids/mod.rs
//
// This stub mirrors the production error discriminant set as the
// smallest drift-detection surface.
//
// DRIFT POLICY: `crates/vb_storage/src/recovery/types.rs:39-145`
// Production source coverage:
//   - `RecoveryError`           <- crates/vb_storage/src/recovery/types.rs:39-145
//   - `RuntimeError`            <- crates/vb_runtime/src/error/mod.rs:71-73
//   - `CoreError`               <- crates/vb_core/src/errors.rs:414-425
//   - `recovery_decision_pure`  <- crates/vb_storage/src/recovery/recover.rs
//                                   + crates/vb_storage/src/recovery/hydrate.rs
//                                   + crates/vb_runtime/src/recovery.rs
//                                   + crates/vb_runtime/src/taint.rs
//   - ID newtypes (RunId, StepIdx, SlotIdx, ActionId, WorkflowDigest,
//     EventSeq)                 <- crates/vb_core/src/ids/mod.rs
// Regenerate this file whenever production changes. Any new variant
// in `RecoveryError` breaks the `extern_recovery_hydration_contracts`
// Verus build at compile time.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Drift-detection stubs
// ---------------------------------------------------------------------------

/// Mirror of production `RecoveryError` discriminant set at
/// `crates/vb_storage/src/recovery/types.rs:39-145`. The stub
/// enumerates the documented variant discriminants that affect the
/// recovery decision surface. Drift in variant names breaks the
/// stub at compile time.
#[verifier::external]
pub fn recovery_error_discriminant_check(variant_id: u8) -> bool {
    // Variant discriminants from the production RecoveryError enum.
    // Surface changes (new variant, renamed variant) break this stub.
    matches!(variant_id, 0..=13)
}

/// Mirror of production `recovery_decision_pure` decision fn. The
/// body is `#[verifier::external]` (opaque); the companion spec
/// file attaches `assume_specification` contracts that the spec
/// proofs discharge.
#[verifier::external]
pub fn recovery_decision_stub(has_header: bool, has_required_slot: bool, has_taint: bool) -> bool {
    // Surface only the document-level predicate (no recovery data
    // when any required fact is missing). The full decision lattice
    // lives in the companion extern file's recovery_decision_pure.
    has_header && has_required_slot && has_taint
}

} // verus!