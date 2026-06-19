#![cfg(all(kani, feature = "kani-vb-god2f-proof-kernels"))]
#![forbid(unsafe_code)]

//! HVR-PO-CORE-001: production `join_taint` lattice replacement harness.

use crate::{Taint, join_taint};

fn taint_from_symbol(symbol: u8) -> Taint {
    match symbol % 3 {
        0 => Taint::Clean,
        1 => Taint::DerivedFromSecret,
        _ => Taint::Secret,
    }
}

fn taint_rank(taint: Taint) -> u8 {
    match taint {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
    }
}

fn expected_join(left: Taint, right: Taint) -> Taint {
    if taint_rank(left) >= taint_rank(right) {
        left
    } else {
        right
    }
}

#[kani::proof]
#[kani::unwind(8)]
fn vb_god2f_core_taint_lattice_replacement() {
    let left = taint_from_symbol(kani::any());
    let right = taint_from_symbol(kani::any());
    let third = taint_from_symbol(kani::any());

    kani::cover!(left == Taint::Clean, "taint domain covers Clean");
    kani::cover!(
        left == Taint::DerivedFromSecret,
        "taint domain covers DerivedFromSecret"
    );
    kani::cover!(left == Taint::Secret, "taint domain covers Secret");

    kani::assert(
        join_taint(left, right) == expected_join(left, right),
        "production join_taint returns independent max-rank expectation",
    );
    kani::assert(
        join_taint(left, right) == join_taint(right, left),
        "production join_taint is commutative",
    );
    kani::assert(
        join_taint(join_taint(left, right), third) == join_taint(left, join_taint(right, third)),
        "production join_taint is associative",
    );
    kani::assert(
        join_taint(left, left) == left,
        "production join_taint is idempotent",
    );
    kani::assert(
        join_taint(left, Taint::Clean) == left && join_taint(Taint::Clean, left) == left,
        "Clean is production join_taint identity",
    );
}
