#![forbid(unsafe_code)]
//! DOT graph generation for compiled workflows.
//!
//! Extracted from `commands_workflow/mod.rs` to keep that file under the
//! 300-line source cap. All items are public to the parent module so
//! existing call sites continue to work.

use vb_core::ids::StepIdx;
use vb_core::{CompiledNodeKind, CompiledWorkflow};

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
            edge_count = edge_count.saturating_add(1);
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
            edge_count = edge_count.saturating_add(1);
        }
    }

    dot_lines.push("}".to_string());

    DotGraph {
        node_count,
        edge_count,
        dot: dot_lines.join("\n"),
    }
}

pub(crate) fn node_kind_label(kind: &CompiledNodeKind) -> &'static str {
    match kind {
        CompiledNodeKind::Nop => "nop",
        CompiledNodeKind::SetConst { .. } => "set_const",
        CompiledNodeKind::Copy { .. } => "copy",
        CompiledNodeKind::EvalExpr { .. } => "eval_expr",
        CompiledNodeKind::BuildObject { .. } => "build_object",
        CompiledNodeKind::BuildList { .. } => "build_list",
        CompiledNodeKind::Do { .. } => "do",
        CompiledNodeKind::Finish { .. } => "finish",
        CompiledNodeKind::WaitUntil { .. } => "wait_until",
        CompiledNodeKind::ChooseSlot { .. } => "choose_slot",
        CompiledNodeKind::RepeatStart { .. } => "repeat_start",
        CompiledNodeKind::RepeatFinish { .. } => "repeat_finish",
        CompiledNodeKind::ForEachStart { .. } => "for_each_start",
        CompiledNodeKind::ForEachJoin { .. } => "for_each_join",
        CompiledNodeKind::CollectStart { .. } => "collect_start",
        CompiledNodeKind::CollectFinish { .. } => "collect_finish",
        CompiledNodeKind::ReduceStart { .. } => "reduce_start",
        CompiledNodeKind::ReduceFinish { .. } => "reduce_finish",
        CompiledNodeKind::TogetherStart { .. } => "together_start",
        CompiledNodeKind::TogetherJoin { .. } => "together_join",
        _ => "other",
    }
}

pub(crate) fn collect_kind_edges(node_idx: u16, kind: &CompiledNodeKind) -> Vec<(u16, u16, String)> {
    let mut edges = Vec::new();
    match kind {
        CompiledNodeKind::ForEachStart { body, done, .. } => {
            edges.push((node_idx, body.get(), String::new()));
            edges.push((node_idx, done.get(), "done".to_string()));
        }
        CompiledNodeKind::ForEachJoin { .. } => {}
        CompiledNodeKind::TogetherStart { branches, join } => {
            for branch in branches.iter() {
                edges.push((node_idx, branch.get(), "branch".to_string()));
            }
            edges.push((node_idx, join.get(), "join".to_string()));
        }
        CompiledNodeKind::ChooseSlot { branches, otherwise } => {
            for branch in branches.iter() {
                edges.push((node_idx, branch.target.get(), "branch".to_string()));
            }
            if let Some(otherwise) = otherwise {
                edges.push((node_idx, otherwise.get(), "otherwise".to_string()));
            }
        }
        _ => {}
    }
    edges
}
