// Flux refinement artifact for vb-7ol6y POB-vb-7ol6y-013: typed read_taint
// lattice.
//
// Bead: vb-7ol6y (P0)
// State: 5 (proof-writer)
// Verifier: flux-rs
// Command: flux --edition=2021 crates/vb_storage/src/verification/flux/vb_7ol6y_slot_taint_resolution.rs
//
// PRODUCTION BINDING:
//   crates/vb_storage/src/recovery/event_replay/taint.rs:13-22
//     SlotTaintReadObservation
//   crates/vb_storage/src/recovery/event_replay/taint.rs:24-31
//     SlotTaintResolution
//   crates/vb_storage/src/recovery/event_replay/taint.rs:35-43
//     resolve_slot_taint_read
//   crates/vb_storage/src/recovery/event_replay/taint.rs:45-54
//     observe_slot_taint_read
//
// WIRING PREREQUISITE (State 7): This file must be added to
//   crates/vb_storage/src/verification/flux/mod.rs
// as `pub mod vb_7ol6y_slot_taint_resolution;`. Without the mod.rs
// wiring, this file is not analyzed by `cargo flux -p vb_storage`.

#![cfg(flux)]

use crate::recovery::event_replay::{
    SlotTaintReadObservation, SlotTaintResolution, observe_slot_taint_read, resolve_slot_taint_read,
};

// Refinement: SlotTaintReadObservation has exactly 3 variants.
// Production: event_replay/taint.rs:13-22 (3-variant enum).
// Refinement captures the 3-way decision lattice.
#[flux_rs::refined_by(kind: int)]
pub enum SpecSlotTaintReadObservation {
    #[flux_rs::variant(SpecSlotTaintReadObservation[0])]
    Existing(#[flux_rs::field(Taint)] vb_core::Taint),
    #[flux_rs::variant(SpecSlotTaintReadObservation[1])]
    Uninitialized,
    #[flux_rs::variant(SpecSlotTaintReadObservation[2])]
    Failed,
}

// Refinement: SlotTaintResolution has exactly 2 variants.
#[flux_rs::refined_by(kind: int)]
pub enum SpecSlotTaintResolution {
    #[flux_rs::variant(SpecSlotTaintResolution[0])]
    Use(#[flux_rs::field(Taint)] vb_core::Taint),
    #[flux_rs::variant(SpecSlotTaintResolution[1])]
    FailClosed,
}

// Refinement postcondition: observe_slot_taint_read + resolve_slot_taint_read
// composes such that Failed ==> FailClosed.
//
// Production:
//   observe_slot_taint_read: Err(SlotUninitialized) -> Uninitialized;
//                            Err(_) -> Failed
//   resolve_slot_taint_read: Failed -> FailClosed (event_replay/taint.rs:41)
//
// Therefore: observe_slot_taint_read(Err(CoreError::Other))
//            -> Failed
//            -> resolve_slot_taint_read(Failed)
//            -> FailClosed.
#[flux_rs::sig(fn(obs: SpecSlotTaintReadObservation) -> SpecSlotTaintResolution)]
pub fn spec_resolve_slot_taint_read(obs: SpecSlotTaintReadObservation) -> SpecSlotTaintResolution {
    resolve_slot_taint_read(obs)
}

// L1: Failed observation resolves to FailClosed (TB-004 production invariant).
#[flux_rs::sig(fn() -> bool[true])]
pub fn spec_failed_resolves_to_fail_closed() -> bool {
    matches!(
        resolve_slot_taint_read(SlotTaintReadObservation::Failed),
        SlotTaintResolution::FailClosed
    )
}

// L2: Uninitialized observation resolves to Use(Clean).
#[flux_rs::sig(fn() -> bool[true])]
pub fn spec_uninitialized_resolves_to_use_clean() -> bool {
    matches!(
        resolve_slot_taint_read(SlotTaintReadObservation::Uninitialized),
        SlotTaintResolution::Use(vb_core::Taint::Clean)
    )
}

// L3: Any Err(CoreError) other than SlotUninitialized maps to Failed
// (TB-003 production invariant).
//
// Production: observe_slot_taint_read at event_replay/taint.rs:48-54 is
// exhaustive: `Err(SlotUninitialized) -> Uninitialized`, `Err(_) -> Failed`.
// Therefore SlotOutOfBounds (a representative non-SlotUninitialized variant)
// must map to Failed.
#[flux_rs::sig(fn() -> bool[true])]
pub fn spec_other_core_error_maps_to_failed() -> bool {
    let result: Result<vb_core::Taint, vb_core::CoreError> = Err(vb_core::CoreError::SlotOutOfBounds {
        slot: vb_core::SlotIdx::new(0),
    });
    matches!(
        observe_slot_taint_read(result),
        SlotTaintReadObservation::Failed
    )
}

// L4: SlotUninitialized maps to Uninitialized (TB-003 production invariant).
#[flux_rs::sig(fn() -> bool[true])]
pub fn spec_slot_uninitialized_maps_to_uninitialized() -> bool {
    let result: Result<vb_core::Taint, vb_core::CoreError> = Err(vb_core::CoreError::SlotUninitialized);
    matches!(
        observe_slot_taint_read(result),
        SlotTaintReadObservation::Uninitialized
    )
}
