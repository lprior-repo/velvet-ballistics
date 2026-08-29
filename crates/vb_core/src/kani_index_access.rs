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
    kani::cover!(slot_count == 1, "slot_count covers single-slot frame");
    kani::cover!(slot_count >= 10, "slot_count covers larger frames");

    let slot_raw: u16 = kani::any();
    kani::assume(slot_raw < slot_count);
    kani::cover!(slot_raw == 0, "slot_raw covers zero index");
    kani::cover!(slot_raw == slot_count - 1, "slot_raw covers last valid index");
    let slot = SlotIdx::new(slot_raw);

    let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
    kani::assume(frame.is_ok());
    kani::cover!(frame.is_ok(), "RunFrame::new succeeds with valid dimensions");
    let mut frame = frame.unwrap();

    let result = frame.write_slot(slot, SlotValue::Null);
    kani::cover!(result.is_ok(), "write_slot returns Ok for in-bounds index");
    kani::assert(result.is_ok(), "write_slot with valid idx returns Ok");
}

/// VB-CORE-IDX-001 H2: read_slot with in-bounds index succeeds after write
#[kani::proof]
fn kani_read_slot_in_bounds() {
    let slot_count: u16 = kani::any();
    kani::assume(slot_count > 0);
    kani::cover!(slot_count == 1, "slot_count covers single-slot frame");
    kani::cover!(slot_count >= 10, "slot_count covers larger frames");

    let slot_raw: u16 = kani::any();
    kani::assume(slot_raw < slot_count);
    kani::cover!(slot_raw == 0, "slot_raw covers zero index");
    kani::cover!(slot_raw == slot_count - 1, "slot_raw covers last valid index");
    let slot = SlotIdx::new(slot_raw);

    let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
    kani::assume(frame.is_ok());
    kani::cover!(frame.is_ok(), "RunFrame::new succeeds with valid dimensions");
    let mut frame = frame.unwrap();

    let write_result = frame.write_slot(slot, SlotValue::I64(42));
    kani::assume(write_result.is_ok());
    kani::cover!(write_result.is_ok(), "write_slot(I64(42)) succeeded");

    let read_result = frame.read_slot(slot);
    kani::cover!(read_result.is_ok(), "read_slot returns Ok for in-bounds index");
    kani::assert(read_result.is_ok(), "read_slot with valid idx returns Ok");
    if let Ok(val) = read_result {
        kani::assert(
            matches!(val, SlotValue::I64(42)),
            "read_slot returns written value",
        );
    }
}

/// VB-CORE-IDX-001 H3: write_slot with out-of-bounds index returns error
#[kani::proof]
fn kani_write_slot_out_of_bounds() {
    let slot_count: u16 = kani::any();
    kani::assume(slot_count > 0);
    kani::assume(slot_count < u16::MAX);
    kani::cover!(slot_count == 1, "OOB test with single-slot frame");
    kani::cover!(slot_count >= 10, "OOB test with larger frame");

    let slot_raw: u16 = kani::any();
    kani::assume(slot_raw >= slot_count);
    kani::cover!(slot_raw == slot_count, "slot_raw covers exact-count boundary");
    kani::cover!(slot_raw == u16::MAX, "slot_raw covers maximum u16");
    let slot = SlotIdx::new(slot_raw);

    let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
    kani::assume(frame.is_ok());
    kani::cover!(frame.is_ok(), "RunFrame::new succeeds for OOB test frame");
    let mut frame = frame.unwrap();

    let result = frame.write_slot(slot, SlotValue::Null);
    kani::cover!(result.is_err(), "write_slot returns Err for out-of-bounds index");
    kani::assert(result.is_err(), "write_slot with OOB idx returns Err");
}

/// VB-CORE-IDX-001 H4: read_slot with out-of-bounds index returns error
#[kani::proof]
fn kani_read_slot_out_of_bounds() {
    let slot_count: u16 = kani::any();
    kani::assume(slot_count > 0);
    kani::assume(slot_count < u16::MAX);
    kani::cover!(slot_count == 1, "OOB read test with single-slot frame");
    kani::cover!(slot_count >= 10, "OOB read test with larger frame");

    let slot_raw: u16 = kani::any();
    kani::assume(slot_raw >= slot_count);
    kani::cover!(slot_raw == slot_count, "slot_raw covers exact-count boundary");
    kani::cover!(slot_raw == u16::MAX, "slot_raw covers maximum u16 for OOB read");
    let slot = SlotIdx::new(slot_raw);

    let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
    kani::assume(frame.is_ok());
    kani::cover!(frame.is_ok(), "RunFrame::new succeeds for OOB read test frame");
    let frame = frame.unwrap();

    let result = frame.read_slot(slot);
    kani::cover!(result.is_err(), "read_slot returns Err for out-of-bounds index");
    kani::assert(result.is_err(), "read_slot with OOB idx returns Err");
}

/// VB-CORE-IDX-001 H5: multiple slots written and read sequentially
#[kani::proof]
#[kani::unwind(17)]
fn kani_multiple_slots_sequential() {
    let slot_count: u16 = kani::any();
    kani::assume(slot_count >= 2);
    kani::assume(slot_count <= 16);
    kani::cover!(slot_count == 2, "sequential test with minimum 2 slots");
    kani::cover!(slot_count == 16, "sequential test with maximum 16 slots");
    kani::cover!(slot_count == 8, "sequential test with mid-range 8 slots");

    let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
    kani::assume(frame.is_ok());
    kani::cover!(frame.is_ok(), "RunFrame::new succeeds for sequential test");
    let mut frame = frame.unwrap();

    for i in 0..slot_count {
        let slot = SlotIdx::new(i);
        let value = SlotValue::I64(i as i64);
        let write_result = frame.write_slot(slot, value);
        kani::cover!(write_result.is_ok(), "write_slot succeeds in sequential loop");
        kani::assert(write_result.is_ok(), "write_slot succeeds for slot");
    }

    for i in 0..slot_count {
        let slot = SlotIdx::new(i);
        let read_result = frame.read_slot(slot);
        kani::cover!(read_result.is_ok(), "read_slot succeeds in sequential loop");
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
