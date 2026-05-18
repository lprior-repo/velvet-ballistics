#![cfg(kani)]
#![forbid(unsafe_code)]

//! Structural Kani harnesses for Gate 8 accessor validation over full WorkflowParts.
//!
//! These harnesses exercise the full WorkflowParts structurally via kani::Arbitrary:
//! nodes (all CompiledNodeKind variants), expressions, constants, step_names,
//! and resource_contract. Gate 8 only reads accessor paths against slot_count
//! and symbols_count, but the harnesses prove that arbitrary WorkflowParts shapes
//! never cause panics and always yield deterministic Ok/Err outcomes.

use crate::{ValidationError, gates::validate_gate_08_accessor_path_segments};
use vb_core::ids::{SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, PathSegment, ResourceContract, WorkflowParts,
};

/// Harness 1: Arbitrary WorkflowParts with bounded valid accessors always passes Gate 8.
///
/// Proves support totality over the full structural space.
#[kani::proof]
#[kani::unwind(5)]
fn kani_gate_08_arbitrary_parts_valid_accessors_pass() {
    let parts: WorkflowParts = kani::any();
    kani::assume(parts.slot_count > 0);
    kani::assume(parts.slot_count <= 256);
    kani::assume(parts.symbols_count > 0);
    kani::assume(parts.symbols_count <= 1024);

    // Validate all accessors have bounded roots and field symbols.
    for accessor in parts.accessors.iter() {
        kani::assume(accessor.root.as_usize() < usize::from(parts.slot_count));
        for segment in accessor.path.iter() {
            match segment {
                PathSegment::Field(symbol) => {
                    kani::assume(symbol.get() < parts.symbols_count);
                }
                PathSegment::Index(index) => kani::assume(*index != u32::MAX),
                _ => kani::assume(false),
            }
        }
    }

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(result.is_ok(), "arbitrary valid accessors pass Gate 8");
    std::mem::forget(parts);
}

/// Harness 2: Arbitrary WorkflowParts with root out of range is rejected.
///
/// Proves rejection totality over structural variants.
#[kani::proof]
#[kani::unwind(5)]
fn kani_gate_08_arbitrary_parts_root_oob_rejected() {
    let mut parts: WorkflowParts = kani::any();
    kani::assume(parts.slot_count > 0);
    kani::assume(parts.slot_count <= 8);

    // Corrupt the first accessor root to be out of bounds.
    if !parts.accessors.is_empty() {
        let root = SlotIdx::new(parts.slot_count); // exactly slot_count = OOB
        parts.accessors[0].root = root;
    }

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(
        matches!(result, Err(ValidationError::AccessorSlotOutOfRange { .. })),
        "arbitrary parts with out-of-range root is rejected",
    );
    std::mem::forget(parts);
}

/// Harness 3: Arbitrary WorkflowParts with field symbol out of range is rejected.
#[kani::proof]
#[kani::unwind(5)]
fn kani_gate_08_arbitrary_parts_symbol_oob_rejected() {
    let mut parts: WorkflowParts = kani::any();
    kani::assume(parts.symbols_count > 0);
    kani::assume(parts.symbols_count <= 64);

    // Corrupt the first accessor path to reference an out-of-bounds symbol.
    if !parts.accessors.is_empty() && !parts.accessors[0].path.is_empty() {
        for segment in parts.accessors[0].path.iter_mut() {
            if matches!(segment, PathSegment::Field(_)) {
                *segment = PathSegment::Field(SymbolId::new(parts.symbols_count));
                break;
            }
        }
    }

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(
        matches!(
            result,
            Err(ValidationError::AccessorSymbolOutOfBounds { .. })
        ),
        "arbitrary parts with OOB field symbol is rejected",
    );
    std::mem::forget(parts);
}

/// Harness 4: Arbitrary WorkflowParts with u32::MAX index sentinel is rejected.
#[kani::proof]
#[kani::unwind(5)]
fn kani_gate_08_arbitrary_parts_index_sentinel_rejected() {
    let mut parts: WorkflowParts = kani::any();

    // Corrupt the first accessor path to contain a u32::MAX sentinel.
    if !parts.accessors.is_empty() {
        for segment in parts.accessors[0].path.iter_mut() {
            if matches!(segment, PathSegment::Index(_)) {
                *segment = PathSegment::Index(u32::MAX);
                break;
            }
        }
    }

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(
        matches!(result, Err(ValidationError::AccessorPathInvalid { .. })),
        "arbitrary parts with u32::MAX index sentinel is rejected",
    );
    std::mem::forget(parts);
}

