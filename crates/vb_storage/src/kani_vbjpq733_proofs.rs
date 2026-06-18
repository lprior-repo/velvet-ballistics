//! vb-jpq7.33 GOD RULE 2 FIX for vb_storage: Proofs that call production functions.

#![forbid(unsafe_code)]

use crate::recovery::UnsupportedRecoveryState;

// PO-029: UnsupportedRecoveryState::union — calls PRODUCTION union

#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_unsupported_union_all_combos() {
    let a = UnsupportedRecoveryState {
        slot_values: kani::any(),
        slot_taint: kani::any(),
        action_payloads: kani::any(),
    };
    let b = UnsupportedRecoveryState {
        slot_values: kani::any(),
        slot_taint: kani::any(),
        action_payloads: kani::any(),
    };
    let u = a.union(b);
    kani::assert(
        a.union_matches_flags(b, u),
        "production union_matches_flags holds",
    );
}

#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_unsupported_union_commutative() {
    let a = UnsupportedRecoveryState {
        slot_values: kani::any(),
        slot_taint: kani::any(),
        action_payloads: kani::any(),
    };
    let b = UnsupportedRecoveryState {
        slot_values: kani::any(),
        slot_taint: kani::any(),
        action_payloads: kani::any(),
    };
    kani::assert(a.union(b) == b.union(a), "production union is commutative");
}

#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_unsupported_union_idempotent() {
    let a = UnsupportedRecoveryState {
        slot_values: kani::any(),
        slot_taint: kani::any(),
        action_payloads: kani::any(),
    };
    kani::assert(a.union(a) == a, "production union is idempotent");
}

#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_unsupported_union_supported_identity() {
    let a = UnsupportedRecoveryState {
        slot_values: kani::any(),
        slot_taint: kani::any(),
        action_payloads: kani::any(),
    };
    kani::assert(
        a.union(UnsupportedRecoveryState::SUPPORTED) == a,
        "SUPPORTED right identity",
    );
    kani::assert(
        UnsupportedRecoveryState::SUPPORTED.union(a) == a,
        "SUPPORTED left identity",
    );
}

// PO-030: UnsupportedRecoveryState::is_fully_supported — calls PRODUCTION

#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_is_fully_supported_supported_constant() {
    kani::assert(
        UnsupportedRecoveryState::SUPPORTED.is_fully_supported(),
        "production SUPPORTED.is_fully_supported() == true",
    );
}

#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_is_fully_supported_all_states() {
    let state = UnsupportedRecoveryState {
        slot_values: kani::any(),
        slot_taint: kani::any(),
        action_payloads: kani::any(),
    };
    let result = state.is_fully_supported();
    let expected = !state.slot_values && !state.slot_taint && !state.action_payloads;
    kani::assert(
        result == expected,
        "production is_fully_supported matches all-false check",
    );
}

#[kani::proof]
#[kani::unwind(4)]
fn vbjpq733_is_fully_supported_each_flag() {
    let s = UnsupportedRecoveryState::SUPPORTED;
    kani::assert(
        !UnsupportedRecoveryState {
            slot_values: true,
            ..s
        }
        .is_fully_supported(),
        "slot_values blocks",
    );
    kani::assert(
        !UnsupportedRecoveryState {
            slot_taint: true,
            ..s
        }
        .is_fully_supported(),
        "slot_taint blocks",
    );
    kani::assert(
        !UnsupportedRecoveryState {
            action_payloads: true,
            ..s
        }
        .is_fully_supported(),
        "action_payloads blocks",
    );
    kani::assert(
        !UnsupportedRecoveryState {
            action_payloads: true,
            ..s
        }
        .is_fully_supported(),
        "action_payloads blocks (duplicate check)",
    );
}
