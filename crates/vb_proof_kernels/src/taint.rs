//! Taint lattice proof kernel.
//!
//! This is a tiny, pure, sequential Rust kernel for taint lattice verification.
//! Suitable for Verus/Aeneas extraction to Lean.
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

// ── Verus verified layer ────────────────────────────────────────────────────
#[cfg(verus_keep_ghost)]
verus! {

// ── Taint enum ─────────────────────────────────────────────────────────
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

// ── Spec: rank ─────────────────────────────────────────────────────────
pub open spec fn spec_rank(t: Taint) -> nat {
    match t {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
    }
}

// ── Spec: join (lattice supremum) ──────────────────────────────────────
// closed so the verifier can unfold in proof contexts
pub closed spec fn spec_join(a: Taint, b: Taint) -> Taint {
    if spec_rank(a) >= spec_rank(b) {
        a
    } else {
        b
    }
}

// ── Spec: is_clean ─────────────────────────────────────────────────────
pub open spec fn spec_is_clean(t: Taint) -> bool {
    spec_rank(t) == 0
}

// ── Spec: is_derived ───────────────────────────────────────────────────
pub open spec fn spec_is_derived(t: Taint) -> bool {
    spec_rank(t) == 1
}

// ── Spec: is_secret ────────────────────────────────────────────────────
pub open spec fn spec_is_secret(t: Taint) -> bool {
    spec_rank(t) == 2
}

// ── Lemma: join is commutative ─────────────────────────────────────────
proof fn lemma_join_is_commutative(a: Taint, b: Taint)
    ensures
        spec_join(a, b) == spec_join(b, a),
{
    assert(spec_rank(a) >= spec_rank(b) || spec_rank(b) >= spec_rank(a));
    // Case analysis on (a, b): (Clean,Clean), (Clean,Derived), (Clean,Secret),
    // (Derived,Clean), (Derived,Derived), (Derived,Secret), (Secret,Clean),
    // (Secret,Derived), (Secret,Secret). In each pair, the higher-rank
    // element is the join, and rank ordering is symmetric for equality.
    assert(spec_join(a, b) == spec_join(b, a));
}

// ── Lemma: join is associative ─────────────────────────────────────────
proof fn lemma_join_is_associative(a: Taint, b: Taint, c: Taint)
    ensures
        spec_join(spec_join(a, b), c) == spec_join(a, spec_join(b, c)),
{
    // max(max(a, b), c) == max(a, max(b, c)) over a totally ordered set.
    // Case analysis on (a, b, c) with 27 combinations; the max element
    // is the same regardless of grouping.
    assert(spec_join(spec_join(a, b), c) == spec_join(a, spec_join(b, c)));
}

// ── Lemma: join is idempotent ──────────────────────────────────────────
proof fn lemma_join_is_idempotent(a: Taint)
    ensures
        spec_join(a, a) == a,
{
    // max(a, a) == a. For any a: if rank(a) >= rank(a) then a else a => a.
    assert(spec_join(a, a) == a);
}

// ── Lemma: Clean is the bottom element ─────────────────────────────────
proof fn lemma_clean_is_bottom()
    ensures
        spec_is_clean(Taint::Clean),
        forall|t: Taint| spec_is_clean(t) ==> Taint::Clean.spec_eq(&t),
{
    assert(spec_is_clean(Taint::Clean));
    // Taint has exactly 3 variants; only Clean has rank 0.
    assert(forall|t: Taint| spec_is_clean(t) ==> Taint::Clean.spec_eq(&t));
}

// ── Lemma: Secret is the top element ───────────────────────────────────
proof fn lemma_secret_is_top()
    ensures
        spec_is_secret(Taint::Secret),
        forall|t: Taint| spec_is_secret(t) ==> Taint::Secret.spec_eq(&t),
{
    assert(spec_is_secret(Taint::Secret));
    assert(forall|t: Taint| spec_is_secret(t) ==> Taint::Secret.spec_eq(&t));
}

// ── Lemma: Clean join any equals the other ─────────────────────────────
proof fn lemma_clean_join(a: Taint)
    ensures
        spec_join(Taint::Clean, a) == a,
{
    // rank(Clean) = 0, so spec_rank(a) >= 0 = spec_rank(Clean) for all a.
    // Therefore spec_join(Clean, a) = a.
    assert(spec_rank(Taint::Clean) <= spec_rank(a));
    assert(spec_join(Taint::Clean, a) == a);
}

// ── Lemma: Secret join any equals Secret ───────────────────────────────
proof fn lemma_secret_join(a: Taint)
    ensures
        spec_join(Taint::Secret, a) == Taint::Secret,
{
    // rank(Secret) = 2 >= rank(a) for all a.
    // Therefore spec_join(Secret, a) = Secret.
    assert(spec_rank(Taint::Secret) >= spec_rank(a));
    assert(spec_join(Taint::Secret, a) == Taint::Secret);
}

// ── Lemma: no downgrades from secret ───────────────────────────────────
proof fn lemma_no_secret_downgrade()
    ensures
        spec_join(Taint::Clean, Taint::Secret) == Taint::Secret,
{
    // rank(Clean)=0 < rank(Secret)=2, so join picks Secret.
    assert(spec_rank(Taint::Clean) < spec_rank(Taint::Secret));
    assert(spec_join(Taint::Clean, Taint::Secret) == Taint::Secret);
}

// ── Lemma: no downgrades from derived ──────────────────────────────────
proof fn lemma_no_derived_downgrade()
    ensures
        spec_join(Taint::Clean, Taint::DerivedFromSecret) == Taint::DerivedFromSecret,
{
    // rank(Clean)=0 < rank(Derived)=1, so join picks Derived.
    assert(spec_rank(Taint::Clean) < spec_rank(Taint::DerivedFromSecret));
    assert(spec_join(Taint::Clean, Taint::DerivedFromSecret) == Taint::DerivedFromSecret);
}

// ── Lemma: join is monotone in first argument ──────────────────────────
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

// ── Lemma: join is monotone in second argument ─────────────────────────
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

// ── Lemma: rank is strictly ordered ────────────────────────────────────
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

// ── Lemma: exactly three distinct elements ─────────────────────────────
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

// ── Exec: join_taint — executable lattice supremum ─────────────────────
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

// ── Exec: is_commutative — verifies join is commutative ────────────────
pub fn is_commutative(a: Taint, b: Taint) -> (equal: bool)
    ensures
        equal == spec_join(a, b).spec_eq(&spec_join(b, a)),
{
    join_taint(a, b).exec_eq(&join_taint(b, a))
}

// ── Exec: is_associative — verifies join is associative ────────────────
pub fn is_associative(a: Taint, b: Taint, c: Taint) -> (equal: bool)
    ensures
        equal == spec_join(spec_join(a, b), c).spec_eq(&spec_join(a, spec_join(b, c))),
{
    join_taint(join_taint(a, b), c).exec_eq(&join_taint(a, join_taint(b, c)))
}

// ── Exec: is_idempotent — verifies join is idempotent ──────────────────
pub fn is_idempotent(a: Taint) -> (equal: bool)
    ensures
        equal == spec_join(a, a).spec_eq(&a),
{
    join_taint(a, a).exec_eq(&a)
}

// ── Exec: has_identity — verifies Clean is identity ────────────────────
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

// ── Exec: join_many — fold over taint lattice with loop invariant ──────
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

// ── Exec: all_lattice_laws — composite lattice law checker ─────────────
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

} // verus!
// ── Regular Rust implementation (non-Verus compilation) ─────────────────────
#[cfg(not(verus_keep_ghost))]
mod cargo_kernel {
    /// Taint classification for values in the proof kernel.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[non_exhaustive]
    pub enum Taint {
        Clean,
        DerivedFromSecret,
        Secret,
    }

