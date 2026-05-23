#![forbid(unsafe_code)]
//! VB-CORE-TAINT-006-KANI: Taint propagation verification
//!
//! Property: Taint tracking via `join_taint` is monotonic (join >= both args),
//! idempotent, commutative, and never panics on valid slot indices.
//!
//! This harness verifies taint propagation invariants in `crate::value::join_taint`
//! and frame taint read/write operations.

use crate::frame::RunFrame;
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint, join_taint};

fn taint_from_u8(v: u8) -> Taint {
    match v % 5 {
        0 => Taint::Clean,
        1 => Taint::DerivedFromSecret,
        2 => Taint::Secret,
        3 => Taint::Random,
        _ => Taint::TimeDependent,
    }
}

fn taint_discriminant(t: Taint) -> u8 {
    match t {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
        Taint::Random => 3,
        Taint::TimeDependent => 4,
    }
}

fn taint_lte(a: Taint, b: Taint) -> bool {
    taint_discriminant(a) <= taint_discriminant(b)
}

/// VB-CORE-TAINT-006-KANI H1: join_taint(a, b) >= a (monotonic, first arg)
#[kani::proof]
#[kani::unwind(4)]
fn kani_join_taint_ge_first_arg() {
    let a_raw = kani::any::<u8>();
    let b_raw = kani::any::<u8>();
    let a = taint_from_u8(a_raw);
    let b = taint_from_u8(b_raw);
    let result = join_taint(a, b);
    kani::assert(taint_lte(a, result), "join_taint(a, b) >= a");
}

/// VB-CORE-TAINT-006-KANI H2: join_taint(a, b) >= b (monotonic, second arg)
#[kani::proof]
#[kani::unwind(4)]
fn kani_join_taint_ge_second_arg() {
    let a_raw = kani::any::<u8>();
    let b_raw = kani::any::<u8>();
    let a = taint_from_u8(a_raw);
    let b = taint_from_u8(b_raw);
    let result = join_taint(a, b);
    kani::assert(taint_lte(b, result), "join_taint(a, b) >= b");
}

/// VB-CORE-TAINT-006-KANI H3: join_taint is idempotent
#[kani::proof]
#[kani::unwind(4)]
fn kani_join_taint_idempotent() {
    let a_raw = kani::any::<u8>();
    let a = taint_from_u8(a_raw);
    let result = join_taint(a, a);
    kani::assert(result == a, "join_taint(a, a) == a");
}

/// VB-CORE-TAINT-006-KANI H4: join_taint is commutative
#[kani::proof]
#[kani::unwind(4)]
fn kani_join_taint_commutative() {
    let a_raw = kani::any::<u8>();
    let b_raw = kani::any::<u8>();
    let a = taint_from_u8(a_raw);
    let b = taint_from_u8(b_raw);
    let result_ab = join_taint(a, b);
    let result_ba = join_taint(b, a);
    kani::assert(
        result_ab == result_ba,
        "join_taint(a, b) == join_taint(b, a)",
    );
}

/// VB-CORE-TAINT-006-KANI H5: read_taint with valid slot returns Ok
#[kani::proof]
#[kani::unwind(4)]
fn kani_read_taint_no_panic() {
    let slot_count: u16 = kani::any();
    // Bound: frame must have at least one slot
    kani::assume(slot_count > 0);
    let slot_raw: u16 = kani::any();
    kani::assume(slot_raw < slot_count);
    let slot = SlotIdx::new(slot_raw);

    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
    kani::assume(frame.is_ok());
    let mut frame = frame.unwrap();

    let init_result = frame.write_slot(slot, SlotValue::Null);
    kani::assume(init_result.is_ok());

    let result = frame.read_taint(slot);
    kani::assert(result.is_ok(), "read_taint with valid idx returns Ok");
}

/// VB-CORE-TAINT-006-KANI H6: write_taint with valid slot returns Ok
#[kani::proof]
#[kani::unwind(4)]
fn kani_write_taint_no_panic() {
    let slot_count: u16 = kani::any();
    // Bound: frame must have at least one slot
    kani::assume(slot_count > 0);
    let slot_raw: u16 = kani::any();
    kani::assume(slot_raw < slot_count);
    let slot = SlotIdx::new(slot_raw);
    let taint_raw = kani::any::<u8>();
    let taint = taint_from_u8(taint_raw);

    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
    kani::assume(frame.is_ok());
    let mut frame = frame.unwrap();

    let init_result = frame.write_slot(slot, SlotValue::Null);
    kani::assume(init_result.is_ok());

    let result = frame.write_taint(slot, taint);
    kani::assert(result.is_ok(), "write_taint with valid idx returns Ok");
}

