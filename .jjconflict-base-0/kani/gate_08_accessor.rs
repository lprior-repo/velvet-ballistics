//! Kani harnesses for Gate 8 - Accessor path segments valid.
//!
//! K3: Accessor path symbol lookup total
//! K4: Accessor path no UB

#![forbid(unsafe_code)]

use vb_core::ids::{SlotIdx, StepIdx, SymbolId};
use vb_core::value::ConstValue;
use vb_core::workflow::{AccessorProgram, PathSegment, ResourceContract, WorkflowParts};
use vb_validate::gates::validate_gate_08_accessor_path_segments;

/// K3: SymbolLookup(segment, symbols) != None for all valid segments.
///
/// For all Field segments, if symbol < symbols_count, validation passes.
#[kani::proof]
fn kani_gate_08_symbol_in_bounds() {
    let symbol_count: u32 = kani::any();
    let symbol: u32 = kani::any();

    // Constrain: symbol < symbol_count
    kani::assume(symbol < symbol_count);
    kani::assume(symbol_count > 0);
    kani::assume(symbol_count <= 100);

    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: Box::new([PathSegment::Field(SymbolId::new(symbol))]),
    };

    let parts = WorkflowParts {
        name: Box::from("kani_g8"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([vb_core::workflow::CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: vb_core::workflow::CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]),
        expressions: Box::new([]),
        accessors: Box::new([accessor]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: symbol_count,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let result = validate_gate_08_accessor_path_segments(&parts);

    kani::assert(
        result.is_ok(),
        "symbol < symbols_count should pass gate 8",
    );
}

/// K4: Accessor path operations do not cause UB.
#[kani::proof]
fn kani_gate_08_no_ub() {
    let slot_count: u16 = kani::any();
    let symbols_count: u32 = kani::any();
    let root_idx: u16 = kani::any();

    kani::assume(root_idx < slot_count);
    kani::assume(slot_count > 0);
    kani::assume(slot_count <= 16);
    kani::assume(symbols_count <= 100);

    let accessor = AccessorProgram {
        root: SlotIdx::new(root_idx),
        path: Box::new([
            PathSegment::Field(SymbolId::new(0)),
            PathSegment::Index(0),
        ]),
    };

    let parts = WorkflowParts {
        name: Box::from("kani_g8_ub"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([vb_core::workflow::CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: vb_core::workflow::CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]),
        expressions: Box::new([]),
        accessors: Box::new([accessor]),
        constants: Box::new([]),
        slot_count,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    // Must not panic
    let _result = validate_gate_08_accessor_path_segments(&parts);
}
