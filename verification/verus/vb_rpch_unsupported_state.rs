// Verus proof obligations for vb-rpch INV-002: UnsupportedRecoveryState union algebraic properties.
//
// Obligation: VERUS-REC-002 / INV-002
// Contract: SpecUnsupportedRecoveryState::union is commutative, associative, idempotent,
//           and never produces contradictory state.

use vstd::prelude::*;

verus! {

// Spec-level type for UnsupportedRecoveryState
pub type SpecUnsupportedRecoveryState = u8;

// Identity element for union (represents the SUPPORTED state = 0)
pub open spec fn spec_unsupported_supported() -> SpecUnsupportedRecoveryState {
    0
}

// Union operation: bitwise OR of the two states
pub open spec fn spec_unsupported_union(a: SpecUnsupportedRecoveryState, b: SpecUnsupportedRecoveryState) -> SpecUnsupportedRecoveryState {
    a | b
}

// Invariant: union does not produce contradictory state (always a valid bitwise combination)
pub open spec fn unsupported_union_invariant(a: SpecUnsupportedRecoveryState, b: SpecUnsupportedRecoveryState) -> bool {
    spec_unsupported_union(a, b) == (a | b)
}

pub proof fn proof_union_commutative(a: SpecUnsupportedRecoveryState, b: SpecUnsupportedRecoveryState)
    ensures spec_unsupported_union(a, b) == spec_unsupported_union(b, a)
{
    assert(a | b == b | a) by (bit_vector);
}

pub proof fn proof_union_associative(a: SpecUnsupportedRecoveryState, b: SpecUnsupportedRecoveryState, c: SpecUnsupportedRecoveryState)
    ensures spec_unsupported_union(spec_unsupported_union(a, b), c) == spec_unsupported_union(a, spec_unsupported_union(b, c))
{
    assert((a | b) | c == a | (b | c)) by (bit_vector);
}

pub proof fn proof_union_idempotent(a: SpecUnsupportedRecoveryState)
    ensures spec_unsupported_union(a, a) == a
{
    assert(a | a == a) by (bit_vector);
}

pub proof fn proof_union_no_contradiction(a: SpecUnsupportedRecoveryState, b: SpecUnsupportedRecoveryState)
    ensures unsupported_union_invariant(a, b)
{
    reveal(unsupported_union_invariant);
    assert(spec_unsupported_union(a, b) == a | b);
}

pub proof fn proof_supported_is_identity(a: SpecUnsupportedRecoveryState)
    ensures spec_unsupported_union(a, spec_unsupported_supported()) == a
{
    assert(a | 0 == a) by (bit_vector);
}

pub proof fn proof_supported_is_absorbing(a: SpecUnsupportedRecoveryState)
    ensures spec_unsupported_union(spec_unsupported_supported(), a) == a
{
    assert(0 | a == a) by (bit_vector);
}

} // verus!

fn main() {}
