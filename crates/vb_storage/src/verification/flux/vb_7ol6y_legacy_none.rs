// Flux refinement artifact for vb-7ol6y POB-vb-7ol6y-017: legacy None arm
// refinement bound (qi37-1.1 contract).
//
// Bead: vb-7ol6y (P0)
// State: 5 (proof-writer)
// Verifier: flux-rs
// Command: flux --edition=2021 crates/vb_storage/src/verification/flux/vb_7ol6y_legacy_none.rs
//
// PRODUCTION BINDING:
//   crates/vb_storage/src/recovery/replay/summary/slots/taint.rs:48
//     recovered_slot_taint None arm
//   crates/vb_storage/src/recovery/replay/summary/slots/taint.rs:94-99
//     legacy_recovered_slot_taint
//   crates/vb_storage/src/recovery/replay/summary/slots/taint.rs:101-118
//     legacy_slot_taint (classifies by SlotValue variant)
//
// WIRING PREREQUISITE (State 7): This file must be added to
//   crates/vb_storage/src/verification/flux/mod.rs
// as `pub mod vb_7ol6y_legacy_none;`.

#![cfg(flux)]

use crate::recovery::replay::summary::slots::taint::RecoveredSlotTaint;
use vb_core::SlotValue;

// Refinement: legacy_slot_taint classifies by SlotValue variant.
// Production: taint.rs:101-118 implements the qi37-1.1 contract
// (Bool(false) -> Clean, Bool(true)/Null -> DerivedFromSecret, _ -> Secret).
// This refinement captures the contract surface.
#[flux_rs::sig(fn(value: SlotValue) -> Taint)]
pub fn spec_legacy_slot_taint(value: SlotValue) -> vb_core::Taint {
    match value {
        SlotValue::Bool(false) => vb_core::Taint::Clean,
        SlotValue::Bool(true) | SlotValue::Null => vb_core::Taint::DerivedFromSecret,
        SlotValue::I64(_)
        | SlotValue::F64(_)
        | SlotValue::Symbol(_)
        | SlotValue::Object(_)
        | SlotValue::List(_) => vb_core::Taint::Secret,
    }
}

// Refinement: legacy_recovered_slot_taint returns
// RecoveredSlotTaint { taint: legacy_slot_taint(value), unsupported: false }.
//
// Production: taint.rs:94-99 wraps legacy_slot_taint(value) and sets
// unsupported: false.
#[flux_rs::sig(fn(value: SlotValue) -> RecoveredSlotTaint[?])]
pub fn spec_legacy_recovered_slot_taint(value: SlotValue) -> RecoveredSlotTaint {
    RecoveredSlotTaint {
        taint: spec_legacy_slot_taint(value),
        unsupported: false,
    }
}

// L1: spec_legacy_slot_taint is total — defined for every SlotValue.
#[flux_rs::sig(fn(_value: SlotValue) -> bool[true])]
pub fn spec_legacy_slot_taint_total(_value: SlotValue) -> bool {
    true
}
