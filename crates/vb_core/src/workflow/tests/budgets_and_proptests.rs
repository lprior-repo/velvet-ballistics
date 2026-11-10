#[test]
fn compiled_workflow_slot_count_returns_correct_value() -> Result<(), String> {
    let mut parts = finish_const_parts_with(resource_contract(1, 5, 1, 0, 0), Box::new([]));
    parts.slot_count = 5;
    let workflow = CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;

    if workflow.slot_count() != 5 {
        return Err(format!("expected 5, got {}", workflow.slot_count()));
    }
    Ok(())
}

#[test]
fn compiled_workflow_digest_returns_correct_value() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 0, 0),
        Box::new([]),
    ))
    .map_err(|e| e.to_string())?;

    let digest = workflow.digest();
    assert_eq!(digest.as_bytes(), [3u8; 32]);
    Ok(())
}

#[test]
fn compiled_workflow_expression_returns_some_for_valid_expression() -> Result<(), String> {
    let expression =
        ExprProgram::try_from_ops(vec![load(0)].into_boxed_slice()).map_err(|e| e.to_string())?;
    let workflow = CompiledWorkflow::try_from_parts(finish_const_parts_with(
        resource_contract(1, 0, 1, 1, 1),
        vec![expression].into_boxed_slice(),
    ))
    .map_err(|e| e.to_string())?;

    let expr = workflow.expression(ExprIdx::new(0));
    if expr.is_none() {
        return Err(String::from("expected Some for valid expression index"));
    }
    Ok(())
}

// =========================================================================
// ExprBranch and SlotBranch construction tests
// =========================================================================

#[test]
fn expr_branch_constructs_and_fields_match() {
    let branch = ExprBranch {
        condition: ExprIdx::new(2),
        target: StepIdx::new(5),
    };
    assert_eq!(branch.condition, ExprIdx::new(2));
    assert_eq!(branch.target, StepIdx::new(5));
}

#[test]
fn slot_branch_constructs_and_fields_match() {
    let branch = SlotBranch {
        condition: SlotIdx::new(1),
        target: StepIdx::new(3),
    };
    assert_eq!(branch.condition, SlotIdx::new(1));
    assert_eq!(branch.target, StepIdx::new(3));
}

// =========================================================================
// WorkflowError display and equality tests
// =========================================================================

#[test]
fn workflow_error_empty_nodes_display() {
    assert_eq!(
        WorkflowError::EmptyNodes.to_string(),
        "compiled workflow must contain at least one node"
    );
}

#[test]
fn workflow_error_empty_branch_table_display() {
    assert_eq!(
        WorkflowError::EmptyBranchTable.to_string(),
        "branch table must contain a branch or otherwise target"
    );
}

#[test]
fn workflow_error_budget_policy_exceeded_display() {
    let error = WorkflowError::BudgetPolicyExceeded {
        detail: BudgetDimension::TotalSteps,
    };
    assert!(error.to_string().contains("max_total_steps"));
}

#[test]
fn budget_dimension_names_are_frozen() {
    let cases = [
        (BudgetDimension::TotalSteps, "max_total_steps"),
        (BudgetDimension::TotalSlots, "max_total_slots"),
        (BudgetDimension::Fanout, "max_fanout"),
        (BudgetDimension::NestingDepth, "max_nesting_depth"),
        (BudgetDimension::Parallel, "max_parallel_in_flight"),
        (BudgetDimension::ActionTickets, "max_action_tickets"),
        (BudgetDimension::RunTime, "max_run_time_seconds"),
        (BudgetDimension::ResultBytes, "max_result_bytes"),
        (BudgetDimension::StepsExecutable, "max_steps_executable"),
        (BudgetDimension::TimerEntries, "max_timer_entries"),
        (BudgetDimension::TraceEvents, "max_trace_events"),
        (
            BudgetDimension::JournalBatchBytes,
            "max_journal_batch_bytes",
        ),
        (BudgetDimension::QueueDepth, "max_queue_depth"),
        (BudgetDimension::IpcPayloadBytes, "max_ipc_payload_bytes"),
        (BudgetDimension::BlobBytes, "max_blob_bytes"),
        (BudgetDimension::InputBytes, "max_input_bytes"),
    ];
    for (dimension, name) in cases {
        assert_eq!(dimension.as_str(), name);
        assert_eq!(dimension.to_string(), name);
    }
}

