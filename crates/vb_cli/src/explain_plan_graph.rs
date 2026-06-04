#![forbid(unsafe_code)]
#[path = "explain_plan_graph_edges.rs"]
mod graph_edges;

use serde_json::Value;
use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, StepIdx};
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlanNode {
    pub(crate) step: u16,
    pub(crate) name: String,
    pub(crate) kind: &'static str,
    pub(crate) output: Option<u16>,
    pub(crate) next: Option<u16>,
    pub(crate) on_error: Option<u16>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlanEdge {
    pub(crate) from: u16,
    pub(crate) to: u16,
    pub(crate) label: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlanAction {
    pub(crate) step: u16,
    pub(crate) name: String,
    pub(crate) action: u16,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SuspensionPoint {
    pub(crate) step: u16,
    pub(crate) name: String,
    pub(crate) kind: &'static str,
}
pub(crate) fn graph_value(compiled: &CompiledWorkflow) -> Value {
    let nodes: Vec<Value> = plan_nodes(compiled).iter().map(plan_node_value).collect();
    let edges: Vec<Value> = plan_edges(compiled).iter().map(plan_edge_value).collect();
    serde_json::json!({
        "nodes": nodes,
        "edges": edges,
    })
}

pub(crate) fn actions_value(compiled: &CompiledWorkflow) -> Value {
    Value::Array(
        plan_actions(compiled)
            .iter()
            .map(plan_action_value)
            .collect(),
    )
}

pub(crate) fn suspension_points_value(compiled: &CompiledWorkflow) -> Value {
    Value::Array(
        plan_suspension_points(compiled)
            .iter()
            .map(suspension_point_value)
            .collect(),
    )
}

pub(crate) fn plan_nodes(compiled: &CompiledWorkflow) -> Vec<PlanNode> {
    (0..compiled.node_count())
        .filter_map(|step| {
            let step_idx = StepIdx::new(step);
            compiled.node(step_idx).map(|node| PlanNode {
                step,
                name: step_name(compiled, step_idx),
                kind: node_kind_label(&node.kind),
                output: node.output.map(vb_core::SlotIdx::get),
                next: node.next.map(StepIdx::get),
                on_error: node.on_error.map(StepIdx::get),
            })
        })
        .collect()
}

pub(crate) fn plan_edges(compiled: &CompiledWorkflow) -> Vec<PlanEdge> {
    (0..compiled.node_count())
        .filter_map(|step| {
            let step_idx = StepIdx::new(step);
            compiled.node(step_idx).map(|node| (step, node))
        })
        .flat_map(|(step, node)| {
            edge_targets(node)
                .into_iter()
                .map(move |(label, target)| PlanEdge {
                    from: step,
                    to: target.get(),
                    label,
                })
        })
        .collect()
}

pub(crate) fn plan_actions(compiled: &CompiledWorkflow) -> Vec<PlanAction> {
    (0..compiled.node_count())
        .filter_map(|step| {
            let step_idx = StepIdx::new(step);
            compiled.node(step_idx).and_then(|node| match node.kind {
                CompiledNodeKind::Do { action, .. } => Some(PlanAction {
                    step,
                    name: step_name(compiled, step_idx),
                    action: action.get(),
                }),
                _ => None,
            })
        })
        .collect()
}

pub(crate) fn plan_suspension_points(compiled: &CompiledWorkflow) -> Vec<SuspensionPoint> {
    (0..compiled.node_count())
        .filter_map(|step| {
            let step_idx = StepIdx::new(step);
            compiled.node(step_idx).and_then(|node| {
                suspension_kind(&node.kind).map(|kind| SuspensionPoint {
                    step,
                    name: step_name(compiled, step_idx),
                    kind,
                })
            })
        })
        .collect()
}

pub(crate) fn option_u16_text(value: Option<u16>) -> String {
    value.map_or_else(|| String::from("none"), |number| number.to_string())
}
fn plan_node_value(node: &PlanNode) -> Value {
    serde_json::json!({
        "step": node.step,
        "name": node.name.as_str(),
        "kind": node.kind,
        "output_slot": node.output,
        "next_step": node.next,
        "on_error_step": node.on_error,
    })
}

fn plan_edge_value(edge: &PlanEdge) -> Value {
    serde_json::json!({
        "from": edge.from,
        "to": edge.to,
        "label": edge.label.as_str(),
    })
}

fn plan_action_value(action: &PlanAction) -> Value {
    serde_json::json!({
        "step": action.step,
        "name": action.name.as_str(),
        "action": action.action,
    })
}

fn suspension_point_value(point: &SuspensionPoint) -> Value {
    serde_json::json!({
        "step": point.step,
        "name": point.name.as_str(),
        "kind": point.kind,
    })
}
fn edge_targets(node: &CompiledNode) -> Vec<(String, StepIdx)> {
    node.next
        .iter()
        .map(|target| (String::from("next"), *target))
        .chain(
            node.on_error
                .iter()
                .map(|target| (String::from("on_error"), *target)),
        )
        .chain(graph_edges::kind_edge_targets(&node.kind))
        .collect()
}

fn step_name(compiled: &CompiledWorkflow, step: StepIdx) -> String {
    compiled
        .step_name(step)
        .map_or_else(|| String::from("<unnamed>"), String::from)
}

fn node_kind_label(kind: &CompiledNodeKind) -> &'static str {
    match kind {
        CompiledNodeKind::Nop => "nop",
        CompiledNodeKind::SetConst { .. } => "set_const",
        CompiledNodeKind::Copy { .. } => "copy",
        CompiledNodeKind::EvalExpr { .. } => "eval_expr",
        CompiledNodeKind::BuildObject { .. } => "build_object",
        CompiledNodeKind::BuildList { .. } => "build_list",
        CompiledNodeKind::Do { .. } => "do",
        CompiledNodeKind::Choose { .. } => "choose",
        CompiledNodeKind::ChooseSlot { .. } => "choose_slot",
        CompiledNodeKind::Finish { .. } => "finish",
        _ => complex_node_kind_label(kind),
    }
}
fn complex_node_kind_label(kind: &CompiledNodeKind) -> &'static str {
    control_node_kind_label(kind)
        .or_else(|| collection_node_kind_label(kind))
        .or_else(|| wait_node_kind_label(kind))
        .unwrap_or("unknown")
}

fn control_node_kind_label(kind: &CompiledNodeKind) -> Option<&'static str> {
    Some(match kind {
        CompiledNodeKind::ForEachStart { .. } => "for_each_start",
        CompiledNodeKind::ForEachNext { .. } => "for_each_next",
        CompiledNodeKind::ForEachJoin { .. } => "for_each_join",
        CompiledNodeKind::TogetherStart { .. } => "together_start",
        CompiledNodeKind::TogetherBranch { .. } => "together_branch",
        CompiledNodeKind::TogetherJoin { .. } => "together_join",
        CompiledNodeKind::RepeatStart { .. } => "repeat_start",
        CompiledNodeKind::RepeatAttempt { .. } => "repeat_attempt",
        CompiledNodeKind::RepeatCheck { .. } => "repeat_check",
        CompiledNodeKind::RepeatFinish { .. } => "repeat_finish",
        CompiledNodeKind::Jump { .. } => "jump",
        _ => return None,
    })
}

