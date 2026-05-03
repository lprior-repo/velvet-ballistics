//! Reachability validation.

use crate::ids::StepIdx;

use super::super::node::CompiledNodeKind;
use super::super::types::{WorkflowError, WorkflowParts};

/// Check A: every node must be reachable from the entry step via a forward walk
/// following `next` edges and kind-specific targets.
pub(crate) fn validate_reachability(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let node_count = parts.nodes.len();
    if node_count == 0 {
        return Ok(());
    }

    let mut visited: Vec<bool> = vec![false; node_count];
    let mut queue: Vec<usize> = Vec::new();

    let entry_usize = parts.entry.as_usize();
    if entry_usize >= node_count {
        return Ok(());
    }
    let Some(entry_flag) = visited.get_mut(entry_usize) else {
        return Err(WorkflowError::EntryOutOfBounds { entry: parts.entry });
    };
    *entry_flag = true;
    queue.push(entry_usize);

    let mut head = 0usize;
    while head < queue.len() {
        let current = match queue.get(head) {
            Some(&v) => v,
            None => break,
        };
        head = match head.checked_add(1) {
            Some(v) => v,
            None => break,
        };

        let mut targets: Vec<StepIdx> = Vec::new();
        let node = match parts.nodes.get(current) {
            Some(n) => n,
            None => break,
        };
        if let Some(next) = node.next {
            targets.push(next);
        }
        collect_node_targets(&node.kind, &mut targets);

        for target in targets {
            let target_usize = target.as_usize();
            if target_usize < node_count {
                let Some(flag) = visited.get_mut(target_usize) else {
                    continue;
                };
                if !*flag {
                    *flag = true;
                    queue.push(target_usize);
                }
            }
        }
    }

    for (index, was_visited) in visited.iter().enumerate() {
        if !was_visited {
            return Err(WorkflowError::UnreachableNode {
                step: StepIdx::new(u16::try_from(index).map_err(|_| {
                    WorkflowError::ResourceContractExceeded {
                        resource: "max_steps",
                    }
                })?),
            });
        }
    }
    Ok(())
}

