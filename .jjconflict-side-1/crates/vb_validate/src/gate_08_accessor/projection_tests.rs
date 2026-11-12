use super::*;
use proptest::prelude::*;
use vb_core::ConstValue;
use vb_core::ids::{AccessorIdx, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use vb_core::workflow::{
    CompiledInputConstraint, CompiledInputSlot, CompiledNodeKind, ExprOp, ExprProgram,
    InputSlotKind, ResourceContract, node,
};

fn parts(accessors: Box<[AccessorProgram]>) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("projection"),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        nodes: Box::new([]),
        expressions: Box::new([]),
        accessors,
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 1,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),
    }
}

fn accessor(root: u16, path: Box<[PathSegment]>) -> AccessorProgram {
    AccessorProgram {
        root: SlotIdx::new(root),
        path,
    }
}

fn assert_public(parts: &WorkflowParts, expected: ValidationResult<()>) {
    assert_eq!(
        crate::gates::validate_gate_08_accessor_path_segments(parts),
        expected
    );
    assert_eq!(
        validate_accessor_programs(&parts.accessors, parts.slot_count, parts.symbols_count),
        expected
    );
}

#[test]
fn projection_preserves_root_before_depth_and_segment_errors() {
    let parts = parts(Box::new([accessor(
        1,
        vec![PathSegment::Index(u32::MAX); 17].into_boxed_slice(),
    )]));
    assert_public(
        &parts,
        Err(ValidationError::AccessorSlotOutOfRange {
            accessor_index: 0,
            slot: 1,
            slot_count: 1,
        }),
    );
}

#[test]
fn projection_preserves_depth_before_segment_error() {
    let parts = parts(Box::new([accessor(
        0,
        vec![PathSegment::Index(u32::MAX); 17].into_boxed_slice(),
    )]));
    assert_public(
        &parts,
        Err(ValidationError::AccessorPathTooDeep {
            accessor_index: 0,
            depth: 17,
            max: 16,
        }),
    );
}

#[test]
fn projection_preserves_first_accessor_before_later_root_error() {
    let parts = parts(Box::new([
        accessor(
            0,
            Box::new([PathSegment::Index(0), PathSegment::Field(SymbolId::new(1))]),
        ),
        accessor(1, Box::new([])),
    ]));
    assert_public(
        &parts,
        Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 1,
            symbol: 1,
            symbols_count: 1,
        }),
    );
}

#[test]
fn projection_preserves_segment_order_and_exact_sentinel_payload() {
    let parts = parts(Box::new([accessor(
        0,
        Box::new([
            PathSegment::Index(0),
            PathSegment::Index(u32::MAX),
            PathSegment::Field(SymbolId::new(1)),
        ]),
    )]));
    assert_public(
        &parts,
        Err(ValidationError::AccessorPathInvalid {
            accessor_index: 0,
            segment_index: 1,
        }),
    );
}

#[test]
fn projection_accepts_depth_sixteen_and_full_valid_index_boundary() {
    let path = vec![PathSegment::Index(u32::MAX.saturating_sub(1)); 16].into_boxed_slice();
    assert_public(&parts(Box::new([accessor(0, path)])), Ok(()));
}

#[test]
fn projection_accepts_no_accessors_with_zero_counts() {
    let mut parts = parts(Box::new([]));
    parts.slot_count = 0;
    parts.symbols_count = 0;
    assert_public(&parts, Ok(()));
}

fn decorate_code_tables(parts: &mut WorkflowParts, seed: u16) {
    parts.nodes = Box::new([node(
        StepIdx::new(seed),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(seed),
        },
        None,
        None,
    )]);
    parts.expressions = Box::new([ExprProgram {
        ops: Box::new([ExprOp::LoadAccessor(AccessorIdx::new(seed))]),
        max_stack: 0,
    }]);
    parts.constants = Box::new([ConstValue::Symbol(SymbolId::new(u32::from(seed)))]);
    parts.entry = StepIdx::new(seed);
}

fn decorate_metadata(parts: &mut WorkflowParts, seed: u16, hash: [u8; 32]) {
    parts.name = format!("unrelated-{seed}").into_boxed_str();
    parts.digest = WorkflowDigest::from_bytes(hash);
    parts.semantic_source_identity = Some(WorkflowDigest::from_bytes(hash));
    parts.resource_contract.max_slots = seed;
    parts.resource_contract.max_steps = seed;
    parts.step_names = Box::new([Box::from("unrelated-name")]);
    parts.input_slots = Box::new([CompiledInputSlot {
        slot: SlotIdx::new(seed),
        kind: InputSlotKind::Any,
        constraint: CompiledInputConstraint::default(),
    }]);
}

proptest! {
    #[test]
    fn public_projection_ignores_unrelated_tables_and_metadata(seed in any::<u16>(), hash in any::<[u8;32]>()) {
        let mut parts = parts(Box::new([accessor(0, Box::new([PathSegment::Field(SymbolId::new(1))]))]));
        let expected = Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0, segment_index: 0, symbol: 1, symbols_count: 1,
        });
        assert_public(&parts, expected.clone());
        decorate_code_tables(&mut parts, seed);
        decorate_metadata(&mut parts, seed, hash);
        assert_public(&parts, expected);
    }
}
