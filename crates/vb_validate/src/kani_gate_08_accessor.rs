#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for Gate 8 accessor validation.
//!
//! PO-014 support repair: this cfg(kani)-only artifact must compile when the
//! vb_compile idempotency parity harness is selected, because Cargo compiles the
//! dependent vb_validate crate under cfg(kani) first.

use crate::kani_gate_08_support::workflow_parts_with_accessors;
use crate::{ValidationError, gates::validate_gate_08_accessor_path_segments};
use vb_core::ids::{SlotIdx, SymbolId};
use vb_core::workflow::{AccessorProgram, PathSegment, WorkflowParts};

#[kani::proof]
#[kani::unwind(18)]
fn kani_gate_08_valid_bounded_parts_pass() {
    let parts = bounded_valid_accessor_parts();
    assume_gate_08_harness_bounds(&parts);

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(result.is_ok(), "bounded valid accessors pass Gate 8");
    std::mem::forget(parts);
}

#[kani::proof]
fn kani_gate_08_valid_zero_accessors_pass() {
    let parts = workflow_parts_with_accessors(Box::new([]), 0, 0);

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(result.is_ok(), "zero accessors pass Gate 8");
    std::mem::forget(parts);
}

#[kani::proof]
fn kani_gate_08_valid_index_without_symbols_pass() {
    let index: u32 = kani::any();
    kani::assume(index != u32::MAX);

    let parts = workflow_parts_with_accessors(
        Box::new([AccessorProgram {
            root: SlotIdx::ZERO,
            path: Box::new([PathSegment::Index(index)]),
        }]),
        1,
        0,
    );

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(
        result.is_ok(),
        "index-only accessor does not require symbols",
    );
    std::mem::forget(parts);
}

#[kani::proof]
#[kani::unwind(18)]
fn kani_gate_08_no_panic_bounded_inputs() {
    let parts = bounded_gate_08_parts_for_no_panic();
    assume_gate_08_harness_bounds(&parts);

    kani::cover(parts.accessors.len() == 1, "one accessor covered");
    kani::cover(parts.slot_count == 0, "zero slot count covered");
    kani::cover(parts.symbols_count == 0, "zero symbols count covered");

    let _result = validate_gate_08_accessor_path_segments(&parts);
    std::mem::forget(parts);
}

#[kani::proof]
fn kani_gate_08_field_symbol_oob_rejected() {
    let symbols_count = bounded_u32_1024();
    let delta = bounded_u32_4();
    let field_id = match symbols_count.checked_add(delta) {
        Some(value) => value,
        None => symbols_count,
    };

    let parts = workflow_parts_with_accessors(
        Box::new([AccessorProgram {
            root: SlotIdx::ZERO,
            path: Box::new([PathSegment::Field(SymbolId::new(field_id))]),
        }]),
        1,
        symbols_count,
    );

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(
        matches!(
            result,
            Err(ValidationError::AccessorSymbolOutOfBounds { .. })
        ),
        "field symbol >= symbols_count is rejected",
    );
    std::mem::forget(parts);
}

#[kani::proof]
fn kani_gate_08_index_u32_max_rejected() {
    let parts = workflow_parts_with_accessors(
        Box::new([AccessorProgram {
            root: SlotIdx::ZERO,
            path: Box::new([PathSegment::Index(u32::MAX)]),
        }]),
        1,
        0,
    );

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(
        matches!(result, Err(ValidationError::AccessorPathInvalid { .. })),
        "u32::MAX index sentinel is rejected",
    );
    std::mem::forget(parts);
}

#[kani::proof]
fn kani_gate_08_path_too_deep_rejected() {
    // Construct an accessor with path length = 17 (exceeds MAX_ACCESSOR_PATH_DEPTH = 16)
    let mut path: Vec<PathSegment> = Vec::with_capacity(17);
    let mut i = 0u32;
    while i < 17 {
        path.push(PathSegment::Field(SymbolId::new(i % 100)));
        i = match i.checked_add(1) {
            Some(n) => n,
            None => break,
        };
    }

    let parts = workflow_parts_with_accessors(
        Box::new([AccessorProgram {
            root: SlotIdx::ZERO,
            path: path.into_boxed_slice(),
        }]),
        1,
        100,
    );

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(
        matches!(result, Err(ValidationError::AccessorPathTooDeep { .. })),
        "accessor path depth exceeding 16 is rejected",
    );
    std::mem::forget(parts);
}

