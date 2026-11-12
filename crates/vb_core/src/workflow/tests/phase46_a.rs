#[test]
fn compiled_workflow_accessor_returns_none_for_out_of_bounds() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|error| error.to_string())?;

    if workflow.accessor(AccessorIdx::new(0)).is_some() {
        return Err(String::from("expected None for out-of-bounds accessor"));
    }
    Ok(())
}

#[test]
fn compiled_workflow_node_returns_none_for_out_of_bounds() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|error| error.to_string())?;

    if workflow.node(StepIdx::new(5)).is_some() {
        return Err(String::from("expected None for out-of-bounds node"));
    }
    Ok(())
}

// --- to_parts roundtrip preserves identity ---

#[test]
fn compiled_workflow_to_parts_roundtrip_preserves_fields() -> Result<(), String> {
    let expression = ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let original = finish_const_parts_with(
        resource_contract(1, 0, 1, 1, 1),
        vec![expression].into_boxed_slice(),
    );
    let workflow = CompiledWorkflow::try_from_parts(original).map_err(|error| error.to_string())?;

    let recovered = workflow.to_parts();
    if recovered.name.as_ref() != workflow.name() {
        return Err(String::from("name mismatch"));
    }
    if recovered.digest != workflow.digest() {
        return Err(String::from("digest mismatch"));
    }
    if recovered.entry != workflow.entry() {
        return Err(String::from("entry mismatch"));
    }
    if recovered.slot_count != workflow.slot_count() {
        return Err(String::from("slot_count mismatch"));
    }
    Ok(())
}

// --- Phase 46: IR structural validation tests ---

fn phase46_parts_with_nodes(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts {
    let max_steps = u16::try_from(nodes.len()).unwrap_or(u16::MAX);
    WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: external_input(InputSlotKind::Any),
        name: Box::<str>::from("phase46"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(max_steps, slot_count, 1, 0, 0),
        step_names: Box::new([]),
    }
}

#[test]
fn phase46_rejects_unreachable_node() -> Result<(), String> {
    let parts = phase46_parts_with_nodes(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        1,
    );
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::UnreachableNode { step }) if step == StepIdx::new(2) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_accepts_reachable_chain() -> Result<(), String> {
    let parts = phase46_parts_with_nodes(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
        ],
        1,
    );
    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn phase46_rejects_backward_next() -> Result<(), String> {
    let parts = phase46_parts_with_nodes(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                Some(StepIdx::new(0)),
            ),
        ],
        1,
    );
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::BackwardEdge { from, to }) => {
            if from == StepIdx::new(1) && to == StepIdx::new(0) {
                Ok(())
            } else {
                Err(format!("wrong from/to: {from:?} -> {to:?}"))
            }
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_rejects_jump_backward() -> Result<(), String> {
    let parts = phase46_parts_with_nodes(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Jump {
                    target: StepIdx::new(0),
                },
                None,
                None,
            ),
        ],
        1,
    );
    let result = CompiledWorkflow::try_from_parts(parts).map(|_| ());
    match result {
        Err(WorkflowError::BackwardEdge { from, to })
            if from == StepIdx::new(1) && to == StepIdx::new(0) =>
        {
            Ok(())
        }
        other => Err(format!(
            "expected exact BackwardEdge variant, got {other:?}"
        )),
    }
}

