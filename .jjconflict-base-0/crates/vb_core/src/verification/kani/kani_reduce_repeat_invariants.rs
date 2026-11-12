#![forbid(unsafe_code)]
//! Bounded Kani harnesses for Reduce/Repeat-adjacent invariants in `vb_core`.
//!
//! Scope (all inputs symbolic; no hardcoded `WorkflowParts`/`RunFrame` shapes,
//! per GOD RULE 1):
//!
//! - `SlotCell` value/taint coherence: an empty cell carries no taint and no
//!   value; a populated cell carries exactly one value and one taint.
//! - Loop-region ordering: for every validated workflow, each loop region
//!   satisfies `start < body < control < done` (Reduce/Repeat regions are
//!   produced by the same `unique_owner` pairing as ForEach/Collect), and all
//!   region indices stay within the node table.
//! - Frame stack bounds: slot reads/writes are total with respect to the slot
//!   array — valid indices succeed, out-of-bounds indices return an error
//!   instead of panicking.
//!
//! Bounds are kept small (step/slot counts `<= 4`, node tables from the
//! existing `kani_workflow_arbitrary` bound of `<= 8`) so CBMC verifies each
//! harness within seconds at `-j 1` under a memory cap.

use crate::frame::RunFrame;
use crate::ids::{RunId, SlotIdx, StepIdx, SymbolId};
use crate::value::{FiniteF64, SlotCell, SlotValue, Taint};
use crate::workflow::{CompiledNodeKind, CompiledWorkflow, WorkflowParts};

/// Bounded frame dimensions shared by the slot/taint harnesses.
/// `slot_count` is in `1..=4` so the frame state space stays tractable.
fn bounded_frame(slot_count: u16) -> RunFrame {
    let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 1, slot_count);
    match frame {
        Ok(frame) => frame,
        Err(_) => unreachable!("bounded_frame dimensions satisfy RunFrame::new"),
    }
}

/// Narrow symbolic slot value: full `kani::any::<SlotValue>()` bit width
/// (64-bit payloads plus handle ids) inflates the frame goto program past
/// residual memory caps, so frame-level harnesses draw a small symbolic
/// payload instead. All variants stay reachable via the symbolic selector.
fn small_slot_value() -> SlotValue {
    match kani::any::<u8>() % 5 {
        0 => SlotValue::Null,
        1 => SlotValue::Bool(kani::any()),
        2 => SlotValue::I64(i64::from(kani::any::<u8>())),
        3 => match FiniteF64::new(f64::from(kani::any::<u8>())) {
            Ok(value) => SlotValue::F64(value),
            Err(_) => SlotValue::Null,
        },
        _ => SlotValue::Symbol(SymbolId::new(u32::from(kani::any::<u8>()))),
    }
}

/// Narrow symbolic taint (mirrors the `taint_from_u8` generator used by the
/// existing taint harnesses).
fn small_taint() -> Taint {
    match kani::any::<u8>() % 5 {
        0 => Taint::Clean,
        1 => Taint::DerivedFromSecret,
        2 => Taint::Secret,
        3 => Taint::Random,
        _ => Taint::TimeDependent,
    }
}

/// H1: `SlotCell::empty()` is coherent — uninitialized, no value, no taint.
#[kani::proof]
#[kani::unwind(4)]
fn slotcell_empty_is_coherent() {
    let cell = SlotCell::empty();
    kani::assert(!cell.is_initialized(), "empty cell is uninitialized");
    kani::assert(cell.value_ref().is_none(), "empty cell has no value");
    kani::assert(cell.taint().is_none(), "empty cell has no taint");
    kani::cover!(true, "empty cell reachable");
}

/// H2: `SlotCell::value(v, t)` round-trips exactly one value and one taint.
#[kani::proof]
#[kani::unwind(4)]
fn slotcell_value_roundtrip() {
    let value: SlotValue = kani::any();
    let taint: Taint = kani::any();
    let cell = SlotCell::value(value, taint);
    kani::assert(cell.is_initialized(), "value cell is initialized");
    kani::assert(cell.value_ref() == Some(&value), "value round-trips");
    kani::assert(cell.taint() == Some(taint), "taint round-trips");
    kani::cover!(true, "value cell reachable");
}

