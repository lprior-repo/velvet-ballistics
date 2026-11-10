#[test]
fn phase46_rejects_unreachable_node_from_entry() -> Result<(), String> {
    // Node 0 finishes immediately; node 1 is never reached.
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("unreachable"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(2, 1, 1, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::UnreachableNode { step }) if step == StepIdx::new(1) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// =========================================================================
// Phase 46: SymbolId, accessor path depth, and untrusted input tests
// =========================================================================

// --- SymbolId range validation in accessor path Field segments ---

#[test]
fn phase46_rejects_accessor_field_symbol_out_of_bounds() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![PathSegment::Field(SymbolId::new(5))].into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("acc_sym_oob"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 3, // Only symbols 0, 1, 2 exist; SymbolId(5) is out of bounds
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SymbolOutOfBounds { symbol }) if symbol == SymbolId::new(5) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_accepts_accessor_field_symbol_at_boundary() -> Result<(), String> {
    // SymbolId(2) should be valid when symbols_count=3 (symbols 0, 1, 2)
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![PathSegment::Field(SymbolId::new(2))].into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("acc_sym_boundary"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 3,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
    };

    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn phase46_rejects_accessor_field_symbol_zero_when_no_symbols() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![PathSegment::Field(SymbolId::new(0))].into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("acc_sym_zero_no_syms"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SymbolOutOfBounds { symbol }) if symbol == SymbolId::new(0) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- Accessor path depth validation ---

#[test]
fn phase46_rejects_accessor_path_too_deep() -> Result<(), String> {
    let deep_path: Vec<PathSegment> = (0..=MAX_PATH_DEPTH)
        .map(|i| PathSegment::Index(u32::try_from(i).unwrap_or(0)))
        .collect();
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: deep_path.into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("acc_deep"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::AccessorPathTooDeep { depth, max }) => {
            if depth == MAX_PATH_DEPTH.saturating_add(1) && max == MAX_PATH_DEPTH {
                Ok(())
            } else {
                Err(format!("wrong depth/max: depth={depth}, max={max}"))
            }
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_accepts_accessor_path_at_max_depth() -> Result<(), String> {
    let path: Vec<PathSegment> = (0..MAX_PATH_DEPTH)
        .map(|i| PathSegment::Index(u32::try_from(i).unwrap_or(0)))
        .collect();
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: path.into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("acc_max_depth"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
    };

    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn phase46_accepts_accessor_empty_path() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: Box::new([]),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("acc_empty_path"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
    };

    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// --- SymbolId range validation in constant pool ---

#[test]
fn phase46_rejects_constant_symbol_out_of_bounds() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("const_sym_oob"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Symbol(SymbolId::new(99))].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 5,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 1, 1, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SymbolOutOfBounds { symbol }) if symbol == SymbolId::new(99) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_accepts_constant_symbol_at_boundary() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("const_sym_boundary"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Symbol(SymbolId::new(4))].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 5,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 1, 1, 0, 0),
        step_names: Box::new([]),
    };

    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn phase46_rejects_constant_symbol_when_zero_symbols() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("const_sym_no_syms"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::new(0)),
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Symbol(SymbolId::new(0))].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 1, 1, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SymbolOutOfBounds { symbol }) if symbol == SymbolId::new(0) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- SymbolId range validation in BuildObject fields ---

#[test]
fn phase46_rejects_build_object_symbol_out_of_bounds() -> Result<(), String> {
    let parts = construction_parts_with_symbols(
        CompiledNodeKind::BuildObject {
            fields: vec![
                (SymbolId::new(0), SlotIdx::new(0)),
                (SymbolId::new(10), SlotIdx::new(0)),
            ]
            .into_boxed_slice(),
        },
        1,
        1,
        5, // only symbols 0..4 exist
    );

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SymbolOutOfBounds { symbol }) if symbol == SymbolId::new(10) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_accepts_build_object_symbols_within_range() -> Result<(), String> {
    let parts = construction_parts_with_symbols(
        CompiledNodeKind::BuildObject {
            fields: vec![
                (SymbolId::new(0), SlotIdx::new(0)),
                (SymbolId::new(1), SlotIdx::new(0)),
            ]
            .into_boxed_slice(),
        },
        1,
        1,
        2,
    );

    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// --- Accessor path index u32::MAX rejection (existing check, new test) ---

#[test]
fn phase46_rejects_accessor_index_u32_max() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![PathSegment::Index(u32::MAX)].into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("acc_u32max"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::Expression(CoreError::InvalidCompiledWorkflow { reason })) => {
            if reason.contains("u32::MAX") {
                Ok(())
            } else {
                Err(format!("unexpected reason: {reason}"))
            }
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- Mixed accessor path with both Field and Index ---

#[test]
fn phase46_accepts_accessor_mixed_path() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![
            PathSegment::Field(SymbolId::new(0)),
            PathSegment::Index(3),
            PathSegment::Field(SymbolId::new(1)),
        ]
        .into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("acc_mixed"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 2,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
    };

    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn phase46_rejects_mixed_path_with_bad_symbol() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![
            PathSegment::Field(SymbolId::new(0)),
            PathSegment::Index(3),
            PathSegment::Field(SymbolId::new(5)), // out of bounds
        ]
        .into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("acc_mixed_bad"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 2,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SymbolOutOfBounds { symbol }) if symbol == SymbolId::new(5) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- symbols_count roundtrip through to_parts ---

#[test]
fn phase46_symbols_count_roundtrip() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("sym_roundtrip"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 42,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 1, 1, 0, 0),
        step_names: Box::new([]),
    };
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    if workflow.symbols_count() != 42 {
        return Err(format!(
            "expected symbols_count 42, got {}",
            workflow.symbols_count()
        ));
    }
    let recovered = workflow.to_parts();
    if recovered.symbols_count != 42 {
        return Err(format!(
            "expected symbols_count 42 in recovered parts, got {}",
            recovered.symbols_count
        ));
    }
    Ok(())
}

// --- Multiple constants with mixed SymbolId validity ---

#[test]
fn phase46_rejects_second_constant_symbol_out_of_bounds() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("const_mixed_oob"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(1),
                },
                None,
                None,
            ),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![
            ConstValue::Symbol(SymbolId::new(0)),  // valid
            ConstValue::Symbol(SymbolId::new(50)), // out of bounds
        ]
        .into_boxed_slice(),
        slot_count: 1,
        symbols_count: 10,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(2, 1, 2, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SymbolOutOfBounds { symbol }) if symbol == SymbolId::new(50) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// --- Accessor root slot validation (existing check, confirmed by test) ---

