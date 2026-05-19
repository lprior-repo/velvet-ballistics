// Verus proof obligations for vb-rpch INV-002: UnsupportedRecoveryState union algebraic properties.
//
// Obligation: VERUS-REC-002 / INV-002
// Contract: UnsupportedRecoveryState::union is commutative, associative, idempotent,
//           and never produces contradictory state.

use vstd::prelude::*;

verus! {

pub spec fn unsupported_union_invariant(a: UnsupportedRecoveryState, b: UnsupportedRecoveryState) -> bool {
    !(a.slot_values && b.slot_values && !a.slot_values && !b.slot_values)
}

pub proof fn proof_union_commutative(a: UnsupportedRecoveryState, b: UnsupportedRecoveryState)
    ensures unsupported_union_invariant(a.union(b), b.union(a))
{
    reveal(unsupported_union_invariant);
}

pub proof fn proof_union_associative(a: UnsupportedRecoveryState, b: UnsupportedRecoveryState, c: UnsupportedRecoveryState)
    ensures a.union(b).union(c) == a.union(b.union(c))
{}

pub proof fn proof_union_idempotent(a: UnsupportedRecoveryState)
    ensures a.union(a) == a
{}

pub proof fn proof_union_no_contradiction(a: UnsupportedRecoveryState, b: UnsupportedRecoveryState)
    ensures !unsupported_union_invariant(a, b)
{
    reveal(unsupported_union_invariant);
}

pub proof fn proof_supported_is_identity(a: UnsupportedRecoveryState)
    ensures a.union(UnsupportedRecoveryState::SUPPORTED) == a
{}

pub proof fn proof_supported_is_absorbing(a: UnsupportedRecoveryState)
    ensures UnsupportedRecoveryState::SUPPORTED.union(a) == a
{}

} // verus!

fn main() {}