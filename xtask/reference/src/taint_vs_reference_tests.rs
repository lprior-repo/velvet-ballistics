//! Differential test for VB-REF-002: the reference taint lattice against the
//! production taint join over every taint level, every pair, and every
//! triple, plus the reference lattice laws checked against production.

use crate::taint_model::{TaintModel, TaintOrder, join_many_ref};
use proptest::prelude::*;
use vb_core::value::{Taint, join_taint};

const ALL_TAINTS: [Taint; 5] = [
    Taint::Clean,
    Taint::DerivedFromSecret,
    Taint::Secret,
    Taint::Random,
    Taint::TimeDependent,
];

fn arb_taint() -> impl Strategy<Value = Taint> {
    prop_oneof![
        Just(Taint::Clean),
        Just(Taint::DerivedFromSecret),
        Just(Taint::Secret),
        Just(Taint::Random),
        Just(Taint::TimeDependent),
    ]
}

#[test]
fn taint_vs_reference() {
    let model = TaintModel::new();
    for a in ALL_TAINTS {
        for b in ALL_TAINTS {
            assert_eq!(join_taint(a, b), model.join(a, b), "join({a:?}, {b:?})");
        }
    }
}

#[test]
fn taint_vs_reference_join_all() {
    let model = TaintModel::new();
    for taints in [
        vec![],
        vec![Taint::Secret],
        vec![Taint::Clean, Taint::Random, Taint::Secret],
        vec![Taint::DerivedFromSecret, Taint::Clean],
        vec![
            Taint::Clean,
            Taint::DerivedFromSecret,
            Taint::Secret,
            Taint::Random,
            Taint::TimeDependent,
        ],
        vec![Taint::Random, Taint::Random, Taint::Clean],
    ] {
        let orders: Vec<TaintOrder> = taints
            .iter()
            .map(|taint| TaintOrder::from_taint(*taint))
            .collect();
        assert_eq!(
            model.join_all(&taints),
            join_many_ref(&orders).to_taint(),
            "join_all({taints:?})"
        );
    }
}

#[test]
fn taint_vs_reference_lattice_laws() {
    for a in ALL_TAINTS {
        for b in ALL_TAINTS {
            assert_eq!(
                join_taint(a, b),
                join_taint(b, a),
                "commutative: join({a:?}, {b:?})"
            );
            assert_eq!(join_taint(a, a), a, "idempotent: join({a:?}, {a:?})");
            assert_eq!(
                join_taint(a, Taint::Clean),
                a,
                "identity: join({a:?}, Clean)"
            );
            for c in ALL_TAINTS {
                assert_eq!(
                    join_taint(join_taint(a, b), c),
                    join_taint(a, join_taint(b, c)),
                    "associative: join(join({a:?}, {b:?}), {c:?})"
                );
            }
        }
        assert_eq!(
            join_taint(Taint::Clean, Taint::Secret),
            Taint::Secret,
            "secret never downgrades"
        );
        assert_eq!(
            join_taint(Taint::Clean, Taint::DerivedFromSecret),
            Taint::DerivedFromSecret,
            "derived never downgrades"
        );
    }
}

proptest! {
    #[test]
    fn taint_vs_reference_proptest(a in arb_taint(), b in arb_taint()) {
        let model = TaintModel::new();
        prop_assert_eq!(join_taint(a, b), model.join(a, b));
    }
}
