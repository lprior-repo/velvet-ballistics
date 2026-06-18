#![cfg(kani)]
#![forbid(unsafe_code)]
//! VB-CORE-TAINT-006-KANI: Taint propagation verification
//!
//! Property: Taint tracking via `join_taint` obeys the minimal lattice laws
//! needed by the runtime: Clean identity, commutativity, and associativity.
//!
//! This harness verifies `crate::value::join_taint` directly with symbolic taint
//! values generated from `kani::any`, avoiding fixed dummy-only fixtures.

use crate::value::{Taint, join_taint};

fn taint_from_u8(v: u8) -> Taint {
    match v % 3 {
        0 => Taint::Clean,
        1 => Taint::DerivedFromSecret,
        _ => Taint::Secret,
    }
}

fn arbitrary_taint() -> Taint {
    taint_from_u8(kani::any::<u8>())
}

/// VB-CORE-TAINT-006-KANI H1: Clean is the two-sided identity for join_taint.
#[kani::proof]
#[kani::unwind(4)]
fn kani_join_taint_clean_identity() {
    let a = arbitrary_taint();

    kani::assert(join_taint(a, Taint::Clean) == a, "right Clean identity");
    kani::assert(join_taint(Taint::Clean, a) == a, "left Clean identity");
}

/// VB-CORE-TAINT-006-KANI H2: join_taint is commutative.
#[kani::proof]

/// VB-CORE-TAINT-006-KANI H2: join_taint is commutative.
#[kani::proof]
#[kani::unwind(4)]
fn kani_join_taint_commutative() {
    let a = arbitrary_taint();
    let b = arbitrary_taint();
    let result_ab = join_taint(a, b);
    let result_ba = join_taint(b, a);

    kani::assert(
        result_ab == result_ba,
        "join_taint(a, b) == join_taint(b, a)",
    );
}

/// VB-CORE-TAINT-006-KANI H3: join_taint is associative.
#[kani::proof]
/// VB-CORE-TAINT-006-KANI H3: join_taint is associative.
#[kani::proof]
#[kani::unwind(4)]
fn kani_join_taint_associative() {
    let a = arbitrary_taint();
    let b = arbitrary_taint();
    let c = arbitrary_taint();
    let ab_c = join_taint(join_taint(a, b), c);
    let a_bc = join_taint(a, join_taint(b, c));

    kani::assert(
        ab_c == a_bc,
        "join_taint(join_taint(a, b), c) == join_taint(a, join_taint(b, c))",
    );
}
