//! PS-006 Verus proof: Slot value validation before timer registration (POB-vb-fzgdn-023)
//! Production binding: crates/vb_runtime/src/shard/helpers.rs timer_registration_required
//!
//! Models: timer_registration_required checks node kind and timeout slot presence
//! before returning true. WaitUntil always requires timer; Do never does.

use vstd::prelude::*;

verus! {

/// Model of compiled node kinds relevant to timer registration.
#[derive(PartialEq, Eq)]
pub enum NodeKindModel {
    WaitUntil,
    WaitEvent { has_timeout: bool },
    Ask { has_timeout: bool },
    Do,
}

/// Spec mirror of timer_registration_required.
pub closed spec fn timer_required_spec(kind: NodeKindModel) -> bool {
    match kind {
        NodeKindModel::WaitUntil => true,
        NodeKindModel::WaitEvent { has_timeout } => has_timeout,
        NodeKindModel::Ask { has_timeout } => has_timeout,
        NodeKindModel::Do => false,
    }
}

/// Theorem: WaitUntil always requires timer registration.
proof fn test_wait_until_always_requires_timer()
    ensures timer_required_spec(NodeKindModel::WaitUntil),
{
    assert(timer_required_spec(NodeKindModel::WaitUntil)) by (compute);
}

/// Theorem: Do never requires timer registration.
proof fn test_do_never_requires_timer()
    ensures !timer_required_spec(NodeKindModel::Do),
{
    assert(!timer_required_spec(NodeKindModel::Do)) by (compute);
}

/// Theorem: WaitEvent requires timer iff has_timeout is true.
proof fn test_wait_event_conditional()
    ensures
        timer_required_spec(NodeKindModel::WaitEvent { has_timeout: true }),
        !timer_required_spec(NodeKindModel::WaitEvent { has_timeout: false }),
{
    assert(timer_required_spec(NodeKindModel::WaitEvent { has_timeout: true })) by (compute);
    assert(!timer_required_spec(NodeKindModel::WaitEvent { has_timeout: false })) by (compute);
}

/// Theorem: Ask requires timer iff has_timeout is true.
proof fn test_ask_conditional()
    ensures
        timer_required_spec(NodeKindModel::Ask { has_timeout: true }),
        !timer_required_spec(NodeKindModel::Ask { has_timeout: false }),
{
    assert(timer_required_spec(NodeKindModel::Ask { has_timeout: true })) by (compute);
    assert(!timer_required_spec(NodeKindModel::Ask { has_timeout: false })) by (compute);
}

} // verus!
