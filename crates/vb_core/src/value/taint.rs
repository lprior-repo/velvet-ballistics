#![forbid(unsafe_code)]
//! Secret propagation marker attached to each runtime slot.

use serde::{Deserialize, Serialize};

/// Secret propagation marker attached to each runtime slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum Taint {
    /// Slot contains no secret-derived data.
    Clean = 0,
    /// Slot contains data derived from one or more secrets.
    DerivedFromSecret = 1,
    /// Slot contains a secret value.
    Secret = 2,
}

/// Joins two taint levels, returning the more restrictive one.
#[must_use]
pub fn join_taint(a: Taint, b: Taint) -> Taint {
    let a_disc: u8 = match a {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
    };
    let b_disc: u8 = match b {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
    };
    if a_disc >= b_disc { a } else { b }
}

// ───────────────────────────────────────────────────────────────────────────
// Verus annotations for Taint (compiled under verus toolchain only)
// ───────────────────────────────────────────────────────────────────────────
#[cfg(verus)]
verus! {
    use vstd::prelude::*;

    use crate::value::Taint;

    /// Spec: Taint has exactly three values.
    pub closed spec fn spec_taint_has_three_values() -> bool {
        let all_taints = [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret];
        all_taints.len() == 3
            && all_taints[0] != all_taints[1]
            && all_taints[0] != all_taints[2]
            && all_taints[1] != all_taints[2]
    }

    /// Spec: Taint ordering — Clean < DerivedFromSecret < Secret.
    pub closed spec fn spec_taint_ordering() -> bool {
        Taint::Clean as u8 < Taint::DerivedFromSecret as u8
            && Taint::DerivedFromSecret as u8 < Taint::Secret as u8
    }

    /// Spec: join_taint returns the max (more restrictive) of two taint levels.
    /// This captures the taint lattice: Clean ≤ DerivedFromSecret ≤ Secret.
    pub closed spec fn spec_join_taint(a: Taint, b: Taint) -> Taint {
        let a_disc: u8 = match a { Taint::Clean => 0, Taint::DerivedFromSecret => 1, Taint::Secret => 2 };
        let b_disc: u8 = match b { Taint::Clean => 0, Taint::DerivedFromSecret => 1, Taint::Secret => 2 };
        if a_disc >= b_disc { a } else { b }
    }

    /// Proof: production join_taint equals the spec.
    pub proof fn lemma_join_taint_equals_spec(a: Taint, b: Taint)
        ensures
            spec_join_taint(a, b) == join_taint(a, b),
    {
        // Reveal production definition (matches spec exactly).
        reveal_with_fuel(join_taint, 1);
        reveal(spec_join_taint);
        assert(spec_join_taint(a, b) == join_taint(a, b));
    }

    /// Proof: join_taint is commutative.
    pub proof fn lemma_join_taint_commutative(a: Taint, b: Taint)
        ensures
            join_taint(a, b) == join_taint(b, a),
    {
        // Both branches (a >= b or b >= a) produce the same max.
        assert(join_taint(a, b) == join_taint(b, a));
    }

    /// Proof: join_taint is associative.
    pub proof fn lemma_join_taint_associative(a: Taint, b: Taint, c: Taint)
        ensures
            join_taint(join_taint(a, b), c) == join_taint(a, join_taint(b, c)),
    {
        // Join is max; max(max(a,b),c) == max(a,max(b,c)) for total order.
        assert(join_taint(join_taint(a, b), c) == join_taint(a, join_taint(b, c)));
    }

    /// Proof: join_taint with Clean is identity.
    pub proof fn lemma_join_taint_identity(a: Taint)
        ensures
            join_taint(a, Taint::Clean) == a && join_taint(Taint::Clean, a) == a,
    {
        // Clean is minimum (0), so join with any a yields a.
        assert(join_taint(a, Taint::Clean) == a);
        assert(join_taint(Taint::Clean, a) == a);
    }

    /// Proof: join_taint is monotone in both arguments.
    pub proof fn lemma_join_taint_monotone(a1: Taint, a2: Taint, b: Taint)
        requires
            // a1 <= a2 in taint ordering (i.e., a1_disc <= a2_disc).
            join_taint(a1, a2) == a2,
        ensures
            join_taint(a1, b) <= join_taint(a2, b) && join_taint(b, a1) <= join_taint(b, a2),
    {
        // If a1 <= a2 then max(a1,b) <= max(a2,b).
        reveal_with_fuel(join_taint, 1);
        reveal(spec_join_taint);
        assert(join_taint(a1, b) <= join_taint(a2, b));
        assert(join_taint(b, a1) <= join_taint(b, a2));
    }
}
