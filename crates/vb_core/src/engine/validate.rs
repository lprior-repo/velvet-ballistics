#![forbid(unsafe_code)]
//! Workflow validation functions.

use crate::ids::StepIdx;
use crate::workflow::{
    CompiledNodeKind, CompiledWorkflow, ExprBranch, SlotBranch, WorkflowError, WorkflowParts,
};

/// Validates compiled workflow IR integrity.
pub fn validate_compiled_workflow(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    CompiledWorkflow::try_from_parts(parts.clone())?;
    Ok(())
}

/// Validates resource contract bounds against hard limits.
pub fn validate_resource_contract(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let contract = parts.resource_contract;
    if usize::from(contract.max_steps) > crate::limits::MAX_STEPS_PER_WORKFLOW {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_steps",
        });
    }
    if usize::from(contract.max_slots) > crate::limits::MAX_SLOTS_PER_WORKFLOW {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_slots",
        });
    }
    if usize::from(contract.max_constants) > crate::limits::MAX_CONSTANTS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_constants",
        });
    }
    if usize::from(contract.max_accessors) > crate::limits::MAX_ACCESSORS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_accessors",
        });
    }
    if usize::from(contract.max_expressions) > crate::limits::MAX_EXPRESSIONS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_expressions",
        });
    }
    if contract.max_expr_stack > crate::limits::MAX_EXPRESSION_STACK {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_expr_stack",
        });
    }
    if contract.max_step_budget_per_tick > crate::limits::MAX_STEP_BUDGET {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_step_budget_per_tick",
        });
    }
    if contract.max_transitions_per_tick > crate::limits::MAX_STEP_BUDGET {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_transitions_per_tick",
        });
    }
    if contract.max_input_bytes > crate::limits::MAX_INPUT_BYTES {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_input_bytes",
        });
    }
    if contract.max_output_bytes > crate::limits::MAX_OUTPUT_BYTES {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_output_bytes",
        });
    }
    if contract.max_blob_bytes > crate::limits::MAX_BLOB_BYTES {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_blob_bytes",
        });
    }
    if contract.max_ipc_payload_bytes > crate::limits::MAX_IPC_PAYLOAD_BYTES {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_ipc_payload_bytes",
        });
    }
    if u32::from(contract.max_retry_attempts) > u32::from(crate::limits::MAX_RETRY_ATTEMPTS) {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_retry_attempts",
        });
    }
    if u32::from(contract.max_fanout) > u32::from(crate::limits::MAX_FANOUT) {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_fanout",
        });
    }
    if contract.max_collect_items > crate::limits::MAX_COLLECT_ITEMS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_collect_items",
        });
    }
    if contract.max_queue_depth > crate::limits::MAX_QUEUE_DEPTH {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_queue_depth",
        });
    }
    if contract.max_journal_batch_bytes > crate::limits::MAX_JOURNAL_BATCH_BYTES {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_journal_batch_bytes",
        });
    }
    Ok(())
}

/// Validates that all node indices are within the node array bounds.
pub fn validate_node_bounds(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let node_count = parts.nodes.len();
    for node in &parts.nodes {
        if node.id.as_usize() >= node_count {
            return Err(WorkflowError::StepOutOfBounds { step: node.id });
        }
        if let Some(next) = node.next
            && next.as_usize() >= node_count
        {
            return Err(WorkflowError::StepOutOfBounds { step: next });
        }
    }
    Ok(())
}

