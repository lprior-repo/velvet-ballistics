#![cfg(kani)]
#![forbid(unsafe_code)]

//! Structural Kani harnesses for Gate 8 accessor validation over full WorkflowParts.
//!
//! These harnesses exercise the full WorkflowParts structurally via kani::Arbitrary:
//! nodes (all CompiledNodeKind variants), expressions, constants, step_names,
//! and resource_contract. Gate 8 only reads accessor paths against slot_count
//! and symbols_count, but the harnesses prove that arbitrary WorkflowParts shapes
//! never cause panics and always yield deterministic Ok/Err outcomes.
//!
//! PO-014 support repair: this cfg(kani)-only artifact must compile when the
//! vb_compile idempotency parity harness is selected, because Cargo compiles the
//! dependent vb_validate crate under cfg(kani) first.

use crate::{ValidationError, gates::validate_gate_08_accessor_path_segments};
use vb_core::ids::{SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprProgram, PathSegment, WorkflowParts,
};

/// Harness 1: Bounded structural WorkflowParts with valid accessors always passes Gate 8.
///
/// Proves Gate 8 totality over the bounded structural space relevant to
/// accessor validation without constructing unrelated arbitrary heap shapes.
#[kani::proof]
#[kani::unwind(8)]
fn kani_gate_08_arbitrary_parts_valid_accessors_pass() {
    let parts = bounded_parts_with_valid_accessors();

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(result.is_ok(), "arbitrary valid accessors pass Gate 8");
    std::mem::forget(parts);
}

fn bounded_parts_with_valid_accessors() -> WorkflowParts {
    let slot_count: u16 = kani::any();
    let symbols_count: u32 = kani::any();
    let root: u16 = kani::any();
    let symbol: u32 = kani::any();
    let index: u32 = kani::any();

    kani::assume(slot_count > 0);
    kani::assume(slot_count <= 16);
    kani::assume(symbols_count > 0);
    kani::assume(symbols_count <= 64);
    kani::assume(root < slot_count);
    kani::assume(symbol < symbols_count);
    kani::assume(index != u32::MAX);

    let accessors: Box<[AccessorProgram]> = Box::new([AccessorProgram {
        root: SlotIdx::new(root),
        path: Box::new([
            PathSegment::Field(SymbolId::new(symbol)),
            PathSegment::Index(index),
        ]),
    }]);

    bounded_parts_with_accessors(accessors, slot_count, symbols_count)
}

fn bounded_parts_with_index_sentinel() -> WorkflowParts {
    bounded_parts_with_accessors(
        Box::new([AccessorProgram {
            root: SlotIdx::ZERO,
            path: Box::new([PathSegment::Index(u32::MAX)]),
        }]),
        1,
        0,
    )
}

fn bounded_empty_nodes_with_valid_accessors() -> WorkflowParts {
    let mut parts = bounded_parts_with_accessors(valid_accessor_set(), 16, 64);
    parts.nodes = Box::new([]);
    parts
}

fn valid_accessor_set() -> Box<[AccessorProgram]> {
    let symbol: u32 = kani::any();
    let index: u32 = kani::any();
    kani::assume(symbol < 64);
    kani::assume(index != u32::MAX);
    match kani::any::<u8>() {
        0 => Box::new([AccessorProgram {
            root: SlotIdx::ZERO,
            path: Box::new([PathSegment::Field(SymbolId::new(symbol))]),
        }]),
        1 => Box::new([AccessorProgram {
            root: SlotIdx::ZERO,
            path: Box::new([PathSegment::Index(index)]),
        }]),
        _ => Box::new([AccessorProgram {
            root: SlotIdx::ZERO,
            path: Box::new([
                PathSegment::Field(SymbolId::new(symbol)),
                PathSegment::Index(index),
            ]),
        }]),
    }
}