    impl Taint {
        #[must_use]
        pub fn rank(&self) -> u8 {
            match self {
                Taint::Clean => 0,
                Taint::DerivedFromSecret => 1,
                Taint::Secret => 2,
            }
        }
    }

    pub fn join_taint(a: Taint, b: Taint) -> Taint {
        if a.rank() >= b.rank() {
            a
        } else {
            b
        }
    }

    pub fn join_many(taints: &[Taint]) -> Taint {
        let mut result = Taint::Clean;
        for &t in taints {
            result = join_taint(result, t);
        }
        result
    }

    pub fn is_commutative(a: Taint, b: Taint) -> bool {
        join_taint(a, b) == join_taint(b, a)
    }

    pub fn is_associative(a: Taint, b: Taint, c: Taint) -> bool {
        join_taint(join_taint(a, b), c) == join_taint(a, join_taint(b, c))
    }

    pub fn is_idempotent(a: Taint) -> bool {
        join_taint(a, a) == a
    }

    pub fn has_identity(a: Taint) -> bool {
        join_taint(a, Taint::Clean) == a
    }

    pub fn secret_never_downgrades() -> bool {
        join_taint(Taint::Clean, Taint::Secret) == Taint::Secret
    }

    pub fn derived_never_downgrades() -> bool {
        join_taint(Taint::Clean, Taint::DerivedFromSecret) == Taint::DerivedFromSecret
    }

