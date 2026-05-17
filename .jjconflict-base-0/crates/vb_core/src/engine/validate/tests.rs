//! Tests for workflow validation.

use crate::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::ConstValue;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, ExprBranch, ResourceContract, SlotBranch, WorkflowError,
    WorkflowParts,
};

use crate::engine::validate::{
    validate_compiled_workflow, validate_node_bounds, validate_resource_contract,
    validate_transition_target,
};

fn valid_parts() -> WorkflowParts {
    WorkflowParts {
        name: Box::<str>::from("test"),
        digest: WorkflowDigest::from_bytes([0x00; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

#[allow(dead_code)]
fn small_contract() -> ResourceContract {
    ResourceContract {
        max_steps: 10,
        max_slots: 10,
        max_constants: 10,
        max_accessors: 10,
        max_expressions: 10,
        max_expr_stack: 10,
        max_step_budget_per_tick: 10,
        max_transitions_per_tick: 10,
        max_input_bytes: 100,
        max_output_bytes: 100,
        max_blob_bytes: 100,
        max_ipc_payload_bytes: 100,
        max_retry_attempts: 3,
        max_fanout: 4,
        max_collect_items: 10,
        max_queue_depth: 10,
        max_journal_batch_bytes: 100,
    }
}

// =========================================================================
// validate_compiled_workflow
// =========================================================================

#[test]
fn validate_compiled_workflow_accepts_valid_parts() {
    let parts = valid_parts();
    let result = validate_compiled_workflow(&parts);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_compiled_workflow_rejects_empty_nodes() {
    let parts = WorkflowParts {
        nodes: Box::new([]),
        ..valid_parts()
    };
    let result = validate_compiled_workflow(&parts);
    assert_eq!(result, Err(WorkflowError::EmptyNodes));
}

#[test]
fn validate_compiled_workflow_rejects_entry_out_of_bounds() {
    let parts = WorkflowParts {
        entry: StepIdx::new(99),
        ..valid_parts()
    };
    let result = validate_compiled_workflow(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::EntryOutOfBounds {
            entry: StepIdx::new(99)
        })
    );
}

#[test]
fn validate_compiled_workflow_rejects_node_id_mismatch() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(5), // should be 0
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_compiled_workflow(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::NodeIdMismatch {
            expected: StepIdx::new(0),
            actual: StepIdx::new(5),
        })
    );
}

#[test]
fn validate_compiled_workflow_rejects_unreachable_node() {
    let parts = WorkflowParts {
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
        ]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_compiled_workflow(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::UnreachableNode {
            step: StepIdx::new(1)
        })
    );
}

// =========================================================================
// validate_resource_contract
// =========================================================================

#[test]
fn validate_resource_contract_accepts_default_contract() {
    let parts = valid_parts();
    let result = validate_resource_contract(&parts);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_resource_contract_rejects_max_steps_over_limit() {
    let mut parts = valid_parts();
    parts.resource_contract.max_steps = u16::MAX;
    let limit = crate::limits::MAX_STEPS_PER_WORKFLOW;
    if u16::MAX as usize > limit {
        let result = validate_resource_contract(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractTooLarge {
                resource: "max_steps",
            })
        );
    }
}

#[test]
fn validate_resource_contract_rejects_max_slots_over_limit() {
    let mut parts = valid_parts();
    parts.resource_contract.max_slots = u16::MAX;
    let limit = crate::limits::MAX_SLOTS_PER_WORKFLOW;
    if u16::MAX as usize > limit {
        let result = validate_resource_contract(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractTooLarge {
                resource: "max_slots",
            })
        );
    }
}

#[test]
fn validate_resource_contract_rejects_max_constants_over_limit() {
    let mut parts = valid_parts();
    parts.resource_contract.max_constants = u16::MAX;
    let limit = crate::limits::MAX_CONSTANTS;
    if u16::MAX as usize > limit {
        let result = validate_resource_contract(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractTooLarge {
                resource: "max_constants",
            })
        );
    }
}

#[test]
fn validate_resource_contract_rejects_max_accessors_over_limit() {
    let mut parts = valid_parts();
    parts.resource_contract.max_accessors = u16::MAX;
    let limit = crate::limits::MAX_ACCESSORS;
    if u16::MAX as usize > limit {
        let result = validate_resource_contract(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractTooLarge {
                resource: "max_accessors",
            })
        );
    }
}

#[test]
fn validate_resource_contract_rejects_max_expressions_over_limit() {
    let mut parts = valid_parts();
    parts.resource_contract.max_expressions = u16::MAX;
    let limit = crate::limits::MAX_EXPRESSIONS;
    if u16::MAX as usize > limit {
        let result = validate_resource_contract(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractTooLarge {
                resource: "max_expressions",
            })
        );
    }
}

#[test]
fn validate_resource_contract_rejects_max_expr_stack_over_limit() {
    let mut parts = valid_parts();
    parts.resource_contract.max_expr_stack = u8::MAX;
    let limit = crate::limits::MAX_EXPRESSION_STACK;
    if u8::MAX > limit {
        let result = validate_resource_contract(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractTooLarge {
                resource: "max_expr_stack",
            })
        );
    }
}

// =========================================================================
// validate_node_bounds
// =========================================================================

#[test]
fn validate_node_bounds_accepts_valid_linear_chain() {
    let parts = WorkflowParts {
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_node_bounds(&parts);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_node_bounds_rejects_node_id_out_of_bounds() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_node_bounds(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(5)
        })
    );
}

