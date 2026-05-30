#![cfg(kani)]
#![forbid(unsafe_code)]

//! vb-jpq7.33 PO-001 REPAIRED: join_taint lattice laws — calls production code.
//!
//! GOD RULE 2 FIX: Calls `vb_core::value::join_taint` (production) directly.
//! Uses production `kani::Arbitrary for Taint` from `kani_workflow_arbitrary.rs`.
//! No local model redefinitions.
//!
//! LF-5 FIX: Removed duplicate `impl kani::Arbitrary for Taint` — uses the
//! canonical impl from `crate::kani_workflow_arbitrary`.

use crate::value::{Taint, join_taint};

fn taint_discriminant(t: Taint) -> u8 {
    match t {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
        Taint::Random => 3,
        Taint::TimeDependent => 4,
    }
}

/// PO-001 H1: commutativity — join_taint(a, b) == join_taint(b, a)
#[kani::proof]
#[kani::unwind(4)]
fn join_taint_commutative_5var() {
    let a: Taint = kani::any();
    let b: Taint = kani::any();
    let ab = join_taint(a, b);
    let ba = join_taint(b, a);
    kani::assert(ab == ba, "join_taint must be commutative");
}

/// PO-001 H2: associativity — join_taint(join_taint(a,b), c) == join_taint(a, join_taint(b,c))
#[kani::proof]
#[kani::unwind(4)]
fn join_taint_associative_5var() {
    let a: Taint = kani::any();
    let b: Taint = kani::any();
    let c: Taint = kani::any();
    let ab_c = join_taint(join_taint(a, b), c);
    let a_bc = join_taint(a, join_taint(b, c));
    kani::assert(ab_c == a_bc, "join_taint must be associative");
}

/// PO-001 H3: idempotence — join_taint(a, a) == a
#[kani::proof]
#[kani::unwind(4)]
fn join_taint_idempotent_5var() {
    let a: Taint = kani::any();
    let result = join_taint(a, a);
    kani::assert(result == a, "join_taint must be idempotent");
}

/// PO-001 H4: Clean is identity element
#[kani::proof]
#[kani::unwind(4)]
fn join_taint_clean_identity_5var() {
    let a: Taint = kani::any();
    kani::assert(join_taint(a, Taint::Clean) == a, "Clean must be identity (right)");
    kani::assert(join_taint(Taint::Clean, a) == a, "Clean must be identity (left)");
}

/// PO-001 H5: Monotonicity — discriminant never decreases
#[kani::proof]
#[kani::unwind(4)]
fn join_taint_monotonic_5var() {
    let a: Taint = kani::any();
    let b: Taint = kani::any();
    let result = join_taint(a, b);
    let disc_a = taint_discriminant(a);
    let disc_b = taint_discriminant(b);
    let disc_r = taint_discriminant(result);
    kani::assert(disc_r >= disc_a, "join(a,b).disc >= a.disc");
    kani::assert(disc_r >= disc_b, "join(a,b).disc >= b.disc");
}

/// PO-003 H1: join_taint(Random, Secret) == Random
#[kani::proof]
#[kani::unwind(4)]
fn join_taint_random_secret_interaction() {
    kani::assert(
        join_taint(Taint::Random, Taint::Secret) == Taint::Random,
        "Random (d=3) > Secret (d=2)",
    );
}

/// PO-003 H2: TimeDependent is top element
#[kani::proof]
#[kani::unwind(4)]
fn join_taint_time_dependent_top() {
    let a: Taint = kani::any();
    kani::assert(
        join_taint(a, Taint::TimeDependent) == Taint::TimeDependent,
        "TimeDependent absorbs all",
    );
}
