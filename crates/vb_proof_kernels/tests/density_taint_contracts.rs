#![forbid(unsafe_code)]

use vb_proof_kernels::taint::{
    Taint, all_lattice_laws, derived_never_downgrades, has_identity, is_associative,
    is_commutative, is_idempotent, join_many, join_taint, secret_never_downgrades,
};

macro_rules! ktest {
    ($(#[$attr:meta])* $name:ident, $body:block) => {
        $(#[$attr])*
        fn $name() $body
    };
}

ktest!(
    #[test]
    taint_clean_rank_is_zero,
    {
        assert_eq!(Taint::Clean.rank(), 0);
    }
);

ktest!(
    #[test]
    taint_derived_rank_is_one,
    {
        assert_eq!(Taint::DerivedFromSecret.rank(), 1);
    }
);

ktest!(
    #[test]
    taint_secret_rank_is_two,
    {
        assert_eq!(Taint::Secret.rank(), 2);
    }
);

ktest!(
    #[test]
    taint_join_clean_clean_is_clean,
    {
        assert_eq!(join_taint(Taint::Clean, Taint::Clean), Taint::Clean);
    }
);

ktest!(
    #[test]
    taint_join_clean_derived_is_derived,
    {
        assert_eq!(
            join_taint(Taint::Clean, Taint::DerivedFromSecret),
            Taint::DerivedFromSecret
        );
    }
);

ktest!(
    #[test]
    taint_join_clean_secret_is_secret,
    {
        assert_eq!(join_taint(Taint::Clean, Taint::Secret), Taint::Secret);
    }
);

ktest!(
    #[test]
    taint_join_derived_clean_is_derived,
    {
        assert_eq!(
            join_taint(Taint::DerivedFromSecret, Taint::Clean),
            Taint::DerivedFromSecret
        );
    }
);

ktest!(
    #[test]
    taint_join_derived_derived_is_derived,
    {
        assert_eq!(
            join_taint(Taint::DerivedFromSecret, Taint::DerivedFromSecret),
            Taint::DerivedFromSecret
        );
    }
);

ktest!(
    #[test]
    taint_join_derived_secret_is_secret,
    {
        assert_eq!(
            join_taint(Taint::DerivedFromSecret, Taint::Secret),
            Taint::Secret
        );
    }
);

ktest!(
    #[test]
    taint_join_secret_clean_is_secret,
    {
        assert_eq!(join_taint(Taint::Secret, Taint::Clean), Taint::Secret);
    }
);

ktest!(
    #[test]
    taint_join_secret_derived_is_secret,
    {
        assert_eq!(
            join_taint(Taint::Secret, Taint::DerivedFromSecret),
            Taint::Secret
        );
    }
);

ktest!(
    #[test]
    taint_join_secret_secret_is_secret,
    {
        assert_eq!(join_taint(Taint::Secret, Taint::Secret), Taint::Secret);
    }
);

ktest!(
    #[test]
    taint_join_many_empty_is_clean,
    {
        assert_eq!(join_many(&[]), Taint::Clean);
    }
);

ktest!(
    #[test]
    taint_join_many_clean_only_is_clean,
    {
        assert_eq!(join_many(&[Taint::Clean, Taint::Clean]), Taint::Clean);
    }
);

ktest!(
    #[test]
    taint_join_many_finds_derived,
    {
        assert_eq!(
            join_many(&[Taint::Clean, Taint::DerivedFromSecret]),
            Taint::DerivedFromSecret
        );
    }
);

ktest!(
    #[test]
    taint_join_many_finds_secret,
    {
        assert_eq!(
            join_many(&[Taint::Clean, Taint::Secret, Taint::DerivedFromSecret]),
            Taint::Secret
        );
    }
);

ktest!(
    #[test]
    taint_commutative_clean_secret,
    {
        assert!(is_commutative(Taint::Clean, Taint::Secret));
    }
);

ktest!(
    #[test]
    taint_commutative_derived_secret,
    {
        assert!(is_commutative(Taint::DerivedFromSecret, Taint::Secret));
    }
);

ktest!(
    #[test]
    taint_associative_clean_derived_secret,
    {
        assert!(is_associative(
            Taint::Clean,
            Taint::DerivedFromSecret,
            Taint::Secret
        ));
    }
);

ktest!(
    #[test]
    taint_idempotent_secret,
    {
        assert!(is_idempotent(Taint::Secret));
    }
);

ktest!(
    #[test]
    taint_identity_derived,
    {
        assert!(has_identity(Taint::DerivedFromSecret));
    }
);

ktest!(
    #[test]
    taint_secret_never_downgrades_contract,
    {
        assert!(secret_never_downgrades());
    }
);

ktest!(
    #[test]
    taint_derived_never_downgrades_contract,
    {
        assert!(derived_never_downgrades());
    }
);

ktest!(
    #[test]
    taint_all_laws_for_clean_derived_secret,
    {
        assert!(all_lattice_laws(
            Taint::Clean,
            Taint::DerivedFromSecret,
            Taint::Secret
        ));
    }
);

ktest!(
    #[test]
    taint_all_laws_for_secret_clean_derived,
    {
        assert!(all_lattice_laws(
            Taint::Secret,
            Taint::Clean,
            Taint::DerivedFromSecret
        ));
    }
);
