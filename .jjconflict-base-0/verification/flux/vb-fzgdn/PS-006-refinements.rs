//! PS-006 Flux refinements: Slot validation for timer nodes (POB-vb-fzgdn-025)
//! Production binding: crates/vb_runtime/src/shard/helpers.rs timer_registration_required
//!
//! Refinement: timer_registration_required returns true iff the step's node kind
//! is WaitUntil or (WaitEvent|Ask with Some(timeout_slot)).

use vb_runtime::shard::RunState;
use vb_core::ids::StepIdx;

/// Refinement module: slot validation returns correct boolean.
mod slot_validation_refinements {
    /// Production code reference:
    ///   crates/vb_runtime/src/shard/helpers.rs:137-147
    ///   pub fn timer_registration_required(state: &RunState, step: StepIdx) -> bool {
    ///       match node.kind {
    ///           CompiledNodeKind::WaitUntil { .. } => true,
    ///           CompiledNodeKind::WaitEvent { timeout_slot, .. }
    ///           | CompiledNodeKind::Ask { timeout_slot, .. } => timeout_slot.is_some(),
    ///           _ => false,
    ///       }
    ///   }
    ///
    /// Refinement: function is pure (no side effects, no fallible operations).
    #[flux_rs::trusted]
    #[flux_rs::sig(fn(&RunState, StepIdx) -> bool)]
    pub fn timer_registration_required(state: &RunState, step: StepIdx) -> bool {
        vb_runtime::shard::helpers::timer_registration_required(state, step)
    }
}