/// H3: For every constructible cell, `is_initialized`, `value_ref().is_some()`
/// and `taint().is_some()` agree (value/taint coherence both directions).
#[kani::proof]
#[kani::unwind(4)]
fn slotcell_discriminator_coherence() {
    let is_value: bool = kani::any();
    let cell = if is_value {
        SlotCell::value(kani::any(), kani::any())
    } else {
        SlotCell::empty()
    };
    kani::assert(
        cell.is_initialized() == cell.value_ref().is_some(),
        "is_initialized agrees with value_ref",
    );
    kani::assert(
        cell.is_initialized() == cell.taint().is_some(),
        "is_initialized agrees with taint",
    );
    match cell {
        SlotCell::Empty => {
            kani::assert(cell.value_ref().is_none(), "empty has no value");
            kani::assert(cell.taint().is_none(), "empty has no taint");
            kani::cover!(true, "empty branch");
        }
        SlotCell::Value { value, taint } => {
            kani::assert(cell.value_ref() == Some(&value), "value agrees");
            kani::assert(cell.taint() == Some(taint), "taint agrees");
            kani::cover!(true, "value branch");
        }
    }
}

/// H4: Frame slot round-trip — write then read returns the same value and
/// taint; a fresh slot reads back as `SlotUninitialized`.
#[kani::proof]
#[kani::unwind(8)]
fn frame_slot_taint_roundtrip() {
    let slot_count: u16 = kani::any();
    kani::assume(slot_count >= 1);
    kani::assume(slot_count <= 2);
    let slot_raw: u16 = kani::any();
    kani::assume(slot_raw < slot_count);
    let slot = SlotIdx::new(slot_raw);
    let value = small_slot_value();
    let taint = small_taint();

    let mut frame = bounded_frame(slot_count);
    kani::assert(
        frame.read_slot(slot).is_err(),
        "fresh slot is uninitialized",
    );
    let write = frame.write_slot_with_taint(slot, value, taint);
    kani::assert(write.is_ok(), "write within bounds succeeds");
    kani::assert(
        frame.read_slot(slot) == Ok(&value),
        "read returns written value",
    );
    kani::assert(
        frame.read_taint(slot) == Ok(taint),
        "read returns written taint",
    );
}

/// H5: `write_taint` preserves the stored value, and the new taint sticks.
#[kani::proof]
#[kani::unwind(8)]
fn frame_write_taint_preserves_value() {
    let slot_count: u16 = kani::any();
    kani::assume(slot_count >= 1);
    kani::assume(slot_count <= 4);
    let slot_raw: u16 = kani::any();
    kani::assume(slot_raw < slot_count);
    let slot = SlotIdx::new(slot_raw);
    let value = small_slot_value();
    let new_taint = small_taint();

    let mut frame = bounded_frame(slot_count);
    kani::assume(frame.write_slot(slot, value).is_ok());
    kani::assert(
        frame.write_taint(slot, new_taint).is_ok(),
        "write_taint on initialized slot succeeds",
    );
    kani::assert(
        frame.read_slot(slot) == Ok(&value),
        "value preserved across taint rewrite",
    );
    kani::assert(frame.read_taint(slot) == Ok(new_taint), "new taint sticks");
}

/// H6: Out-of-bounds slot access returns `SlotOutOfBounds` on every accessor
/// instead of panicking (frame stack bounds).
#[kani::proof]
#[kani::unwind(8)]
fn frame_out_of_bounds_is_error() {
    let slot_count: u16 = kani::any();
    kani::assume(slot_count >= 1);
    kani::assume(slot_count <= 4);
    let slot_raw: u16 = kani::any();
    kani::assume(slot_raw >= slot_count);
    kani::assume(slot_raw < 16);
    let slot = SlotIdx::new(slot_raw);

    let frame = bounded_frame(slot_count);
    kani::assert(
        frame.read_slot(slot).is_err(),
        "read_slot out of bounds errs",
    );
    kani::assert(
        frame.read_taint(slot).is_err(),
        "read_taint out of bounds errs",
    );
    let mut frame = frame;
    kani::assert(
        frame.write_slot(slot, SlotValue::Null).is_err(),
        "write_slot out of bounds errs",
    );
    kani::assert(
        frame
            .write_slot_with_taint(slot, SlotValue::Null, Taint::Clean)
            .is_err(),
        "write_slot_with_taint out of bounds errs",
    );
    kani::assert(
        frame.write_taint(slot, Taint::Clean).is_err(),
        "write_taint out of bounds errs",
    );
    kani::cover!(true, "out-of-bounds branch");
}

