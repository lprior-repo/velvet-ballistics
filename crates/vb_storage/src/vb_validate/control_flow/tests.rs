use crate::vb_validate::*;

fn linear_flow(count: usize) -> WorkflowFlow {
    WorkflowFlow {
        steps: (0..count)
            .map(|i| StepFlow {
                id: Some(format!("step_{i}")),
                branch_targets: Vec::new(),
                then_target: None,
            })
            .collect(),
    }
}

fn branching_flow() -> WorkflowFlow {
    WorkflowFlow {
        steps: vec![
            StepFlow {
                id: Some("save".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
            StepFlow {
                id: Some("choose".to_owned()),
                branch_targets: vec![2, 3],
                then_target: None,
            },
            StepFlow {
                id: Some("true_branch".to_owned()),
                branch_targets: vec![],
                then_target: Some(3),
            },
            StepFlow {
                id: Some("done".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
        ],
    }
}

#[test]
fn accepts_linear_flow() {
    let flow = linear_flow(3);
    assert_eq!(validate_control_flow(&flow), Ok(()));
}

#[test]
fn accepts_branching_flow() {
    let flow = branching_flow();
    assert_eq!(validate_control_flow(&flow), Ok(()));
}

#[test]
fn rejects_backward_branch() {
    let flow = WorkflowFlow {
        steps: vec![
            StepFlow {
                id: Some("first".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
            StepFlow {
                id: Some("loop".to_owned()),
                branch_targets: vec![0, 2],
                then_target: None,
            },
            StepFlow {
                id: Some("done".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
        ],
    };
    assert!(matches!(
        validate_control_flow(&flow),
        Err(ValidationError::ControlFlowCycle)
    ));
}

#[test]
fn rejects_self_cycle() {
    let flow = WorkflowFlow {
        steps: vec![
            StepFlow {
                id: Some("first".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
            StepFlow {
                id: Some("loop".to_owned()),
                branch_targets: vec![1, 2],
                then_target: None,
            },
            StepFlow {
                id: Some("done".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
        ],
    };
    assert!(matches!(
        validate_control_flow(&flow),
        Err(ValidationError::ControlFlowCycle)
    ));
}

#[test]
fn rejects_unreachable_step() {
    let flow = WorkflowFlow {
        steps: vec![
            StepFlow {
                id: Some("start".to_owned()),
                branch_targets: vec![2, 3],
                then_target: Some(2),
            },
            StepFlow {
                id: Some("skipped".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
            StepFlow {
                id: Some("target".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
            StepFlow {
                id: Some("done".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
        ],
    };
    assert!(matches!(
        validate_control_flow(&flow),
        Err(ValidationError::UnreachableStep { .. })
    ));
}

#[test]
fn rejects_target_out_of_bounds() {
    let flow = WorkflowFlow {
        steps: vec![StepFlow {
            id: Some("start".to_owned()),
            branch_targets: vec![5],
            then_target: None,
        }],
    };
    assert!(matches!(
        validate_control_flow(&flow),
        Err(ValidationError::InvalidThenTarget)
    ));
}

#[test]
fn accepts_single_step() {
    let flow = WorkflowFlow {
        steps: vec![StepFlow {
            id: Some("done".to_owned()),
            branch_targets: vec![],
            then_target: None,
        }],
    };
    assert_eq!(validate_control_flow(&flow), Ok(()));
}

#[test]
fn rejects_empty_workflow() {
    let flow = WorkflowFlow { steps: vec![] };
    assert!(matches!(
        validate_control_flow(&flow),
        Err(ValidationError::UnreachableStep { .. })
    ));
}

// ---------------------------------------------------------------------------
// BDD exact-assertion tests
// ---------------------------------------------------------------------------

#[test]
fn validate_control_flow_accepts_linear_chain() {
    // Given a linear flow of 3 steps
    let flow = linear_flow(3);
    // When validate_control_flow is called
    let result = validate_control_flow(&flow);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_control_flow_accepts_branch_and_merge() {
    // Given a branching flow with merge
    let flow = branching_flow();
    // When validate_control_flow is called
    let result = validate_control_flow(&flow);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_control_flow_rejects_invalid_merge_without_branch() {
    // Given a flow with a backward branch (cycle)
    let flow = WorkflowFlow {
        steps: vec![
            StepFlow {
                id: Some("first".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
            StepFlow {
                id: Some("loop_back".to_owned()),
                branch_targets: vec![0],
                then_target: None,
            },
        ],
    };
    // When validate_control_flow is called
    let result = validate_control_flow(&flow);
    // Then it returns ControlFlowCycle
    assert_eq!(result, Err(ValidationError::ControlFlowCycle));
}

#[test]
fn validate_reachability_returns_ok_for_connected_graph() {
    // Given a linear flow where all steps are reachable
    let flow = linear_flow(4);
    // When validate_reachability is called
    let result = validate_reachability(&flow);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_reachability_returns_error_for_orphan_step() {
    // Given a flow where step 1 is orphaned (step 0 branches to 2 and 3, skipping 1)
    let flow = WorkflowFlow {
        steps: vec![
            StepFlow {
                id: Some("start".to_owned()),
                branch_targets: vec![2, 3],
                then_target: Some(2),
            },
            StepFlow {
                id: Some("orphaned".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
            StepFlow {
                id: Some("target".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
            StepFlow {
                id: Some("done".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
        ],
    };
    // When validate_reachability is called
    let result = validate_reachability(&flow);
    // Then it returns UnreachableStep with the orphan id
    assert_eq!(
        result,
        Err(ValidationError::UnreachableStep {
            step: "orphaned".to_owned(),
        })
    );
}

#[test]
fn validate_forward_only_then_rejects_backward_branch() {
    // Given a flow where a step has a backward branch target
    let flow = WorkflowFlow {
        steps: vec![
            StepFlow {
                id: Some("a".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
            StepFlow {
                id: Some("b".to_owned()),
                branch_targets: vec![0],
                then_target: None,
            },
        ],
    };
    // When validate_forward_only_then is called
    let result = validate_forward_only_then(&flow);
    // Then it returns ControlFlowCycle
    assert_eq!(result, Err(ValidationError::ControlFlowCycle));
}

#[test]
fn validate_forward_only_then_accepts_forward_branch() {
    // Given a flow where all targets point forward
    let flow = WorkflowFlow {
        steps: vec![
            StepFlow {
                id: Some("choose".to_owned()),
                branch_targets: vec![1, 2],
                then_target: None,
            },
            StepFlow {
                id: Some("left".to_owned()),
                branch_targets: vec![],
                then_target: Some(2),
            },
            StepFlow {
                id: Some("done".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
        ],
    };
    // When validate_forward_only_then is called
    let result = validate_forward_only_then(&flow);
    // Then it returns Ok
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_control_flow_rejects_out_of_bounds_target_exact() {
    // Given a flow with a branch target beyond step count
    let flow = WorkflowFlow {
        steps: vec![StepFlow {
            id: Some("start".to_owned()),
            branch_targets: vec![99],
            then_target: None,
        }],
    };
    // When validate_control_flow is called
    let result = validate_control_flow(&flow);
    // Then it returns InvalidThenTarget
    assert_eq!(result, Err(ValidationError::InvalidThenTarget));
}

#[test]
fn validate_control_flow_rejects_empty_steps_exact() {
    // Given a flow with no steps
    let flow = WorkflowFlow { steps: vec![] };
    // When validate_control_flow is called
    let result = validate_control_flow(&flow);
    // Then it returns UnreachableStep with message about no steps
    assert_eq!(
        result,
        Err(ValidationError::UnreachableStep {
            step: "workflow has no steps".to_owned(),
        })
    );
}

// ---------------------------------------------------------------------------
// Adversarial BDD tests: validation bypass attacks
// ---------------------------------------------------------------------------

#[test]
fn adversarial_three_step_cycle_a_to_b_to_c_to_a_is_rejected() {
    // Given a three-step cycle: step_0 branches to step_2, step_1 branches to step_0, step_2 branches to step_1
    let flow = WorkflowFlow {
        steps: vec![
            StepFlow {
                id: Some("a".to_owned()),
                branch_targets: vec![2],
                then_target: None,
            },
            StepFlow {
                id: Some("b".to_owned()),
                branch_targets: vec![0],
                then_target: None,
            },
            StepFlow {
                id: Some("c".to_owned()),
                branch_targets: vec![1],
                then_target: None,
            },
        ],
    };
    // When validate_control_flow is called
    let result = validate_control_flow(&flow);
    // Then it returns ControlFlowCycle (E0302) -- step 0 has backward branch from step 1
    assert_eq!(result, Err(ValidationError::ControlFlowCycle));
}

#[test]
fn adversarial_then_target_pointing_to_self_is_rejected() {
    // Given a step whose then_target points to itself (self-loop)
    let flow = WorkflowFlow {
        steps: vec![StepFlow {
            id: Some("loop_to_self".to_owned()),
            branch_targets: vec![],
            then_target: Some(0), // points to itself
        }],
    };
    // When validate_forward_only_then is called
    let result = validate_forward_only_then(&flow);
    // Then it returns ControlFlowCycle (E0302)
    assert_eq!(result, Err(ValidationError::ControlFlowCycle));
}

#[test]
fn adversarial_then_target_out_of_bounds_is_rejected() {
    // Given a step whose then_target is beyond the step count
    let flow = WorkflowFlow {
        steps: vec![
            StepFlow {
                id: Some("first".to_owned()),
                branch_targets: vec![],
                then_target: Some(99), // way out of bounds
            },
            StepFlow {
                id: Some("second".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
        ],
    };
    // When validate_forward_only_then is called
    let result = validate_forward_only_then(&flow);
    // Then it returns InvalidThenTarget (E0301)
    assert_eq!(result, Err(ValidationError::InvalidThenTarget));
}

#[test]
fn adversarial_backward_then_target_is_rejected() {
    // Given a step whose then_target points backward
    let flow = WorkflowFlow {
        steps: vec![
            StepFlow {
                id: Some("start".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
            StepFlow {
                id: Some("go_back".to_owned()),
                branch_targets: vec![],
                then_target: Some(0), // backward
            },
        ],
    };
    // When validate_forward_only_then is called
    let result = validate_forward_only_then(&flow);
    // Then it returns ControlFlowCycle (E0302)
    assert_eq!(result, Err(ValidationError::ControlFlowCycle));
}

#[test]
fn adversarial_large_branch_target_out_of_bounds_is_rejected() {
    // Given a step with a branch target far exceeding step count
    let flow = WorkflowFlow {
        steps: vec![StepFlow {
            id: Some("big_jump".to_owned()),
            branch_targets: vec![1000],
            then_target: None,
        }],
    };
    // When validate_control_flow is called
    let result = validate_control_flow(&flow);
    // Then it returns InvalidThenTarget (E0301)
    assert_eq!(result, Err(ValidationError::InvalidThenTarget));
}

#[test]
fn adversarial_isolated_step_not_reachable_from_entry_is_rejected() {
    // Given a flow where step 0 has explicit then_target skipping step 1
    // and step 1 has no incoming edges
    let flow = WorkflowFlow {
        steps: vec![
            StepFlow {
                id: Some("start".to_owned()),
                branch_targets: vec![],
                then_target: Some(2), // skips step 1
            },
            StepFlow {
                id: Some("orphan".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
            StepFlow {
                id: Some("target".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
        ],
    };
    // When validate_control_flow is called
    let result = validate_control_flow(&flow);
    // Then it returns UnreachableStep (E0303) for "orphan"
    assert_eq!(
        result,
        Err(ValidationError::UnreachableStep {
            step: "orphan".to_owned(),
        })
    );
}

#[test]
fn adversarial_step_without_id_orphan_reports_generic_name() {
    // Given a flow where an orphaned step has no id set
    let flow = WorkflowFlow {
        steps: vec![
            StepFlow {
                id: Some("start".to_owned()),
                branch_targets: vec![],
                then_target: Some(2),
            },
            StepFlow {
                id: None, // no id
                branch_targets: vec![],
                then_target: None,
            },
            StepFlow {
                id: Some("end".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
        ],
    };
    // When validate_control_flow is called
    let result = validate_control_flow(&flow);
    // Then it returns UnreachableStep with a generated name "step_1"
    assert_eq!(
        result,
        Err(ValidationError::UnreachableStep {
            step: "step_1".to_owned(),
        })
    );
}

#[test]
fn adversarial_multiple_branches_all_forward_are_accepted() {
    // Given a step with multiple forward branch targets, all valid
    let flow = WorkflowFlow {
        steps: vec![
            StepFlow {
                id: Some("router".to_owned()),
                branch_targets: vec![1, 2, 3],
                then_target: None,
            },
            StepFlow {
                id: Some("branch_a".to_owned()),
                branch_targets: vec![],
                then_target: Some(3),
            },
            StepFlow {
                id: Some("branch_b".to_owned()),
                branch_targets: vec![],
                then_target: Some(3),
            },
            StepFlow {
                id: Some("merge".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
        ],
    };
    // When validate_control_flow is called
    let result = validate_control_flow(&flow);
    // Then it returns Ok -- all targets are valid and forward
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_zero_target_is_backward_from_step_zero() {
    // Given step 0 with a branch target of 0 (self-cycle via branch)
    let flow = WorkflowFlow {
        steps: vec![StepFlow {
            id: Some("self_loop".to_owned()),
            branch_targets: vec![0],
            then_target: None,
        }],
    };
    // When validate_control_flow is called
    let result = validate_control_flow(&flow);
    // Then it returns ControlFlowCycle (E0302) -- target <= step_index
    assert_eq!(result, Err(ValidationError::ControlFlowCycle));
}

#[test]
fn adversarial_exact_boundary_target_equals_step_count_is_rejected() {
    // Given a step with branch target exactly equal to steps.len()
    let flow = WorkflowFlow {
        steps: vec![
            StepFlow {
                id: Some("start".to_owned()),
                branch_targets: vec![2], // but only 2 steps (indices 0 and 1)
                then_target: None,
            },
            StepFlow {
                id: Some("second".to_owned()),
                branch_targets: vec![],
                then_target: None,
            },
        ],
    };
    // When validate_control_flow is called
    let result = validate_control_flow(&flow);
    // Then it returns InvalidThenTarget (E0301) -- index 2 out of bounds
    assert_eq!(result, Err(ValidationError::InvalidThenTarget));
}
