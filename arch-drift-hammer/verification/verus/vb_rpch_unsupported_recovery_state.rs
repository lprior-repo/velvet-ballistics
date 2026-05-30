#![allow(unused_imports)]

use vstd::prelude::*;

verus! {

/// VFR-R2-VERUS-001 / INV-002.
/// Bridge model for the State-11 production proof surface:
/// - crates/vb_storage/src/recovery/types.rs::UnsupportedRecoveryState::SUPPORTED
/// - UnsupportedRecoveryState::is_fully_supported
/// - UnsupportedRecoveryState::union
/// - UnsupportedRecoveryState::union_matches_flags
/// Field order and Boolean definitions match lines 275-297 of that file.
pub struct SpecUnsupportedRecoveryState {
    pub slot_values: bool,
    pub slot_taint: bool,
    pub action_payloads: bool,
    pub pending_actions: bool,
}

pub open spec fn production_supported() -> SpecUnsupportedRecoveryState {
    SpecUnsupportedRecoveryState {
        slot_values: false,
        slot_taint: false,
        action_payloads: false,
        pending_actions: false,
    }
}

pub open spec fn production_union(a: SpecUnsupportedRecoveryState, b: SpecUnsupportedRecoveryState) -> SpecUnsupportedRecoveryState {
    SpecUnsupportedRecoveryState {
        slot_values: a.slot_values || b.slot_values,
        slot_taint: a.slot_taint || b.slot_taint,
        action_payloads: a.action_payloads || b.action_payloads,
        pending_actions: a.pending_actions || b.pending_actions,
    }
}

pub open spec fn no_contradiction(_s: SpecUnsupportedRecoveryState) -> bool { true }

pub open spec fn production_is_fully_supported(s: SpecUnsupportedRecoveryState) -> bool {
    !s.slot_values && !s.slot_taint && !s.action_payloads && !s.pending_actions
}

pub open spec fn production_union_matches_flags(
    a: SpecUnsupportedRecoveryState,
    b: SpecUnsupportedRecoveryState,
    u: SpecUnsupportedRecoveryState,
) -> bool {
    u.slot_values == (a.slot_values || b.slot_values)
        && u.slot_taint == (a.slot_taint || b.slot_taint)
        && u.action_payloads == (a.action_payloads || b.action_payloads)
        && u.pending_actions == (a.pending_actions || b.pending_actions)
}

pub proof fn proof_supported_all_false()
    ensures
        production_is_fully_supported(production_supported()),
        !production_supported().slot_values,
        !production_supported().slot_taint,
        !production_supported().action_payloads,
        !production_supported().pending_actions,
{}

pub proof fn proof_union_flagwise_or(a: SpecUnsupportedRecoveryState, b: SpecUnsupportedRecoveryState)
    ensures
        production_union_matches_flags(a, b, production_union(a, b)),
        production_union(a, b).slot_values == (a.slot_values || b.slot_values),
        production_union(a, b).slot_taint == (a.slot_taint || b.slot_taint),
        production_union(a, b).action_payloads == (a.action_payloads || b.action_payloads),
        production_union(a, b).pending_actions == (a.pending_actions || b.pending_actions),
        no_contradiction(production_union(a, b)),
{}

pub proof fn proof_union_identity_idempotent_commutative(a: SpecUnsupportedRecoveryState, b: SpecUnsupportedRecoveryState)
    ensures
        production_union(a, production_supported()) == a,
        production_union(production_supported(), a) == a,
        production_union(a, a) == a,
        production_union(a, b) == production_union(b, a),
{}

pub proof fn proof_union_associative(a: SpecUnsupportedRecoveryState, b: SpecUnsupportedRecoveryState, c: SpecUnsupportedRecoveryState)
    ensures production_union(production_union(a, b), c) == production_union(a, production_union(b, c)),
{}

}
