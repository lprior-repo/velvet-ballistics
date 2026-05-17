#![forbid(unsafe_code)]
//! VB-CORE-IDX-001: Index access bounds verification
//!
//! Property: Slot/Step index access with u16 values is always bounds-checked
//! and never panics on valid indices.
//!
//! This harness verifies safe index access patterns.

use crate::frame::RunFrame;
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::SlotValue;

/// VB-CORE-IDX-001 H1: write_slot with in-bounds index succeeds
#[kani::proof]
fn kani_write_slot_in_bounds() {
    let slot_count: u16 = kani::any();
    kani::assume(slot_count > 0);

    let slot_raw: u16 = kani::any();
    kani::assume(slot_raw < slot_count);
    let slot = SlotIdx::new(slot_raw);

    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
    kani::assume(frame.is_ok());
    let mut frame = frame.unwrap();

    let result = frame.write_slot(slot, SlotValue::Null);
    kani::assert(result.is_ok(), "write_slot with valid idx returns Ok");
}

/// VB-CORE-IDX-001 H2: read_slot with in-bounds index succeeds after write
#[kani::proof]
fn kani_read_slot_in_bounds() {
    let slot_count: u16 = kani::any();
    kani::assume(slot_count > 0);

    let slot_raw: u16 = kani::any();
    kani::assume(slot_raw < slot_count);
    let slot = SlotIdx::new(slot_raw);

    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
    kani::assume(frame.is_ok());
    let mut frame = frame.unwrap();

    let write_result = frame.write_slot(slot, SlotValue::I64(42));
    kani::assume(write_result.is_ok());

    let read_result = frame.read_slot(slot);
    kani::assert(read_result.is_ok(), "read_slot with valid idx returns Ok");
    if let Ok(val) = read_result {
        kani::assert(matches!(val, SlotValue::I64(42)), "read_slot returns written value");
    }
}

/// VB-CORE-IDX-001 H3: write_slot with out-of-bounds index returns error
#[kani::proof]
fn kani_write_slot_out_of_bounds() {
    let slot_count: u16 = kani::any();
    kani::assume(slot_count > 0);
    kani::assume(slot_count < u16::MAX);

    let slot_raw: u16 = kani::any();
    kani::assume(slot_raw >= slot_count);
    let slot = SlotIdx::new(slot_raw);

    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
    kani::assume(frame.is_ok());
    let mut frame = frame.unwrap();

    let result = frame.write_slot(slot, SlotValue::Null);
    kani::assert(result.is_err(), "write_slot with OOB idx returns Err");
}

/// VB-CORE-IDX-001 H4: read_slot with out-of-bounds index returns error
#[kani::proof]
fn kani_read_slot_out_of_bounds() {
    let slot_count: u16 = kani::any();
    kani::assume(slot_count > 0);
    kani::assume(slot_count < u16::MAX);

    let slot_raw: u16 = kani::any();
    kani::assume(slot_raw >= slot_count);
    let slot = SlotIdx::new(slot_raw);

    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
    kani::assume(frame.is_ok());
    let mut frame = frame.unwrap();

    let result = frame.read_slot(slot);
    kani::assert(result.is_err(), "read_slot with OOB idx returns Err");
}

/// VB-CORE-IDX-001 H5: multiple slots written and read sequentially
#[kani::proof]
#[kani::unwind(17)]
fn kani_multiple_slots_sequential() {
    let slot_count: u16 = kani::any();
    kani::assume(slot_count >= 2);
    kani::assume(slot_count <= 16);

    let mut frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
    kani::assume(frame.is_ok());
    let mut frame = frame.unwrap();

    for i in 0..slot_count {
        let slot = SlotIdx::new(i);
        let value = SlotValue::I64(i as i64);
        let write_result = frame.write_slot(slot, value);
        kani::assert(write_result.is_ok(), "write_slot succeeds for slot");
    }

    for i in 0..slot_count {
        let slot = SlotIdx::new(i);
        let read_result = frame.read_slot(slot);
        kani::assert(read_result.is_ok(), "read_slot succeeds for slot");
    }
}

/// VB-CORE-IDX-001 H6: StepIdx::new with valid value
#[kani::proof]
fn kani_step_idx_valid() {
    let raw: u16 = kani::any();
    let idx = StepIdx::new(raw);
    kani::assert(idx.get() == raw, "StepIdx::new preserves value");
}

/// VB-CORE-IDX-001 H7: SlotIdx::new with valid value
#[kani::proof]
fn kani_slot_idx_valid() {
    let raw: u16 = kani::any();
    let idx = SlotIdx::new(raw);
    kani::assert(idx.get() == raw, "SlotIdx::new preserves value");
}