#[test]
fn contract_resource_names_are_frozen() {
    let cases = [
        (ContractResource::Steps, "max_steps"),
        (ContractResource::Slots, "max_slots"),
        (ContractResource::Constants, "max_constants"),
        (ContractResource::Accessors, "max_accessors"),
        (ContractResource::Expressions, "max_expressions"),
        (ContractResource::ExprStack, "max_expr_stack"),
        (
            ContractResource::StepBudgetPerTick,
            "max_step_budget_per_tick",
        ),
        (
            ContractResource::TransitionsPerTick,
            "max_transitions_per_tick",
        ),
        (ContractResource::InputBytes, "max_input_bytes"),
        (ContractResource::OutputBytes, "max_output_bytes"),
        (ContractResource::BlobBytes, "max_blob_bytes"),
        (ContractResource::IpcPayloadBytes, "max_ipc_payload_bytes"),
        (ContractResource::RetryAttempts, "max_retry_attempts"),
        (ContractResource::Fanout, "max_fanout"),
        (ContractResource::CollectItems, "max_collect_items"),
        (ContractResource::QueueDepth, "max_queue_depth"),
        (
            ContractResource::JournalBatchBytes,
            "max_journal_batch_bytes",
        ),
        (ContractResource::BranchCount, "branch_count"),
        (ContractResource::LoopRegions, "loop_regions"),
        (ContractResource::ListItems, "list_items"),
        (ContractResource::ObjectFields, "object_fields"),
    ];
    for (resource, name) in cases {
        assert_eq!(resource.as_str(), name);
        assert_eq!(resource.to_string(), name);
    }
}

#[test]
fn workflow_error_step_count_overflow_exact_variant_fields() -> Result<(), String> {
    match (WorkflowError::StepCountOverflow { actual: u64::MAX }) {
        WorkflowError::StepCountOverflow { actual } if actual == u64::MAX => Ok(()),
        other => Err(format!(
            "expected exact StepCountOverflow variant, got {other:?}"
        )),
    }
}

#[test]
fn workflow_error_jump_cycle_exact_variant_fields() -> Result<(), String> {
    match (WorkflowError::JumpCycle {
        step: StepIdx::new(7),
        target: StepIdx::new(3),
    }) {
        WorkflowError::JumpCycle { step, target }
            if step == StepIdx::new(7) && target == StepIdx::new(3) =>
        {
            Ok(())
        }
        other => Err(format!("expected exact JumpCycle variant, got {other:?}")),
    }
}

#[test]
fn validate_budget_maps_every_budget_error_variant_to_exact_detail() -> Result<(), String> {
    assert_budget_detail(total_steps_error(), BudgetDimension::TotalSteps)?;
    assert_budget_detail(total_slots_error(), BudgetDimension::TotalSlots)?;
    assert_budget_detail(fanout_error(), BudgetDimension::Fanout)?;
    assert_budget_detail(nesting_depth_error(), BudgetDimension::NestingDepth)?;
    assert_budget_detail(parallel_error(), BudgetDimension::Parallel)?;
    assert_budget_detail(action_tickets_error(), BudgetDimension::ActionTickets)?;
    assert_budget_detail(run_time_error(), BudgetDimension::RunTime)?;
    assert_budget_detail(result_bytes_error(), BudgetDimension::ResultBytes)?;
    assert_budget_detail(steps_executable_error(), BudgetDimension::StepsExecutable)?;
    Ok(())
}

#[test]
fn workflow_budget_validation_reports_total_steps_detail() -> Result<(), String> {
    assert_workflow_budget_detail(total_steps_budget_parts(), BudgetDimension::TotalSteps)
}