#[test]
fn validate_node_bounds_rejects_next_step_out_of_bounds() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(99)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_node_bounds(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(99)
        })
    );
}

#[test]
fn validate_node_bounds_accepts_single_node_with_no_next() {
    let parts = valid_parts();
    let result = validate_node_bounds(&parts);
    assert_eq!(result, Ok(()));
}

// =========================================================================
// validate_transition_target
// =========================================================================

#[test]
fn validate_transition_target_accepts_valid_linear_chain() {
    let parts = WorkflowParts {
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_transition_target_rejects_jump_to_out_of_bounds() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Jump {
                target: StepIdx::new(99),
            },
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(99)
        })
    );
}

#[test]
fn validate_transition_target_rejects_choose_branch_out_of_bounds() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Choose {
                branches: vec![ExprBranch {
                    condition: ExprIdx::new(0),
                    target: StepIdx::new(50),
                }]
                .into_boxed_slice(),
                otherwise: None,
            },
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(50)
        })
    );
}

#[test]
fn validate_transition_target_rejects_choose_otherwise_out_of_bounds() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Choose {
                branches: vec![].into_boxed_slice(),
                otherwise: Some(StepIdx::new(77)),
            },
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(77)
        })
    );
}

#[test]
fn validate_transition_target_rejects_choose_slot_branch_out_of_bounds() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ChooseSlot {
                branches: vec![SlotBranch {
                    condition: SlotIdx::new(0),
                    target: StepIdx::new(50),
                }]
                .into_boxed_slice(),
                otherwise: None,
            },
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(50)
        })
    );
}

#[test]
fn validate_transition_target_rejects_choose_slot_otherwise_out_of_bounds() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ChooseSlot {
                branches: vec![].into_boxed_slice(),
                otherwise: Some(StepIdx::new(88)),
            },
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(88)
        })
    );
}

#[test]
fn validate_transition_target_rejects_for_each_start_body_out_of_bounds() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(99),
                done: StepIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(99)
        })
    );
}

#[test]
fn validate_transition_target_rejects_for_each_start_done_out_of_bounds() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(0),
                done: StepIdx::new(99),
            },
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(99)
        })
    );
}

#[test]
fn validate_transition_target_rejects_together_start_branch_out_of_bounds() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(77)].into_boxed_slice(),
                join: StepIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(77)
        })
    );
}

#[test]
fn validate_transition_target_rejects_together_start_join_out_of_bounds() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(0)].into_boxed_slice(),
                join: StepIdx::new(55),
            },
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(55)
        })
    );
}

#[test]
fn validate_transition_target_rejects_together_branch_entry_out_of_bounds() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherBranch {
                branch: 0,
                entry: StepIdx::new(66),
                join: StepIdx::new(0),
                accumulator: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(66)
        })
    );
}

#[test]
fn validate_transition_target_rejects_together_branch_join_out_of_bounds() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherBranch {
                branch: 0,
                entry: StepIdx::new(0),
                join: StepIdx::new(44),
                accumulator: SlotIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(44)
        })
    );
}

#[test]
fn validate_transition_target_rejects_repeat_check_done_out_of_bounds() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatCheck {
                attempt_slot: SlotIdx::new(0),
                done: StepIdx::new(33),
            },
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(33)
        })
    );
}

#[test]
fn validate_transition_target_rejects_retry_check_body_out_of_bounds() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::new(0),
                body: StepIdx::new(22),
                exhausted: StepIdx::new(0),
            },
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(22)
        })
    );
}

#[test]
fn validate_transition_target_rejects_error_handler_body_out_of_bounds() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(11),
                handler: StepIdx::new(0),
                error_slot: None,
            },
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(11)
        })
    );
}

#[test]
fn validate_transition_target_rejects_error_handler_handler_out_of_bounds() {
    let parts = WorkflowParts {
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(0),
                handler: StepIdx::new(11),
                error_slot: None,
            },
        }]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(
        result,
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(11)
        })
    );
}

#[test]
fn validate_transition_target_accepts_valid_jump() {
    let parts = WorkflowParts {
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Jump {
                    target: StepIdx::new(1),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_transition_target_accepts_valid_for_each_start() {
    let parts = WorkflowParts {
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 10,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_transition_target_accepts_valid_together_start() {
    let parts = WorkflowParts {
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherStart {
                    branches: vec![StepIdx::new(1), StepIdx::new(2)].into_boxed_slice(),
                    join: StepIdx::new(3),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        ..valid_parts()
    };
    let result = validate_transition_target(&parts);
    assert_eq!(result, Ok(()));
}

// =========================================================================
// Combined validation: all validators agree on valid input
// =========================================================================

#[test]
fn all_validators_accept_minimal_valid_parts() {
    let parts = valid_parts();
    assert_eq!(validate_compiled_workflow(&parts), Ok(()));
    assert_eq!(validate_resource_contract(&parts), Ok(()));
    assert_eq!(validate_node_bounds(&parts), Ok(()));
    assert_eq!(validate_transition_target(&parts), Ok(()));
}

#[test]
fn all_validators_accept_three_node_linear_chain() {
    let parts = WorkflowParts {
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]
        .into_boxed_slice(),
        constants: vec![ConstValue::I64(42)].into_boxed_slice(),
        ..valid_parts()
    };
    assert_eq!(validate_compiled_workflow(&parts), Ok(()));
    assert_eq!(validate_resource_contract(&parts), Ok(()));
    assert_eq!(validate_node_bounds(&parts), Ok(()));
    assert_eq!(validate_transition_target(&parts), Ok(()));
}