    pub fn all_lattice_laws(a: Taint, b: Taint, c: Taint) -> bool {
        is_commutative(a, b)
            && is_associative(a, b, c)
            && is_idempotent(a)
            && has_identity(a)
            && secret_never_downgrades()
            && derived_never_downgrades()
    }
}
#[cfg(not(verus_keep_ghost))]
pub use cargo_kernel::*;

// ── Tests (compiled in both modes) ──────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_clean_clean() {
        assert_eq!(join_taint(Taint::Clean, Taint::Clean), Taint::Clean);
    }

    #[test]
    fn test_join_clean_secret() {
        assert_eq!(join_taint(Taint::Clean, Taint::Secret), Taint::Secret);
    }

    #[test]
    fn test_join_secret_clean() {
        assert_eq!(join_taint(Taint::Secret, Taint::Clean), Taint::Secret);
    }

    #[test]
    fn test_join_secret_derived() {
        assert_eq!(
            join_taint(Taint::Secret, Taint::DerivedFromSecret),
            Taint::Secret
        );
    }

    #[test]
    fn test_join_derived_derived() {
        assert_eq!(
            join_taint(Taint::DerivedFromSecret, Taint::DerivedFromSecret),
            Taint::DerivedFromSecret
        );
    }

    #[test]
    fn test_commutative() {
        for &a in &[Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
            for &b in &[Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
                assert!(
                    is_commutative(a, b),
                    " {:?} ⊔ {:?} != {:?} ⊔ {:?}",
                    a,
                    b,
                    b,
                    a
                );
            }
        }
    }

    #[test]
    fn test_associative() {
        for &a in &[Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
            for &b in &[Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
                for &c in &[Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
                    assert!(is_associative(a, b, c), "associativity failed");
                }
            }
        }
    }

    #[test]
    fn test_idempotent() {
        for &a in &[Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
            assert!(is_idempotent(a), "idempotence failed for {:?}", a);
        }
    }

    #[test]
    fn test_identity() {
        for &a in &[Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
            assert!(has_identity(a), "identity failed for {:?}", a);
        }
    }

    #[test]
    fn test_secret_never_downgrades() {
        assert!(secret_never_downgrades());
    }

    #[test]
    fn test_derived_never_downgrades() {
        assert!(derived_never_downgrades());
    }

    // ── Taint::rank ───────────────────────────────────────────────────────

    #[test]
    fn test_rank_clean() {
        assert_eq!(Taint::Clean.rank(), 0);
    }

    #[test]
    fn test_rank_derived_from_secret() {
        assert_eq!(Taint::DerivedFromSecret.rank(), 1);
    }

    #[test]
    fn test_rank_secret() {
        assert_eq!(Taint::Secret.rank(), 2);
    }

    #[test]
    fn test_rank_strict_ordering() {
        assert!(Taint::Clean.rank() < Taint::DerivedFromSecret.rank());
        assert!(Taint::DerivedFromSecret.rank() < Taint::Secret.rank());
        assert!(Taint::Clean.rank() < Taint::Secret.rank());
    }

    // ── join_many ─────────────────────────────────────────────────────────

    #[test]
    fn test_join_many_empty_slice() {
        let taints: [Taint; 0] = [];
        assert_eq!(join_many(&taints), Taint::Clean);
    }

    #[test]
    fn test_join_many_single_clean() {
        let taints = [Taint::Clean];
        assert_eq!(join_many(&taints), Taint::Clean);
    }

    #[test]
    fn test_join_many_single_secret() {
        let taints = [Taint::Secret];
        assert_eq!(join_many(&taints), Taint::Secret);
    }

    #[test]
    fn test_join_many_single_derived() {
        let taints = [Taint::DerivedFromSecret];
        assert_eq!(join_many(&taints), Taint::DerivedFromSecret);
    }

    #[test]
    fn test_join_many_all_clean() {
        let taints = [Taint::Clean, Taint::Clean, Taint::Clean];
        assert_eq!(join_many(&taints), Taint::Clean);
    }

    #[test]
    fn test_join_many_with_secret() {
        let taints = [Taint::Clean, Taint::Secret, Taint::DerivedFromSecret];
        assert_eq!(join_many(&taints), Taint::Secret);
    }

    #[test]
    fn test_join_many_derived_and_clean() {
        let taints = [Taint::Clean, Taint::DerivedFromSecret, Taint::Clean];
        assert_eq!(join_many(&taints), Taint::DerivedFromSecret);
    }

    #[test]
    fn test_join_many_multiple_secrets() {
        let taints = [Taint::Secret, Taint::Secret, Taint::Secret];
        assert_eq!(join_many(&taints), Taint::Secret);
    }

    #[test]
    fn test_join_many_many_elements() {
        let taints = [
            Taint::Clean,
            Taint::Clean,
            Taint::DerivedFromSecret,
            Taint::Clean,
            Taint::Secret,
            Taint::Clean,
        ];
        assert_eq!(join_many(&taints), Taint::Secret);
    }

    #[test]
    fn test_join_many_is_commutative_across_slice() {
        let taints_a = [Taint::Clean, Taint::Secret, Taint::DerivedFromSecret];
        let taints_b = [Taint::DerivedFromSecret, Taint::Clean, Taint::Secret];
        assert_eq!(join_many(&taints_a), join_many(&taints_b));
    }

    // ── all_lattice_laws ──────────────────────────────────────────────────

    #[test]
    fn test_all_lattice_laws_clean_clean_clean() {
        assert!(all_lattice_laws(Taint::Clean, Taint::Clean, Taint::Clean));
    }

    #[test]
    fn test_all_lattice_laws_secret_secret_secret() {
        assert!(all_lattice_laws(
            Taint::Secret,
            Taint::Secret,
            Taint::Secret
        ));
    }

    #[test]
    fn test_all_lattice_laws_derived_derived_derived() {
        assert!(all_lattice_laws(
            Taint::DerivedFromSecret,
            Taint::DerivedFromSecret,
            Taint::DerivedFromSecret
        ));
    }

    #[test]
    fn test_all_lattice_laws_mixed_all_pairs() {
        for &a in &[Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
            for &b in &[Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
                for &c in &[Taint::Clean, Taint::DerivedFromSecret, Taint::Secret] {
                    assert!(
                        all_lattice_laws(a, b, c),
                        "lattice laws failed for {:?}, {:?}, {:?}",
                        a,
                        b,
                        c
                    );
                }
            }
        }
    }

    // ── Lattice law invariants via join_many ─────────────────────────────

    #[test]
    fn test_join_many_idempotent_via_many() {
        let taints = [Taint::Secret, Taint::Secret, Taint::Secret];
        assert_eq!(join_many(&taints), Taint::Secret);
    }

    #[test]
    fn test_join_many_order_insensitive() {
        let a = [Taint::DerivedFromSecret, Taint::Secret, Taint::Clean];
        let b = [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret];
        assert_eq!(join_many(&a), join_many(&b));
    }

    // ── Taint derived traits ──────────────────────────────────────────────

    #[test]
    fn test_taint_debug() {
        assert_eq!(format!("{:?}", Taint::Clean), "Clean");
        assert_eq!(
            format!("{:?}", Taint::DerivedFromSecret),
            "DerivedFromSecret"
        );
        assert_eq!(format!("{:?}", Taint::Secret), "Secret");
    }

    #[test]
    fn test_taint_clone() {
        assert_eq!(Taint::Clean.clone(), Taint::Clean);
        assert_eq!(Taint::DerivedFromSecret.clone(), Taint::DerivedFromSecret);
        assert_eq!(Taint::Secret.clone(), Taint::Secret);
    }

    #[test]
    fn test_taint_copy() {
        let t: Taint = Taint::DerivedFromSecret;
        let _copied: Taint = t;
        assert_eq!(t, Taint::DerivedFromSecret);
    }

    #[test]
    fn test_taint_partial_eq_positive() {
        assert_eq!(Taint::Clean, Taint::Clean);
        assert_eq!(Taint::DerivedFromSecret, Taint::DerivedFromSecret);
        assert_eq!(Taint::Secret, Taint::Secret);
    }

    #[test]
    fn test_taint_partial_eq_negative() {
        assert_ne!(Taint::Clean, Taint::DerivedFromSecret);
        assert_ne!(Taint::Clean, Taint::Secret);
        assert_ne!(Taint::DerivedFromSecret, Taint::Secret);
    }

    #[test]
    fn test_taint_eq() {
        assert!(Taint::Clean == Taint::Clean);
        assert!(Taint::Secret != Taint::Clean);
        assert!(Taint::DerivedFromSecret == Taint::DerivedFromSecret);
    }

    // ── all_lattice_laws additional combos ─────────────────────────────────

    #[test]
    fn test_all_lattice_laws_clean_secret_derived() {
        assert!(all_lattice_laws(
            Taint::Clean,
            Taint::Secret,
            Taint::DerivedFromSecret
        ));
    }

    #[test]
    fn test_all_lattice_laws_secret_derived_clean() {
        assert!(all_lattice_laws(
            Taint::Secret,
            Taint::DerivedFromSecret,
            Taint::Clean
        ));
    }

    #[test]
    fn test_all_lattice_laws_derived_clean_secret() {
        assert!(all_lattice_laws(
            Taint::DerivedFromSecret,
            Taint::Clean,
            Taint::Secret
        ));
    }

    #[test]
    fn test_all_lattice_laws_secret_clean_derived() {
        assert!(all_lattice_laws(
            Taint::Secret,
            Taint::Clean,
            Taint::DerivedFromSecret
        ));
    }

    // ── join_taint explicit rank comparison cases ───────────────────────────

    #[test]
    fn test_join_taint_derived_wins_over_clean() {
        assert_eq!(
            join_taint(Taint::DerivedFromSecret, Taint::Clean),
            Taint::DerivedFromSecret
        );
    }

    #[test]
    fn test_join_taint_secret_wins_over_both() {
        assert_eq!(
            join_taint(Taint::Secret, Taint::DerivedFromSecret),
            Taint::Secret
        );
        assert_eq!(join_taint(Taint::Secret, Taint::Clean), Taint::Secret);
        assert_eq!(
            join_taint(Taint::DerivedFromSecret, Taint::Secret),
            Taint::Secret
        );
    }

    #[test]
    fn test_join_taint_equal_ranks_keeps_first() {
        assert_eq!(
            join_taint(Taint::DerivedFromSecret, Taint::DerivedFromSecret),
            Taint::DerivedFromSecret
        );
        assert_eq!(join_taint(Taint::Secret, Taint::Secret), Taint::Secret);
    }
}

// ── Kani harness (runs under cfg(kani)) ─────────────────────────────────────
#[cfg(kani)]
mod kani_taint_harnesses {
    use super::*;

    /// PO-KANI-007: Verify lattice laws for all combinations of Taint.
    #[cfg(kani)]
    #[kani::proof]
    fn taint_lattice_laws_kani() {
        let a_raw: u8 = kani::any();
        let b_raw: u8 = kani::any();
        let c_raw: u8 = kani::any();

        let a = match a_raw % 3 {
            0 => Taint::Clean,
            1 => Taint::DerivedFromSecret,
            _ => Taint::Secret,
        };
        let b = match b_raw % 3 {
            0 => Taint::Clean,
            1 => Taint::DerivedFromSecret,
            _ => Taint::Secret,
        };
        let c = match c_raw % 3 {
            0 => Taint::Clean,
            1 => Taint::DerivedFromSecret,
            _ => Taint::Secret,
        };

        assert!(is_commutative(a, b), "join must be commutative");
        assert!(is_associative(a, b, c), "join must be associative");
        assert!(is_idempotent(a), "join must be idempotent");
        assert!(has_identity(a), "Clean must be identity");
    }
}