fn collection_node_kind_label(kind: &CompiledNodeKind) -> Option<&'static str> {
    Some(match kind {
        CompiledNodeKind::CollectStart { .. } => "collect_start",
        CompiledNodeKind::CollectPage { .. } => "collect_page",
        CompiledNodeKind::CollectNext { .. } => "collect_next",
        CompiledNodeKind::CollectFinish { .. } => "collect_finish",
        CompiledNodeKind::ReduceStart { .. } => "reduce_start",
        CompiledNodeKind::ReduceNext { .. } => "reduce_next",
        CompiledNodeKind::ReduceFinish { .. } => "reduce_finish",
        _ => return None,
    })
}

fn wait_node_kind_label(kind: &CompiledNodeKind) -> Option<&'static str> {
    Some(match kind {
        CompiledNodeKind::WaitUntil { .. } => "wait_until",
        CompiledNodeKind::WaitEvent { .. } => "wait_event",
        CompiledNodeKind::Ask { .. } => "ask",
        CompiledNodeKind::AskResume { .. } => "ask_resume",
        CompiledNodeKind::RetryCheck { .. } => "retry_check",
        CompiledNodeKind::ErrorHandler { .. } => "error_handler",
        _ => return None,
    })
}
fn suspension_kind(kind: &CompiledNodeKind) -> Option<&'static str> {
    match kind {
        CompiledNodeKind::Do { .. } => Some("do_action"),
        CompiledNodeKind::WaitUntil { .. } => Some("wait_until"),
        CompiledNodeKind::WaitEvent { .. } => Some("wait_event"),
        CompiledNodeKind::Ask { .. } => Some("ask"),
        CompiledNodeKind::RetryCheck { .. } => Some("retry"),
        CompiledNodeKind::TogetherStart { .. } => Some("fanout"),
        CompiledNodeKind::TogetherJoin { .. } => Some("fanout_join"),
        CompiledNodeKind::CollectStart { .. } => Some("collect"),
        CompiledNodeKind::ReduceStart { .. } => Some("reduce"),
        CompiledNodeKind::RepeatStart { .. } => Some("repeat"),
        _ => None,
    }
}
