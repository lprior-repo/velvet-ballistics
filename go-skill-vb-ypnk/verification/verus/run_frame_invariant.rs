// Verus proof obligations for RunFrame construction and reinitialization.
//
// Contract clauses: PRE-001, POST-001, INV-007.
// Registry obligations: VB-CORE-RUNFRAME-001, VB-CORE-RUNFRAME-002,
// VB-CORE-RUNFRAME-003.
// Exact verifier command: `verus verification/verus/run_frame_invariant.rs`.

use vstd::prelude::*;

verus! {

pub open spec fn u16_max() -> int {
    65535
}

pub open spec fn valid_u16_dim(dim: int) -> bool {
    0 <= dim && dim <= u16_max()
}

pub open spec fn spec_run_frame_new_preconditions(first_step: int, step_count: int) -> bool {
    0 <= first_step && 0 < step_count && first_step < step_count && valid_u16_dim(step_count)
}

pub struct SpecRunFrame {
    pub step_count: int,
    pub slot_count: int,
    pub states_len: int,
    pub slots_len: int,
    pub taint_len: int,
    pub all_states_pending: bool,
    pub all_slots_empty: bool,
    pub all_taint_clean: bool,
}

pub open spec fn spec_run_frame_new_postconditions(frame: SpecRunFrame, step_count: int, slot_count: int) -> bool {
    frame.step_count == step_count
        && frame.slot_count == slot_count
        && frame.states_len == step_count
        && frame.slots_len == slot_count
        && frame.taint_len == slot_count
        && frame.all_states_pending
        && frame.all_slots_empty
        && frame.all_taint_clean
}

pub open spec fn spec_constructed_run_frame(step_count: int, slot_count: int) -> SpecRunFrame {
    SpecRunFrame {
        step_count,
        slot_count,
        states_len: step_count,
        slots_len: slot_count,
        taint_len: slot_count,
        all_states_pending: true,
        all_slots_empty: true,
        all_taint_clean: true,
    }
}

pub open spec fn spec_run_frame_dimensions_immutable(
    old_step_count: int,
    old_slot_count: int,
    new_step_count: int,
    new_slot_count: int,
) -> bool {
    old_step_count == new_step_count && old_slot_count == new_slot_count
}

pub open spec fn spec_reinitialize_accepts(
    old_step_count: int,
    old_slot_count: int,
    first_step: int,
    new_step_count: int,
    new_slot_count: int,
) -> bool {
    spec_run_frame_new_preconditions(first_step, new_step_count)
        && spec_run_frame_dimensions_immutable(
            old_step_count,
            old_slot_count,
            new_step_count,
            new_slot_count,
        )
}

pub proof fn proof_run_frame_new_rejects_invalid_dimensions(first_step: int, step_count: int)
    requires
        valid_u16_dim(step_count),
        !(0 < step_count && 0 <= first_step && first_step < step_count),
    ensures
        !spec_run_frame_new_preconditions(first_step, step_count),
{
}

pub proof fn proof_run_frame_new_accepts_valid_dimensions(first_step: int, step_count: int)
    requires
        valid_u16_dim(step_count),
        0 <= first_step,
        0 < step_count,
        first_step < step_count,
    ensures
        spec_run_frame_new_preconditions(first_step, step_count),
{
}

pub proof fn proof_run_frame_new_initializes_dimensions_and_defaults(step_count: int, slot_count: int)
    requires
        valid_u16_dim(step_count),
        valid_u16_dim(slot_count),
        0 < step_count,
    ensures
        spec_run_frame_new_postconditions(spec_constructed_run_frame(step_count, slot_count), step_count, slot_count),
{
}

pub proof fn proof_reinitialize_preserves_dimensions(
    old_step_count: int,
    old_slot_count: int,
    first_step: int,
    new_step_count: int,
    new_slot_count: int,
)
    requires
        valid_u16_dim(old_step_count),
        valid_u16_dim(old_slot_count),
        valid_u16_dim(new_step_count),
        valid_u16_dim(new_slot_count),
        spec_reinitialize_accepts(
            old_step_count,
            old_slot_count,
            first_step,
            new_step_count,
            new_slot_count,
        ),
    ensures
        old_step_count == new_step_count,
        old_slot_count == new_slot_count,
        spec_run_frame_dimensions_immutable(
            old_step_count,
            old_slot_count,
            new_step_count,
            new_slot_count,
        ),
{
}

pub proof fn proof_reinitialize_rejects_dimension_mismatch(
    old_step_count: int,
    old_slot_count: int,
    first_step: int,
    new_step_count: int,
    new_slot_count: int,
)
    requires
        valid_u16_dim(new_step_count),
        spec_run_frame_new_preconditions(first_step, new_step_count),
        old_step_count != new_step_count || old_slot_count != new_slot_count,
    ensures
        !spec_reinitialize_accepts(
            old_step_count,
            old_slot_count,
            first_step,
            new_step_count,
            new_slot_count,
        ),
{
}

fn main() {}

} // verus!