#[test]
fn phase46_rejects_accessor_root_slot_out_of_bounds() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(5),
        path: Box::new([]),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("acc_root_oob"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: vec![accessor].into_boxed_slice(),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(5) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// =========================================================================
// CompiledNodeKind variant construction tests — all 34 variants
// =========================================================================

#[test]
fn compiled_node_kind_nop_constructs() {
    let kind = CompiledNodeKind::Nop;
    let node = crate::workflow::node(StepIdx::new(0), kind, None, None);
    assert!(matches!(node.kind, CompiledNodeKind::Nop));
}

#[test]
fn compiled_node_kind_set_const_constructs() {
    let kind = CompiledNodeKind::SetConst {
        value: ConstIdx::new(42),
    };
    let CompiledNodeKind::SetConst { value } = kind else {
        panic!("expected SetConst");
    };
    assert_eq!(value, ConstIdx::new(42));
}

#[test]
fn compiled_node_kind_copy_constructs() {
    let kind = CompiledNodeKind::Copy {
        source: SlotIdx::new(7),
    };
    let CompiledNodeKind::Copy { source } = kind else {
        panic!("expected Copy");
    };
    assert_eq!(source, SlotIdx::new(7));
}

#[test]
fn compiled_node_kind_eval_expr_constructs() {
    let kind = CompiledNodeKind::EvalExpr {
        expr: ExprIdx::new(3),
    };
    let CompiledNodeKind::EvalExpr { expr } = kind else {
        panic!("expected EvalExpr");
    };
    assert_eq!(expr, ExprIdx::new(3));
}

#[test]
fn compiled_node_kind_build_object_constructs() {
    let kind = CompiledNodeKind::BuildObject {
        fields: vec![(SymbolId::new(1), SlotIdx::new(0))].into_boxed_slice(),
    };
    let CompiledNodeKind::BuildObject { fields } = kind else {
        panic!("expected BuildObject");
    };
    assert_eq!(fields.len(), 1);
}

#[test]
fn compiled_node_kind_build_list_constructs() {
    let kind = CompiledNodeKind::BuildList {
        items: vec![SlotIdx::new(0), SlotIdx::new(1)].into_boxed_slice(),
    };
    let CompiledNodeKind::BuildList { items } = kind else {
        panic!("expected BuildList");
    };
    assert_eq!(items.len(), 2);
}
