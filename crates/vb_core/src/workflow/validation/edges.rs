//! Forward-edge graph validation.

use crate::ids::StepIdx;

use super::super::node::CompiledNodeKind;
use super::super::types::{WorkflowError, WorkflowParts};

/// Check B: all edges must point forward except recognized loop back-edges.
/// Check D: loop spans must be properly nested (no overlapping loops).
pub(crate) fn validate_forward_edges(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let mut loop_spans: Vec<(usize, usize)> = Vec::new();

    for (index, node) in parts.nodes.iter().enumerate() {
        let current_id = StepIdx::new(u16::try_from(index).map_err(|_| {
            WorkflowError::ResourceContractExceeded {
                resource: "max_steps",
            }
        })?);

        if let Some(next) = node.next {
            validate_forward_target(next, index, current_id)?;
        }

        validate_kind_edges(&node.kind, index, current_id)?;

        push_loop_span(&node.kind, index, &mut loop_spans)?;
    }
    Ok(())
}

/// Validates that kind-specific edges respect the forward-only rule.
#[allow(clippy::match_same_arms)]
fn validate_kind_edges(
    kind: &CompiledNodeKind,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    match kind {
        CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::BuildObject { .. }
        | CompiledNodeKind::BuildList { .. }
        | CompiledNodeKind::Do { .. }
        | CompiledNodeKind::ForEachJoin { .. }
        | CompiledNodeKind::TogetherJoin { .. }
        | CompiledNodeKind::CollectFinish { .. }
        | CompiledNodeKind::ReduceFinish { .. }
        | CompiledNodeKind::RepeatFinish { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. }
        | CompiledNodeKind::Finish { .. } => Ok(()),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            for branch in branches.as_ref() {
                validate_forward_target(branch.target, ci, cid)?;
            }
            if let Some(fallback) = *otherwise {
                validate_forward_target(fallback, ci, cid)?;
            }
            Ok(())
        }
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            for branch in branches.as_ref() {
                validate_forward_target(branch.target, ci, cid)?;
            }
            if let Some(fallback) = *otherwise {
                validate_forward_target(fallback, ci, cid)?;
            }
            Ok(())
        }
        CompiledNodeKind::ForEachStart { body, done, .. } => {
            let _ = body;
            validate_forward_target(*done, ci, cid)
        }
        CompiledNodeKind::ForEachNext { body, done, .. } => {
            let _ = body;
            validate_forward_target(*done, ci, cid)
        }
        CompiledNodeKind::TogetherStart { branches, join } => {
            let _ = branches;
            validate_forward_target(*join, ci, cid)
        }
        CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::CollectPage { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. }
        | CompiledNodeKind::RepeatStart { body, done, .. }
        | CompiledNodeKind::RepeatAttempt { body, done, .. } => {
            let _ = body;
            validate_forward_target(*done, ci, cid)
        }
        CompiledNodeKind::RepeatCheck { done, .. } => validate_forward_target(*done, ci, cid),
        CompiledNodeKind::RetryCheck {
            body, exhausted, ..
        } => {
            let _ = body;
            validate_forward_target(*exhausted, ci, cid)
        }
        CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            let _ = body;
            validate_forward_target(*handler, ci, cid)
        }
        CompiledNodeKind::Jump { .. } => Ok(()),
    }
}

/// Validates a target step is strictly forward from the current node.
fn validate_forward_target(target: StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    if target.as_usize() > ci {
        Ok(())
    } else {
        Err(WorkflowError::BackwardEdge {
            from: cid,
            to: target,
        })
    }
}

