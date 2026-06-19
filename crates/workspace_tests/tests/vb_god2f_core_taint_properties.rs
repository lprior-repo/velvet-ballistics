#![forbid(unsafe_code)]

//! HVR-PO-CORE-002: generated behavior pressure for production taint joins.

use proptest::prelude::*;
use proptest::strategy::Strategy;
use vb_core::{Taint, join_taint};

fn taint_strategy() -> impl Strategy<Value = Taint> {
    prop_oneof![
        Just(Taint::Clean),
        Just(Taint::DerivedFromSecret),
        Just(Taint::Secret)
    ]
}

fn rank(value: Taint) -> u8 {
    match value {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
        _ => 3,
    }
}

fn expected_join(left: Taint, right: Taint) -> Taint {
    if rank(left) >= rank(right) {
        left
    } else {
        right
    }
}

proptest! {
    #[test]
    fn vb_god2f_core_taint_properties(a in taint_strategy(), b in taint_strategy(), c in taint_strategy()) {
        prop_assert_eq!(join_taint(a, b), expected_join(a, b));
        prop_assert_eq!(join_taint(a, b), join_taint(b, a));
        prop_assert_eq!(join_taint(a, a), a);
        prop_assert_eq!(join_taint(a, Taint::Clean), a);
        prop_assert_eq!(join_taint(Taint::Clean, a), a);
        prop_assert_eq!(join_taint(join_taint(a, b), c), join_taint(a, join_taint(b, c)));
        if rank(a) <= rank(b) {
            prop_assert!(rank(join_taint(a, c)) <= rank(join_taint(b, c)));
            prop_assert!(rank(join_taint(c, a)) <= rank(join_taint(c, b)));
        }
    }
}

#[test]
fn vb_god2f_core_taint_matrix_matches_contract_text() {
    let values = [Taint::Clean, Taint::DerivedFromSecret, Taint::Secret];
    for left in values {
        for right in values {
            assert_eq!(join_taint(left, right), expected_join(left, right));
        }
    }
}