#[kani::proof]
fn kani_gate_08_root_oob_rejected() {
    let slot_count: u16 = kani::any();
    let root: u16 = kani::any();
    kani::assume(slot_count <= 8);
    kani::assume(root >= slot_count);
    kani::assume(root <= 10);

    let parts = workflow_parts_with_accessors(
        Box::new([AccessorProgram {
            root: SlotIdx::new(root),
            path: Box::new([]),
        }]),
        slot_count,
        0,
    );

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(
        matches!(result, Err(ValidationError::AccessorSlotOutOfRange { .. })),
        "root >= slot_count is rejected",
    );
    std::mem::forget(parts);
}

fn bounded_u32_1024() -> u32 {
    let value: u32 = kani::any();
    kani::assume(value <= 1024);
    value
}

fn bounded_u32_4() -> u32 {
    let value: u32 = kani::any();
    kani::assume(value <= 4);
    value
}

fn bounded_valid_accessor_parts() -> WorkflowParts {
    let slot_count = bounded_nonzero_u16_256();
    let symbols_count = bounded_nonzero_u32_1024();
    let root = bounded_u16_below(slot_count);
    let symbol = bounded_u32_below(symbols_count);
    let index = valid_index_segment();

    kani::cover(slot_count == 1, "minimum nonzero slot count covered");
    kani::cover(symbols_count == 1, "minimum nonzero symbols count covered");
    kani::cover(root == 0, "root lower boundary covered");
    kani::cover(symbol == 0, "symbol lower boundary covered");
    kani::cover(index == 0, "index lower boundary covered");

    let accessors: Box<[AccessorProgram]> = Box::new([AccessorProgram {
        root: SlotIdx::new(root),
        path: Box::new([
            PathSegment::Field(SymbolId::new(symbol)),
            PathSegment::Index(index),
        ]),
    }]);

    workflow_parts_with_accessors(accessors, slot_count, symbols_count)
}

fn bounded_gate_08_parts_for_no_panic() -> WorkflowParts {
    let slot_count = bounded_u16_256();
    let symbols_count = bounded_u32_1024();
    let root = bounded_u16_257();
    let symbol = bounded_u32_1025();
    let index: u32 = kani::any();

    kani::cover(root >= slot_count, "root out-of-range covered");
    kani::cover(symbol >= symbols_count, "symbol out-of-range covered");
    kani::cover(index == u32::MAX, "index sentinel covered");

    let accessors: Box<[AccessorProgram]> = Box::new([AccessorProgram {
        root: SlotIdx::new(root),
        path: Box::new([
            PathSegment::Field(SymbolId::new(symbol)),
            PathSegment::Index(index),
        ]),
    }]);

    workflow_parts_with_accessors(accessors, slot_count, symbols_count)
}

fn bounded_nonzero_u16_256() -> u16 {
    let value: u16 = kani::any();
    kani::assume(value > 0);
    kani::assume(value <= 256);
    value
}

fn bounded_u16_256() -> u16 {
    let value: u16 = kani::any();
    kani::assume(value <= 256);
    value
}

fn bounded_u16_257() -> u16 {
    let value: u16 = kani::any();
    kani::assume(value <= 257);
    value
}

fn bounded_nonzero_u32_1024() -> u32 {
    let value: u32 = kani::any();
    kani::assume(value > 0);
    kani::assume(value <= 1024);
    value
}

fn bounded_u32_1025() -> u32 {
    let value: u32 = kani::any();
    kani::assume(value <= 1025);
    value
}

fn bounded_u16_below(exclusive_upper: u16) -> u16 {
    let value: u16 = kani::any();
    kani::assume(value < exclusive_upper);
    value
}

fn bounded_u32_below(exclusive_upper: u32) -> u32 {
    let value: u32 = kani::any();
    kani::assume(value < exclusive_upper);
    value
}

fn valid_index_segment() -> u32 {
    let index: u32 = kani::any();
    kani::assume(index != u32::MAX);
    index
}

fn assume_gate_08_harness_bounds(parts: &WorkflowParts) {
    kani::assume(parts.accessors.len() <= 2);
    if let Some(accessor) = parts.accessors.first() {
        kani::assume(accessor.path.len() <= 2);
    }
    if let Some(accessor) = parts.accessors.get(1) {
        kani::assume(accessor.path.len() <= 2);
    }
}