/// Tracks loop spans for nesting validation (Check D).
fn push_loop_span(
    kind: &CompiledNodeKind,
    ci: usize,
    spans: &mut Vec<(usize, usize)>,
) -> Result<(), WorkflowError> {
    let done_usize: Option<usize> = match kind {
        CompiledNodeKind::ForEachStart { done, .. }
        | CompiledNodeKind::CollectStart { done, .. }
        | CompiledNodeKind::ReduceStart { done, .. }
        | CompiledNodeKind::RepeatStart { done, .. } => Some(done.as_usize()),
        CompiledNodeKind::TogetherStart { join, .. } => Some(join.as_usize()),
        _ => None,
    };

    let Some(done_idx) = done_usize else {
        return Ok(());
    };

    if let Some(&(_outer_start, outer_done)) = spans.last()
        && done_idx > outer_done
    {
        return Err(WorkflowError::ImproperLoopNesting {
            inner: StepIdx::new(u16::try_from(ci).map_err(|_| {
                WorkflowError::ResourceContractExceeded {
                    resource: "max_steps",
                }
            })?),
            outer_done: StepIdx::new(u16::try_from(outer_done).map_err(|_| {
                WorkflowError::ResourceContractExceeded {
                    resource: "max_steps",
                }
            })?),
        });
    }

    while spans
        .last()
        .is_some_and(|&(_, done): &(usize, usize)| done <= ci)
    {
        spans.pop();
    }

    spans.push((ci, done_idx));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{SlotIdx, StepIdx, ConstIdx};
    use crate::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowDigest, WorkflowParts};
    use crate::value::ConstValue;

    fn nop_node(id: u16, next: Option<u16>) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: next.map(StepIdx::new),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }
    }

    fn finish_node(id: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish { result: SlotIdx::new(0) },
        }
    }

    fn make_parts(nodes: Vec<CompiledNode>) -> WorkflowParts {
        let max_steps = u16::try_from(nodes.len()).map_or(u16::MAX, |v| v);
        WorkflowParts {
            name: Box::<str>::from("edges_test"),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract {
                max_steps,
                max_slots: 10,
                max_constants: 10,
                max_accessors: 10,
                max_expressions: 10,
                max_expr_stack: 64,
                max_step_budget_per_tick: 10_000,
                max_input_bytes: 1_048_576,
                max_output_bytes: 1_048_576,
                max_blob_bytes: 16_777_216,
                max_ipc_payload_bytes: 1_048_576,
                max_retry_attempts: 3,
                max_fanout: 64,
                max_collect_items: 1_024,
                max_queue_depth: 1_024,
                max_journal_batch_bytes: 1_048_576,
            },
            step_names: Box::new([]),
        }
    }

    // -- validate_forward_edges: forward next edges --

    #[test]
    fn forward_edges_accepts_linear_chain() {
        let parts = make_parts(vec![
            nop_node(0, Some(1)),
            nop_node(1, Some(2)),
            finish_node(2),
        ]);
        assert_eq!(validate_forward_edges(&parts), Ok(()));
    }

    #[test]
    fn forward_edges_rejects_backward_next() {
        let parts = make_parts(vec![
            nop_node(0, Some(1)),
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: Some(StepIdx::new(0)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish { result: SlotIdx::new(0) },
            },
        ]);
        let result = validate_forward_edges(&parts);
        assert!(matches!(
            result,
            Err(WorkflowError::BackwardEdge { from, to })
            if from == StepIdx::new(1) && to == StepIdx::new(0)
        ));
    }

    #[test]
    fn forward_edges_accepts_jump_backward() {
        let nodes = vec![
            nop_node(0, Some(1)),
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Jump { target: StepIdx::new(0) },
            },
        ];
        let parts = make_parts(nodes);
        // Jump is allowed to point backward
        assert_eq!(validate_forward_edges(&parts), Ok(()));
    }

    // -- validate_forward_edges: kind-specific edges --

    #[test]
    fn forward_edges_accepts_choose_slot_forward_branches() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![crate::workflow::SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(1),
                    }].into_boxed_slice(),
                    otherwise: Some(StepIdx::new(2)),
                },
            },
            finish_node(1),
            finish_node(2),
        ];
        let parts = make_parts(nodes);
        assert_eq!(validate_forward_edges(&parts), Ok(()));
    }

    #[test]
    fn forward_edges_rejects_choose_slot_backward_branch() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![crate::workflow::SlotBranch {
                        condition: SlotIdx::new(0),
                        target: StepIdx::new(0),
                    }].into_boxed_slice(),
                    otherwise: None,
                },
            },
        ];
        let parts = make_parts(nodes);
        let result = validate_forward_edges(&parts);
        assert!(matches!(
            result,
            Err(WorkflowError::BackwardEdge { from, to })
            if from == StepIdx::new(0) && to == StepIdx::new(0)
        ));
    }

    // -- validate_forward_edges: loop span (done target) --

    #[test]
    fn forward_edges_accepts_for_each_done_forward() {
        let nodes = vec![
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
            nop_node(1, None),
            finish_node(2),
        ];
        let parts = make_parts(nodes);
        assert_eq!(validate_forward_edges(&parts), Ok(()));
    }

    #[test]
    fn forward_edges_rejects_for_each_done_backward() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 10,
                    body: StepIdx::new(0),
                    done: StepIdx::new(0),
                },
            },
            nop_node(0, None),
        ];
        let parts = make_parts(nodes);
        let result = validate_forward_edges(&parts);
        assert!(matches!(result, Err(WorkflowError::BackwardEdge { .. })));
    }

    // -- validate_forward_edges: error handler forward target --

    #[test]
    fn forward_edges_accepts_error_handler_forward() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ErrorHandler {
                    body: StepIdx::new(1),
                    handler: StepIdx::new(2),
                },
            },
            nop_node(1, None),
            finish_node(2),
        ];
        let parts = make_parts(nodes);
        assert_eq!(validate_forward_edges(&parts), Ok(()));
    }

    #[test]
    fn forward_edges_rejects_error_handler_backward() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ErrorHandler {
                    body: StepIdx::new(0),
                    handler: StepIdx::new(2),
                },
            },
            nop_node(0, None),
        ];
        let parts = make_parts(nodes);
        let result = validate_forward_edges(&parts);
        assert!(matches!(result, Err(WorkflowError::BackwardEdge { .. })));
    }

    // -- validate_forward_edges: self-loop on next --

    #[test]
    fn forward_edges_rejects_self_loop_via_next() {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(0)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }];
        let parts = make_parts(nodes);
        let result = validate_forward_edges(&parts);
        assert!(matches!(
            result,
            Err(WorkflowError::BackwardEdge { from, to })
            if from == StepIdx::new(0) && to == StepIdx::new(0)
        ));
    }

    // -- Loop nesting: properly nested loops --

    #[test]
    fn forward_edges_accepts_properly_nested_loops() {
        let nodes = vec![
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
                    done: StepIdx::new(4),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(1),
                    item_slot: SlotIdx::new(2),
                    limit: 10,
                    body: StepIdx::new(2),
                    done: StepIdx::new(3),
                },
            },
            nop_node(2, None),
            finish_node(3),
            finish_node(4),
        ];
        let parts = make_parts(nodes);
        assert_eq!(validate_forward_edges(&parts), Ok(()));
    }

    #[test]
    fn forward_edges_rejects_improperly_nested_loops() {
        let nodes = vec![
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
                    done: StepIdx::new(3),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(1),
                    item_slot: SlotIdx::new(2),
                    limit: 10,
                    body: StepIdx::new(2),
                    done: StepIdx::new(4),
                },
            },
            nop_node(2, None),
            finish_node(3),
            finish_node(4),
        ];
        let parts = make_parts(nodes);
        let result = validate_forward_edges(&parts);
        // Inner loop (at 1) has done=4, which exceeds outer done=3
        assert!(matches!(
            result,
            Err(WorkflowError::ImproperLoopNesting { inner, outer_done })
            if inner == StepIdx::new(1) && outer_done == StepIdx::new(3)
        ));
    }

    // -- Non-loop kinds do not create loop spans --

    #[test]
    fn forward_edges_accepts_nop_chain_without_loops() {
        let parts = make_parts(vec![
            nop_node(0, Some(1)),
            nop_node(1, Some(2)),
            nop_node(2, Some(3)),
            finish_node(3),
        ]);
        assert_eq!(validate_forward_edges(&parts), Ok(()));
    }

    // -- TogetherStart loop span validation --

    #[test]
    fn forward_edges_accepts_together_start_forward_join() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherStart {
                    branches: vec![StepIdx::new(1)].into_boxed_slice(),
                    join: StepIdx::new(2),
                },
            },
            nop_node(1, None),
            finish_node(2),
        ];
        let parts = make_parts(nodes);
        assert_eq!(validate_forward_edges(&parts), Ok(()));
    }
}
