//! Direct verification of the actual recovery refusal flag calculations.
//! The live replay summary folds this exact union before refusal classification.
use vstd::prelude::*;

#[path = "../../crates/vb_storage/src/recovery/types/unsupported.rs"]
mod production;

verus! {

pub fn union_supports_exactly_when_both_inputs_do(
    left: production::UnsupportedRecoveryState,
    right: production::UnsupportedRecoveryState,
) -> (out: bool)
    ensures
        out == (!left.slot_values && !left.slot_taint && !left.action_payloads
            && !left.pending_actions && !right.slot_values && !right.slot_taint
            && !right.action_payloads && !right.pending_actions),
{
    left.union(right).is_fully_supported()
}

pub fn union_never_clears_either_input(
    left: production::UnsupportedRecoveryState,
    right: production::UnsupportedRecoveryState,
) -> (out: production::UnsupportedRecoveryState)
    ensures
        (left.slot_values || right.slot_values) ==> out.slot_values,
        (left.slot_taint || right.slot_taint) ==> out.slot_taint,
        (left.action_payloads || right.action_payloads) ==> out.action_payloads,
        (left.pending_actions || right.pending_actions) ==> out.pending_actions,
{
    left.union(right)
}

} // verus!