/// Validates that all step transition targets reference valid node indices.
pub fn validate_transition_target(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let node_count = parts.nodes.len();
    for node in &parts.nodes {
        match &node.kind {
            CompiledNodeKind::Jump { target } if target.as_usize() >= node_count => {
                return Err(WorkflowError::StepOutOfBounds { step: *target });
            }
            CompiledNodeKind::Choose {
                branches,
                otherwise,
            } => {
                validate_branch_targets(branches, *otherwise, node_count)?;
            }
            CompiledNodeKind::ChooseSlot {
                branches,
                otherwise,
            } => {
                validate_slot_branch_targets(branches, *otherwise, node_count)?;
            }
            CompiledNodeKind::ForEachStart { body, done, .. }
            | CompiledNodeKind::ForEachNext { body, done, .. }
            | CompiledNodeKind::CollectStart { body, done, .. }
            | CompiledNodeKind::CollectPage { body, done, .. }
            | CompiledNodeKind::CollectNext { body, done, .. }
            | CompiledNodeKind::ReduceStart { body, done, .. }
            | CompiledNodeKind::ReduceNext { body, done, .. }
            | CompiledNodeKind::RepeatStart { body, done, .. }
            | CompiledNodeKind::RepeatAttempt { body, done, .. } => {
                validate_two_step_targets(*body, *done, node_count)?;
            }
            CompiledNodeKind::TogetherStart { branches, join } => {
                for branch in branches {
                    if branch.as_usize() >= node_count {
                        return Err(WorkflowError::StepOutOfBounds { step: *branch });
                    }
                }
                if join.as_usize() >= node_count {
                    return Err(WorkflowError::StepOutOfBounds { step: *join });
                }
            }
            CompiledNodeKind::TogetherBranch { entry, join, .. } => {
                validate_two_step_targets(*entry, *join, node_count)?;
            }
            CompiledNodeKind::RepeatCheck { done, .. } if done.as_usize() >= node_count => {
                return Err(WorkflowError::StepOutOfBounds { step: *done });
            }
            CompiledNodeKind::RetryCheck {
                body, exhausted, ..
            } => {
                validate_two_step_targets(*body, *exhausted, node_count)?;
            }
            CompiledNodeKind::ErrorHandler { body, handler, .. } => {
                validate_two_step_targets(*body, *handler, node_count)?;
            }
            _ => {}
        }
    }
    // Reject nested `TogetherStart` (RE-001, option (b)): nesting causes
    // `compute_max_parallel_in_flight` (in vb_runtime::engine::drive) to
    // report a misleading per-node maximum and yields spurious
    // `ParallelLimitExceeded` failures at runtime. Surface the structural
    // problem at validation time with a typed error.
    validate_no_nested_together(parts)?;
    Ok(())
}

/// Rejects `TogetherStart` nodes that are reachable from the body of
/// another `TogetherStart` node.
///
/// `compute_max_parallel_in_flight` (in `vb_runtime::engine::drive`)
/// returns a per-node maximum branch count rather than the true peak
/// parallel-in-flight, so a nested `TogetherStart` produces a
/// `ParallelLimitExceeded` at runtime even when no real concurrency
/// limit has been violated. Surfacing the structural error at compile
/// time (RE-001, option (b)) gives operators a typed, deterministic
/// rejection with a precise location instead of a misleading runtime
/// signal.
pub fn validate_no_nested_together(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let node_count = parts.nodes.len();
    // Identify all `TogetherStart` step indices for quick lookup.
    let together_starts: Vec<StepIdx> = parts
        .nodes
        .iter()
        .filter_map(|node| match &node.kind {
            CompiledNodeKind::TogetherStart { .. } => Some(node.id),
            _ => None,
        })
        .collect();
    if together_starts.len() < 2 {
        // Cannot be nested with zero or one `TogetherStart` nodes.
        return Ok(());
    }
    let together_set: std::collections::BTreeSet<StepIdx> =
        together_starts.iter().copied().collect();
    for node in &parts.nodes {
        let CompiledNodeKind::TogetherStart { branches, join } = &node.kind else {
            continue;
        };
        let outer = node.id;
        for branch_entry in branches.iter() {
            let entry_idx = branch_entry.as_usize();
            if entry_idx >= node_count {
                // Out-of-bounds branch is reported by `validate_transition_target`.
                continue;
            }
            let join_idx = join.as_usize();
            if join_idx >= node_count {
                continue;
            }
            // Depth-first walk from `branch_entry`; stop at the `join` step
            // (which is the outer TogetherStart's join, exclusive). The walk
            // follows the linear `next` chain emitted by the compiler for
            // TogetherStart branch bodies, so it stays bounded to the outer
            // TogetherStart's body.
            if walk_contains_together_start(&parts.nodes, &together_set, entry_idx, join_idx, outer)
            {
                return Err(WorkflowError::NestedTogether {
                    outer,
                    inner: find_inner_together_start(
                        &parts.nodes,
                        &together_set,
                        entry_idx,
                        join_idx,
                        outer,
                    ),
                });
            }
        }
    }
    Ok(())
}

