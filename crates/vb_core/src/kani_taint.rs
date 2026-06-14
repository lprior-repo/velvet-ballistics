#![cfg(kani)]

use crate::frame::RunFrame;
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint, join_taint};

fn taint_from_u8(v: u8) -> Taint {
    match v % 3 {
        0 => Taint::Clean,
        1 => Taint::DerivedFromSecret,
        _ => Taint::Secret,
    }
}

fn taint_discriminant(t: Taint) -> u8 {
    match t {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
    }
}

fn taint_lte(a: Taint, b: Taint) -> bool {
    taint_discriminant(a) <= taint_discriminant(b)
}

#[kani::proof]
fn join_taint_ge_first_arg() {
    let a_raw = kani::any::<u8>();
    let b_raw = kani::any::<u8>();
    let a = taint_from_u8(a_raw);
    let b = taint_from_u8(b_raw);
    let result = join_taint(a, b);
    kani::assert(taint_lte(a, result), "join_taint(a, b) >= a");
}

#[kani::proof]
fn join_taint_ge_second_arg() {
    let a_raw = kani::any::<u8>();
    let b_raw = kani::any::<u8>();
    let a = taint_from_u8(a_raw);
    let b = taint_from_u8(b_raw);
    let result = join_taint(a, b);
    kani::assert(taint_lte(b, result), "join_taint(a, b) >= b");
}

#[kani::proof]
fn join_taint_idempotent() {
    let a_raw = kani::any::<u8>();
    let a = taint_from_u8(a_raw);
    let result = join_taint(a, a);
    kani::assert(result == a, "join_taint(a, a) == a");
}

#[kani::proof]
fn join_taint_commutative() {
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

#[kani::proof]
fn read_taint_no_panic() {
    let slot_count: u16 = kani::any();
    kani::assume(slot_count > 0);

    let slot_raw: u16 = kani::any();
    kani::assume(slot_raw < slot_count);
    let slot = SlotIdx::new(slot_raw);

    let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
    kani::assume(frame.is_ok());
    let mut frame = match frame {
        Ok(v) => v,
        Err(_) => { kani::assume(false); return; }
    };

    let init_result = frame.write_slot(slot, SlotValue::Null);
    kani::assume(init_result.is_ok());

    let result = frame.read_taint(slot);
    kani::assert(result.is_ok(), "read_taint with valid idx returns Ok");
}

#[kani::proof]
fn write_taint_no_panic() {
    let slot_count: u16 = kani::any();
    kani::assume(slot_count > 0);

    let slot_raw: u16 = kani::any();
    kani::assume(slot_raw < slot_count);
    let slot = SlotIdx::new(slot_raw);

    let taint_raw = kani::any::<u8>();
    let taint = taint_from_u8(taint_raw);

    let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
    kani::assume(frame.is_ok());
    let mut frame = match frame {
        Ok(v) => v,
        Err(_) => { kani::assume(false); return; }
    };

    let init_result = frame.write_slot(slot, SlotValue::Null);
    kani::assume(init_result.is_ok());

    let result = frame.write_taint(slot, taint);
    kani::assert(result.is_ok(), "write_taint with valid idx returns Ok");
}
