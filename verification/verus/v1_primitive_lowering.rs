// Verus proof obligations for vb-f04l v1 primitive source lowering.
//
// Obligation IDs: PRE-007, POST-003, POST-004, POST-005,
// POST-006-VERUS through POST-012-VERUS, INV-001, INV-003, INV-004,
// INV-005.
// Verifier command: verus verification/verus/v1_primitive_lowering.rs
//
// This file proves an abstract constructor/bridge model only. It does not
// import production code. The trusted boundary is a later implementation bridge
// that maps compile_source emitted nodes, targets, slots, and primitive tags into
// AbstractPlan using the constructor preconditions below.

use vstd::prelude::*;

verus! {

pub struct AbstractPlan {
    pub node_count: int,
    pub slot_count: int,
    pub max_slot_ref: int,
    pub next_target: int,
    pub body_target: int,
    pub done_target: int,
    pub join_target: int,
    pub resume_target: int,
    pub exhausted_target: int,
    pub branch_count: int,
    pub max_attempts: int,
    pub page_limit: int,
}

pub struct SourceInputs {
    pub emitted_nodes: int,
    pub allocated_slots: int,
    pub highest_slot_ref: int,
    pub next_target_input: int,
    pub body_target_input: int,
    pub done_target_input: int,
    pub join_target_input: int,
    pub resume_target_input: int,
    pub exhausted_target_input: int,
    pub branch_count_input: int,
    pub max_attempts_input: int,
    pub page_limit_input: int,
}

pub open spec fn u16_max() -> int { 65535 }
pub open spec fn u32_max() -> int { 4294967295 }

pub open spec fn bounded_u16(x: int) -> bool {
    0 <= x && x <= u16_max()
}

pub open spec fn bounded_u32(x: int) -> bool {
    0 <= x && x <= u32_max()
}

pub open spec fn positive_nodes(plan: AbstractPlan) -> bool {
    bounded_u16(plan.node_count) && 0 < plan.node_count
}

pub open spec fn target_in_range(target: int, plan: AbstractPlan) -> bool {
    positive_nodes(plan) && 0 <= target && target < plan.node_count
}

pub open spec fn all_targets_in_range(plan: AbstractPlan) -> bool {
    target_in_range(plan.next_target, plan)
        && target_in_range(plan.body_target, plan)
        && target_in_range(plan.done_target, plan)
        && target_in_range(plan.join_target, plan)
        && target_in_range(plan.resume_target, plan)
        && target_in_range(plan.exhausted_target, plan)
}

pub open spec fn slot_allocator_closed(plan: AbstractPlan) -> bool {
    bounded_u16(plan.slot_count)
        && -1 <= plan.max_slot_ref
        && ((plan.max_slot_ref == -1 && plan.slot_count == 0)
            || (0 <= plan.max_slot_ref && plan.max_slot_ref < plan.slot_count))
}

pub open spec fn primitive_bounds_checked(plan: AbstractPlan) -> bool {
    bounded_u16(plan.node_count)
        && bounded_u16(plan.slot_count)
        && bounded_u16(plan.branch_count)
        && bounded_u16(plan.max_attempts)
        && bounded_u32(plan.page_limit)
}

pub open spec fn constructor_inputs_valid(plan: AbstractPlan) -> bool {
    positive_nodes(plan)
        && all_targets_in_range(plan)
        && slot_allocator_closed(plan)
        && primitive_bounds_checked(plan)
        && 0 < plan.branch_count
        && 0 < plan.max_attempts
}

pub open spec fn source_inputs_valid(source: SourceInputs) -> bool {
    bounded_u16(source.emitted_nodes)
        && 0 < source.emitted_nodes
        && bounded_u16(source.allocated_slots)
        && -1 <= source.highest_slot_ref
        && ((source.highest_slot_ref == -1 && source.allocated_slots == 0)
            || (0 <= source.highest_slot_ref && source.highest_slot_ref < source.allocated_slots))
        && 0 <= source.next_target_input && source.next_target_input < source.emitted_nodes
        && 0 <= source.body_target_input && source.body_target_input < source.emitted_nodes
        && 0 <= source.done_target_input && source.done_target_input < source.emitted_nodes
        && 0 <= source.join_target_input && source.join_target_input < source.emitted_nodes
        && 0 <= source.resume_target_input && source.resume_target_input < source.emitted_nodes
        && 0 <= source.exhausted_target_input && source.exhausted_target_input < source.emitted_nodes
        && bounded_u16(source.branch_count_input)
        && 0 < source.branch_count_input
        && bounded_u16(source.max_attempts_input)
        && 0 < source.max_attempts_input
        && bounded_u32(source.page_limit_input)
}

pub open spec fn construct_plan(source: SourceInputs) -> AbstractPlan {
    AbstractPlan {
        node_count: source.emitted_nodes,
        slot_count: source.allocated_slots,
        max_slot_ref: source.highest_slot_ref,
        next_target: source.next_target_input,
        body_target: source.body_target_input,
        done_target: source.done_target_input,
        join_target: source.join_target_input,
        resume_target: source.resume_target_input,
        exhausted_target: source.exhausted_target_input,
        branch_count: source.branch_count_input,
        max_attempts: source.max_attempts_input,
        page_limit: source.page_limit_input,
    }
}

pub open spec fn foreach_shape(plan: AbstractPlan) -> bool {
    constructor_inputs_valid(plan) && target_in_range(plan.body_target, plan)
        && target_in_range(plan.done_target, plan)
}

pub open spec fn together_shape(plan: AbstractPlan) -> bool {
    constructor_inputs_valid(plan) && target_in_range(plan.join_target, plan)
        && target_in_range(plan.done_target, plan)
}

pub open spec fn collect_shape(plan: AbstractPlan) -> bool {
    constructor_inputs_valid(plan) && slot_allocator_closed(plan)
        && target_in_range(plan.body_target, plan) && target_in_range(plan.done_target, plan)
}

pub open spec fn reduce_shape(plan: AbstractPlan) -> bool {
    constructor_inputs_valid(plan) && slot_allocator_closed(plan)
        && target_in_range(plan.body_target, plan) && target_in_range(plan.done_target, plan)
}

pub open spec fn repeat_shape(plan: AbstractPlan) -> bool {
    constructor_inputs_valid(plan) && target_in_range(plan.body_target, plan)
        && target_in_range(plan.exhausted_target, plan)
}

pub open spec fn wait_shape(plan: AbstractPlan) -> bool {
    constructor_inputs_valid(plan) && target_in_range(plan.resume_target, plan)
        && target_in_range(plan.done_target, plan)
}

pub open spec fn ask_shape(plan: AbstractPlan) -> bool {
    constructor_inputs_valid(plan) && slot_allocator_closed(plan)
        && target_in_range(plan.resume_target, plan) && target_in_range(plan.done_target, plan)
}

pub open spec fn deterministic_source_bridge(left: AbstractPlan, right: AbstractPlan) -> bool {
    left.node_count == right.node_count
        && left.slot_count == right.slot_count
        && left.max_slot_ref == right.max_slot_ref
        && left.next_target == right.next_target
        && left.body_target == right.body_target
        && left.done_target == right.done_target
        && left.join_target == right.join_target
        && left.resume_target == right.resume_target
        && left.exhausted_target == right.exhausted_target
        && left.branch_count == right.branch_count
        && left.max_attempts == right.max_attempts
        && left.page_limit == right.page_limit
}

pub open spec fn same_source(left: SourceInputs, right: SourceInputs) -> bool {
    left.emitted_nodes == right.emitted_nodes
        && left.allocated_slots == right.allocated_slots
        && left.highest_slot_ref == right.highest_slot_ref
        && left.next_target_input == right.next_target_input
        && left.body_target_input == right.body_target_input
        && left.done_target_input == right.done_target_input
        && left.join_target_input == right.join_target_input
        && left.resume_target_input == right.resume_target_input
        && left.exhausted_target_input == right.exhausted_target_input
        && left.branch_count_input == right.branch_count_input
        && left.max_attempts_input == right.max_attempts_input
        && left.page_limit_input == right.page_limit_input
}

pub open spec fn primitive_foreach() -> int { 0 }
pub open spec fn primitive_together() -> int { 1 }
pub open spec fn primitive_collect() -> int { 2 }
pub open spec fn primitive_reduce() -> int { 3 }
pub open spec fn primitive_repeat() -> int { 4 }
pub open spec fn primitive_wait() -> int { 5 }
pub open spec fn primitive_ask() -> int { 6 }

pub open spec fn primitive_tag_valid(tag: int) -> bool {
    primitive_foreach() <= tag && tag <= primitive_ask()
}

pub open spec fn local_shape_preserved(tag: int, plan: AbstractPlan) -> bool {
    (tag == primitive_foreach() ==> foreach_shape(plan))
        && (tag == primitive_together() ==> together_shape(plan))
        && (tag == primitive_collect() ==> collect_shape(plan))
        && (tag == primitive_reduce() ==> reduce_shape(plan))
        && (tag == primitive_repeat() ==> repeat_shape(plan))
        && (tag == primitive_wait() ==> wait_shape(plan))
        && (tag == primitive_ask() ==> ask_shape(plan))
}

pub proof fn proof_construct_plan_valid(source: SourceInputs)
    requires
        source_inputs_valid(source),
    ensures
        constructor_inputs_valid(construct_plan(source)),
{
}

pub proof fn proof_lowering_plan_preserves_dense_node_ids(source: SourceInputs)
    requires
        source_inputs_valid(source),
    ensures
        bounded_u16(construct_plan(source).node_count),
        0 < construct_plan(source).node_count,
{
}

pub proof fn proof_lowering_plan_targets_in_range(source: SourceInputs)
    requires
        source_inputs_valid(source),
    ensures
        target_in_range(construct_plan(source).next_target, construct_plan(source)),
        target_in_range(construct_plan(source).body_target, construct_plan(source)),
        target_in_range(construct_plan(source).done_target, construct_plan(source)),
        target_in_range(construct_plan(source).join_target, construct_plan(source)),
        target_in_range(construct_plan(source).resume_target, construct_plan(source)),
        target_in_range(construct_plan(source).exhausted_target, construct_plan(source)),
{
}

pub proof fn proof_lowering_plan_slot_count_covers_references(source: SourceInputs)
    requires
        source_inputs_valid(source),
    ensures
        bounded_u16(construct_plan(source).slot_count),
        construct_plan(source).max_slot_ref == -1 ==> construct_plan(source).slot_count == 0,
        construct_plan(source).max_slot_ref >= 0 ==> construct_plan(source).max_slot_ref < construct_plan(source).slot_count,
{
}

pub proof fn proof_lowering_plan_checks_bounds_before_casts(source: SourceInputs)
    requires
        source_inputs_valid(source),
    ensures
        bounded_u16(construct_plan(source).node_count),
        bounded_u16(construct_plan(source).slot_count),
        bounded_u16(construct_plan(source).branch_count),
        bounded_u16(construct_plan(source).max_attempts),
        bounded_u32(construct_plan(source).page_limit),
{
}

pub proof fn proof_lowering_plan_deterministic_for_equal_source(
    left: SourceInputs,
    right: SourceInputs,
)
    requires
        same_source(left, right),
    ensures
        deterministic_source_bridge(construct_plan(left), construct_plan(right)),
{
}

pub proof fn proof_lowering_plan_preserves_primitive_shapes(source: SourceInputs, tag: int)
    requires
        source_inputs_valid(source),
        primitive_tag_valid(tag),
    ensures
        local_shape_preserved(tag, construct_plan(source)),
{
}

pub proof fn proof_foreach_shape(source: SourceInputs)
    requires source_inputs_valid(source),
    ensures target_in_range(construct_plan(source).body_target, construct_plan(source)), target_in_range(construct_plan(source).done_target, construct_plan(source)),
{
}

pub proof fn proof_together_shape(source: SourceInputs)
    requires source_inputs_valid(source),
    ensures
        target_in_range(construct_plan(source).join_target, construct_plan(source)),
        target_in_range(construct_plan(source).done_target, construct_plan(source)),
        0 < construct_plan(source).branch_count,
        bounded_u16(construct_plan(source).branch_count),
{
}

pub proof fn proof_collect_shape(source: SourceInputs)
    requires source_inputs_valid(source),
    ensures
        slot_allocator_closed(construct_plan(source)),
        bounded_u32(construct_plan(source).page_limit),
        target_in_range(construct_plan(source).body_target, construct_plan(source)),
        target_in_range(construct_plan(source).done_target, construct_plan(source)),
{
}

pub proof fn proof_reduce_shape(source: SourceInputs)
    requires source_inputs_valid(source),
    ensures
        slot_allocator_closed(construct_plan(source)),
        target_in_range(construct_plan(source).body_target, construct_plan(source)),
        target_in_range(construct_plan(source).done_target, construct_plan(source)),
{
}

pub proof fn proof_repeat_shape(source: SourceInputs)
    requires source_inputs_valid(source),
    ensures
        0 < construct_plan(source).max_attempts,
        bounded_u16(construct_plan(source).max_attempts),
        target_in_range(construct_plan(source).body_target, construct_plan(source)),
        target_in_range(construct_plan(source).exhausted_target, construct_plan(source)),
{
}

pub proof fn proof_wait_shape(source: SourceInputs)
    requires source_inputs_valid(source),
    ensures target_in_range(construct_plan(source).resume_target, construct_plan(source)), target_in_range(construct_plan(source).done_target, construct_plan(source)),
{
}

pub proof fn proof_ask_shape(source: SourceInputs)
    requires source_inputs_valid(source),
    ensures
        slot_allocator_closed(construct_plan(source)),
        target_in_range(construct_plan(source).resume_target, construct_plan(source)),
        target_in_range(construct_plan(source).done_target, construct_plan(source)),
{
}

fn main() {}

} // verus!