#[test]
fn loop_start_body_must_be_forward_edge() -> Result<(), String> {
    // Node 0 is a ForEachStart whose `body` points back at itself (StepIdx(0)).
    // Before the split, `validate_loop_done_only` ignored `body`, so this
    // configuration passed. After the split, `validate_loop_start_edges`
    // rejects the backward `body` edge with WorkflowError::BackwardEdge.
    let parts = phase46_parts_with_nodes(
        vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 1.into(),
                    body: StepIdx::new(0),
                    done: StepIdx::new(1),
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
        ],
        2,
    );
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::BackwardEdge { from, to })
            if from == StepIdx::new(0) && to == StepIdx::new(0) =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_accepts_foreach_forward() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: external_input(InputSlotKind::List),
        name: Box::<str>::from("phase46_foreach"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 10.into(),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                Some(SlotIdx::new(2)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(2)),
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::ForEachNext {
                    iterator_slot: SlotIdx::new(2),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
                Some(SlotIdx::new(1)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(3),
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
        slot_count: 3,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(4, 3, 1, 0, 0),
        step_names: Box::new([]),
    };
    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn phase46_rejects_improper_nesting() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("phase46_nesting"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 10.into(),
                    body: StepIdx::new(1),
                    done: StepIdx::new(4),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(1),
                    item_slot: SlotIdx::new(2),
                    limit: 10.into(),
                    body: StepIdx::new(2),
                    done: StepIdx::new(5),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(3)),
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::ForEachNext {
                    iterator_slot: SlotIdx::new(3),
                    body: StepIdx::new(2),
                    done: StepIdx::new(5),
                },
                None,
                None,
            ),
            crate::workflow::node(
                StepIdx::new(4),
                CompiledNodeKind::ForEachNext {
                    iterator_slot: SlotIdx::new(4),
                    body: StepIdx::new(1),
                    done: StepIdx::new(5),
                },
                Some(SlotIdx::new(1)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(5),
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
        slot_count: 5,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(6, 5, 1, 0, 0),
        step_names: Box::new([]),
    };
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::ImproperLoopNesting { inner, outer_done }) => {
            if inner == StepIdx::new(1) && outer_done == StepIdx::new(4) {
                Ok(())
            } else {
                Err(format!(
                    "wrong inner/outer: inner={inner:?}, outer_done={outer_done:?}"
                ))
            }
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_accepts_proper_nesting() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: external_input(InputSlotKind::List),
        name: Box::<str>::from("phase46_proper"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![
            crate::workflow::node(
                StepIdx::new(0),
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 10.into(),
                    body: StepIdx::new(1),
                    done: StepIdx::new(5),
                },
                Some(SlotIdx::new(4)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(1),
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(1),
                    item_slot: SlotIdx::new(2),
                    limit: 10.into(),
                    body: StepIdx::new(2),
                    done: StepIdx::new(4),
                },
                Some(SlotIdx::new(3)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(2),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(3)),
            ),
            crate::workflow::node(
                StepIdx::new(3),
                CompiledNodeKind::ForEachNext {
                    iterator_slot: SlotIdx::new(3),
                    body: StepIdx::new(2),
                    done: StepIdx::new(4),
                },
                Some(SlotIdx::new(2)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(4),
                CompiledNodeKind::ForEachNext {
                    iterator_slot: SlotIdx::new(4),
                    body: StepIdx::new(1),
                    done: StepIdx::new(5),
                },
                Some(SlotIdx::new(1)),
                None,
            ),
            crate::workflow::node(
                StepIdx::new(5),
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
        slot_count: 5,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(6, 5, 1, 0, 0),
        step_names: Box::new([]),
    };
    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn phase46_accepts_accessor_field() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![PathSegment::Field(SymbolId::new(42))].into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: external_input(InputSlotKind::Any),
        name: Box::<str>::from("phase46_acc_field"),
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
        symbols_count: 43,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::new([]),
    };
    CompiledWorkflow::try_from_parts(parts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn phase46_accepts_accessor_index() -> Result<(), String> {
    let accessor = AccessorProgram {
        root: SlotIdx::new(0),
        path: vec![PathSegment::Index(7)].into_boxed_slice(),
    };
    let mut contract = resource_contract(1, 1, 1, 0, 0);
    contract.max_accessors = 1;
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: external_input(InputSlotKind::Any),
        name: Box::<str>::from("phase46_acc_index"),
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

// =========================================================================
// Phase 46 adversarial tests -- IR structural validation edge cases
// =========================================================================

#[test]
fn phase46_rejects_cycle_via_backward_next_edge() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("cycle_next"),
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
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
                Some(StepIdx::new(0)),
            ),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(2, 1, 1, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::BackwardEdge { from, to })
            if from == StepIdx::new(1) && to == StepIdx::new(0) =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_rejects_duplicate_step_idx_via_node_id_mismatch() -> Result<(), String> {
    // Two nodes with the same StepIdx (both claim to be step 0).
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("dup_step_idx"),
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
            // Second node at index 1 but claims to be step 0 (duplicate id).
            crate::workflow::node(
                StepIdx::new(0),
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
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(2, 1, 1, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::NodeIdMismatch { expected, actual })
            if expected == StepIdx::new(1) && actual == StepIdx::new(0) =>
        {
            Ok(())
        }
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_rejects_slot_idx_out_of_bounds_in_finish() -> Result<(), String> {
    // Finish node references slot 99 but slot_count is only 1.
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("slot_oob_finish"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(99),
            },
            None,
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 1, 1, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(99) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_rejects_slot_idx_out_of_bounds_in_build_list() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("slot_oob_list"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::BuildList {
                items: vec![SlotIdx::new(0), SlotIdx::new(50)].into_boxed_slice(),
            },
            Some(SlotIdx::new(0)),
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 2, 0, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(50) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn phase46_rejects_slot_idx_out_of_bounds_in_output() -> Result<(), String> {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("slot_oob_output"),
        digest: WorkflowDigest::from_bytes([0x46; 32]),
        nodes: vec![crate::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
            Some(SlotIdx::new(200)),
            None,
        )]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(1)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: resource_contract(1, 1, 1, 0, 0),
        step_names: Box::new([]),
    };

    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(200) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}