/// Collects all StepIdx targets referenced by a node kind (branch targets,
/// loop body/done, jump target, etc.) but NOT the `next` field.
#[allow(clippy::match_same_arms)]
pub(crate) fn collect_node_targets(kind: &CompiledNodeKind, targets: &mut Vec<StepIdx>) {
    match kind {
        CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::BuildObject { .. }
        | CompiledNodeKind::BuildList { .. }
        | CompiledNodeKind::Do { .. }
        | CompiledNodeKind::ForEachJoin { .. }
        | CompiledNodeKind::CollectFinish { .. }
        | CompiledNodeKind::ReduceFinish { .. }
        | CompiledNodeKind::RepeatFinish { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. }
        | CompiledNodeKind::Finish { .. } => {}
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            for branch in branches.as_ref() {
                targets.push(branch.target);
            }
            if let Some(fallback) = *otherwise {
                targets.push(fallback);
            }
        }
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            for branch in branches.as_ref() {
                targets.push(branch.target);
            }
            if let Some(fallback) = *otherwise {
                targets.push(fallback);
            }
        }
        CompiledNodeKind::ForEachStart { body, done, .. }
        | CompiledNodeKind::ForEachNext { body, done, .. }
        | CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::CollectPage { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. }
        | CompiledNodeKind::RepeatStart { body, done, .. }
        | CompiledNodeKind::RepeatAttempt { body, done, .. }
        | CompiledNodeKind::RetryCheck {
            body,
            exhausted: done,
            ..
        } => {
            targets.push(*body);
            targets.push(*done);
        }
        CompiledNodeKind::RepeatCheck { done, .. } => {
            targets.push(*done);
        }
        CompiledNodeKind::TogetherStart { branches, join } => {
            for branch in branches.as_ref() {
                targets.push(*branch);
            }
            targets.push(*join);
        }
        CompiledNodeKind::TogetherJoin { .. } => {}
        CompiledNodeKind::WaitEvent { .. } => {}
        CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            targets.push(*body);
            targets.push(*handler);
        }
        CompiledNodeKind::Jump { target } => {
            targets.push(*target);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{SlotIdx, StepIdx};
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

    fn make_parts_from_nodes(nodes: Vec<CompiledNode>) -> WorkflowParts {
        let max_steps = u16::try_from(nodes.len()).map_or(u16::MAX, |v| v);
        WorkflowParts {
            name: Box::<str>::from("reachability_test"),
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

    // -- validate_reachability --

    #[test]
    fn reachability_accepts_empty_nodes() {
        let parts = WorkflowParts {
            name: Box::<str>::from("empty"),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: Box::new([]),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 0,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };
        assert_eq!(validate_reachability(&parts), Ok(()));
    }

    #[test]
    fn reachability_accepts_single_finish_node() {
        let parts = make_parts_from_nodes(vec![finish_node(0)]);
        assert_eq!(validate_reachability(&parts), Ok(()));
    }

    #[test]
    fn reachability_accepts_linear_chain() {
        let parts = make_parts_from_nodes(vec![
            nop_node(0, Some(1)),
            nop_node(1, Some(2)),
            finish_node(2),
        ]);
        assert_eq!(validate_reachability(&parts), Ok(()));
    }

    #[test]
    fn reachability_rejects_unreachable_tail_node() {
        let parts = make_parts_from_nodes(vec![
            finish_node(0),
            finish_node(1),
        ]);
        let result = validate_reachability(&parts);
        assert!(matches!(result, Err(WorkflowError::UnreachableNode { step }) if step == StepIdx::new(1)));
    }

    #[test]
    fn reachability_rejects_unreachable_branch_node() {
        let parts = make_parts_from_nodes(vec![
            nop_node(0, Some(1)),
            finish_node(1),
            finish_node(2),
        ]);
        let result = validate_reachability(&parts);
        assert!(matches!(result, Err(WorkflowError::UnreachableNode { step }) if step == StepIdx::new(2)));
    }

    #[test]
    fn reachability_accepts_jump_reaching_orphan() {
        // Node 0 jumps to node 1, making it reachable even though no `next` points to it
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Jump { target: StepIdx::new(1) },
            },
            finish_node(1),
        ];
        let parts = make_parts_from_nodes(nodes);
        assert_eq!(validate_reachability(&parts), Ok(()));
    }

    #[test]
    fn reachability_accepts_choose_slot_branches() {
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
        let parts = make_parts_from_nodes(nodes);
        assert_eq!(validate_reachability(&parts), Ok(()));
    }

    // -- collect_node_targets --

    #[test]
    fn collect_targets_nop_yields_nothing() {
        let mut targets = Vec::new();
        collect_node_targets(&CompiledNodeKind::Nop, &mut targets);
        assert!(targets.is_empty());
    }

    #[test]
    fn collect_targets_finish_yields_nothing() {
        let mut targets = Vec::new();
        collect_node_targets(
            &CompiledNodeKind::Finish { result: SlotIdx::new(0) },
            &mut targets,
        );
        assert!(targets.is_empty());
    }

    #[test]
    fn collect_targets_jump_yields_target() {
        let mut targets = Vec::new();
        collect_node_targets(
            &CompiledNodeKind::Jump { target: StepIdx::new(5) },
            &mut targets,
        );
        assert_eq!(targets, vec![StepIdx::new(5)]);
    }

    #[test]
    fn collect_targets_for_each_start_yields_body_and_done() {
        let mut targets = Vec::new();
        collect_node_targets(
            &CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
            &mut targets,
        );
        assert_eq!(targets, vec![StepIdx::new(1), StepIdx::new(3)]);
    }

    #[test]
    fn collect_targets_together_start_yields_branches_and_join() {
        let mut targets = Vec::new();
        collect_node_targets(
            &CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(1), StepIdx::new(2)].into_boxed_slice(),
                join: StepIdx::new(3),
            },
            &mut targets,
        );
        assert_eq!(targets, vec![StepIdx::new(1), StepIdx::new(2), StepIdx::new(3)]);
    }

    #[test]
    fn collect_targets_error_handler_yields_body_and_handler() {
        let mut targets = Vec::new();
        collect_node_targets(
            &CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
            },
            &mut targets,
        );
        assert_eq!(targets, vec![StepIdx::new(1), StepIdx::new(2)]);
    }

    #[test]
    fn collect_targets_choose_slot_yields_branch_targets_and_otherwise() {
        let mut targets = Vec::new();
        collect_node_targets(
            &CompiledNodeKind::ChooseSlot {
                branches: vec![
                    crate::workflow::SlotBranch { condition: SlotIdx::new(0), target: StepIdx::new(1) },
                    crate::workflow::SlotBranch { condition: SlotIdx::new(1), target: StepIdx::new(2) },
                ].into_boxed_slice(),
                otherwise: Some(StepIdx::new(3)),
            },
            &mut targets,
        );
        assert_eq!(targets, vec![StepIdx::new(1), StepIdx::new(2), StepIdx::new(3)]);
    }

    #[test]
    fn collect_targets_choose_slot_without_otherwise_yields_only_branches() {
        let mut targets = Vec::new();
        collect_node_targets(
            &CompiledNodeKind::ChooseSlot {
                branches: vec![
                    crate::workflow::SlotBranch { condition: SlotIdx::new(0), target: StepIdx::new(1) },
                ].into_boxed_slice(),
                otherwise: None,
            },
            &mut targets,
        );
        assert_eq!(targets, vec![StepIdx::new(1)]);
    }
}
