use super::*;

#[test]
fn choose_width_counts_branch_body_steps() {
    let branches = vec![
        crate::ChooseBranch {
            when: "0".to_string(),
            steps: vec![choose_body_set_step("body_a", "1")],
        },
        crate::ChooseBranch {
            when: "1".to_string(),
            steps: vec![
                choose_body_set_step("body_b", "2"),
                choose_body_set_step("body_c", "3"),
            ],
        },
    ];
    let width = choose_width(&branches).expect("valid Set bodies must compute width");
    assert_eq!(width, 4, "ChooseSlot plus three body nodes");
}

#[test]
fn choose_width_64_empty_branches_returns_1() {
    let branches: Vec<crate::ChooseBranch> = (0..64)
        .map(|i| crate::ChooseBranch {
            when: i.to_string(),
            steps: vec![],
        })
        .collect();
    let width = choose_width(&branches).expect("64 empty branches must compute width");
    assert_eq!(width, 1, "ChooseSlot only, no body nodes");
}

#[test]
fn lower_canonical_choose_accepts_two_branches() {
    let branches = vec![
        crate::ChooseBranch {
            when: "0".to_string(),
            steps: vec![],
        },
        crate::ChooseBranch {
            when: "1".to_string(),
            steps: vec![],
        },
    ];
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
    assert!(
        result.is_ok(),
        "two-branch choose must compile: {:?}",
        result
    );
}

#[test]
fn lower_canonical_choose_single_body_set_targets_body_start() {
    let branches = vec![crate::ChooseBranch {
        when: "0".to_string(),
        steps: vec![choose_body_set_step("body_a", "7")],
    }];
    let step_names: [Box<str>; 2] = [Box::from("pick"), Box::from("done")];
    let mut builder = crate::SlotCompiler::new();
    lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("done"),
        Some(vb_core::ids::StepIdx::new(2)),
        &step_names,
        &mut builder,
    )
    .expect("single Set body must lower");

    assert_eq!(builder.nodes.len(), 2, "ChooseSlot plus body node");
    assert_eq!(builder.nodes[0].id, vb_core::ids::StepIdx::new(0));
    match &builder.nodes[0].kind {
        vb_core::CompiledNodeKind::ChooseSlot { branches, .. } => {
            assert_eq!(branches[0].target, vb_core::ids::StepIdx::new(1));
        }
        other => panic!("expected ChooseSlot, got {other:?}"),
    }
}

// ============================================================================
// vb-282my tests are at the end of the file.
#[test]
fn lower_canonical_choose_multi_body_steps_chain_to_common_next() {
    let branches = vec![crate::ChooseBranch {
        when: "0".to_string(),
        steps: vec![
            choose_body_set_step("body_a", "7"),
            choose_body_set_step("body_b", "8"),
        ],
    }];
    let step_names: [Box<str>; 2] = [Box::from("pick"), Box::from("done")];
    let mut builder = crate::SlotCompiler::new();
    lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("done"),
        Some(vb_core::ids::StepIdx::new(3)),
        &step_names,
        &mut builder,
    )
    .expect("multi-step Set body must lower");

    assert_eq!(builder.nodes.len(), 3, "ChooseSlot plus two body nodes");
    match &builder.nodes[0].kind {
        vb_core::CompiledNodeKind::ChooseSlot { branches, .. } => {
            assert_eq!(branches[0].target, vb_core::ids::StepIdx::new(1));
        }
        other => panic!("expected ChooseSlot, got {other:?}"),
    }
    assert_eq!(
        builder.nodes[1].routing.next,
        vb_core::workflow::NodeContinuation::Step(vb_core::ids::StepIdx::new(2))
    );
    assert_eq!(
        builder.nodes[2].routing.next,
        vb_core::workflow::NodeContinuation::Step(vb_core::ids::StepIdx::new(3))
    );
}

#[test]
fn lower_canonical_choose_rejects_unknown_otherwise_label() {
    let branches = vec![crate::ChooseBranch {
        when: "0".to_string(),
        steps: vec![],
    }];
    let step_names: [Box<str>; 1] = [Box::from("pick")];
    let mut builder = crate::SlotCompiler::new();
    let result = lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("missing"),
        Some(vb_core::ids::StepIdx::new(1)),
        &step_names,
        &mut builder,
    );
    match result {
        Err(crate::CompileErrors(errors)) => {
            assert!(
                errors.iter().any(|e| matches!(e, crate::CompileError::UnknownStepLabel { label, .. } if label.as_ref() == "missing")),
                "unknown otherwise label must return UnknownStepLabel with actual label text, got: {:?}",
                errors
            );
        }
        other => panic!(
            "expected error for unknown otherwise label, got: {:?}",
            other
        ),
    }
}

#[test]
fn lower_canonical_choose_rejects_65_branches() {
    let branches: Vec<crate::ChooseBranch> = (0..65)
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
    match result {
        Err(crate::CompileErrors(errors)) => {
            assert!(
                errors.iter().any(|e| matches!(e, crate::CompileError::PrimitiveLoweringLimitExceeded { primitive, field, value, limit } if *primitive == "choose" && *field == "branches" && *value == 65 && *limit == 64)),
                "65-branch choose must fail with PrimitiveLoweringLimitExceeded, got: {:?}",
                errors
            );
        }
        other => panic!("expected error for 65 branches, got: {:?}", other),
    }
}
