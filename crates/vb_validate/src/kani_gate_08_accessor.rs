#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for Gate 8 accessor validation.
//!
//! PO-014 support repair: this cfg(kani)-only artifact must compile when the
//! vb_compile idempotency parity harness is selected, because Cargo compiles the
//! dependent vb_validate crate under cfg(kani) first.

use crate::{ValidationError, gates::validate_gate_08_accessor_path_segments};
use vb_core::ids::{SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, PathSegment, ResourceContract, WorkflowParts,
};

#[kani::proof]
#[kani::unwind(5)]
fn kani_gate_08_valid_bounded_parts_pass() {
    let parts: WorkflowParts = kani::any();
    kani::assume(parts.slot_count > 0);
    kani::assume(parts.slot_count <= 256);
    kani::assume(parts.symbols_count > 0);
    kani::assume(parts.symbols_count <= 1024);

    for accessor in parts.accessors.iter() {
        kani::assume(accessor.root.get() < parts.slot_count);
        for segment in accessor.path.iter() {
            match segment {
                PathSegment::Field(symbol) => kani::assume(symbol.get() < parts.symbols_count),
                PathSegment::Index(index) => kani::assume(*index != u32::MAX),
                _ => kani::assume(false),
            }
        }
    }

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(result == Ok(()), "bounded valid accessors pass Gate 8");
    std::mem::forget(parts);
}

#[kani::proof]
fn kani_gate_08_valid_zero_accessors_pass() {
    let parts = workflow_parts_with_accessors(Box::new([]), 0, 0);

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(result == Ok(()), "zero accessors pass Gate 8");
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
        result == Ok(()),
        "index-only accessor does not require symbols",
    );
}

#[kani::proof]
#[kani::unwind(5)]
fn kani_gate_08_no_panic_bounded_inputs() {
    let parts: WorkflowParts = kani::any();
    kani::assume(parts.slot_count <= 256);
    kani::assume(parts.symbols_count <= 1024);

    kani::cover(parts.accessors.is_empty(), "zero accessors covered");
    kani::cover(parts.accessors.len() == 3, "three accessors covered");
    kani::cover(
        parts
            .accessors
            .iter()
            .any(|accessor| accessor.path.len() == 3),
        "three path segments covered",
    );

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
            Err(ValidationError::AccessorSymbolOutOfBounds {
                accessor_index: 0,
                segment_index: 0,
                symbol,
                symbols_count: observed_symbols_count,
            }) if symbol == field_id && observed_symbols_count == symbols_count
        ),
        "field symbol >= symbols_count is rejected",
    );
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
        matches!(
            result,
            Err(ValidationError::AccessorPathInvalid {
                accessor_index: 0,
                segment_index: 0,
            })
        ),
        "u32::MAX index sentinel is rejected",
    );
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
        matches!(
            result,
            Err(ValidationError::AccessorSlotOutOfRange {
                accessor_index: 0,
                slot,
                slot_count: observed_slot_count,
            }) if slot == usize::from(root) && observed_slot_count == usize::from(slot_count)
        ),
        "root >= slot_count is rejected",
    );
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

fn workflow_parts_with_accessors(
    accessors: Box<[AccessorProgram]>,
    slot_count: u16,
    symbols_count: u32,
) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("kani_workflow"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::new([CompiledNode {
            id: StepIdx::ZERO,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
        }]),
        expressions: Box::new([]),
        accessors,
        constants: Box::new([]),
        slot_count,
        symbols_count,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}
