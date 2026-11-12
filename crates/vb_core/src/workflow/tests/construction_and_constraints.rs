#[test]
fn compiled_node_nop_construction_equality_and_debug() {
    let node = crate::workflow::node(StepIdx::new(0), CompiledNodeKind::Nop, None, None);
    let node2 = crate::workflow::node(StepIdx::new(0), CompiledNodeKind::Nop, None, None);
    assert_eq!(node, node2, "identical Nop nodes must be equal");
    let debug_str = format!("{node:?}");
    assert!(
        debug_str.contains("Nop"),
        "Debug output for Nop node must contain 'Nop': {debug_str}"
    );
}

#[test]
fn compiled_node_kind_set_const_field_access() -> Result<(), String> {
    let const_idx = ConstIdx::new(42);
    let kind = CompiledNodeKind::SetConst { value: const_idx };
    let CompiledNodeKind::SetConst { value } = kind else {
        return Err(String::from("expected SetConst variant"));
    };
    if value != const_idx {
        return Err(String::from("SetConst value field mismatch"));
    }
    Ok(())
}

#[test]
fn compiled_node_kind_copy_field_access() -> Result<(), String> {
    let slot = SlotIdx::new(7);
    let kind = CompiledNodeKind::Copy { source: slot };
    let CompiledNodeKind::Copy { source } = kind else {
        return Err(String::from("expected Copy variant"));
    };
    if source != slot {
        return Err(String::from("Copy source field mismatch"));
    }
    Ok(())
}

#[test]
fn compiled_node_kind_do_field_access() -> Result<(), String> {
    let action = ActionId::new(3);
    let input = SlotIdx::new(5);
    let kind = CompiledNodeKind::Do { action, input };
    let CompiledNodeKind::Do {
        action: a,
        input: i,
    } = kind
    else {
        return Err(String::from("expected Do variant"));
    };
    if a != action {
        return Err(String::from("Do action field mismatch"));
    }
    if i != input {
        return Err(String::from("Do input field mismatch"));
    }
    Ok(())
}

#[test]
fn compiled_node_kind_finish_equality_and_debug() {
    let kind_a = CompiledNodeKind::Finish {
        result: SlotIdx::new(99),
    };
    let kind_b = CompiledNodeKind::Finish {
        result: SlotIdx::new(99),
    };
    assert_eq!(kind_a, kind_b, "identical Finish variants must be equal");
    let debug_str = format!("{kind_a:?}");
    assert!(
        debug_str.contains("Finish"),
        "Debug output for Finish must contain 'Finish': {debug_str}"
    );
}

#[test]
fn compiled_node_kind_variant_inequality() {
    let nop = CompiledNodeKind::Nop;
    let set_const = CompiledNodeKind::SetConst {
        value: ConstIdx::new(0),
    };
    let finish = CompiledNodeKind::Finish {
        result: SlotIdx::new(0),
    };
    assert_ne!(nop, set_const, "Nop and SetConst must not be equal");
    assert_ne!(nop, finish, "Nop and Finish must not be equal");
    assert_ne!(set_const, finish, "SetConst and Finish must not be equal");
}

#[test]
fn expr_op_comparison_variants_pairwise_distinct() {
    let ops = [
        ExprOp::Eq,
        ExprOp::NotEq,
        ExprOp::Gt,
        ExprOp::Gte,
        ExprOp::Lt,
        ExprOp::Lte,
    ];
    assert_pairwise_distinct(&ops);
}

#[test]
fn expr_op_arithmetic_variants_pairwise_distinct() {
    let ops = [ExprOp::Add, ExprOp::Sub, ExprOp::Mul, ExprOp::Div];
    assert_pairwise_distinct(&ops);
}

