#[test]
fn budget_simple_linear_workflow() {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(1)),
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(2)),
        ),
        node(
            StepIdx::new(2),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(3, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 3 && b.max_fanout == 0 && b.max_nesting_depth == 0);

    assert!(budget.is_some(), "linear workflow budget mismatch");
}

#[test]
fn budget_branching_workflow() {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::ChooseSlot {
                branches: vec![
                    SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(2),
                    },
                ]
                .into_boxed_slice(),
                otherwise: Some(StepIdx::new(3)).into(),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(2),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(3),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(4, 1);
    match WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract) {
        Ok(actual) => {
            assert_eq!(
                actual.max_total_steps, 2,
                "conditional path uses max branch cost"
            );
            assert_eq!(
                actual.max_fanout, 2,
                "choose branch fanout is counted exactly"
            );
        }
        Err(error) => panic!("expected branching budget, got {error:?}"),
    }
}

#[test]
fn budget_nested_loop_depth() {
    let nodes = vec![
        node(
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
        node(
            StepIdx::new(1),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(2),
                item_slot: SlotIdx::new(3),
                limit: 10.into(),
                body: StepIdx::new(2),
                done: StepIdx::new(3),
            },
            None,
            None,
        ),
        node(StepIdx::new(2), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(3),
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(4),
            },
            Some(SlotIdx::new(4)),
            None,
        ),
        node(
            StepIdx::new(4),
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(5),
            },
            Some(SlotIdx::new(5)),
            None,
        ),
        node(
            StepIdx::new(5),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(4),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(6, 6);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_nesting_depth == 2);

    assert!(budget.is_some(), "nested loop depth mismatch");
}

#[test]
fn whole_workflow_budget_multiplies_nested_fanout_loop_body_steps() -> Result<(), String> {
    let nodes = nested_fanout_loop_nodes();
    let contract = test_contract(8, 8);

    let actual = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|error| format!("unexpected budget error: {error:?}"))?;

    ensure_equal(actual.max_total_steps, 11)?;
    ensure_equal(actual.max_fanout, 2)?;
    ensure_equal(actual.max_for_each_iterations, 2)?;
    ensure_equal(actual.max_together_branches, 2)
}

#[test]
fn whole_workflow_budget_accumulates_sequential_collect_reduce_repeat_and_resources()
-> Result<(), String> {
    let nodes = sequential_collect_reduce_repeat_wait_nodes();
    let contract = ResourceContract {
        max_collect_items: 3,
        max_retry_attempts: 4,
        max_output_bytes: 123,
        max_blob_bytes: 456,
        max_ipc_payload_bytes: 78,
        max_queue_depth: 9,
        max_journal_batch_bytes: 10,
        max_input_bytes: 11,
        ..test_contract(12, 12)
    };

    let actual = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|error| format!("unexpected budget error: {error:?}"))?;

    ensure_equal(actual.max_gather_pages, 3)?;
    ensure_equal(actual.max_gather_items, 3)?;
    ensure_equal(actual.max_repeat_attempts, 2)?;
    ensure_equal(actual.max_timer_entries, 1)?;
    ensure_equal(actual.max_result_bytes, 123)?;
    ensure_equal(actual.max_blob_bytes, 456)?;
    ensure_equal(actual.max_ipc_payload_bytes, 78)?;
    ensure_equal(actual.max_queue_depth, 9)?;
    ensure_equal(actual.max_journal_batch_bytes, 10)?;
    ensure_equal(actual.max_input_bytes, 11)
}

#[test]
fn whole_workflow_budget_uses_conditional_max_instead_of_branch_sum() -> Result<(), String> {
    let nodes = conditional_max_nodes();
    let contract = test_contract(8, 4);

    let actual = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|error| format!("unexpected budget error: {error:?}"))?;

    ensure_equal(actual.max_total_steps, 4)?;
    ensure_equal(actual.max_fanout, 2)
}