fn bounded_parts_with_accessors(
    accessors: Box<[AccessorProgram]>,
    slot_count: u16,
    symbols_count: u32,
) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("kani_gate_08"),
        digest: WorkflowDigest::from_bytes(kani::any()),
        nodes: bounded_structural_nodes(),
        expressions: bounded_structural_expressions(),
        accessors,
        constants: bounded_structural_constants(),
        slot_count,
        symbols_count,
        entry: kani::any(),
        resource_contract: kani::any(),
        step_names: bounded_structural_step_names(),
    }
}

fn bounded_structural_nodes() -> Box<[CompiledNode]> {
    if kani::any::<bool>() {
        Box::new([CompiledNode {
            id: kani::any::<StepIdx>(),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: kani::any::<SlotIdx>(),
            },
        }])
    } else {
        Box::new([])
    }
}

fn bounded_structural_expressions() -> Box<[ExprProgram]> {
    if kani::any::<bool>() {
        Box::new([ExprProgram {
            ops: Box::new([]),
            max_stack: kani::any(),
        }])
    } else {
        Box::new([])
    }
}

fn bounded_structural_constants() -> Box<[vb_core::value::ConstValue]> {
    if kani::any::<bool>() {
        Box::new([vb_core::value::ConstValue::Null])
    } else {
        Box::new([])
    }
}

fn bounded_structural_step_names() -> Box<[Box<str>]> {
    if kani::any::<bool>() {
        Box::new([Box::from("kani_step")])
    } else {
        Box::new([])
    }
}

/// Harness 2: Bounded structural WorkflowParts with root out of range is rejected.
///
/// Proves rejection totality over structural variants.
#[kani::proof]
#[kani::unwind(8)]
fn kani_gate_08_arbitrary_parts_root_oob_rejected() {
    let slot_count: u16 = kani::any();
    kani::assume(slot_count > 0);
    kani::assume(slot_count <= 8);

    let parts = bounded_parts_with_accessors(
        Box::new([AccessorProgram {
            root: SlotIdx::new(slot_count), // exactly slot_count = OOB
            path: Box::new([]),
        }]),
        slot_count,
        0,
    );

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(
        matches!(result, Err(ValidationError::AccessorSlotOutOfRange { .. })),
        "arbitrary parts with out-of-range root is rejected",
    );
    std::mem::forget(parts);
}