#[test]
fn expr_op_load_variants_with_max_indices() -> Result<(), String> {
    let load_slot = ExprOp::LoadSlot(SlotIdx::new(u16::MAX));
    let load_const = ExprOp::LoadConst(ConstIdx::new(u16::MAX));
    let load_accessor = ExprOp::LoadAccessor(AccessorIdx::new(u16::MAX));

    match load_slot {
        ExprOp::LoadSlot(idx) if idx.get() == u16::MAX => {}
        _ => return Err(String::from("LoadSlot max index mismatch")),
    }
    match load_const {
        ExprOp::LoadConst(idx) if idx.get() == u16::MAX => {}
        _ => return Err(String::from("LoadConst max index mismatch")),
    }
    match load_accessor {
        ExprOp::LoadAccessor(idx) if idx.get() == u16::MAX => {}
        _ => return Err(String::from("LoadAccessor max index mismatch")),
    }

    assert_ne!(load_slot, load_const);
    assert_ne!(load_slot, load_accessor);
    assert_ne!(load_const, load_accessor);
    Ok(())
}

#[test]
fn expr_program_construction_and_field_access() -> Result<(), String> {
    let ops: Box<[ExprOp]> = vec![load(0)].into_boxed_slice();
    let program = ExprProgram::try_from_ops(ops.clone()).map_err(|e| e.to_string())?;
    if program.ops.len() != 1 {
        return Err(format!("expected 1 op, got {}", program.ops.len()));
    }
    if program.max_stack != 1 {
        return Err(format!("expected max_stack=1, got {}", program.max_stack));
    }
    Ok(())
}

#[test]
fn expr_branch_zero_offset_construction_and_equality() {
    let branch_a = ExprBranch {
        condition: ExprIdx::new(0),
        target: StepIdx::new(0),
    };
    let branch_b = ExprBranch {
        condition: ExprIdx::new(0),
        target: StepIdx::new(0),
    };
    assert_eq!(
        branch_a, branch_b,
        "identical ExprBranch with zero offsets must be equal"
    );
    assert_eq!(branch_a.condition.get(), 0);
    assert_eq!(branch_a.target.get(), 0);

    let debug_str = format!("{branch_a:?}");
    assert!(
        debug_str.contains("ExprBranch"),
        "Debug output must contain 'ExprBranch': {debug_str}"
    );
}

#[test]
fn expr_branch_max_values() {
    let branch = ExprBranch {
        condition: ExprIdx::new(u16::MAX),
        target: StepIdx::new(u16::MAX),
    };
    assert_eq!(branch.condition.get(), u16::MAX);
    assert_eq!(branch.target.get(), u16::MAX);

    let branch_zero = ExprBranch {
        condition: ExprIdx::new(0),
        target: StepIdx::new(0),
    };
    assert_ne!(branch, branch_zero, "max and zero ExprBranch must differ");
}

#[test]
fn slot_branch_zero_and_max_values() {
    let branch_zero = SlotBranch {
        condition: SlotIdx::new(0),
        target: StepIdx::new(0),
    };
    let branch_max = SlotBranch {
        condition: SlotIdx::new(u16::MAX),
        target: StepIdx::new(u16::MAX),
    };
    assert_eq!(branch_zero.condition.get(), 0);
    assert_eq!(branch_zero.target.get(), 0);
    assert_eq!(branch_max.condition.get(), u16::MAX);
    assert_eq!(branch_max.target.get(), u16::MAX);
    assert_ne!(branch_zero, branch_max);
}

#[test]
fn expr_op_boolean_and_helper_variants_equality() {
    assert_eq!(ExprOp::And, ExprOp::And);
    assert_eq!(ExprOp::Or, ExprOp::Or);
    assert_eq!(ExprOp::Not, ExprOp::Not);
    assert_ne!(ExprOp::And, ExprOp::Or);
    assert_ne!(ExprOp::Not, ExprOp::And);

    assert_eq!(ExprOp::Contains, ExprOp::Contains);
    assert_eq!(ExprOp::StartsWith, ExprOp::StartsWith);
    assert_eq!(ExprOp::EndsWith, ExprOp::EndsWith);
    assert_ne!(ExprOp::Contains, ExprOp::StartsWith);
    assert_ne!(ExprOp::StartsWith, ExprOp::EndsWith);

    assert_eq!(ExprOp::Has, ExprOp::Has);
    assert_eq!(ExprOp::Exists, ExprOp::Exists);
    assert_eq!(ExprOp::Length, ExprOp::Length);
    assert_eq!(ExprOp::Empty, ExprOp::Empty);
    assert_ne!(ExprOp::Has, ExprOp::Exists);
    assert_ne!(ExprOp::Length, ExprOp::Empty);

    assert_eq!(ExprOp::Append, ExprOp::Append);
    assert_eq!(ExprOp::AppendIf, ExprOp::AppendIf);
    assert_eq!(ExprOp::Merge, ExprOp::Merge);
    assert_eq!(ExprOp::Sum, ExprOp::Sum);
    assert_eq!(ExprOp::Count, ExprOp::Count);
    assert_eq!(ExprOp::Unique, ExprOp::Unique);
    assert_ne!(ExprOp::Append, ExprOp::AppendIf);
    assert_ne!(ExprOp::Sum, ExprOp::Count);
    assert_ne!(ExprOp::Merge, ExprOp::Unique);
}

