use super::*;

// ============================================================================
// vb-282my: Choose lowering refinement (CL-02, CL-10, CL-13)
// ============================================================================
#[test]
fn lower_canonical_choose_empty_branches_without_otherwise_returns_empty_branch_table_error() {
    let branches: Vec<crate::ChooseBranch> = vec![];
    let step_names: [Box<str>; 1] = [Box::from("pick")];
    let mut builder = crate::SlotCompiler::new();
    let result = lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        None,
        Some(vb_core::ids::StepIdx::new(1)),
        &step_names,
        &mut builder,
    );
    match result {
        Err(crate::CompileErrors(errors)) => {
            assert!(
                errors.iter().any(|e| matches!(
                    e,
                    crate::CompileError::Workflow(
                        // Use the same pattern as part_08: vb_core::WorkflowError
                        vb_core::WorkflowError::EmptyBranchTable
                    )
                )),
                "empty branches without otherwise must return EmptyBranchTable, got: {errors:?}"
            );
        }
        other => panic!("expected EmptyBranchTable error, got {other:?}"),
    }
}

#[test]
fn lower_canonical_choose_accepts_exactly_64_branches() {
    let branches: Vec<crate::ChooseBranch> = (0..64)
        .map(|i| crate::ChooseBranch {
            when: i.to_string(),
            steps: vec![],
        })
        .collect();
    let step_names: [Box<str>; 2] = [Box::from("pick"), Box::from("done")];
    let mut builder = crate::SlotCompiler::new();
    let result = lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("done"),
        Some(vb_core::ids::StepIdx::new(1)),
        &step_names,
        &mut builder,
    );
    // Verify the lowering succeeded (no error)
    result.expect("64 branches is the maximum allowed, must succeed");
    // Verify a ChooseSlot node was emitted with exactly 64 branches
    match &builder.nodes[0].kind {
        vb_core::CompiledNodeKind::ChooseSlot {
            branches: slot_branches,
            ..
        } => {
            assert_eq!(
                slot_branches.len(),
                64,
                "ChooseSlot must contain exactly 64 branches, got {}",
                slot_branches.len()
            );
        }
        other => panic!("expected ChooseSlot node, got {other:?}"),
    }
}

#[test]
fn direct_digest_collect_body_set_id_changes_digest()
-> Result<(), crate::mod_compile_errors::CompileErrors> {
    let body_a = vec![StepAst {
        id: "step_a".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".to_string(),
            value: "1".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    let mut body_b = body_a.clone();
    body_b[0].id = "step_b".to_string();

    let prim_a = make_collect("x", "items", Some(10), Some(50), body_a);
    let prim_b = make_collect("x", "items", Some(10), Some(50), body_b);

    let digest_a = try_digest_primitive(&prim_a)?;
    let digest_b = try_digest_primitive(&prim_b)?;

    assert_ne!(digest_a, digest_b, "Set step id changes Collect digest");
    Ok(())
}

#[test]
fn direct_digest_collect_body_set_output_changes_digest()
-> Result<(), crate::mod_compile_errors::CompileErrors> {
    let body_a = vec![StepAst {
        id: "step_a".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".to_string(),
            value: "1".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    let mut body_b = body_a.clone();
    body_b[0].primitive = StepPrimitive::Set {
        output: "y".to_string(),
        value: "1".to_string(),
    };

    let prim_a = make_collect("x", "items", Some(10), Some(50), body_a);
    let prim_b = make_collect("x", "items", Some(10), Some(50), body_b);

    let digest_a = try_digest_primitive(&prim_a)?;
    let digest_b = try_digest_primitive(&prim_b)?;

    assert_ne!(digest_a, digest_b, "Set output changes Collect digest");
    Ok(())
}

#[test]
fn direct_digest_collect_body_set_value_changes_digest()
-> Result<(), crate::mod_compile_errors::CompileErrors> {
    let body_a = vec![StepAst {
        id: "step_a".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".to_string(),
            value: "1".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    let mut body_b = body_a.clone();
    body_b[0].primitive = StepPrimitive::Set {
        output: "x".to_string(),
        value: "2".to_string(),
    };

    let prim_a = make_collect("x", "items", Some(10), Some(50), body_a);
    let prim_b = make_collect("x", "items", Some(10), Some(50), body_b);

    let digest_a = try_digest_primitive(&prim_a)?;
    let digest_b = try_digest_primitive(&prim_b)?;

    assert_ne!(digest_a, digest_b, "Set value changes Collect digest");
    Ok(())
}

#[test]
fn direct_digest_reduce_body_set_id_changes_digest()
-> Result<(), crate::mod_compile_errors::CompileErrors> {
    let body_a = vec![StepAst {
        id: "step_a".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".to_string(),
            value: "1".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    let mut body_b = body_a.clone();
    body_b[0].id = "step_b".to_string();

    let prim_a = make_reduce("acc", "items", "0", body_a);
    let prim_b = make_reduce("acc", "items", "0", body_b);

    let digest_a = try_digest_primitive(&prim_a)?;
    let digest_b = try_digest_primitive(&prim_b)?;

    assert_ne!(digest_a, digest_b, "Set step id changes Aggregate digest");
    Ok(())
}

#[test]
fn direct_digest_reduce_body_set_output_changes_digest()
-> Result<(), crate::mod_compile_errors::CompileErrors> {
    let body_a = vec![StepAst {
        id: "step_a".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".to_string(),
            value: "1".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    let mut body_b = body_a.clone();
    body_b[0].primitive = StepPrimitive::Set {
        output: "y".to_string(),
        value: "1".to_string(),
    };

    let prim_a = make_reduce("acc", "items", "0", body_a);
    let prim_b = make_reduce("acc", "items", "0", body_b);

    let digest_a = try_digest_primitive(&prim_a)?;
    let digest_b = try_digest_primitive(&prim_b)?;

    assert_ne!(digest_a, digest_b, "Set output changes Aggregate digest");
    Ok(())
}

#[test]
fn direct_digest_reduce_body_set_value_changes_digest()
-> Result<(), crate::mod_compile_errors::CompileErrors> {
    let body_a = vec![StepAst {
        id: "step_a".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".to_string(),
            value: "1".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    let mut body_b = body_a.clone();
    body_b[0].primitive = StepPrimitive::Set {
        output: "x".to_string(),
        value: "2".to_string(),
    };

    let prim_a = make_reduce("acc", "items", "0", body_a);
    let prim_b = make_reduce("acc", "items", "0", body_b);

    let digest_a = try_digest_primitive(&prim_a)?;
    let digest_b = try_digest_primitive(&prim_b)?;

    assert_ne!(digest_a, digest_b, "Set value changes Aggregate digest");
    Ok(())
}