/// Regression: a Choose branch that enters a `ForEachStart` loop must
/// multiply the loop body by the iteration limit in the branch's worst-case
/// step count. Previously, branch counts used a path-counting helper that
/// ignored loop headers, so a 1 + (10 * 1) + 1 + 1 branch was reported as
/// 5 steps instead of 14 (1 Choose + 1 ForEachStart + 10 body
/// iterations + 1 Finish + 1 done-continuation node 3).
#[test]
fn choose_branch_accounts_for_nested_for_each_loop_iterations() -> Result<(), String> {
    // Workflow:
    //   0: Choose { branch0 -> 1, otherwise -> 3 }
    //   1: ForEachStart { limit = 10, body = 4, done = 2 }
    //   2: Finish
    //   3: Nop
    //   4: Nop
    // Branch 0 path worst case = 1 (Choose) + 1 (ForEachStart)
    //                          + 10 * 1 (body iterations) + 1 (Finish)
    //                          + 1 (push_done_continuation pushes node 3) = 14.
    // Branch otherwise path = 1 (Choose) + 1 (Nop) = 2.
    // Overall max_total_steps = 14.
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::Choose {
                branches: vec![compiled_node_kind_expr_branch(0, 1)].into_boxed_slice(),
                otherwise: Some(StepIdx::new(3)).into(),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10.into(),
                body: StepIdx::new(4),
                done: StepIdx::new(2),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(2),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
        test_node(3, None, CompiledNodeKind::Nop),
        test_node(4, None, CompiledNodeKind::Nop),
    ];
    let contract = test_contract(5, 2);
    let actual = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .map_err(|error| format!("unexpected budget error: {error:?}"))?;

    ensure_equal(actual.max_total_steps, 14)?;
    ensure_equal(actual.max_for_each_iterations, 10)
}

fn compiled_node_kind_expr_branch(condition: u16, target: u16) -> crate::workflow::ExprBranch {
    crate::workflow::ExprBranch {
        condition: ExprIdx::new(condition),
        target: StepIdx::new(target),
    }
}

#[test]
fn whole_workflow_budget_rejects_unbounded_jump_cycle_with_exact_error() -> Result<(), String> {
    let nodes = vec![node(
        StepIdx::new(0),
        CompiledNodeKind::Jump {
            target: StepIdx::new(0),
        },
        None,
        None,
    )];
    let contract = test_contract(1, 1);

    match WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract) {
        Err(WorkflowError::JumpCycle { step, target }) => {
            ensure_equal(step, StepIdx::new(0))?;
            ensure_equal(target, StepIdx::new(0))
        }
        other => Err(format!("expected JumpCycle for self jump, got {other:?}")),
    }
}

