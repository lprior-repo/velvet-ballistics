//! Gate 11: ForEach/Together body graph is well-formed.

#![allow(unreachable_pub)]

use crate::{ValidationError, ValidationResult};
use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

pub fn validate_gate_11_loop_body_graph(parts: &WorkflowParts) -> ValidationResult<()> {
    let node_count = parts.nodes.len();
    for (index, node) in parts.nodes.iter().enumerate() {
        match &node.kind {
            CompiledNodeKind::ForEachStart { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "for_each body")?;
                check_step_in_range(*done, node_count, index, "for_each done")?;
                check_loop_span(index, *body, *done, node_count)?;
            }
            CompiledNodeKind::ForEachNext { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "for_each_next body")?;
                check_step_in_range(*done, node_count, index, "for_each_next done")?;
            }
            CompiledNodeKind::TogetherStart { branches, join } => {
                for (bi, branch) in branches.iter().enumerate() {
                    check_step_in_range(*branch, node_count, index, &format!("together branch {bi}"))?;
                }
                check_step_in_range(*join, node_count, index, "together join")?;
                check_together_span(index, branches, *join, node_count)?;
            }
            CompiledNodeKind::TogetherBranch { entry, join, .. } => {
                check_step_in_range(*entry, node_count, index, "together_branch entry")?;
                check_step_in_range(*join, node_count, index, "together_branch join")?;
            }
            CompiledNodeKind::TogetherJoin { .. } => {}
            CompiledNodeKind::CollectStart { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "collect body")?;
                check_step_in_range(*done, node_count, index, "collect done")?;
                check_loop_span(index, *body, *done, node_count)?;
            }
            CompiledNodeKind::CollectPage { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "collect_page body")?;
                check_step_in_range(*done, node_count, index, "collect_page done")?;
            }
            CompiledNodeKind::CollectNext { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "collect_next body")?;
                check_step_in_range(*done, node_count, index, "collect_next done")?;
            }
            CompiledNodeKind::ReduceStart { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "reduce body")?;
                check_step_in_range(*done, node_count, index, "reduce done")?;
                check_loop_span(index, *body, *done, node_count)?;
            }
            CompiledNodeKind::ReduceNext { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "reduce_next body")?;
                check_step_in_range(*done, node_count, index, "reduce_next done")?;
            }
            CompiledNodeKind::RepeatStart { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "repeat body")?;
                check_step_in_range(*done, node_count, index, "repeat done")?;
                check_loop_span(index, *body, *done, node_count)?;
            }
            CompiledNodeKind::RepeatAttempt { body, done, .. } => {
                check_step_in_range(*body, node_count, index, "repeat_attempt body")?;
                check_step_in_range(*done, node_count, index, "repeat_attempt done")?;
            }
            CompiledNodeKind::RepeatCheck { done, .. } => {
                check_step_in_range(*done, node_count, index, "repeat_check done")?;
            }
            CompiledNodeKind::RetryCheck { body, exhausted, .. } => {
                check_step_in_range(*body, node_count, index, "retry_check body")?;
                check_step_in_range(*exhausted, node_count, index, "retry_check exhausted")?;
            }
            CompiledNodeKind::ErrorHandler { body, handler, error_slot: _ } => {
                check_step_in_range(*body, node_count, index, "error_handler body")?;
                check_step_in_range(*handler, node_count, index, "error_handler handler")?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn check_step_in_range(step: StepIdx, node_count: usize, source_index: usize, label: &str) -> ValidationResult<()> {
    if step.as_usize() >= node_count {
        return Err(ValidationError::LoopBodyStepOutOfRange {
            step: step.as_usize(),
            node_count,
            source_node: source_index,
            label: label.to_owned(),
        });
    }
    Ok(())
}

fn check_loop_span(start_index: usize, body: StepIdx, done: StepIdx, node_count: usize) -> ValidationResult<()> {
    let body_usize = body.as_usize();
    let done_usize = done.as_usize();
    if body_usize <= start_index {
        return Err(ValidationError::LoopBodyStepOutOfRange {
            step: body_usize, node_count, source_node: start_index,
            label: "loop body must be after loop start".to_owned(),
        });
    }
    if done_usize <= body_usize {
        return Err(ValidationError::LoopBodyStepOutOfRange {
            step: done_usize, node_count, source_node: start_index,
            label: "loop done must be after loop body".to_owned(),
        });
    }
    Ok(())
}

fn check_together_span(start_index: usize, branches: &[StepIdx], join: StepIdx, node_count: usize) -> ValidationResult<()> {
    let join_usize = join.as_usize();
    for (bi, branch) in branches.iter().enumerate() {
        let bu = branch.as_usize();
        if bu <= start_index {
            return Err(ValidationError::LoopBodyStepOutOfRange {
                step: bu, node_count, source_node: start_index,
                label: format!("together branch {bi} must be after start"),
            });
        }
        if join_usize <= bu {
            return Err(ValidationError::LoopBodyStepOutOfRange {
                step: join_usize, node_count, source_node: start_index,
                label: format!("together join must be after branch {bi}"),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{ConstIdx, SlotIdx};
    use vb_core::workflow::{CompiledNode, ResourceContract};

    fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts {
        WorkflowParts {
            name: Box::from("test"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        }
    }

    fn nop_node(index: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: Some(StepIdx::new(index.saturating_add(1))),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }
    }

    fn finish_node(index: u16, result_slot: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(result_slot),
            },
        }
    }

    // -- Pass cases --

    #[test]
    fn accepts_nop_workflow() {
        let parts = make_parts(vec![nop_node(0), finish_node(1, 0)], 1);
        assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
    }

    #[test]
    fn accepts_single_node_workflow() {
        let parts = make_parts(vec![finish_node(0, 0)], 1);
        assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_for_each() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
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
                kind: CompiledNodeKind::ForEachNext {
                    iterator_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            },
            finish_node(2, 0),
        ];
        let parts = make_parts(nodes, 2);
        assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_together() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherStart {
                    branches: Box::new([StepIdx::new(1), StepIdx::new(2)]),
                    join: StepIdx::new(3),
                },
            },
            nop_node(1),
            nop_node(2),
            finish_node(3, 0),
        ];
        let parts = make_parts(nodes, 1);
        assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
    }

    #[test]
    fn accepts_together_with_empty_branches() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherStart {
                    branches: Box::new([]),
                    join: StepIdx::new(1),
                },
            },
            finish_node(1, 0),
        ];
        let parts = make_parts(nodes, 1);
        assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_repeat() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::RepeatStart {
                    max_attempts: 3,
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::RepeatAttempt {
                    attempt_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    done: StepIdx::new(3),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::RepeatCheck {
                    attempt_slot: SlotIdx::new(0),
                    done: StepIdx::new(3),
                },
            },
            finish_node(3, 0),
        ];
        let parts = make_parts(nodes, 1);
        assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_collect() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectStart {
                    source: SlotIdx::new(0),
                    limit: 100,
                    page_size: 10,
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
                kind: CompiledNodeKind::CollectPage {
                    collector_slot: SlotIdx::new(0),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            },
            finish_node(2, 0),
        ];
        let parts = make_parts(nodes, 1);
        assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
    }

    #[test]
    fn accepts_valid_reduce() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ReduceStart {
                    input: SlotIdx::new(0),
                    accumulator: SlotIdx::new(1),
                    initial: ConstIdx::new(0),
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
                kind: CompiledNodeKind::ReduceNext {
                    iterator_slot: SlotIdx::new(0),
                    accumulator: SlotIdx::new(1),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            },
            finish_node(2, 0),
        ];
        let parts = make_parts(nodes, 2);
        assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
    }

    #[test]
    fn accepts_error_handler_with_valid_steps() {
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
                    error_slot: None,
                },
            },
            nop_node(1),
            finish_node(2, 0),
        ];
        let parts = make_parts(nodes, 1);
        assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
    }

    // -- Fail cases --

    #[test]
    fn rejects_for_each_body_out_of_range() {
        let nodes = vec![CompiledNode {
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
                done: StepIdx::new(2),
            },
        }];
        let parts = make_parts(nodes, 2);
        assert!(matches!(
            validate_gate_11_loop_body_graph(&parts),
            Err(ValidationError::LoopBodyStepOutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_for_each_done_out_of_range() {
        let nodes = vec![CompiledNode {
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
                done: StepIdx::new(99),
            },
        }];
        let parts = make_parts(nodes, 2);
        assert!(matches!(
            validate_gate_11_loop_body_graph(&parts),
            Err(ValidationError::LoopBodyStepOutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_together_branch_out_of_range() {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(99)]),
                join: StepIdx::new(1),
            },
        }];
        let parts = make_parts(nodes, 1);
        assert!(matches!(
            validate_gate_11_loop_body_graph(&parts),
            Err(ValidationError::LoopBodyStepOutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_loop_body_not_after_start() {
        let nodes = vec![CompiledNode {
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
                done: StepIdx::new(1),
            },
        }];
        let parts = make_parts(nodes, 2);
        assert!(matches!(
            validate_gate_11_loop_body_graph(&parts),
            Err(ValidationError::LoopBodyStepOutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_done_before_body() {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(2),
                done: StepIdx::new(1),
            },
        }];
        let parts = make_parts(nodes, 2);
        assert!(matches!(
            validate_gate_11_loop_body_graph(&parts),
            Err(ValidationError::LoopBodyStepOutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_together_branch_before_start() {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(0)]),
                join: StepIdx::new(3),
            },
        }];
        let parts = make_parts(nodes, 1);
        assert!(matches!(
            validate_gate_11_loop_body_graph(&parts),
            Err(ValidationError::LoopBodyStepOutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_together_join_before_branch() {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(2)]),
                join: StepIdx::new(1),
            },
        }];
        let parts = make_parts(nodes, 3);
        assert!(matches!(
            validate_gate_11_loop_body_graph(&parts),
            Err(ValidationError::LoopBodyStepOutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_retry_check_body_out_of_range() {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RetryCheck {
                policy_slot: SlotIdx::new(0),
                body: StepIdx::new(99),
                exhausted: StepIdx::new(1),
            },
        }];
        let parts = make_parts(nodes, 1);
        assert!(matches!(
            validate_gate_11_loop_body_graph(&parts),
            Err(ValidationError::LoopBodyStepOutOfRange { .. })
        ));
    }

    #[test]
    fn rejects_error_handler_body_out_of_range() {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(99),
                handler: StepIdx::new(1),
                error_slot: None,
            },
        }];
        let parts = make_parts(nodes, 1);
        assert!(matches!(
            validate_gate_11_loop_body_graph(&parts),
            Err(ValidationError::LoopBodyStepOutOfRange { .. })
        ));
    }
}
