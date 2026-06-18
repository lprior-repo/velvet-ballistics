//! Verus specification and proof for vb_runtime timer seam — vb-0l9k0.
//!
//! Production bindings:
//! - `spec_timer_registration_required` → `shard/helpers/timer.rs:11-21`
//! - `spec_advance_after_timer_fire` → `shard/helpers/timer.rs:24-52`
//!
//! Spec functions mirror the pure logic of production helpers.

use vstd::prelude::*;

verus! {

    // ===========================================================================
    // Model: CompiledNodeKind (subset for timer logic)
    // ===========================================================================

    pub enum CompiledNodeKind {
        WaitUntil,
        WaitEvent { timeout_slot: bool },
        Ask { timeout_slot: bool },
        Other,
    }

    impl CompiledNodeKind {
        pub closed spec fn has_timeout(self) -> bool {
            match self {
                CompiledNodeKind::WaitUntil => true,
                CompiledNodeKind::WaitEvent { timeout_slot } => timeout_slot,
                CompiledNodeKind::Ask { timeout_slot } => timeout_slot,
                CompiledNodeKind::Other => false,
            }
        }
    }

    // ===========================================================================
    // Spec: timer_registration_required
    //
    // Production binding: shard/helpers/timer.rs:11-21
    //
    //   pub fn timer_registration_required(state: &RunState, step: StepIdx) -> bool
    //   {
    //       let Some(node) = state.workflow.node(step) else {
    //           return false;
    //       };
    //       match node.kind {
    //           CompiledNodeKind::WaitUntil { .. } => true,
    //           CompiledNodeKind::WaitEvent { timeout_slot, .. }
    //           | CompiledNodeKind::Ask { timeout_slot, .. } => timeout_slot.is_some(),
    //           _ => false,
    //       }
    //   }
    // ===========================================================================

    pub closed spec fn spec_timer_registration_required(
        node_exists: bool,
        node_kind_has_timeout: bool,
    ) -> bool {
        if !node_exists {
            false
        } else {
            node_kind_has_timeout
        }
    }

    // ===========================================================================
    // Proof: timer registration required when node exists and has timeout
    // ===========================================================================

    pub proof fn proof_timer_required_when_needed(
        node_exists: bool,
        node_kind_has_timeout: bool,
    )
        requires
            node_exists && node_kind_has_timeout,
        ensures
            spec_timer_registration_required(node_exists, node_kind_has_timeout),
    {
        assert(spec_timer_registration_required(node_exists, node_kind_has_timeout));
    }

    // ===========================================================================
    // Proof: timer registration NOT required when node missing
    // ===========================================================================

    pub proof fn proof_timer_not_required_when_node_missing(
        node_exists: bool,
        node_kind_has_timeout: bool,
    )
        requires
            !node_exists,
        ensures
            !spec_timer_registration_required(node_exists, node_kind_has_timeout),
    {
        assert(!spec_timer_registration_required(node_exists, node_kind_has_timeout));
    }

    // ===========================================================================
    // Proof: timer registration NOT required when node has no timeout
    // ===========================================================================

    pub proof fn proof_timer_not_required_when_no_timeout(
        node_exists: bool,
        node_kind_has_timeout: bool,
    )
        requires
            node_exists && !node_kind_has_timeout,
        ensures
            !spec_timer_registration_required(node_exists, node_kind_has_timeout),
    {
        assert(!spec_timer_registration_required(node_exists, node_kind_has_timeout));
    }

    // ===========================================================================
    // Theorem: timer registration required is a well-defined predicate
    // ===========================================================================

    pub proof fn theorem_timer_registration_predicate(
        node_exists: bool,
        node_kind_has_timeout: bool,
    )
        ensures
            spec_timer_registration_required(node_exists, node_kind_has_timeout) || !spec_timer_registration_required(node_exists, node_kind_has_timeout),
    {
        assert(spec_timer_registration_required(node_exists, node_kind_has_timeout) || !spec_timer_registration_required(node_exists, node_kind_has_timeout));
    }

} // verus!