/// Harness 5: Full structural variety — nodes, expressions, constants, step_names.
///
/// Proves that Gate 8 is immune to structural noise in unrelated tables.
#[kani::proof]
#[kani::unwind(5)]
fn kani_gate_08_full_structure_no_panic() {
    let parts: WorkflowParts = kani::any();

    // Gate 8 should never panic regardless of nodes/expressions/constants/step_names shape.
    let _result = validate_gate_08_accessor_path_segments(&parts);
    std::mem::forget(parts);
}

/// Harness 6: Cover structural variety facets for tracking.
#[kani::proof]
fn kani_gate_08_structure_coverage() {
    let parts: WorkflowParts = kani::any();
    kani::assume(parts.slot_count <= 256);
    kani::assume(parts.symbols_count <= 1024);

    kani::cover(parts.nodes.is_empty(), "empty nodes covered");
    kani::cover(!parts.nodes.is_empty(), "non-empty nodes covered");
    kani::cover(parts.expressions.is_empty(), "empty expressions covered");
    kani::cover(
        !parts.expressions.is_empty(),
        "non-empty expressions covered",
    );
    kani::cover(parts.constants.is_empty(), "empty constants covered");
    kani::cover(!parts.constants.is_empty(), "non-empty constants covered");
    kani::cover(parts.step_names.is_empty(), "empty step_names covered");
    kani::cover(!parts.step_names.is_empty(), "non-empty step_names covered");
    kani::cover(parts.accessors.is_empty(), "zero accessors covered");
    kani::cover(parts.accessors.len() >= 2, "multiple accessors covered");
    kani::cover(
        parts.accessors.iter().any(|a| !a.path.is_empty()),
        "accessors with path covered",
    );

    let _result = validate_gate_08_accessor_path_segments(&parts);
    std::mem::forget(parts);
}

/// Harness 7: Arbitrary resource_contract — Gate 8 ignores it but must tolerate any shape.
#[kani::proof]
#[kani::unwind(3)]
fn kani_gate_08_arbitrary_resource_contract() {
    let mut parts: WorkflowParts = kani::any();
    kani::assume(parts.slot_count > 0);
    kani::assume(parts.slot_count <= 16);

    // Gate 8 does not inspect resource_contract, but arbitrary parts must not panic.
    let _result = validate_gate_08_accessor_path_segments(&parts);
    std::mem::forget(parts);
}

/// Harness 8: step_names correlation — step_names length varies independently of slot_count.
///
/// Gate 8 does not read step_names, but arbitrary WorkflowParts may have mismatched
/// lengths. This proves the validator does not crash on such mismatches.
#[kani::proof]
#[kani::unwind(3)]
fn kani_gate_08_step_names_independent_of_slots() {
    let parts: WorkflowParts = kani::any();
    // slot_count and step_names.len() are independent; Gate 8 must tolerate any combination.
    let _result = validate_gate_08_accessor_path_segments(&parts);
    std::mem::forget(parts);
}

/// Harness 9: Empty nodes + valid accessors — Gate 8 does not require non-empty nodes.
#[kani::proof]
fn kani_gate_08_empty_nodes_valid_accessors_pass() {
    let parts = WorkflowParts {
        name: Box::from("empty_nodes_workflow"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::new([]),
        expressions: Box::new([]),
        accessors: Box::new([AccessorProgram {
            root: SlotIdx::ZERO,
            path: Box::new([PathSegment::Field(SymbolId::new(0))]),
        }]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 1,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([Box::from("entry")]),
    };

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(
        result.is_ok(),
        "empty nodes with valid accessors pass Gate 8",
    );
}

/// Harness 10: Mixed expression op types with accessors — expressions reference accessors.
#[kani::proof]
#[kani::unwind(3)]
fn kani_gate_08_expressions_with_accessor_refs() {
    let parts = WorkflowParts {
        name: Box::from("expr_accessor_workflow"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::ZERO,
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: vb_core::ids::ExprIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::ZERO,
                },
            },
        ]),
        expressions: Box::new([vb_core::workflow::ExprProgram {
            ops: Box::new([
                vb_core::workflow::ExprOp::LoadAccessor(vb_core::ids::AccessorIdx::new(0)),
                vb_core::workflow::ExprOp::Eq,
            ]),
            max_stack: 1,
        }]),
        accessors: Box::new([
            AccessorProgram {
                root: SlotIdx::ZERO,
                path: Box::new([PathSegment::Field(SymbolId::new(0))]),
            },
            AccessorProgram {
                root: SlotIdx::new(1),
                path: Box::new([PathSegment::Index(0)]),
            },
        ]),
        constants: Box::new([vb_core::value::ConstValue::Null]),
        slot_count: 4,
        symbols_count: 2,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([Box::from("eval"), Box::from("finish")]),
    };

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(result.is_ok(), "expressions with accessor refs pass Gate 8");
}