/// Walks nodes starting at `start_idx` and following each node's `next`
/// field, stopping once `stop_idx` is reached (exclusive). Returns `true`
/// if any reachable node is a `TogetherStart` distinct from `outer`.
fn walk_contains_together_start(
    nodes: &[crate::workflow::CompiledNode],
    together_set: &std::collections::BTreeSet<StepIdx>,
    start_idx: usize,
    stop_idx: usize,
    outer: StepIdx,
) -> bool {
    let mut visited: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    let mut stack: Vec<usize> = vec![start_idx];
    while let Some(idx) = stack.pop() {
        if idx >= nodes.len() || idx == stop_idx || !visited.insert(idx) {
            continue;
        }
        let Ok(step_u16) = u16::try_from(idx) else {
            continue;
        };
        let step = StepIdx::new(step_u16);
        if step != outer && together_set.contains(&step) {
            return true;
        }
        let Some(node) = nodes.get(idx) else {
            continue;
        };
        if let Some(next) = node.next {
            stack.push(next.as_usize());
        }
    }
    false
}

/// Helper that returns the first inner `TogetherStart` step reachable
/// from `start_idx` (without crossing `stop_idx`).
fn find_inner_together_start(
    nodes: &[crate::workflow::CompiledNode],
    together_set: &std::collections::BTreeSet<StepIdx>,
    start_idx: usize,
    stop_idx: usize,
    outer: StepIdx,
) -> StepIdx {
    let mut visited: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    let mut stack: Vec<usize> = vec![start_idx];
    while let Some(idx) = stack.pop() {
        if idx >= nodes.len() || idx == stop_idx || !visited.insert(idx) {
            continue;
        }
        let Ok(step_u16) = u16::try_from(idx) else {
            return outer;
        };
        let step = StepIdx::new(step_u16);
        if step != outer && together_set.contains(&step) {
            return step;
        }
        let Some(node) = nodes.get(idx) else {
            continue;
        };
        if let Some(next) = node.next {
            stack.push(next.as_usize());
        }
    }
    outer
}

fn validate_branch_targets(
    branches: &[ExprBranch],
    otherwise: Option<StepIdx>,
    node_count: usize,
) -> Result<(), WorkflowError> {
    for branch in branches {
        if branch.target.as_usize() >= node_count {
            return Err(WorkflowError::StepOutOfBounds {
                step: branch.target,
            });
        }
    }
    if let Some(target) = otherwise
        && target.as_usize() >= node_count
    {
        return Err(WorkflowError::StepOutOfBounds { step: target });
    }
    Ok(())
}

fn validate_slot_branch_targets(
    branches: &[SlotBranch],
    otherwise: Option<StepIdx>,
    node_count: usize,
) -> Result<(), WorkflowError> {
    for branch in branches {
        if branch.target.as_usize() >= node_count {
            return Err(WorkflowError::StepOutOfBounds {
                step: branch.target,
            });
        }
    }
    if let Some(target) = otherwise
        && target.as_usize() >= node_count
    {
        return Err(WorkflowError::StepOutOfBounds { step: target });
    }
    Ok(())
}

fn validate_two_step_targets(
    first: StepIdx,
    second: StepIdx,
    node_count: usize,
) -> Result<(), WorkflowError> {
    if first.as_usize() >= node_count {
        return Err(WorkflowError::StepOutOfBounds { step: first });
    }
    if second.as_usize() >= node_count {
        return Err(WorkflowError::StepOutOfBounds { step: second });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        validate_compiled_workflow, validate_node_bounds, validate_resource_contract,
        validate_transition_target,
    };
    use crate::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx, WorkflowDigest};
    use crate::value::ConstValue;
    use crate::workflow::{
        CompiledNode, CompiledNodeKind, ExprBranch, ResourceContract, SlotBranch, WorkflowError,
        WorkflowParts,
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
            ..ResourceContract::DEFAULT
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
        if usize::from(u16::MAX) > limit {
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
        if usize::from(u16::MAX) > limit {
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
        if usize::from(u16::MAX) > limit {
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
        if usize::from(u16::MAX) > limit {
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
        if usize::from(u16::MAX) > limit {
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

    // --- Valid transition targets pass validation ---

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

    // RED PHASE: additional resource contract tests
    mod red_phase_behavior_tests;
    mod red_phase_tests;
}
