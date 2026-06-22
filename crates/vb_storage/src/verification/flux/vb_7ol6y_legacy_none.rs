// Flux refinement artifact for vb-7ol6y POB-vb-7ol6y-017: legacy None arm
// refinement bound (qi37-1.1 contract).
//
// Bead: vb-7ol6y (P0)
// State: 5 (proof-writer)
// Verifier: flux-rs
// Command: bash scripts/flux-check-package.sh vb_storage
//
// PRODUCTION BINDING:
//   crates/vb_storage/src/recovery/replay/summary/slots/taint.rs:48
//     recovered_slot_taint None arm
//   crates/vb_storage/src/recovery/replay/summary/slots/taint.rs:97-102
//     legacy_recovered_slot_taint
//   crates/vb_storage/src/recovery/replay/summary/slots/taint.rs:112-126
//     legacy_slot_taint (classifies by SlotValue variant)
//
// WIRING (State 5 REDO): This file is wired into
//   crates/vb_storage/src/verification/flux/mod.rs
// as `pub mod vb_7ol6y_legacy_none;`.
//
// Non-vacuity: each spec fn body calls PRODUCTION recovered_slot_taint
// (the public-but-crate-internal helper) so the refinement is bound
// to actual implementation behavior. Postconditions assert SPECIFIC
// return shapes, not `true`.

#![forbid(unsafe_code)]

use crate::recovery::replay::summary::slots::taint::{RecoveredSlotTaint, recovered_slot_taint};
use vb_core::{SlotIdx, SlotValue, Taint};

// ============================================================================
// Production-bound spec fns.
// ============================================================================

/// Mirror of production recovered_slot_taint with extra == None.
/// Production: taint.rs:48 returns Ok(legacy_recovered_slot_taint(value))
/// which wraps legacy_slot_taint(value).
#[flux_rs::sig(
    fn(slot: SlotIdx, value: SlotValue) -> Result<RecoveredSlotTaint[?], _>
)]
pub fn spec_legacy_none(slot: SlotIdx, value: SlotValue) -> Result<RecoveredSlotTaint, crate::recovery::RecoveryError> {
    recovered_slot_taint(slot, value, None)
}

// ============================================================================
// Concrete (non-tautological) refinement postconditions.
//
// Each body invokes the production helper directly and asserts the
// SPECIFIC taint classification the qi37-1.1 contract requires.
// ============================================================================

/// L1: Bool(false) → Taint::Clean (production invariant at
/// taint.rs:114). Body calls production helper and matches.
#[flux_rs::sig(fn(slot: SlotIdx) -> bool[true])]
pub fn spec_bool_false_classifies_as_clean(slot: SlotIdx) -> bool {
    match spec_legacy_none(slot, SlotValue::Bool(false)) {
        Ok(r) => r.taint == Taint::Clean && r.unsupported == false,
        Err(_) => false,
    }
}

/// L2: Bool(true) → Taint::DerivedFromSecret (production invariant at
/// taint.rs:115). Body calls production helper and matches.
#[flux_rs::sig(fn(slot: SlotIdx) -> bool[true])]
pub fn spec_bool_true_classifies_as_derived_from_secret(slot: SlotIdx) -> bool {
    match spec_legacy_none(slot, SlotValue::Bool(true)) {
        Ok(r) => r.taint == Taint::DerivedFromSecret && r.unsupported == false,
        Err(_) => false,
    }
}

/// L3: Null → Taint::DerivedFromSecret (production invariant at
/// taint.rs:115). Body calls production helper and matches.
#[flux_rs::sig(fn(slot: SlotIdx) -> bool[true])]
pub fn spec_null_classifies_as_derived_from_secret(slot: SlotIdx) -> bool {
    match spec_legacy_none(slot, SlotValue::Null) {
        Ok(r) => r.taint == Taint::DerivedFromSecret && r.unsupported == false,
        Err(_) => false,
    }
}

/// L4: I64 → Taint::Secret (production invariant at taint.rs:116-117).
/// Body calls production helper and matches.
#[flux_rs::sig(fn(slot: SlotIdx) -> bool[true])]
pub fn spec_i64_classifies_as_secret(slot: SlotIdx) -> bool {
    match spec_legacy_none(slot, SlotValue::I64(42)) {
        Ok(r) => r.taint == Taint::Secret && r.unsupported == false,
        Err(_) => false,
    }
}
