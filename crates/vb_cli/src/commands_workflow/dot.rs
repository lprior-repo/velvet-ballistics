#![forbid(unsafe_code)]
//! DOT graph generation for workflow visualization.

use vb_core::ids::StepIdx;
use vb_core::{CompiledNodeKind, CompiledWorkflow};

use super::helpers::{node_kind_label, saturating_add};

pub(crate) struct DotGraph {
    pub node_count: usize,
    pub edge_count: usize,
    pub dot: String,
}

pub(crate) fn generate_dot(workflow: &CompiledWorkflow) -> DotGraph {
    let node_count = usize::from(workflow.node_count());
    let mut dot_lines: Vec<String> = Vec::new();
    dot_lines.push("digraph workflow {".to_string());
    dot_lines.push("    node [shape=box];".to_string());

    let mut edge_count: usize = 0;

    // Declare all nodes
    for i in 0..node_count {
        let step = StepIdx::new(u16::try_from(i).unwrap_or(u16::MAX));
        let label = match workflow.step_name(step) {
            Some(name) => format!("{i}: {name}"),
            None => {
                let node = match workflow.node(step) {
                    Some(n) => n,
                    None => continue,
                };
                let kind_label = node_kind_label(&node.kind);
                format!("{i}: {kind_label}")
            }
        };
        let escaped = label.replace('"', "\\\"");
        dot_lines.push(format!("    node_{i} [label=\"{escaped}\"];"));
    }

    // Add edges
    for i in 0..node_count {
        let step = StepIdx::new(u16::try_from(i).unwrap_or(u16::MAX));
        let node = match workflow.node(step) {
            Some(n) => n,
            None => continue,
        };

        if let Some(next) = node.next {
            dot_lines.push(format!("    node_{i} -> node_{};", next.get()));
            edge_count = saturating_add(edge_count, 1);
        }

        let extra_edges = collect_kind_edges(u16::try_from(i).unwrap_or(u16::MAX), &node.kind);
        for (from, to, label) in &extra_edges {
            let edge_decl = if label.is_empty() {
                format!("    node_{from} -> node_{to};")
            } else {
                let escaped = label.replace('"', "\\\"");
                format!("    node_{from} -> node_{to} [label=\"{escaped}\"];")
            };
            dot_lines.push(edge_decl);
            edge_count = saturating_add(edge_count, 1);
        }
    }

    dot_lines.push("}".to_string());

    DotGraph {
        node_count,
        edge_count,
        dot: dot_lines.join("\n"),
    }
}

fn collect_kind_edges(node_idx: u16, kind: &CompiledNodeKind) -> Vec<(u16, u16, String)> {
    let mut edges: Vec<(u16, u16, String)> = Vec::new();
    match kind {
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            for branch in branches.iter() {
                edges.push((node_idx, branch.target.get(), String::new()));
            }
            if let Some(fallback) = otherwise {
                edges.push((node_idx, fallback.get(), "otherwise".to_string()));
            }
        }
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            for branch in branches.iter() {
                edges.push((node_idx, branch.target.get(), String::new()));
            }
            if let Some(fallback) = otherwise {
                edges.push((node_idx, fallback.get(), "otherwise".to_string()));
            }
        }
        CompiledNodeKind::ForEachStart { body, done, .. }
        | CompiledNodeKind::ForEachNext { body, done, .. } => {
            edges.push((node_idx, body.get(), "body".to_string()));
            edges.push((node_idx, done.get(), "done".to_string()));
        }
        CompiledNodeKind::TogetherStart { branches, join } => {
            for branch_step in branches.iter() {
                edges.push((node_idx, branch_step.get(), "branch".to_string()));
            }
            edges.push((node_idx, join.get(), "join".to_string()));
        }
        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            edges.push((node_idx, entry.get(), "entry".to_string()));
            edges.push((node_idx, join.get(), "join".to_string()));
        }
        CompiledNodeKind::TogetherJoin { .. } => {}
        CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::CollectPage { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. } => {
            edges.push((node_idx, body.get(), "body".to_string()));
            edges.push((node_idx, done.get(), "done".to_string()));
        }
        CompiledNodeKind::CollectFinish { .. } => {}
        CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. } => {
            edges.push((node_idx, body.get(), "body".to_string()));
            edges.push((node_idx, done.get(), "done".to_string()));
        }
        CompiledNodeKind::ReduceFinish { .. } => {}
        CompiledNodeKind::RepeatStart { body, done, .. }
        | CompiledNodeKind::RepeatAttempt { body, done, .. } => {
            edges.push((node_idx, body.get(), "body".to_string()));
            edges.push((node_idx, done.get(), "done".to_string()));
        }
        CompiledNodeKind::RepeatCheck { done, .. } => {
            edges.push((node_idx, done.get(), "done".to_string()));
        }
        CompiledNodeKind::RepeatFinish { .. } => {}
        CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            edges.push((node_idx, body.get(), "body".to_string()));
            edges.push((node_idx, handler.get(), "handler".to_string()));
        }
        CompiledNodeKind::Jump { target } => {
            edges.push((node_idx, target.get(), String::new()));
        }
        CompiledNodeKind::ForEachJoin { .. }
        | CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::BuildObject { .. }
        | CompiledNodeKind::BuildList { .. }
        | CompiledNodeKind::Do { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. }
        | CompiledNodeKind::RetryCheck { .. }
        | CompiledNodeKind::Finish { .. } => {}
        _ => {}
    }
    edges
}