/// H7: Every loop region of a validated workflow satisfies
/// `start < body < control < done`, with all indices (and `done`) inside the
/// node table. `try_from_parts` is the production constructor that populates
/// the region cache; the harness is conditional on validation succeeding.
#[kani::proof]
#[kani::unwind(8)]
fn loop_region_ordering() {
    let parts: WorkflowParts = kani::any();
    let plan = match CompiledWorkflow::try_from_parts(parts) {
        Ok(plan) => plan,
        Err(_) => return,
    };
    let node_count = plan.node_count();
    kani::assume(node_count >= 1);
    kani::assume(node_count <= 8);
    let control_raw: u16 = kani::any();
    kani::assume(control_raw < node_count);
    let control = StepIdx::new(control_raw);

    let Some(region) = plan.loop_region(control) else {
        kani::cover!(true, "non-control step");
        return;
    };
    kani::cover!(true, "loop region found");
    kani::assert(region.start() < region.body(), "ordering start<body");
    kani::assert(region.body() < region.control(), "ordering body<control");
    kani::assert(region.control() < region.done(), "ordering control<done");
    kani::assert(
        region.done().as_usize() < usize::from(node_count),
        "done within node table",
    );
    kani::assert(
        region.start().as_usize() < usize::from(node_count),
        "start within node table",
    );
}

/// H8: Loop regions are disjoint or nested (replicates the constructor's
/// `disjoint_or_nested` relation over at most four symbolic controls).
#[kani::proof]
#[kani::unwind(8)]
fn loop_regions_disjoint_or_nested() {
    let parts: WorkflowParts = kani::any();
    let plan = match CompiledWorkflow::try_from_parts(parts) {
        Ok(plan) => plan,
        Err(_) => return,
    };
    let node_count = plan.node_count();
    kani::assume(node_count <= 8);
    let controls: Vec<StepIdx> = kani::bounded_any::<Vec<StepIdx>, 4>();
    kani::assume(
        controls
            .iter()
            .all(|&c| c.as_usize() < usize::from(node_count)),
    );
    for &first in &controls {
        for &second in &controls {
            let (Some(first_region), Some(second_region)) =
                (plan.loop_region(first), plan.loop_region(second))
            else {
                continue;
            };
            if first_region.control() >= second_region.control() {
                continue;
            }
            kani::assert(
                first_region.control() < second_region.body()
                    || second_region.control() < first_region.body()
                    || (first_region.body() < second_region.body()
                        && second_region.control() < first_region.control())
                    || (second_region.body() < first_region.body()
                        && first_region.control() < second_region.control()),
                "regions are disjoint or nested",
            );
            kani::cover!(true, "region pair compared");
        }
    }
}

/// H9: Reduce/Repeat node kinds observed in a validated workflow carry body
/// and done targets inside the node table (they feed the region pairing).
#[kani::proof]
#[kani::unwind(8)]
fn reduce_repeat_kinds_observe_bounds() {
    let parts: WorkflowParts = kani::any();
    let plan = match CompiledWorkflow::try_from_parts(parts) {
        Ok(plan) => plan,
        Err(_) => return,
    };
    let node_count = plan.node_count();
    kani::assume(node_count >= 1);
    kani::assume(node_count <= 8);
    let raw: u16 = kani::any();
    kani::assume(raw < node_count);
    let step = StepIdx::new(raw);
    let Some(node) = plan.node(step) else {
        kani::assert(false, "node lookup within bounded table succeeds");
        return;
    };
    let count = usize::from(node_count);
    match &node.kind {
        CompiledNodeKind::ReduceStart { body, done, .. } => {
            kani::cover!(true, "ReduceStart observed");
            kani::assert(body.as_usize() < count, "reduce body in bounds");
            kani::assert(done.as_usize() < count, "reduce done in bounds");
        }
        CompiledNodeKind::RepeatStart { body, done, .. } => {
            kani::cover!(true, "RepeatStart observed");
            kani::assert(body.as_usize() < count, "repeat body in bounds");
            kani::assert(done.as_usize() < count, "repeat done in bounds");
        }
        _ => {}
    }
}
