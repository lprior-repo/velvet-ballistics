//! Tests for workflow validation.

use crate::engine::validate::{
    validate_compiled_workflow, validate_node_bounds, validate_resource_contract,
    validate_transition_target,
};
use crate::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::ConstValue;
use crate::workflow::{
    CompiledNodeKind, ContractResource, ExprBranch, ResourceContract, SlotBranch, WorkflowError,
    WorkflowParts, node,
};

fn valid_parts() -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("test"),
        digest: WorkflowDigest::from_bytes([0x00; 32]),
        nodes: vec![node(
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
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
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
        semantic_source_identity: None,
        nodes: Box::new([]),
        ..valid_parts()
    };
    let result = validate_compiled_workflow(&parts);
    assert_eq!(result, Err(WorkflowError::EmptyNodes));
}

#[test]
fn validate_compiled_workflow_rejects_entry_out_of_bounds() {
    let parts = WorkflowParts {
        semantic_source_identity: None,
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
        semantic_source_identity: None,
        nodes: vec![node(StepIdx::new(5), CompiledNodeKind::Nop, None, None)].into_boxed_slice(),
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
        semantic_source_identity: None,
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
            node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
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
    if usize::from(u16::MAX) > limit {
        let result = validate_resource_contract(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractTooLarge {
                resource: ContractResource::Steps,
            })
        );
    }
}

#[test]
fn validate_resource_contract_rejects_max_slots_over_limit() {
    let mut parts = valid_parts();
    parts.resource_contract.max_slots = u16::MAX;
    let limit = crate::limits::MAX_SLOTS_PER_WORKFLOW;
    if usize::from(u16::MAX) > limit {
        let result = validate_resource_contract(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractTooLarge {
                resource: ContractResource::Slots,
            })
        );
    }
}

#[test]
fn validate_resource_contract_rejects_max_constants_over_limit() {
    let mut parts = valid_parts();
    parts.resource_contract.max_constants = u16::MAX;
    let limit = crate::limits::MAX_CONSTANTS;
    if usize::from(u16::MAX) > limit {
        let result = validate_resource_contract(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractTooLarge {
                resource: ContractResource::Constants,
            })
        );
    }
}

#[test]
fn validate_resource_contract_rejects_max_accessors_over_limit() {
    let mut parts = valid_parts();
    parts.resource_contract.max_accessors = u16::MAX;
    let limit = crate::limits::MAX_ACCESSORS;
    if usize::from(u16::MAX) > limit {
        let result = validate_resource_contract(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractTooLarge {
                resource: ContractResource::Accessors,
            })
        );
    }
}