/// VB-CORE-TAINT-006-KANI H7: join_taint is associative
#[kani::proof]
#[kani::unwind(4)]
fn kani_join_taint_associative() {
    let a_raw = kani::any::<u8>();
    let b_raw = kani::any::<u8>();
    let c_raw = kani::any::<u8>();
    let a = taint_from_u8(a_raw);
    let b = taint_from_u8(b_raw);
    let c = taint_from_u8(c_raw);
    let ab_c = join_taint(join_taint(a, b), c);
    let a_bc = join_taint(a, join_taint(b, c));
    kani::assert(
        ab_c == a_bc,
        "join_taint(join_taint(a, b), c) == join_taint(a, join_taint(b, c))",
    );
}

/// VB-CORE-TAINT-006-KANI H8: taint lattice order is transitive
#[kani::proof]
#[kani::unwind(4)]
fn kani_taint_lattice_transitive() {
    let a_raw = kani::any::<u8>();
    let b_raw = kani::any::<u8>();
    let c_raw = kani::any::<u8>();
    let a = taint_from_u8(a_raw);
    let b = taint_from_u8(b_raw);
    let c = taint_from_u8(c_raw);
    let ab = join_taint(a, b);
    let bc = join_taint(b, c);
    // If a <= b (i.e., join(a,b) == b) and b <= c (i.e., join(b,c) == c),
    // then a <= c (i.e., join(a,c) == c)
    let a_lte_b = ab == b;
    let b_lte_c = bc == c;
    kani::assume(a_lte_b && b_lte_c);
    let ac = join_taint(a, c);
    kani::assert(ac == c, "transitivity: if a <= b and b <= c then join(a,c) == c");
}

/// VB-CORE-TAINT-006-KANI H9: join_taint never returns a result lower than either input
#[kani::proof]
#[kani::unwind(4)]
fn kani_join_taint_monotonic() {
    let a_raw = kani::any::<u8>();
    let b_raw = kani::any::<u8>();
    let a = taint_from_u8(a_raw);
    let b = taint_from_u8(b_raw);
    let result = join_taint(a, b);
    // Because join_taint returns the taint with the higher discriminant,
    // result must be >= a and >= b in the lattice ordering.
    let disc_a = taint_discriminant(a);
    let disc_b = taint_discriminant(b);
    let disc_r = taint_discriminant(result);
    kani::assert(disc_r >= disc_a, "join(a,b).disc >= a.disc");
    kani::assert(disc_r >= disc_b, "join(a,b).disc >= b.disc");
}

/// VB-CORE-TAINT-006-KANI H10: join_taint lattice top absorption
#[kani::proof]
#[kani::unwind(4)]
fn kani_time_dependent_is_lattice_top() {
    let a_raw = kani::any::<u8>();
    let a = taint_from_u8(a_raw);
    let result = join_taint(a, Taint::TimeDependent);
    kani::assert(
        result == Taint::TimeDependent,
        "TimeDependent absorbs all taint levels",
    );
}

/// VB-CORE-TAINT-006-KANI H11: join_taint lattice bottom identity
#[kani::proof]
#[kani::unwind(4)]
fn kani_clean_is_lattice_bottom() {
    let a_raw = kani::any::<u8>();
    let a = taint_from_u8(a_raw);
    let result = join_taint(a, Taint::Clean);
    kani::assert(
        result == a,
        "Clean is identity for join_taint",
    );
}

/// VB-CORE-TAINT-006-KANI H12: join_taint result ranks Random below TimeDependent
#[kani::proof]
#[kani::unwind(4)]
fn kani_random_below_time_dependent() {
    let a_raw = kani::any::<u8>();
    let a = taint_from_u8(a_raw);
    let result = join_taint(Taint::Random, a);
    // If a is TimeDependent, result is TimeDependent
    // In all other cases, Random >= a means result is Random
    // So result is either Random or TimeDependent, never lower
    kani::assert(
        result == Taint::Random || result == Taint::TimeDependent,
        "join(Random, a) is Random or TimeDependent",
    );
}
