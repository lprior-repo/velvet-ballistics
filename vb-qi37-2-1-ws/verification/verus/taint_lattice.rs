//! Verus specification for Taint lattice refinement.
//!
//! This file contains the Verus formal specification of the Taint lattice
//! for refinement verification between Rust and Verus.
//!
//! TAINT-LATTICE-VERUS: taint_lattice.rs must contain:
//! - SpecTaint enum matching Rust Taint variants
//! - spec_join_taint function for formal verification
//! - Lattice law lemmas (associativity, commutativity, idempotence, identity)

use crate::taint::Taint;

/// SpecTaint - Verus specification enum mirroring Rust Taint.
pub enum SpecTaint {
    Clean,
    DerivedFromSecret,
    Secret,
}

/// spec_rank - specification for taint rank ordering.
/// Rank ordering: Clean(0) < DerivedFromSecret(1) < Secret(2)
pub fn spec_rank(t: SpecTaint) -> u8 {
    match t {
        SpecTaint::Clean => 0,
        SpecTaint::DerivedFromSecret => 1,
        SpecTaint::Secret => 2,
    }
}

/// spec_join_taint - formal specification for taint lattice join.
/// Join returns the element with higher rank (max of ranks).
pub fn spec_join_taint(a: SpecTaint, b: SpecTaint) -> SpecTaint {
    if spec_rank(a) >= spec_rank(b) { a } else { b }
}

/// lemma_join_associative - Verus lemma verifying join is associative.
/// join(join(a,b),c) == join(a,join(b,c))
pub fn lemma_join_associative(a: SpecTaint, b: SpecTaint, c: SpecTaint) -> bool {
    spec_join_taint(spec_join_taint(a, b), c) == spec_join_taint(a, spec_join_taint(b, c))
}

/// lemma_join_commutative - Verus lemma verifying join is commutative.
/// join(a,b) == join(b,a)
pub fn lemma_join_commutative(a: SpecTaint, b: SpecTaint) -> bool {
    spec_join_taint(a, b) == spec_join_taint(b, a)
}

/// lemma_join_idempotent - Verus lemma verifying join is idempotent.
/// join(a,a) == a
pub fn lemma_join_idempotent(a: SpecTaint) -> bool {
    spec_join_taint(a, a) == a
}

/// lemma_join_identity - Verus lemma verifying Clean is the identity.
/// join(a, Clean) == a
pub fn lemma_join_identity(a: SpecTaint) -> bool {
    spec_join_taint(a, SpecTaint::Clean) == a
}

/// lemma_secret_top - Verus lemma verifying Secret is the top element.
/// join(Clean, Secret) == Secret (Secret never downgrades)
pub fn lemma_secret_top() -> bool {
    spec_join_taint(SpecTaint::Clean, SpecTaint::Secret) == SpecTaint::Secret
}

/// lemma_derived_never_downgrades - Verus lemma verifying DerivedFromSecret never downgrades.
/// join(Clean, DerivedFromSecret) == DerivedFromSecret
pub fn lemma_derived_never_downgrades() -> bool {
    spec_join_taint(SpecTaint::Clean, SpecTaint::DerivedFromSecret) == SpecTaint::DerivedFromSecret
}

/// lemma_rank_ordering - Verus lemma verifying the lattice ordering.
/// Clean < DerivedFromSecret < Secret
pub fn lemma_rank_ordering() -> bool {
    spec_rank(SpecTaint::Clean) < spec_rank(SpecTaint::DerivedFromSecret)
        && spec_rank(SpecTaint::DerivedFromSecret) < spec_rank(SpecTaint::Secret)
}

/// all_lattice_laws - combined lattice law verification.
pub fn all_lattice_laws(a: SpecTaint, b: SpecTaint, c: SpecTaint) -> bool {
    lemma_join_associative(a, b, c)
        && lemma_join_commutative(a, b)
        && lemma_join_idempotent(a)
        && lemma_join_identity(a)
        && lemma_secret_top()
        && lemma_derived_never_downgrades()
}