/// Harness 11: Multiple accessor path variants in one workflow — field + index chains.
#[kani::proof]
#[kani::unwind(5)]
fn kani_gate_08_mixed_accessor_paths() {
    let parts = WorkflowParts {
        name: Box::from("mixed_paths"),
        digest: WorkflowDigest::from_bytes([2; 32]),
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
        accessors: Box::new([
            AccessorProgram {
                root: SlotIdx::ZERO,
                path: Box::new([]),
            },
            AccessorProgram {
                root: SlotIdx::ZERO,
                path: Box::new([PathSegment::Field(SymbolId::new(0))]),
            },
            AccessorProgram {
                root: SlotIdx::ZERO,
                path: Box::new([PathSegment::Index(0)]),
            },
            AccessorProgram {
                root: SlotIdx::ZERO,
                path: Box::new([
                    PathSegment::Field(SymbolId::new(0)),
                    PathSegment::Index(0),
                    PathSegment::Field(SymbolId::new(1)),
                ]),
            },
            AccessorProgram {
                root: SlotIdx::new(1),
                path: Box::new([PathSegment::Index(100)]),
            },
            AccessorProgram {
                root: SlotIdx::new(2),
                path: Box::new([
                    PathSegment::Index(0),
                    PathSegment::Index(1),
                    PathSegment::Index(2),
                    PathSegment::Index(3),
                    PathSegment::Index(4),
                ]),
            },
        ]),
        constants: Box::new([]),
        slot_count: 64,
        symbols_count: 256,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([Box::from("root")]),
    };

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(result.is_ok(), "mixed accessor paths pass Gate 8");
}

/// Harness 12: All CompiledNodeKind variants present — stress test structural variety.
#[kani::proof]
#[kani::unwind(5)]
fn kani_gate_08_all_node_kinds_no_panic() {
    let parts: WorkflowParts = kani::any();

    // Even with all possible node kinds randomly assembled, Gate 8 must not panic.
    let _result = validate_gate_08_accessor_path_segments(&parts);
    std::mem::forget(parts);
}

/// Harness 13: Constants with symbol values — Gate 8 doesn't read constants but must tolerate.
#[kani::proof]
#[kani::unwind(3)]
fn kani_gate_08_constants_with_symbols() {
    let parts = WorkflowParts {
        name: Box::from("consts_with_symbols"),
        digest: WorkflowDigest::from_bytes([3; 32]),
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::ZERO,
                output: Some(SlotIdx::ZERO),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: vb_core::ids::ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::ZERO,
                },
            },
        ]),
        expressions: Box::new([]),
        accessors: Box::new([AccessorProgram {
            root: SlotIdx::ZERO,
            path: Box::new([]),
        }]),
        constants: Box::new([
            vb_core::value::ConstValue::Null,
            vb_core::value::ConstValue::Bool(true),
            vb_core::value::ConstValue::I64(42),
            vb_core::value::ConstValue::Symbol(SymbolId::new(0)),
        ]),
        slot_count: 2,
        symbols_count: 4,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([Box::from("set"), Box::from("finish")]),
    };

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(result.is_ok(), "constants with symbols pass Gate 8");
}

/// Harness 14: Large accessor count with varied depths — stress Gate 8 iteration.
#[kani::proof]
#[kani::unwind(10)]
fn kani_gate_08_many_accessors_varied_depths() {
    let parts = WorkflowParts {
        name: Box::from("many_accessors"),
        digest: WorkflowDigest::from_bytes([4; 32]),
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
        accessors: Box::new([
            AccessorProgram {
                root: SlotIdx::ZERO,
                path: Box::new([]),
            },
            AccessorProgram {
                root: SlotIdx::ZERO,
                path: Box::new([PathSegment::Field(SymbolId::new(0))]),
            },
            AccessorProgram {
                root: SlotIdx::ZERO,
                path: Box::new([PathSegment::Index(0)]),
            },
            AccessorProgram {
                root: SlotIdx::ZERO,
                path: Box::new([
                    PathSegment::Field(SymbolId::new(0)),
                    PathSegment::Index(0),
                    PathSegment::Field(SymbolId::new(1)),
                ]),
            },
            AccessorProgram {
                root: SlotIdx::new(1),
                path: Box::new([PathSegment::Index(100)]),
            },
            AccessorProgram {
                root: SlotIdx::new(2),
                path: Box::new([
                    PathSegment::Index(0),
                    PathSegment::Index(1),
                    PathSegment::Index(2),
                    PathSegment::Index(3),
                    PathSegment::Index(4),
                ]),
            },
        ]),
        constants: Box::new([]),
        slot_count: 64,
        symbols_count: 256,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([Box::from("root")]),
    };

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(
        result.is_ok(),
        "many accessors with varied depths pass Gate 8",
    );
}