#[test]
fn compiled_node_kind_jump_and_eval_expr_field_access() -> Result<(), String> {
    let jump = CompiledNodeKind::Jump {
        target: StepIdx::new(10),
    };
    let CompiledNodeKind::Jump { target } = jump else {
        return Err(String::from("expected Jump variant"));
    };
    if target.get() != 10 {
        return Err(String::from("Jump target mismatch"));
    }

    let eval_expr = CompiledNodeKind::EvalExpr {
        expr: ExprIdx::new(5),
    };
    let CompiledNodeKind::EvalExpr { expr } = eval_expr else {
        return Err(String::from("expected EvalExpr variant"));
    };
    if expr.get() != 5 {
        return Err(String::from("EvalExpr expr mismatch"));
    }
    Ok(())
}

// =========================================================================
// vb-hbav B25-B27: blake3 digest coherence tests.
// =========================================================================

fn make_minimal_workflow_parts(name: &str, entry: StepIdx, slot_count: u16) -> WorkflowParts {
    let digest = WorkflowDigest::from_bytes([0u8; 32]);
    WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: name.into(),
        digest,
        nodes: Box::new([crate::workflow::node(
            entry,
            CompiledNodeKind::Finish {
                result: SlotIdx::ZERO,
            },
            None,
            None,
        )]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

#[test]
fn blake3_digest_is_deterministic_for_identical_parts() {
    let parts1 = make_minimal_workflow_parts("alpha", StepIdx::ZERO, 1);
    let parts2 = make_minimal_workflow_parts("alpha", StepIdx::ZERO, 1);
    let bytes1 = postcard::to_allocvec(&parts1).expect("serialize should succeed");
    let bytes2 = postcard::to_allocvec(&parts2).expect("serialize should succeed");
    let hash1 = blake3::hash(&bytes1);
    let hash2 = blake3::hash(&bytes2);
    assert_eq!(
        hash1.as_bytes(),
        hash2.as_bytes(),
        "identical WorkflowParts must produce identical digests"
    );
}

#[test]
fn blake3_digest_differs_when_name_differs() {
    let parts_alpha = make_minimal_workflow_parts("alpha", StepIdx::ZERO, 1);
    let parts_beta = make_minimal_workflow_parts("beta", StepIdx::ZERO, 1);
    let bytes_alpha = postcard::to_allocvec(&parts_alpha).expect("serialize should succeed");
    let bytes_beta = postcard::to_allocvec(&parts_beta).expect("serialize should succeed");
    let hash_alpha = blake3::hash(&bytes_alpha);
    let hash_beta = blake3::hash(&bytes_beta);
    assert_ne!(
        hash_alpha.as_bytes(),
        hash_beta.as_bytes(),
        "different name must produce different digest"
    );
}

#[test]
fn blake3_digest_differs_when_node_count_differs() {
    let digest = WorkflowDigest::from_bytes([0u8; 32]);
    let node = crate::workflow::node(
        StepIdx::ZERO,
        CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
        None,
        None,
    );
    let parts1 = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: "test".into(),
        digest,
        nodes: Box::new([node.clone()]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let node2 = crate::workflow::node(
        StepIdx::new(1),
        CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
        None,
        None,
    );
    let parts2 = WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: "test".into(),
        digest,
        nodes: Box::new([node, node2]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let hash1 = blake3::hash(&postcard::to_allocvec(&parts1).expect("serialize should succeed"));
    let hash2 = blake3::hash(&postcard::to_allocvec(&parts2).expect("serialize should succeed"));
    assert_ne!(
        hash1.as_bytes(),
        hash2.as_bytes(),
        "different node_count must produce different digest"
    );
}

#[test]
fn blake3_digest_differs_when_entry_step_differs() {
    let parts_entry0 = make_minimal_workflow_parts("test", StepIdx::ZERO, 1);
    let parts_entry1 = make_minimal_workflow_parts("test", StepIdx::new(1), 1);
    let hash0 =
        blake3::hash(&postcard::to_allocvec(&parts_entry0).expect("serialize should succeed"));
    let hash1 =
        blake3::hash(&postcard::to_allocvec(&parts_entry1).expect("serialize should succeed"));
    assert_ne!(
        hash0.as_bytes(),
        hash1.as_bytes(),
        "different entry step must produce different digest"
    );
}

#[test]
fn blake3_digest_valid_for_zero_slot_workflow() {
    let parts = make_minimal_workflow_parts("zero_slot", StepIdx::ZERO, 0);
    let bytes = postcard::to_allocvec(&parts).expect("serialize should succeed");
    let hash = blake3::hash(&bytes);
    let hash_bytes = hash.as_bytes();
    assert_eq!(hash_bytes.len(), 32, "blake3 must produce 32-byte hash");
    assert_ne!(hash_bytes, &[0u8; 32], "hash must not be all zeros");
}

fn parts_with_constraint(constraint: CompiledInputConstraint) -> WorkflowParts {
    let mut parts = make_minimal_workflow_parts("constraint_case", StepIdx::ZERO, 1);
    parts.input_slots = Box::new([CompiledInputSlot {
        slot: SlotIdx::ZERO,
        kind: InputSlotKind::I64,
        constraint,
    }]);
    parts
}

#[test]
fn constraint_default_is_legacy_permissive() {
    let unconstrained = CompiledInputConstraint::unconstrained();
    assert!(!unconstrained.requires_value());
    assert_eq!(
        unconstrained,
        CompiledInputConstraint::default(),
        "serde default for pre-constraint artifacts must stay permissive"
    );
    let required = CompiledInputConstraint::required();
    assert!(required.requires_value());
    assert!(!required.presence.is_optional());
}

#[test]
fn inconsistent_min_max_rejected() {
    let mut constraint = CompiledInputConstraint::required();
    constraint.min = Some(10);
    constraint.max = Some(9);
    match CompiledWorkflow::try_from_parts(parts_with_constraint(constraint)) {
        Err(WorkflowError::InconsistentInputConstraint { slot, issue }) => {
            assert_eq!(slot, SlotIdx::ZERO);
            assert!(matches!(issue, InputConstraintIssue::MinExceedsMax));
        }
        other => panic!("expected InconsistentInputConstraint, got {other:?}"),
    }
}

#[test]
fn inconsistent_length_bounds_rejected() {
    let mut constraint = CompiledInputConstraint::required();
    constraint.min_length = Some(4);
    constraint.max_length = Some(3);
    match CompiledWorkflow::try_from_parts(parts_with_constraint(constraint)) {
        Err(WorkflowError::InconsistentInputConstraint { issue, .. }) => {
            assert!(matches!(
                issue,
                InputConstraintIssue::MinLengthExceedsMaxLength
            ));
        }
        other => panic!("expected InconsistentInputConstraint, got {other:?}"),
    }
}

#[test]
fn negative_length_bound_rejected() {
    let mut constraint = CompiledInputConstraint::required();
    constraint.max_length = Some(-1);
    match CompiledWorkflow::try_from_parts(parts_with_constraint(constraint)) {
        Err(WorkflowError::InconsistentInputConstraint { issue, .. }) => {
            assert!(matches!(issue, InputConstraintIssue::NegativeLengthBound));
        }
        other => panic!("expected InconsistentInputConstraint, got {other:?}"),
    }
}

#[test]
fn consistent_bounds_accepted() {
    let mut constraint = CompiledInputConstraint::required();
    constraint.min = Some(-5);
    constraint.max = Some(5);
    constraint.min_length = Some(0);
    constraint.max_length = Some(8);
    assert!(CompiledWorkflow::try_from_parts(parts_with_constraint(constraint)).is_ok());
}