#[test]
fn validate_resource_contract_rejects_max_expressions_over_limit() {
    let mut parts = valid_parts();
    parts.resource_contract.max_expressions = u16::MAX;
    let limit = crate::limits::MAX_EXPRESSIONS;
    if usize::from(u16::MAX) > limit {
        let result = validate_resource_contract(&parts);
        assert_eq!(
            result,
            Err(WorkflowError::ResourceContractTooLarge {
                resource: ContractResource::Expressions,
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
                resource: ContractResource::ExprStack,
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
        semantic_source_identity: None,
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
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
        semantic_source_identity: None,
        nodes: vec![node(StepIdx::new(5), CompiledNodeKind::Nop, None, None)].into_boxed_slice(),
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
        semantic_source_identity: None,
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(99)),
        )]
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
        semantic_source_identity: None,
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Nop,
                None,
                Some(StepIdx::new(1)),
            ),
            node(
                StepIdx::new(1),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
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
        semantic_source_identity: None,
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::Jump {
                target: StepIdx::new(99),
            },
            None,
            None,
        )]
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
        semantic_source_identity: None,
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::Choose {
                branches: vec![ExprBranch {
                    condition: ExprIdx::new(0),
                    target: StepIdx::new(50),
                }]
                .into_boxed_slice(),
                otherwise: None.into(),
            },
            None,
            None,
        )]
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
        semantic_source_identity: None,
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::Choose {
                branches: vec![].into_boxed_slice(),
                otherwise: Some(StepIdx::new(77)).into(),
            },
            None,
            None,
        )]
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
        semantic_source_identity: None,
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::ChooseSlot {
                branches: vec![SlotBranch {
                    condition: SlotIdx::new(0),
                    target: StepIdx::new(50),
                }]
                .into_boxed_slice(),
                otherwise: None.into(),
            },
            None,
            None,
        )]
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
        semantic_source_identity: None,
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::ChooseSlot {
                branches: vec![].into_boxed_slice(),
                otherwise: Some(StepIdx::new(88)).into(),
            },
            None,
            None,
        )]
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
        semantic_source_identity: None,
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10.into(),
                body: StepIdx::new(99),
                done: StepIdx::new(0),
            },
            None,
            None,
        )]
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
        semantic_source_identity: None,
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10.into(),
                body: StepIdx::new(0),
                done: StepIdx::new(99),
            },
            None,
            None,
        )]
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
        semantic_source_identity: None,
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(77)].into_boxed_slice(),
                join: StepIdx::new(0),
            },
            None,
            None,
        )]
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
        semantic_source_identity: None,
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(0)].into_boxed_slice(),
                join: StepIdx::new(55),
            },
            None,
            None,
        )]
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
        semantic_source_identity: None,
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::TogetherBranch {
                branch: 0.into(),
                entry: StepIdx::new(66),
                join: StepIdx::new(0),
                accumulator: SlotIdx::new(0),
            },
            None,
            None,
        )]
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
        semantic_source_identity: None,
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::TogetherBranch {
                branch: 0.into(),
                entry: StepIdx::new(0),
                join: StepIdx::new(44),
                accumulator: SlotIdx::new(0),
            },
            None,
            None,
        )]
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
        semantic_source_identity: None,
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::RepeatCheck {
                attempt_slot: SlotIdx::new(0),
                done: StepIdx::new(33),
            },
            None,
            None,
        )]
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
        semantic_source_identity: None,
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::new(0),
                body: StepIdx::new(22),
                exhausted: StepIdx::new(0),
            },
            None,
            None,
        )]
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
        semantic_source_identity: None,
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(11),
                handler: StepIdx::new(0),
                error_slot: None.into(),
            },
            None,
            None,
        )]
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
        semantic_source_identity: None,
        nodes: vec![node(
            StepIdx::new(0),
            CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(0),
                handler: StepIdx::new(11),
                error_slot: None.into(),
            },
            None,
            None,
        )]
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

// --- Valid transition targets pass validation ---

#[test]
fn validate_transition_target_accepts_valid_jump() {
    let parts = WorkflowParts {
        semantic_source_identity: None,
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::Jump {
                    target: StepIdx::new(1),
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
        semantic_source_identity: None,
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 10.into(),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
                None,
                None,
            ),
            node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
            node(
                StepIdx::new(2),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
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
        semantic_source_identity: None,
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::TogetherStart {
                    branches: vec![StepIdx::new(1), StepIdx::new(2)].into_boxed_slice(),
                    join: StepIdx::new(3),
                },
                None,
                None,
            ),
            node(StepIdx::new(1), CompiledNodeKind::Nop, None, None),
            node(StepIdx::new(2), CompiledNodeKind::Nop, None, None),
            node(
                StepIdx::new(3),
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
                None,
                None,
            ),
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
        semantic_source_identity: None,
        nodes: vec![
            node(
                StepIdx::new(0),
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(SlotIdx::new(0)),
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

// RED PHASE: additional resource contract tests
mod red_phase_behavior_tests;
mod red_phase_cycle_detection_tests;
mod red_phase_tests;
// vb-x46ae: differential proof for the unified resource-contract validator.
mod differential_contract_tests;
