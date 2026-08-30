#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for Gate 8 accessor validation.
//!
//! PO-014 support repair: this cfg(kani)-only artifact must compile when the
//! vb_compile idempotency parity harness is selected, because Cargo compiles the
//! dependent vb_validate crate under cfg(kani) first.

use crate::vb_validate::{ValidationError, gates::validate_gate_08_accessor_path_segments};
use vb_core::ids::{SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprProgram, PathSegment, ResourceContract,
    WorkflowParts,
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

fn workflow_parts_with_accessors(
    accessors: Box<[AccessorProgram]>,
    slot_count: u16,
    symbols_count: u32,
) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("kani_workflow"),
        digest: WorkflowDigest::from_bytes(kani::any()),
        nodes: bounded_nodes(),
        expressions: bounded_expressions(),
        accessors,
        constants: bounded_constants(),
        slot_count,
        symbols_count,
        entry: kani::any(),
        resource_contract: kani::any::<ResourceContract>(),
        step_names: bounded_step_names(),
    }
}

fn bounded_nodes() -> Box<[CompiledNode]> {
    if kani::any::<bool>() {
        let result: SlotIdx = kani::any();
        Box::new([CompiledNode {
            id: kani::any::<StepIdx>(),
            output: if kani::any::<bool>() {
                Some(kani::any::<SlotIdx>())
            } else {
                None
            },
            next: if kani::any::<bool>() {
                Some(kani::any::<StepIdx>())
            } else {
                None
            },
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish { result },
        }])
    } else {
        Box::new([])
    }
}

fn bounded_expressions() -> Box<[ExprProgram]> {
    if kani::any::<bool>() {
        let ops: Box<[vb_core::workflow::ExprOp]> = if kani::any::<bool>() {
            Box::new([kani::any::<vb_core::workflow::ExprOp>()])
        } else {
            Box::new([])
        };
        Box::new([ExprProgram {
            ops,
            max_stack: kani::any(),
        }])
    } else {
        Box::new([])
    }
}

fn bounded_constants() -> Box<[vb_core::value::ConstValue]> {
    if kani::any::<bool>() {
        Box::new([kani::any::<vb_core::value::ConstValue>()])
    } else {
        Box::new([])
    }
}

fn bounded_step_names() -> Box<[Box<str>]> {
    if kani::any::<bool>() {
        Box::new([Box::from("kani_step")])
    } else {
        Box::new([])
    }
}