#[test]
fn budget_rejects_excessive_steps() {
    let budget = test_budget(3, 10, 1, 0);
    let policy = test_policy(2, 10, 64, 8);

    match policy.validate(&budget) {
        Err(BudgetError::TotalStepsExceeded {
            actual: 3,
            limit: 2,
        }) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn budget_rejects_excessive_fanout() {
    let budget = test_budget(1, 10, 3, 0);
    let policy = test_policy(1_000_000, 65_535, 2, 8);

    match policy.validate(&budget) {
        Err(BudgetError::FanoutExceeded {
            actual: 3,
            limit: 2,
        }) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn budget_accepts_within_policy() {
    let budget = test_budget(10, 100, 4, 2);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    assert_eq!(result, Ok(()));
}

#[test]
fn budget_rejects_excessive_nesting_depth() {
    let budget = test_budget(1, 10, 1, 10);
    let policy = test_policy(1_000_000, 65_535, 64, 4);

    match policy.validate(&budget) {
        Err(BudgetError::NestingDepthExceeded {
            actual: 10,
            limit: 4,
        }) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn budget_rejects_excessive_total_slots() {
    let budget = test_budget(1, 200_000, 1, 0);
    let policy = test_policy(1_000_000, 65_535, 64, 8);

    match policy.validate(&budget) {
        Err(BudgetError::TotalSlotsExceeded {
            actual: 200_000,
            limit: 65_535,
        }) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn budget_default_policy_accepts_reasonable_budget() {
    let budget = test_budget(500_000, 10_000, 32, 4);
    let result = BoundednessPolicy::DEFAULT.validate(&budget);
    assert_eq!(result, Ok(()));
}

#[test]
fn budget_compute_rejects_entry_out_of_bounds() {
    let nodes = vec![node(StepIdx::new(0), CompiledNodeKind::Nop, None, None)];
    let contract = test_contract(1, 0);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(5), &contract);

    match result {
        Err(WorkflowError::EntryOutOfBounds { entry }) if entry == StepIdx::new(5) => {}
        other => panic!("unexpected result: {other:?}"),
    }
}

#[test]
fn budget_together_start_fanout() {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(1), StepIdx::new(2), StepIdx::new(3)]
                    .into_boxed_slice(),
                join: StepIdx::new(4),
            },
            None,
            None,
        ),
        node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
        node(StepIdx::new(2), CompiledNodeKind::Nop, None, None),
        node(StepIdx::new(3), CompiledNodeKind::Nop, None, None),
        node(
            StepIdx::new(4),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(5, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_fanout == 3 && b.max_total_steps == 5);

    assert!(budget.is_some(), "together start fanout mismatch");
}

#[test]
fn budget_single_node_workflow() {
    let nodes = vec![node(
        StepIdx::new(0),
        CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
        None,
        None,
    )];
    let contract = test_contract(1, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_total_steps == 1 && b.max_fanout == 0 && b.max_nesting_depth == 0);

    assert!(budget.is_some(), "single-node workflow budget mismatch");
}

#[test]
fn budget_error_display_formatting() {
    let err = BudgetError::TotalStepsExceeded {
        actual: 5,
        limit: 3,
    };
    assert_eq!(format!("{err}"), "total steps exceeded: 5 > 3");

    let err = BudgetError::TotalSlotsExceeded {
        actual: 200,
        limit: 100,
    };
    assert_eq!(format!("{err}"), "total slots exceeded: 200 > 100");

    let err = BudgetError::FanoutExceeded {
        actual: 10,
        limit: 4,
    };
    assert_eq!(format!("{err}"), "fanout exceeded: 10 > 4");

    let err = BudgetError::NestingDepthExceeded {
        actual: 16,
        limit: 8,
    };
    assert_eq!(format!("{err}"), "nesting depth exceeded: 16 > 8");

    let err = BudgetError::ParallelExceeded {
        actual: 128,
        limit: 64,
    };
    assert_eq!(format!("{err}"), "parallel exceeded: 128 > 64");

    let err = BudgetError::ActionTicketsExceeded {
        actual: 200_000,
        limit: 100_000,
    };
    assert_eq!(format!("{err}"), "action tickets exceeded: 200000 > 100000");

    let err = BudgetError::RunTimeExceeded {
        actual: 3_000_000,
        limit: 2_592_000,
    };
    assert_eq!(format!("{err}"), "run time exceeded: 3000000 > 2592000");

    let err = BudgetError::ResultBytesExceeded {
        actual: 524_288,
        limit: 262_144,
    };
    assert_eq!(format!("{err}"), "result bytes exceeded: 524288 > 262144");

    let err = BudgetError::StepsExecutableExceeded {
        actual: 2_000_000,
        limit: 1_000_000,
    };
    assert_eq!(
        format!("{err}"),
        "steps executable exceeded: 2000000 > 1000000"
    );
}

#[test]
fn budget_step_count_overflow_detected() {
    // Construct a workflow where a node's next points out of bounds,
    // verifying error propagation through the count path.
    let nodes = vec![node(
        StepIdx::new(0),
        CompiledNodeKind::Nop,
        None,
        Some(StepIdx::new(99)),
    )];
    let contract = test_contract(1, 0);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Err(WorkflowError::StepOutOfBounds { .. }) => {}
        other => panic!("expected StepOutOfBounds, got {other:?}"),
    }
}

#[test]
fn budget_empty_nodes_rejected() {
    let nodes: Vec<CompiledNode> = vec![];
    let contract = test_contract(0, 0);
    let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
    match result {
        Err(WorkflowError::EntryOutOfBounds { .. }) => {}
        other => panic!("expected EntryOutOfBounds for empty nodes, got {other:?}"),
    }
}

#[test]
fn budget_choose_fanout_counted() {
    let nodes = vec![
        node(
            StepIdx::new(0),
            CompiledNodeKind::Choose {
                branches: vec![
                    ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    ExprBranch {
                        condition: ExprIdx::new(0),
                        target: StepIdx::new(2),
                    },
                ]
                .into_boxed_slice(),
                otherwise: None.into(),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(2),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let contract = test_contract(3, 1);
    let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
        .ok()
        .filter(|b| b.max_fanout == 2 && b.max_total_steps == 2);

    assert!(budget.is_some(), "choose fanout budget mismatch");
}
