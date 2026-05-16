#![forbid(unsafe_code)]
//! Graph construction from compiled workflow.

use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use crate::workflow::node_mapping::node_kind_to_visual;

use super::types::{EdgeType, NodeBadge, WorkflowEdge, WorkflowGraph, WorkflowNode};

#[must_use]
pub fn build_graph(workflow: &CompiledWorkflow) -> WorkflowGraph {
    let parts = workflow.to_parts();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for compiled in parts.nodes.iter() {
        let step_idx = compiled.id;
        let visual = node_kind_to_visual(&compiled.kind);
        let kind_name = visual.label.clone();
        let badges = build_badges(compiled, parts.resource_contract.max_retry_attempts);

        nodes.push(WorkflowNode {
            step_idx,
            kind_name,
            visual,
            position: None,
            badges,
        });

        if let Some(next) = compiled.next {
            let label = if is_loop_back_edge(step_idx, next, &parts.nodes) {
                Some(String::from("loop"))
            } else {
                None
            };
            let edge_type = classify_sequential_edge(step_idx, next, &parts.nodes);
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: next,
                edge_type,
                label,
            });
        }

        emit_kind_edges(step_idx, &compiled.kind, &mut edges);
    }

    WorkflowGraph {
        nodes,
        edges,
        entry_step: parts.entry,
        slot_count: parts.slot_count,
        workflow_name: parts.name.into_string(),
    }
}

fn build_badges(node: &CompiledNode, max_retry_attempts: u16) -> Vec<NodeBadge> {
    let mut badges = Vec::new();

    match &node.kind {
        CompiledNodeKind::Do { action, .. } => {
            badges.push(NodeBadge::ActionId(action.get()));
        }
        CompiledNodeKind::RepeatStart { max_attempts, .. } => {
            badges.push(NodeBadge::RetryMax(*max_attempts));
        }
        CompiledNodeKind::RetryCheck { .. } => {
            badges.push(NodeBadge::RetryMax(max_retry_attempts));
        }
        _ => {}
    }

    badges
}

fn is_loop_back_edge(from: StepIdx, to: StepIdx, nodes: &[CompiledNode]) -> bool {
    if to.get() > from.get() {
        return false;
    }
    let target_node = match nodes.get(to.as_usize()) {
        Some(n) => n,
        None => return false,
    };
    matches!(
        target_node.kind,
        CompiledNodeKind::ForEachNext { .. }
            | CompiledNodeKind::ForEachStart { .. }
            | CompiledNodeKind::RepeatAttempt { .. }
            | CompiledNodeKind::RepeatCheck { .. }
            | CompiledNodeKind::CollectNext { .. }
            | CompiledNodeKind::CollectPage { .. }
            | CompiledNodeKind::ReduceNext { .. }
    )
}

fn classify_sequential_edge(from: StepIdx, to: StepIdx, nodes: &[CompiledNode]) -> EdgeType {
    if is_loop_back_edge(from, to, nodes) {
        return EdgeType::RetryRoute;
    }

    let target_node = match nodes.get(to.as_usize()) {
        Some(n) => n,
        None => return EdgeType::Sequential,
    };

    if matches!(
        target_node.kind,
        CompiledNodeKind::ForEachJoin { .. }
            | CompiledNodeKind::TogetherJoin { .. }
            | CompiledNodeKind::CollectFinish { .. }
            | CompiledNodeKind::ReduceFinish { .. }
            | CompiledNodeKind::RepeatFinish { .. }
    ) {
        return EdgeType::JoinRoute;
    }

    EdgeType::Sequential
}