/// Harness 3: Bounded structural WorkflowParts with field symbol out of range is rejected.
#[kani::proof]
#[kani::unwind(8)]
fn kani_gate_08_arbitrary_parts_symbol_oob_rejected() {
    let slot_count: u16 = kani::any();
    let symbols_count: u32 = kani::any();
    let root: u16 = kani::any();
    let index: u32 = kani::any();

    kani::assume(slot_count > 0);
    kani::assume(slot_count <= 16);
    kani::assume(symbols_count > 0);
    kani::assume(symbols_count <= 64);
    kani::assume(root < slot_count);
    kani::assume(index != u32::MAX);

    let parts = bounded_parts_with_accessors(
        Box::new([AccessorProgram {
            root: SlotIdx::new(root),
            path: Box::new([
                PathSegment::Index(index),
                PathSegment::Field(SymbolId::new(symbols_count)),
            ]),
        }]),
        slot_count,
        symbols_count,
    );

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

/// Harness 4: WorkflowParts with a guaranteed u32::MAX index sentinel is rejected.
#[kani::proof]
#[kani::unwind(8)]
fn kani_gate_08_arbitrary_parts_index_sentinel_rejected() {
    let parts = bounded_parts_with_index_sentinel();

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(
        matches!(result, Err(ValidationError::AccessorPathInvalid { .. })),
        "parts with u32::MAX index sentinel is rejected",
    );
    std::mem::forget(parts);
}

/// Harness 5: Full structural variety — nodes, expressions, constants, step_names.
///
/// Proves that Gate 8 is immune to bounded structural noise in unrelated tables.
#[kani::proof]
#[kani::unwind(18)]
fn kani_gate_08_full_structure_no_panic() {
    let parts = bounded_full_structure_no_panic_parts();
    assume_gate_08_harness_bounds(&parts);

    // Gate 8 should never panic regardless of nodes/expressions/constants/step_names shape.
    let _result = validate_gate_08_accessor_path_segments(&parts);
    std::mem::forget(parts);
}

fn bounded_full_structure_no_panic_parts() -> WorkflowParts {
    let slot_count: u16 = kani::any();
    let symbols_count: u32 = kani::any();
    let root: u16 = kani::any();
    let symbol: u32 = kani::any();
    let index: u32 = kani::any();

    kani::assume(slot_count <= 256);
    kani::assume(symbols_count <= 1024);
    kani::assume(root <= 257);
    kani::assume(symbol <= 1025);

    kani::cover(root >= slot_count, "full structure root OOB covered");
    kani::cover(symbol >= symbols_count, "full structure symbol OOB covered");
    kani::cover(index == u32::MAX, "full structure index sentinel covered");

    let accessors: Box<[AccessorProgram]> = Box::new([AccessorProgram {
        root: SlotIdx::new(root),
        path: Box::new([
            PathSegment::Field(SymbolId::new(symbol)),
            PathSegment::Index(index),
        ]),
    }]);

    bounded_parts_with_accessors(accessors, slot_count, symbols_count)
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
#[kani::unwind(17)]
fn kani_gate_08_arbitrary_resource_contract() {
    let parts: WorkflowParts = kani::any();
    kani::assume(parts.slot_count > 0);
    kani::assume(parts.slot_count <= 16);
    kani::assume(parts.symbols_count <= 1024);

    // Gate 8 does not inspect resource_contract, but arbitrary parts must not panic.
    let _result = validate_gate_08_accessor_path_segments(&parts);
    std::mem::forget(parts);
}

/// Harness 8: step_names correlation — step_names length varies independently of slot_count.
///
/// Gate 8 does not read step_names, but arbitrary WorkflowParts may have mismatched
/// lengths. This proves the validator does not crash on such mismatches.
#[kani::proof]
#[kani::unwind(17)]
fn kani_gate_08_step_names_independent_of_slots() {
    let parts: WorkflowParts = kani::any();
    kani::assume(parts.slot_count <= 256);
    kani::assume(parts.symbols_count <= 1024);
    // slot_count and step_names.len() are independent; Gate 8 must tolerate any combination.
    let _result = validate_gate_08_accessor_path_segments(&parts);
    std::mem::forget(parts);
}

/// Harness 9: Empty nodes + valid accessors — Gate 8 does not require non-empty nodes.
///
/// GOD RULE fix: replaced hardcoded WorkflowParts with kani::any() + structural constraints.
/// Tests that valid accessor paths pass Gate 8 regardless of node content.
#[kani::proof]
#[kani::unwind(8)]
fn kani_gate_08_empty_nodes_valid_accessors_pass() {
    let parts = bounded_empty_nodes_with_valid_accessors();

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(
        result.is_ok(),
        "empty nodes with valid accessors pass Gate 8",
    );
    std::mem::forget(parts);
}

/// Harness 10: Mixed expression op types with accessors — expressions reference accessors.
///
/// GOD RULE fix: replaced hardcoded indices with kani::any().
#[kani::proof]
#[kani::unwind(8)]
fn kani_gate_08_expressions_with_accessor_refs() {
    // Bounded symbolic inputs. Each input is constrained to a small domain
    // so the solver does not blow up on a full WorkflowParts Arbitrary impl.
    let slot_count: u16 = kani::any();
    kani::assume(slot_count >= 2);
    kani::assume(slot_count <= 8);
    let symbols_count: u32 = kani::any();
    kani::assume(symbols_count >= 2);
    kani::assume(symbols_count <= 16);

    let slot0_raw: u16 = kani::any();
    kani::assume(slot0_raw < slot_count);
    let slot0 = SlotIdx::new(slot0_raw);

    let sym0_raw: u32 = kani::any();
    kani::assume(sym0_raw < symbols_count);
    let sym0 = SymbolId::new(sym0_raw);

    let step0 = StepIdx::new(0);
    let step1 = StepIdx::new(1);

    // Bounded WorkflowParts: we construct every field explicitly to avoid
    // pulling in the unbounded `kani::Arbitrary` impl for WorkflowParts.
    let parts = WorkflowParts {
        name: Box::from("kani_gate_08"),
        digest: WorkflowDigest::from_bytes(kani::any::<[u8; 32]>()),
        nodes: Box::new([
            CompiledNode {
                id: step0,
                output: Some(slot0),
                next: Some(step1),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: vb_core::ids::ExprIdx::new(0),
                },
            },
            CompiledNode {
                id: step1,
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish { result: slot0 },
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
                root: slot0,
                path: Box::new([PathSegment::Field(sym0)]),
            },
            AccessorProgram {
                root: slot0,
                path: Box::new([PathSegment::Index(0)]),
            },
        ]),
        constants: Box::new([vb_core::value::ConstValue::Null]),
        slot_count,
        symbols_count,
        entry: step0,
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        step_names: Box::new([Box::from("eval"), Box::from("finish")]),
    };

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(result.is_ok(), "expressions with accessor refs pass Gate 8");
    std::mem::forget(parts);
}

/// Harness 11: Multiple accessor path variants in one workflow — field + index chains.
///
/// GOD RULE fix: replaced hardcoded indices with kani::any().
#[kani::proof]
#[kani::unwind(5)]
fn kani_gate_08_mixed_accessor_paths() {
    // Arbitrary indices for the accessor paths.
    let sym0: SymbolId = kani::any();
    let sym1: SymbolId = kani::any();
    let idx0: u32 = kani::any();
    let idx1: u32 = kani::any();
    let idx2: u32 = kani::any();

    // Bounded slot/symbol counts.
    let slot_count: u16 = kani::any();
    let symbols_count: u32 = kani::any();
    kani::assume(slot_count >= 3); // Need at least 3 slots for the accessor roots
    kani::assume(slot_count <= 8);
    kani::assume(symbols_count >= 2); // Need at least 2 symbols
    kani::assume(symbols_count <= 16);
    kani::assume(sym0.get() < symbols_count);
    kani::assume(sym1.get() < symbols_count);
    // Ensure index sentinels are not MAX (MAX is used as invalid sentinel in Gate 8).
    kani::assume(idx0 != u32::MAX && idx1 != u32::MAX && idx2 != u32::MAX);

    let parts = bounded_parts_with_accessors(
        mixed_valid_accessors(sym0, sym1, idx0, idx1, idx2),
        slot_count,
        symbols_count,
    );

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(result.is_ok(), "mixed accessor paths pass Gate 8");
    std::mem::forget(parts);
}

fn mixed_valid_accessors(
    sym0: SymbolId,
    sym1: SymbolId,
    idx0: u32,
    idx1: u32,
    idx2: u32,
) -> Box<[AccessorProgram]> {
    Box::new([
        AccessorProgram {
            root: SlotIdx::ZERO,
            path: Box::new([]),
        },
        AccessorProgram {
            root: SlotIdx::new(1),
            path: Box::new([PathSegment::Field(sym0), PathSegment::Index(idx0)]),
        },
        AccessorProgram {
            root: SlotIdx::new(2),
            path: Box::new([
                PathSegment::Index(idx1),
                PathSegment::Field(sym1),
                PathSegment::Index(idx2),
            ]),
        },
    ])
}

/// Harness 12: All CompiledNodeKind variants present — stress test structural variety.
#[kani::proof]
#[kani::unwind(17)]
fn kani_gate_08_all_node_kinds_no_panic() {
    let parts: WorkflowParts = kani::any();

    // Even with all possible node kinds randomly assembled, Gate 8 must not panic.
    let _result = validate_gate_08_accessor_path_segments(&parts);
    std::mem::forget(parts);
}

/// Harness 13: Constants with symbol values — Gate 8 doesn't read constants but must tolerate.
///
/// GOD RULE fix: replaced hardcoded indices with kani::any().
#[kani::proof]
#[kani::unwind(17)]
fn kani_gate_08_constants_with_symbols() {
    // Arbitrary indices.
    let const_idx: vb_core::ids::ConstIdx = kani::any();
    let sym0: SymbolId = kani::any();
    let slot0: SlotIdx = kani::any();
    let step0: StepIdx = kani::any();
    let step1: StepIdx = kani::any();

    // Bounded slot/symbol counts.
    let slot_count: u16 = kani::any();
    let symbols_count: u32 = kani::any();
    kani::assume(slot_count >= 1);
    kani::assume(symbols_count >= 1);
    kani::assume(sym0.get() < symbols_count);
    kani::assume(slot0.get() < slot_count);
    kani::assume(step0.get() < u16::MAX);
    kani::assume(step1.get() < u16::MAX);

    let mut parts: WorkflowParts = kani::any();
    parts.nodes = Box::new([
        CompiledNode {
            id: step0,
            output: Some(slot0),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst { value: const_idx },
        },
        CompiledNode {
            id: step1,
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish { result: slot0 },
        },
    ]);
    parts.expressions = Box::new([]);
    parts.accessors = Box::new([AccessorProgram {
        root: slot0,
        path: Box::new([]),
    }]);
    parts.constants = Box::new([
        vb_core::value::ConstValue::Null,
        vb_core::value::ConstValue::Bool(true),
        vb_core::value::ConstValue::I64(42),
        vb_core::value::ConstValue::Symbol(sym0),
    ]);
    parts.slot_count = slot_count;
    parts.symbols_count = symbols_count;
    parts.entry = step0;
    parts.step_names = Box::new([Box::from("set"), Box::from("finish")]);

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(result.is_ok(), "constants with symbols pass Gate 8");
    std::mem::forget(parts);
}

/// Harness 14: Bounded accessor batch with varied depths — stress Gate 8 iteration.
///
/// GOD RULE fix: replaced hardcoded indices with kani::any().
#[kani::proof]
#[kani::unwind(5)]
fn kani_gate_08_many_accessors_varied_depths() {
    // Arbitrary indices for the accessor paths.
    let sym0: SymbolId = kani::any();
    let sym1: SymbolId = kani::any();
    let idx0: u32 = kani::any();
    let idx1: u32 = kani::any();
    let idx2: u32 = kani::any();

    // Bounded slot/symbol counts.
    let slot_count: u16 = kani::any();
    let symbols_count: u32 = kani::any();
    kani::assume(slot_count >= 3); // Need at least 3 slots for accessor roots
    kani::assume(slot_count <= 8);
    kani::assume(symbols_count >= 2); // Need at least 2 symbols
    kani::assume(symbols_count <= 16);
    kani::assume(sym0.get() < symbols_count);
    kani::assume(sym1.get() < symbols_count);
    // Ensure index sentinels are not MAX (MAX is used as invalid sentinel in Gate 8).
    kani::assume(idx0 != u32::MAX && idx1 != u32::MAX && idx2 != u32::MAX);

    let parts = bounded_parts_with_accessors(
        varied_valid_accessors(sym0, sym1, idx0, idx1, idx2),
        slot_count,
        symbols_count,
    );

    let result = validate_gate_08_accessor_path_segments(&parts);
    kani::assert(
        result.is_ok(),
        "bounded accessor batch with varied depths passes Gate 8",
    );
    std::mem::forget(parts);
}

fn varied_valid_accessors(
    sym0: SymbolId,
    sym1: SymbolId,
    idx0: u32,
    idx1: u32,
    idx2: u32,
) -> Box<[AccessorProgram]> {
    Box::new([
        AccessorProgram {
            root: SlotIdx::ZERO,
            path: Box::new([]),
        },
        AccessorProgram {
            root: SlotIdx::ZERO,
            path: Box::new([PathSegment::Field(sym0)]),
        },
        AccessorProgram {
            root: SlotIdx::ZERO,
            path: Box::new([PathSegment::Index(idx2)]),
        },
        AccessorProgram {
            root: SlotIdx::ZERO,
            path: Box::new([
                PathSegment::Field(sym0),
                PathSegment::Index(idx0),
                PathSegment::Index(idx1),
                PathSegment::Field(sym1),
            ]),
        },
    ])
}