#[test]
fn workflow_budget_validation_reports_fanout_detail() -> Result<(), String> {
    assert_workflow_budget_detail(fanout_budget_parts(), BudgetDimension::Fanout)
}

#[test]
fn workflow_budget_validation_reports_nesting_depth_detail() -> Result<(), String> {
    assert_workflow_budget_detail(nesting_depth_budget_parts(), BudgetDimension::NestingDepth)
}

#[test]
fn workflow_budget_validation_reports_result_bytes_detail() -> Result<(), String> {
    assert_workflow_budget_detail(result_bytes_budget_parts(), BudgetDimension::ResultBytes)
}

fn total_steps_error() -> BudgetError {
    BudgetError::TotalStepsExceeded {
        actual: 2,
        limit: 1,
    }
}

fn total_slots_error() -> BudgetError {
    BudgetError::TotalSlotsExceeded {
        actual: 2,
        limit: 1,
    }
}

fn fanout_error() -> BudgetError {
    BudgetError::FanoutExceeded {
        actual: 2,
        limit: 1,
    }
}

fn nesting_depth_error() -> BudgetError {
    BudgetError::NestingDepthExceeded {
        actual: 2,
        limit: 1,
    }
}

fn parallel_error() -> BudgetError {
    BudgetError::ParallelExceeded {
        actual: 2,
        limit: 1,
    }
}

fn action_tickets_error() -> BudgetError {
    BudgetError::ActionTicketsExceeded {
        actual: 2,
        limit: 1,
    }
}

fn run_time_error() -> BudgetError {
    BudgetError::RunTimeExceeded {
        actual: 2,
        limit: 1,
    }
}

fn result_bytes_error() -> BudgetError {
    BudgetError::ResultBytesExceeded {
        actual: 2,
        limit: 1,
    }
}

fn steps_executable_error() -> BudgetError {
    BudgetError::StepsExecutableExceeded {
        actual: 2,
        limit: 1,
    }
}

fn assert_budget_detail(error: BudgetError, detail: BudgetDimension) -> Result<(), String> {
    match validate_budget_result(Err(error)) {
        Err(WorkflowError::BudgetPolicyExceeded { detail: actual }) if actual == detail => Ok(()),
        other => Err(format!("unexpected budget validation result: {other:?}")),
    }
}

fn assert_workflow_budget_detail(
    parts: WorkflowParts,
    detail: BudgetDimension,
) -> Result<(), String> {
    match CompiledWorkflow::try_from_parts(parts) {
        Err(WorkflowError::BudgetPolicyExceeded { detail: actual }) if actual == detail => Ok(()),
        other => Err(format!("unexpected workflow validation result: {other:?}")),
    }
}

fn total_steps_budget_parts() -> WorkflowParts {
    budget_parts(
        vec![
            budget_node(
                0,
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(0),
                    limit: 1_000_001,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            ),
            budget_node(1, CompiledNodeKind::Nop),
            budget_node(2, CompiledNodeKind::Nop),
        ],
        1,
        budget_contract(3, 1),
    )
}

fn fanout_budget_parts() -> WorkflowParts {
    budget_parts(
        vec![
            budget_node(
                0,
                CompiledNodeKind::ChooseSlot {
                    branches: fanout_branches(),
                    otherwise: None,
                },
            ),
            budget_node(1, CompiledNodeKind::Nop),
        ],
        1,
        budget_contract(2, 1),
    )
}

fn nesting_depth_budget_parts() -> WorkflowParts {
    budget_parts(nesting_nodes(), 1, budget_contract(10, 1))
}

fn result_bytes_budget_parts() -> WorkflowParts {
    budget_parts(
        vec![budget_node(
            0,
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        )],
        1,
        ResourceContract {
            max_output_bytes: 262_145,
            ..budget_contract(1, 1)
        },
    )
}