fn emit_kind_edges(step_idx: StepIdx, kind: &CompiledNodeKind, edges: &mut Vec<WorkflowEdge>) {
    match kind {
        CompiledNodeKind::Choose { branches, otherwise } => {
            for (i, branch) in branches.iter().enumerate() {
                edges.push(WorkflowEdge {
                    from_step: step_idx,
                    to_step: branch.target,
                    edge_type: EdgeType::Branch { condition_index: i },
                    label: Some(format!("cond-{i}")),
                });
            }
            if let Some(fallback) = otherwise {
                edges.push(WorkflowEdge {
                    from_step: step_idx,
                    to_step: *fallback,
                    edge_type: EdgeType::Branch {
                        condition_index: branches.len(),
                    },
                    label: Some(String::from("otherwise")),
                });
            }
        }

        CompiledNodeKind::ChooseSlot { branches, otherwise } => {
            for (i, branch) in branches.iter().enumerate() {
                edges.push(WorkflowEdge {
                    from_step: step_idx,
                    to_step: branch.target,
                    edge_type: EdgeType::Branch { condition_index: i },
                    label: Some(format!("slot-cond-{i}")),
                });
            }
            if let Some(fallback) = otherwise {
                edges.push(WorkflowEdge {
                    from_step: step_idx,
                    to_step: *fallback,
                    edge_type: EdgeType::Branch {
                        condition_index: branches.len(),
                    },
                    label: Some(String::from("otherwise")),
                });
            }
        }

        CompiledNodeKind::ForEachStart { body, done, .. }
        | CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. } => {
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *body,
                edge_type: EdgeType::Sequential,
                label: Some(String::from("body")),
            });
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *done,
                edge_type: EdgeType::JoinRoute,
                label: Some(String::from("done")),
            });
        }

        CompiledNodeKind::ForEachNext { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. } => {
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *body,
                edge_type: EdgeType::RetryRoute,
                label: Some(String::from("body")),
            });
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *done,
                edge_type: EdgeType::JoinRoute,
                label: Some(String::from("done")),
            });
        }

        CompiledNodeKind::CollectPage { body, done, .. } => {
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *body,
                edge_type: EdgeType::RetryRoute,
                label: Some(String::from("body")),
            });
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *done,
                edge_type: EdgeType::JoinRoute,
                label: Some(String::from("done")),
            });
        }

        CompiledNodeKind::RepeatStart { body, done, .. } => {
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *body,
                edge_type: EdgeType::RetryRoute,
                label: Some(String::from("body")),
            });
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *done,
                edge_type: EdgeType::JoinRoute,
                label: Some(String::from("done")),
            });
        }

        CompiledNodeKind::RepeatAttempt { body, done, .. } => {
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *body,
                edge_type: EdgeType::RetryRoute,
                label: Some(String::from("body")),
            });
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *done,
                edge_type: EdgeType::JoinRoute,
                label: Some(String::from("done")),
            });
        }

        CompiledNodeKind::RepeatCheck { done, .. } => {
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *done,
                edge_type: EdgeType::JoinRoute,
                label: Some(String::from("done")),
            });
        }

        CompiledNodeKind::RetryCheck { body, exhausted, .. } => {
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *body,
                edge_type: EdgeType::RetryRoute,
                label: Some(String::from("retry")),
            });
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *exhausted,
                edge_type: EdgeType::ErrorRoute,
                label: Some(String::from("exhausted")),
            });
        }

        CompiledNodeKind::TogetherStart { branches, join } => {
            for (i, branch_entry) in branches.iter().enumerate() {
                edges.push(WorkflowEdge {
                    from_step: step_idx,
                    to_step: *branch_entry,
                    edge_type: EdgeType::Sequential,
                    label: Some(format!("branch-{i}")),
                });
            }
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *join,
                edge_type: EdgeType::JoinRoute,
                label: Some(String::from("join")),
            });
        }

        CompiledNodeKind::TogetherBranch { join, .. } => {
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *join,
                edge_type: EdgeType::JoinRoute,
                label: Some(String::from("join")),
            });
        }

        CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *body,
                edge_type: EdgeType::Sequential,
                label: Some(String::from("body")),
            });
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *handler,
                edge_type: EdgeType::ErrorRoute,
                label: Some(String::from("handler")),
            });
        }

        CompiledNodeKind::Jump { target } => {
            edges.push(WorkflowEdge {
                from_step: step_idx,
                to_step: *target,
                edge_type: EdgeType::Sequential,
                label: Some(String::from("jump")),
            });
        }

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
        | CompiledNodeKind::Finish { .. } => {}
    }
}
