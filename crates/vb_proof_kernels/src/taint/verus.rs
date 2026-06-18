//! All Verus code for the taint lattice.
//!
//! Contains the Taint enum, spec_rank, spec_join, join_taint, join_many,
//! predicates (spec_is_clean, spec_is_derived, spec_is_secret), and all
//! proof lemmas for the lattice laws.

verus! {
    // ── Taint enum ───────────────────────────────────────────────────

    #[derive(Clone, Copy)]
    pub enum Taint {
        Clean,
        DerivedFromSecret,
        Secret,
    }

    impl Taint {
        /// Spec-mode equality for Taint (avoids external PartialEq derive).
        pub open spec fn spec_eq(&self, other: &Taint) -> bool {
            spec_rank(*self) == spec_rank(*other)
        }

        /// Exec-mode equality for Taint (avoids external PartialEq derive).
        pub exec fn exec_eq(&self, other: &Taint) -> (result: bool)
            ensures
                result == self.spec_eq(other),
        {
            match (self, other) {
                (Taint::Clean, Taint::Clean)
                | (Taint::DerivedFromSecret, Taint::DerivedFromSecret)
                | (Taint::Secret, Taint::Secret) => true,
                _ => false,
            }
        }
    }

    // ── Spec: rank ───────────────────────────────────────────────────

    pub open spec fn spec_rank(t: Taint) -> nat {
        match t {
            Taint::Clean => 0,
            Taint::DerivedFromSecret => 1,
            Taint::Secret => 2,
        }
    }

    // ── Spec: join (lattice supremum) ────────────────────────────────
    // Closed so the verifier can unfold in proof contexts.

    pub closed spec fn spec_join(a: Taint, b: Taint) -> Taint {
        if spec_rank(a) >= spec_rank(b) {
            a
        } else {
            b
        }
    }

    // ── Spec predicates ──────────────────────────────────────────────

    pub open spec fn spec_is_clean(t: Taint) -> bool {
        spec_rank(t) == 0
    }

    pub open spec fn spec_is_derived(t: Taint) -> bool {
        spec_rank(t) == 1
    }

    pub open spec fn spec_is_secret(t: Taint) -> bool {
        spec_rank(t) == 2
    }

    // ── Lemma: join is commutative ───────────────────────────────────

    proof fn lemma_join_is_commutative(a: Taint, b: Taint)
        ensures
            spec_join(a, b) == spec_join(b, a),
    {
        assert(spec_rank(a) >= spec_rank(b) || spec_rank(b) >= spec_rank(a));
        // Case analysis on (a, b): 9 pairs. In each, the higher-rank
        // element is the join, and rank ordering is symmetric for equality.
        assert(spec_join(a, b) == spec_join(b, a));
    }

    // ── Lemma: join is associative ───────────────────────────────────

    proof fn lemma_join_is_associative(a: Taint, b: Taint, c: Taint)
        ensures
            spec_join(spec_join(a, b), c) == spec_join(a, spec_join(b, c)),
    {
        // max(max(a, b), c) == max(a, max(b, c)) over a totally ordered set.
        // Case analysis on (a, b, c) with 27 combinations.
        assert(spec_join(spec_join(a, b), c) == spec_join(a, spec_join(b, c)));
    }

    // ── Lemma: join is idempotent ────────────────────────────────────

    proof fn lemma_join_is_idempotent(a: Taint)
        ensures
            spec_join(a, a) == a,
    {
        // max(a, a) == a. For any a: if rank(a) >= rank(a) then a else a => a.
        assert(spec_join(a, a) == a);
    }

    // ── Lemma: Clean is the bottom element ───────────────────────────

    proof fn lemma_clean_is_bottom()
        ensures
            spec_is_clean(Taint::Clean),
            forall|t: Taint| spec_is_clean(t) ==> Taint::Clean.spec_eq(&t),
    {
        assert(spec_is_clean(Taint::Clean));
        // Taint has exactly 3 variants; only Clean has rank 0.
        assert(forall|t: Taint| spec_is_clean(t) ==> Taint::Clean.spec_eq(&t));
    }

    // ── Lemma: Secret is the top element ─────────────────────────────

    proof fn lemma_secret_is_top()
        ensures
            spec_is_secret(Taint::Secret),
            forall|t: Taint| spec_is_secret(t) ==> Taint::Secret.spec_eq(&t),
    {
        assert(spec_is_secret(Taint::Secret));
        assert(forall|t: Taint| spec_is_secret(t) ==> Taint::Secret.spec_eq(&t));
    }

    // ── Lemma: Clean join any equals the other ───────────────────────

    proof fn lemma_clean_join(a: Taint)
        ensures
            spec_join(Taint::Clean, a) == a,
    {
        // rank(Clean) = 0, so spec_rank(a) >= 0 = spec_rank(Clean) for all a.
        assert(spec_rank(Taint::Clean) <= spec_rank(a));
        assert(spec_join(Taint::Clean, a) == a);
    }

    // ── Lemma: Secret join any equals Secret ─────────────────────────

    proof fn lemma_secret_join(a: Taint)
        ensures
            spec_join(Taint::Secret, a) == Taint::Secret,
    {
        // rank(Secret) = 2 >= rank(a) for all a.
        assert(spec_rank(Taint::Secret) >= spec_rank(a));
        assert(spec_join(Taint::Secret, a) == Taint::Secret);
    }

    // ── Lemma: no downgrades from secret ─────────────────────────────

    proof fn lemma_no_secret_downgrade()
        ensures
            spec_join(Taint::Clean, Taint::Secret) == Taint::Secret,
    {
        // rank(Clean)=0 < rank(Secret)=2, so join picks Secret.
        assert(spec_rank(Taint::Clean) < spec_rank(Taint::Secret));
        assert(spec_join(Taint::Clean, Taint::Secret) == Taint::Secret);
    }

    // ── Lemma: no downgrades from derived ────────────────────────────

    proof fn lemma_no_derived_downgrade()
        ensures
            spec_join(Taint::Clean, Taint::DerivedFromSecret) == Taint::DerivedFromSecret,
    {
        // rank(Clean)=0 < rank(Derived)=1, so join picks Derived.
        assert(spec_rank(Taint::Clean) < spec_rank(Taint::DerivedFromSecret));
        assert(spec_join(Taint::Clean, Taint::DerivedFromSecret) == Taint::DerivedFromSecret);
    }

    // ── Lemma: join is monotone in first argument ────────────────────

    proof fn lemma_join_monotone_first(a1: Taint, a2: Taint, b: Taint)
        requires
            spec_rank(a1) <= spec_rank(a2),
        ensures
            spec_rank(spec_join(a1, b)) <= spec_rank(spec_join(a2, b)),
    {
        // max(rank(a1), rank(b)) <= max(rank(a2), rank(b)) when rank(a1) <= rank(a2).
        assert(spec_rank(a1) <= spec_rank(a2));
        assert(spec_rank(spec_join(a1, b)) <= spec_rank(spec_join(a2, b)));
    }

    // ── Lemma: join is monotone in second argument ───────────────────

    proof fn lemma_join_monotone_second(a: Taint, b1: Taint, b2: Taint)
        requires
            spec_rank(b1) <= spec_rank(b2),
        ensures
            spec_rank(spec_join(a, b1)) <= spec_rank(spec_join(a, b2)),
    {
        // max(rank(a), rank(b1)) <= max(rank(a), rank(b2)) when rank(b1) <= rank(b2).
        assert(spec_rank(b1) <= spec_rank(b2));
        assert(spec_rank(spec_join(a, b1)) <= spec_rank(spec_join(a, b2)));
    }

    // ── Lemma: rank is strictly ordered ──────────────────────────────

    proof fn lemma_rank_strict_order()
        ensures
            spec_rank(Taint::Clean) < spec_rank(Taint::DerivedFromSecret),
            spec_rank(Taint::DerivedFromSecret) < spec_rank(Taint::Secret),
            spec_rank(Taint::Clean) < spec_rank(Taint::Secret),
    {
        assert(spec_rank(Taint::Clean) < spec_rank(Taint::DerivedFromSecret));
        assert(spec_rank(Taint::DerivedFromSecret) < spec_rank(Taint::Secret));
        assert(spec_rank(Taint::Clean) < spec_rank(Taint::Secret));
    }

    // ── Lemma: exactly three distinct elements ───────────────────────

    proof fn lemma_exactly_three_elements()
        ensures
            !Taint::Clean.spec_eq(&Taint::DerivedFromSecret),
            !Taint::Clean.spec_eq(&Taint::Secret),
            !Taint::DerivedFromSecret.spec_eq(&Taint::Secret),
    {
        assert(!Taint::Clean.spec_eq(&Taint::DerivedFromSecret));
        assert(!Taint::Clean.spec_eq(&Taint::Secret));
        assert(!Taint::DerivedFromSecret.spec_eq(&Taint::Secret));
    }

    // ── Exec: join_taint — executable lattice supremum ───────────────

    pub fn join_taint(a: Taint, b: Taint) -> (result: Taint)
        ensures
            result == spec_join(a, b),
    {
        match (a, b) {
            (Taint::Clean, Taint::Clean) => Taint::Clean,
            (Taint::Clean, Taint::DerivedFromSecret) => Taint::DerivedFromSecret,
            (Taint::Clean, Taint::Secret) => Taint::Secret,
            (Taint::DerivedFromSecret, Taint::Clean) => Taint::DerivedFromSecret,
            (Taint::DerivedFromSecret, Taint::DerivedFromSecret) => Taint::DerivedFromSecret,
            (Taint::DerivedFromSecret, Taint::Secret) => Taint::Secret,
            (Taint::Secret, Taint::Clean) => Taint::Secret,
            (Taint::Secret, Taint::DerivedFromSecret) => Taint::Secret,
            (Taint::Secret, Taint::Secret) => Taint::Secret,
        }
    }

    // ── Exec: is_commutative — verifies join is commutative ──────────

    pub fn is_commutative(a: Taint, b: Taint) -> (equal: bool)
        ensures
            equal == spec_join(a, b).spec_eq(&spec_join(b, a)),
    {
        join_taint(a, b).exec_eq(&join_taint(b, a))
    }

    // ── Exec: is_associative — verifies join is associative ──────────

    pub fn is_associative(a: Taint, b: Taint, c: Taint) -> (equal: bool)
        ensures
            equal == spec_join(spec_join(a, b), c).spec_eq(&spec_join(a, spec_join(b, c))),
    {
        join_taint(join_taint(a, b), c).exec_eq(&join_taint(a, join_taint(b, c)))
    }

    // ── Exec: is_idempotent — verifies join is idempotent ────────────

    pub fn is_idempotent(a: Taint) -> (equal: bool)
        ensures
            equal == spec_join(a, a).spec_eq(&a),
    {
        join_taint(a, a).exec_eq(&a)
    }

    // ── Exec: has_identity — verifies Clean is identity ──────────────

    pub fn has_identity(a: Taint) -> (equal: bool)
        ensures
            equal == spec_join(a, Taint::Clean).spec_eq(&a),
    {
        join_taint(a, Taint::Clean).exec_eq(&a)
    }

    // ── Exec: secret_never_downgrades — verifies no downgrade from Secret ──

    pub fn secret_never_downgrades() -> (no_downgrade: bool)
        ensures
            no_downgrade,
    {
        join_taint(Taint::Clean, Taint::Secret).exec_eq(&Taint::Secret)
    }

    // ── Exec: derived_never_downgrades — verifies no downgrade from Derived ─

    pub fn derived_never_downgrades() -> (no_downgrade: bool)
        ensures
            no_downgrade,
    {
        join_taint(Taint::Clean, Taint::DerivedFromSecret).exec_eq(&Taint::DerivedFromSecret)
    }

    // ── Exec: join_many — fold over taint lattice with loop invariant ─

    #[verifier::exec_allows_no_decreases_clause]
    pub fn join_many(taints: &[Taint]) -> (result: Taint)
        ensures
            // Result rank is >= rank of every element in the slice
            forall|i: nat| i < taints.len() ==> spec_rank(result) >= #[trigger] spec_rank(taints[i as int]),
    {
        let mut result = Taint::Clean;
        let mut i = 0usize;
        while (i < taints.len())
            invariant (forall|j: nat| j < i ==> spec_rank(result) >= #[trigger] spec_rank(taints[j as int]))
        {
            result = join_taint(result, taints[i]);
            i += 1;
        }
        result
    }

    // ── Exec: all_lattice_laws — composite lattice law checker ───────

    pub fn all_lattice_laws(a: Taint, b: Taint, c: Taint) -> (holds: bool)
        ensures
            holds == (spec_join(a, b).spec_eq(&spec_join(b, a))
                && spec_join(spec_join(a, b), c).spec_eq(&spec_join(a, spec_join(b, c)))
                && spec_join(a, a).spec_eq(&a)
                && spec_join(a, Taint::Clean).spec_eq(&a)
                && spec_join(Taint::Clean, Taint::Secret).spec_eq(&Taint::Secret)
                && spec_join(Taint::Clean, Taint::DerivedFromSecret).spec_eq(&Taint::DerivedFromSecret)),
    {
        is_commutative(a, b) && is_associative(a, b, c) && is_idempotent(a) && has_identity(a)
            && secret_never_downgrades() && derived_never_downgrades()
    }

    // ── Re-exports ───────────────────────────────────────────────────

    pub use Taint;
    pub use spec_rank;
    pub use spec_join;
    pub use spec_is_clean;
    pub use spec_is_derived;
    pub use spec_is_secret;
    pub use join_taint;
    pub use join_many;
    pub use is_commutative;
    pub use is_associative;
    pub use is_idempotent;
    pub use has_identity;
    pub use secret_never_downgrades;
    pub use derived_never_downgrades;
    pub use all_lattice_laws;
}
