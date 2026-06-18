//! Comprehensive unit tests for the taint lattice.
//!
//! Covers join combinations, lattice law verification, rank ordering,
//! join_many across slice sizes and orderings, and derive trait behavior.

#[cfg(test)]
mod tests {
    use crate::taint::{Taint, join_taint, join_many, is_commutative, is_associative, is_idempotent, has_identity, secret_never_downgrades, derived_never_downgrades, all_lattice_laws};

    // ── join_taint ──────────────────────────────────────────────────────

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

    // ── Lattice law verification ────────────────────────────────────────

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

    // ── Taint::rank ─────────────────────────────────────────────────────

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

    // ── join_many ───────────────────────────────────────────────────────

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

    // ── Taint derive traits ─────────────────────────────────────────────

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
}
