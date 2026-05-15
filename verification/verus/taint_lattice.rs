// Verus proof obligations for the taint lattice.
//
// Source model: `crates/vb_proof_kernels/src/taint.rs`.
// Registry obligations: VB-CORE-TAINT-001 through VB-CORE-TAINT-006.
// Exact verifier command: `verus verification/verus/taint_lattice.rs`.

use vstd::prelude::*;

verus! {

pub enum SpecTaint {
    Clean,
    DerivedFromSecret,
    Secret,
}

pub open spec fn spec_rank(t: SpecTaint) -> nat {
    match t {
        SpecTaint::Clean => 0,
        SpecTaint::DerivedFromSecret => 1,
        SpecTaint::Secret => 2,
    }
}

pub open spec fn spec_join_taint(a: SpecTaint, b: SpecTaint) -> SpecTaint {
    if spec_rank(a) >= spec_rank(b) {
        a
    } else {
        b
    }
}

pub proof fn lemma_rank_ordering()
    ensures
        spec_rank(SpecTaint::Clean) < spec_rank(SpecTaint::DerivedFromSecret),
        spec_rank(SpecTaint::DerivedFromSecret) < spec_rank(SpecTaint::Secret),
{
    assert(spec_rank(SpecTaint::Clean) < spec_rank(SpecTaint::DerivedFromSecret)) by(compute);
    assert(spec_rank(SpecTaint::DerivedFromSecret) < spec_rank(SpecTaint::Secret)) by(compute);
}

pub proof fn lemma_join_selects_left_when_rank_ge(a: SpecTaint, b: SpecTaint)
    requires
        spec_rank(a) >= spec_rank(b),
    ensures
        spec_join_taint(a, b) == a,
{
}

pub proof fn lemma_join_selects_right_when_rank_lt(a: SpecTaint, b: SpecTaint)
    requires
        spec_rank(a) < spec_rank(b),
    ensures
        spec_join_taint(a, b) == b,
{
}

pub proof fn lemma_join_associative(a: SpecTaint, b: SpecTaint, c: SpecTaint)
    ensures
        spec_join_taint(spec_join_taint(a, b), c) == spec_join_taint(a, spec_join_taint(b, c)),
{
    assert(spec_join_taint(spec_join_taint(a, b), c) == spec_join_taint(a, spec_join_taint(b, c))) by(compute);
}

pub proof fn lemma_join_commutative(a: SpecTaint, b: SpecTaint)
    ensures
        spec_join_taint(a, b) == spec_join_taint(b, a),
{
    assert(spec_join_taint(a, b) == spec_join_taint(b, a)) by(compute);
}

pub proof fn lemma_join_idempotent(a: SpecTaint)
    ensures
        spec_join_taint(a, a) == a,
{
    assert(spec_join_taint(a, a) == a) by(compute);
}

pub proof fn lemma_join_identity(a: SpecTaint)
    ensures
        spec_join_taint(a, SpecTaint::Clean) == a,
        spec_join_taint(SpecTaint::Clean, a) == a,
{
    assert(spec_join_taint(a, SpecTaint::Clean) == a) by(compute);
    assert(spec_join_taint(SpecTaint::Clean, a) == a) by(compute);
}

pub proof fn lemma_secret_top(a: SpecTaint)
    ensures
        spec_join_taint(a, SpecTaint::Secret) == SpecTaint::Secret,
        spec_join_taint(SpecTaint::Secret, a) == SpecTaint::Secret,
{
    assert(spec_join_taint(a, SpecTaint::Secret) == SpecTaint::Secret) by(compute);
    assert(spec_join_taint(SpecTaint::Secret, a) == SpecTaint::Secret) by(compute);
}

pub proof fn lemma_derived_never_downgrades(a: SpecTaint)
    requires
        spec_rank(a) >= spec_rank(SpecTaint::DerivedFromSecret),
    ensures
        spec_rank(spec_join_taint(a, SpecTaint::DerivedFromSecret)) >= spec_rank(SpecTaint::DerivedFromSecret),
        spec_rank(spec_join_taint(SpecTaint::DerivedFromSecret, a)) >= spec_rank(SpecTaint::DerivedFromSecret),
{
    assert(spec_rank(spec_join_taint(a, SpecTaint::DerivedFromSecret)) >= spec_rank(SpecTaint::DerivedFromSecret)) by(compute);
    assert(spec_rank(spec_join_taint(SpecTaint::DerivedFromSecret, a)) >= spec_rank(SpecTaint::DerivedFromSecret)) by(compute);
}

pub proof fn lemma_join_never_downgrades_left(a: SpecTaint, b: SpecTaint)
    ensures
        spec_rank(spec_join_taint(a, b)) >= spec_rank(a),
{
    assert(spec_rank(spec_join_taint(a, b)) >= spec_rank(a)) by(compute);
}

pub proof fn lemma_join_never_downgrades_right(a: SpecTaint, b: SpecTaint)
    ensures
        spec_rank(spec_join_taint(a, b)) >= spec_rank(b),
{
    assert(spec_rank(spec_join_taint(a, b)) >= spec_rank(b)) by(compute);
}

pub proof fn lemma_all_lattice_laws(a: SpecTaint, b: SpecTaint, c: SpecTaint)
    ensures
        spec_join_taint(spec_join_taint(a, b), c) == spec_join_taint(a, spec_join_taint(b, c)),
        spec_join_taint(a, b) == spec_join_taint(b, a),
        spec_join_taint(a, a) == a,
        spec_join_taint(a, SpecTaint::Clean) == a,
        spec_join_taint(SpecTaint::Clean, a) == a,
        spec_rank(spec_join_taint(a, b)) >= spec_rank(a),
        spec_rank(spec_join_taint(a, b)) >= spec_rank(b),
{
    lemma_join_associative(a, b, c);
    lemma_join_commutative(a, b);
    lemma_join_idempotent(a);
    lemma_join_identity(a);
    lemma_join_never_downgrades_left(a, b);
    lemma_join_never_downgrades_right(a, b);
}

fn main() {}

} // verus!