fn fanout_branches() -> Box<[SlotBranch]> {
    (0..65)
        .map(|_| SlotBranch {
            condition: SlotIdx::new(0),
            target: StepIdx::new(1),
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn nesting_nodes() -> Vec<CompiledNode> {
    (0..9)
        .map(|index| {
            budget_node(
                index,
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(0),
                    limit: 1,
                    body: StepIdx::new(
                        u16::try_from(index.saturating_add(1)).map_or(u16::MAX, |v| v),
                    ),
                    done: StepIdx::new(9),
                },
            )
        })
        .chain(std::iter::once(budget_node(9, CompiledNodeKind::Nop)))
        .collect()
}

fn budget_contract(max_steps: u16, max_slots: u16) -> ResourceContract {
    ResourceContract {
        max_steps,
        max_slots,
        ..ResourceContract::DEFAULT
    }
}

fn budget_parts(
    nodes: Vec<CompiledNode>,
    slot_count: u16,
    resource_contract: ResourceContract,
) -> WorkflowParts {
    WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("budget_validation"),
        digest: WorkflowDigest::from_bytes([0x71; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract,
        step_names: Box::default(),
    }
}

fn budget_node(index: u16, kind: CompiledNodeKind) -> CompiledNode {
    crate::workflow::node(StepIdx::new(index), kind, None, None)
}

#[cfg(test)]
mod proptests {
    use super::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstValue, ContractResource,
        ResourceContract, WorkflowError, WorkflowParts,
    };
    use crate::ids::{ConstIdx, SlotIdx, StepIdx, WorkflowDigest};
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn resource_contract_max_steps_is_positive(_unused in 0u8..1u8) {
            let contract = ResourceContract::DEFAULT;
            prop_assert!(contract.max_steps > 0);
        }
    }

    proptest! {
        #[test]
        fn resource_contract_max_slots_is_positive(_unused in 0u8..1u8) {
            let contract = ResourceContract::DEFAULT;
            prop_assert!(contract.max_slots > 0);
        }
    }

    // =========================================================================
    // Property A: Valid minimal workflow always passes validation
    //
    // Generate random (but structurally valid) workflows with 2-10 steps,
    // each forming a SetConst -> ... -> Finish chain.
    // =========================================================================

    /// Builds a valid linear workflow with `step_count` nodes.
    /// Nodes 0..N-2 are SetConst, node N-1 is Finish.
    /// slot_count = 1 (slot 0 is used throughout).
    fn build_valid_chain(step_count: usize) -> WorkflowParts {
        let last = step_count.saturating_sub(1);
        let mut nodes: Vec<CompiledNode> = (0..last)
            .map(|i| {
                let next_step = u16::try_from(i.saturating_add(1)).map_or(u16::MAX, |v| v);
                crate::workflow::node(
                    StepIdx::new(u16::try_from(i).map_or(u16::MAX, |v| v)),
                    CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                    Some(SlotIdx::new(0)),
                    Some(StepIdx::new(next_step)),
                )
            })
            .collect();
        nodes.push(crate::workflow::node(
            StepIdx::new(u16::try_from(last).map_or(u16::MAX, |v| v)),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ));
        let max_steps = u16::try_from(nodes.len()).map_or(u16::MAX, |v| v);
        WorkflowParts {
            semantic_source_identity: None,
            source_metadata: None,
            input_slots: Box::new([]),
            name: Box::<str>::from("proptest_valid_chain"),
            digest: WorkflowDigest::from_bytes([0xAA; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: resource_contract(max_steps, 1, 1, 0, 0),
            step_names: Box::new([]),
        }
    }

    fn chain_node(index: usize, total: usize) -> CompiledNode {
        let is_last = index == total.saturating_sub(1);
        let next = if is_last {
            None
        } else {
            Some(StepIdx::new(
                u16::try_from(index.saturating_add(1)).map_or(u16::MAX, |v| v),
            ))
        };
        let kind = if is_last {
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            }
        } else {
            CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            }
        };
        crate::workflow::node(
            StepIdx::new(u16::try_from(index).map_or(u16::MAX, |v| v)),
            kind,
            Some(SlotIdx::new(0)),
            next,
        )
    }

    fn duplicate_step_node(index: usize, total: usize, duplicate_position: usize) -> CompiledNode {
        let claimed_id = if index == duplicate_position {
            StepIdx::new(0)
        } else {
            StepIdx::new(u16::try_from(index).map_or(u16::MAX, |v| v))
        };
        CompiledNode {
            retry: None,
            id: claimed_id,
            ..chain_node(index, total)
        }
    }

    fn unreachable_finish_node(index: usize) -> CompiledNode {
        crate::workflow::node(
            StepIdx::new(u16::try_from(index).map_or(u16::MAX, |v| v)),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        )
    }

    proptest! {
        #[test]
        fn prop_a_valid_chain_workflow_passes_validation(step_count in 2u16..10u16) {
            let parts = build_valid_chain(usize::from(step_count));
            let result = CompiledWorkflow::try_from_parts(parts);
            prop_assert!(
                result.is_ok(),
                "valid chain with {} steps should pass validation, got {:?}",
                step_count,
                result
            );
        }
    }

    // =========================================================================
    // Property B: SlotIdx out of bounds always rejected
    //
    // Generate workflows where Finish or SetConst references a slot >= slot_count.
    // =========================================================================

    proptest! {
        #[test]
        fn prop_b_finish_slot_out_of_bounds_rejected(
            slot_count in 1u16..10u16,
            bad_slot_delta in 1u16..50u16
        ) {
            let bad_slot = slot_count.saturating_add(bad_slot_delta);
            let parts = WorkflowParts {
                semantic_source_identity: None,
                source_metadata: None,
                input_slots: Box::new([]),
                name: Box::<str>::from("prop_b_finish_oob"),
                digest: WorkflowDigest::from_bytes([0xBB; 32]),
                nodes: vec![crate::workflow::node(StepIdx::new(0), CompiledNodeKind::Finish { result: SlotIdx::new(bad_slot), }, None, None)]
                .into_boxed_slice(),
                expressions: Box::new([]),
                accessors: Box::new([]),
                constants: vec![ConstValue::Null].into_boxed_slice(),
                slot_count,
                symbols_count: 0,
                entry: StepIdx::new(0),
                resource_contract: resource_contract(1, slot_count, 1, 0, 0),
        step_names: Box::new([]),
            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::SlotOutOfBounds { slot }) => {
                    prop_assert_eq!(slot, SlotIdx::new(bad_slot));
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected SlotOutOfBounds, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    proptest! {
        #[test]
        fn prop_b_setconst_output_slot_out_of_bounds_rejected(
            slot_count in 1u16..10u16,
            bad_slot_delta in 1u16..50u16
        ) {
            let bad_slot = slot_count.saturating_add(bad_slot_delta);
            let parts = WorkflowParts {
                semantic_source_identity: None,
                source_metadata: None,
                input_slots: Box::new([]),
                name: Box::<str>::from("prop_b_output_oob"),
                digest: WorkflowDigest::from_bytes([0xBC; 32]),
                nodes: vec![
                    crate::workflow::node(StepIdx::new(0), CompiledNodeKind::SetConst { value: ConstIdx::new(0), }, Some(SlotIdx::new(bad_slot)), Some(StepIdx::new(1))),
                    crate::workflow::node(StepIdx::new(1), CompiledNodeKind::Finish { result: SlotIdx::new(0), }, None, None),
                ]
                .into_boxed_slice(),
                expressions: Box::new([]),
                accessors: Box::new([]),
                constants: vec![ConstValue::Null].into_boxed_slice(),
                slot_count,
                symbols_count: 0,
                entry: StepIdx::new(0),
                resource_contract: resource_contract(2, slot_count, 1, 0, 0),
        step_names: Box::new([]),
            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::SlotOutOfBounds { slot }) => {
                    prop_assert_eq!(slot, SlotIdx::new(bad_slot));
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected SlotOutOfBounds, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    // =========================================================================
    // Property C: Duplicate StepIdx always rejected
    //
    // Generate workflows with two nodes claiming the same StepIdx.
    // =========================================================================

    proptest! {
        #[test]
        fn prop_c_duplicate_step_idx_rejected(
            step_count in 3u16..10u16,
            duplicate_id_pos in 1u16..9u16
        ) {
            let n = usize::from(step_count);
            let dup_pos = usize::from(duplicate_id_pos.min(step_count.saturating_sub(1)));
            let nodes = (0..n)
                .map(|index| duplicate_step_node(index, n, dup_pos))
                .collect::<Vec<_>>();
            let max_steps = u16::try_from(n).map_or(u16::MAX, |v| v);
            let parts = WorkflowParts {
                semantic_source_identity: None,
                source_metadata: None,
                input_slots: Box::new([]),
                name: Box::<str>::from("prop_c_dup"),
                digest: WorkflowDigest::from_bytes([0xCC; 32]),
                nodes: nodes.into_boxed_slice(),
                expressions: Box::new([]),
                accessors: Box::new([]),
                constants: vec![ConstValue::Null].into_boxed_slice(),
                slot_count: 1,
                symbols_count: 0,
                entry: StepIdx::new(0),
                resource_contract: resource_contract(max_steps, 1, 1, 0, 0),
        step_names: Box::new([]),
            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::NodeIdMismatch { expected, actual }) => {
                    prop_assert_eq!(expected, StepIdx::new(u16::try_from(dup_pos).map_or(u16::MAX, |v| v)));
                    prop_assert_eq!(actual, StepIdx::new(0));
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected NodeIdMismatch, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    // =========================================================================
    // Property D: Unreachable nodes always rejected
    //
    // Generate workflows where an extra node exists but no other node points to it.
    // =========================================================================

    proptest! {
        #[test]
        fn prop_d_unreachable_node_rejected(
            chain_len in 2u16..8u16,
            unreachable_count in 1u16..3u16
        ) {
            let chain_n = usize::from(chain_len);
            let extra_n = usize::from(unreachable_count);
            let total = chain_n.saturating_add(extra_n);
            let nodes = (0..chain_n)
                .map(|index| chain_node(index, chain_n))
                .chain((chain_n..total).map(unreachable_finish_node))
                .collect::<Vec<_>>();

            let max_steps = u16::try_from(total).map_or(u16::MAX, |v| v);
            let parts = WorkflowParts {
                semantic_source_identity: None,
                source_metadata: None,
                input_slots: Box::new([]),
                name: Box::<str>::from("prop_d_unreachable"),
                digest: WorkflowDigest::from_bytes([0xDD; 32]),
                nodes: nodes.into_boxed_slice(),
                expressions: Box::new([]),
                accessors: Box::new([]),
                constants: vec![ConstValue::Null].into_boxed_slice(),
                slot_count: 1,
                symbols_count: 0,
                entry: StepIdx::new(0),
                resource_contract: resource_contract(max_steps, 1, 1, 0, 0),
        step_names: Box::new([]),
            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::UnreachableNode { step }) => {
                    // The first unreachable node should be at index chain_len.
                    prop_assert_eq!(step, StepIdx::new(u16::try_from(chain_n).map_or(u16::MAX, |v| v)));
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected UnreachableNode, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    // =========================================================================
    // Property E: Resource contract bounds respected
    //
    // Workflows with step_count > max_steps fail with ResourceContractExceeded.
    // =========================================================================

    proptest! {
        #[test]
        fn prop_e_resource_contract_max_steps_violated(
            actual_steps in 2u16..10u16,
            shortfall in 1u16..5u16
        ) {
            let max_steps_declared = actual_steps.saturating_sub(shortfall);
            // Build a valid chain but with a contract that doesn't cover it.
            let valid_parts = build_valid_chain(usize::from(actual_steps));
            let parts = WorkflowParts {
                semantic_source_identity: None,
                source_metadata: None,
                resource_contract: resource_contract(max_steps_declared, 1, 1, 0, 0),
        step_names: Box::new([]),
                ..valid_parts
            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::ResourceContractExceeded { resource }) => {
                    prop_assert_eq!(resource, ContractResource::Steps);
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected ResourceContractExceeded for max_steps, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    proptest! {
        #[test]
        fn prop_e_resource_contract_max_slots_violated(
            actual_slots in 1u16..10u16,
            shortfall in 1u16..5u16
        ) {
            let declared_slots = actual_slots.saturating_sub(shortfall);
            // Single node that uses a slot at the boundary.
            let parts = WorkflowParts {
                semantic_source_identity: None,
                source_metadata: None,
                input_slots: Box::new([]),
                name: Box::<str>::from("prop_e_slots"),
                digest: WorkflowDigest::from_bytes([0xEE; 32]),
                nodes: vec![crate::workflow::node(StepIdx::new(0), CompiledNodeKind::Finish { result: SlotIdx::new(0), }, None, None)]
                .into_boxed_slice(),
                expressions: Box::new([]),
                accessors: Box::new([]),
                constants: vec![ConstValue::Null].into_boxed_slice(),
                slot_count: actual_slots,
                symbols_count: 0,
                entry: StepIdx::new(0),
                resource_contract: resource_contract(1, declared_slots, 1, 0, 0),
        step_names: Box::new([]),
            };
            let result = CompiledWorkflow::try_from_parts(parts);
            match result {
                Err(WorkflowError::ResourceContractExceeded { resource }) => {
                    prop_assert_eq!(resource, ContractResource::Slots);
                }
                other => {
                    return Err(proptest::test_runner::TestCaseError::Fail(
                        format!("expected ResourceContractExceeded for max_slots, got {:?}", other).into()
                    ));
                }
            }
        }
    }

    fn resource_contract(
        max_steps: u16,
        max_slots: u16,
        max_constants: u16,
        max_expressions: u16,
        max_expr_stack: u8,
    ) -> ResourceContract {
        ResourceContract {
            max_steps,
            max_slots,
            max_constants,
            max_accessors: 0,
            max_expressions,
            max_expr_stack,
            max_step_budget_per_tick: 1,
            max_transitions_per_tick: 1,
            max_input_bytes: 1,
            max_output_bytes: 1,
            max_blob_bytes: 1,
            max_ipc_payload_bytes: 1,
            max_retry_attempts: 0,
            max_fanout: 0,
            max_collect_items: 0,
            max_queue_depth: 1,
            max_journal_batch_bytes: 1,
            ..ResourceContract::DEFAULT
        }
    }

    // =========================================================================
    // Phase 45 tests — ResourceContract default values
    // =========================================================================

    #[test]
    fn resource_contract_default_has_reasonable_max_steps() {
        assert_eq!(ResourceContract::DEFAULT.max_steps, 10_000);
    }

    #[test]
    fn resource_contract_default_has_reasonable_max_slots() {
        assert_eq!(ResourceContract::DEFAULT.max_slots, 1_024);
    }

    #[test]
    fn resource_contract_default_has_reasonable_max_fanout() {
        assert_eq!(ResourceContract::DEFAULT.max_fanout, 64);
    }

    #[test]
    fn resource_contract_default_has_reasonable_step_budget_per_tick() {
        assert_eq!(ResourceContract::DEFAULT.max_step_budget_per_tick, 10_000);
    }

    #[test]
    fn resource_contract_default_max_steps_is_not_u16_max() {
        assert_ne!(ResourceContract::DEFAULT.max_steps, u16::MAX);
    }

    #[test]
    fn resource_contract_default_max_slots_is_not_u16_max() {
        assert_ne!(ResourceContract::DEFAULT.max_slots, u16::MAX);
    }

    #[test]
    fn resource_contract_default_max_fanout_is_not_u16_max() {
        assert_ne!(ResourceContract::DEFAULT.max_fanout, u16::MAX);
    }

    #[test]
    fn resource_contract_default_max_retry_attempts_is_not_u16_max() {
        assert_ne!(ResourceContract::DEFAULT.max_retry_attempts, u16::MAX);
    }
}

// =========================================================================
// Edge-case tests — CompiledNode variants, CompiledNodeKind, ExprOp,
// ExprProgram, ExprBranch/SlotBranch
// =========================================================================
