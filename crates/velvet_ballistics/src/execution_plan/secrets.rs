use serde_json::{Value, json};
use vb_core::{CompiledNodeKind as K, CompiledWorkflow, SlotIdx, WorkflowParts};

pub(super) fn trigger(bytes: &[u8]) -> Result<Value, String> {
    let source = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    let source = vb_compile::parse_workflow_source(source).map_err(|error| error.to_string())?;
    Ok(match source.trigger() {
        vb_compile::yaml_ast::TriggerAst::Manual => json!({"kind": "manual"}),
        vb_compile::yaml_ast::TriggerAst::Webhook => json!({"kind": "webhook"}),
        vb_compile::yaml_ast::TriggerAst::Schedule { cron } => {
            json!({"kind": "schedule", "cron": cron})
        }
        vb_compile::yaml_ast::TriggerAst::Event { event_type } => {
            json!({"kind": "event", "event_type": event_type})
        }
        _ => json!({"kind": "unknown"}),
    })
}

pub(super) fn requirements(workflow: &CompiledWorkflow) -> Value {
    let Some(metadata) = workflow.source_metadata() else {
        return json!([]);
    };
    let parts = workflow.to_parts();
    json!(metadata.secret_requirements().iter().map(|requirement| {
        let steps: Vec<_> = parts.nodes.iter().filter(|node| requirement.slot().is_some_and(|slot| reads(&node.kind, slot, &parts)))
            .map(|node| json!({"step": node.id.get(), "name": workflow.step_name(node.id)})).collect();
        json!({"alias": metadata.symbol(requirement.alias()), "reference": metadata.symbol(requirement.requirement()),
            "input_slot": requirement.slot().map(SlotIdx::get), "direct_reference_steps": steps})
    }).collect::<Vec<_>>())
}

fn reads(kind: &K, slot: SlotIdx, parts: &WorkflowParts) -> bool {
    reads_value(kind, slot) || reads_control(kind, slot, parts)
}

fn reads_value(kind: &K, slot: SlotIdx) -> bool {
    match kind {
        K::Copy { source } => *source == slot,
        K::Do { input, .. }
        | K::ForEachStart { input, .. }
        | K::ForEachConcurrentStart { input, .. }
        | K::ReduceStart { input, .. } => *input == slot,
        K::BuildList { items } => items.contains(&slot),
        K::BuildObject { fields } => fields.iter().any(|(_, value)| *value == slot),
        K::Finish { result } | K::RepeatFinish { result } => *result == slot,
        K::CollectStart { source, .. } => *source == slot,
        K::ConcurrentLaneEnd { result, .. } => *result == Some(slot),
        _ => false,
    }
}

fn reads_control(kind: &K, slot: SlotIdx, parts: &WorkflowParts) -> bool {
    match kind {
        K::EvalExpr { expr } => expression(*expr, slot, parts),
        K::Choose { branches, .. } => branches
            .iter()
            .any(|branch| expression(branch.condition, slot, parts)),
        K::ChooseSlot { branches, .. } => branches.iter().any(|branch| branch.condition == slot),
        K::WaitUntil { deadline_slot } => *deadline_slot == slot,
        K::WaitEvent {
            event,
            timeout_slot,
        } => *event == slot || timeout_slot.slot() == Some(slot),
        K::Ask {
            prompt,
            timeout_slot,
        } => *prompt == slot || timeout_slot.slot() == Some(slot),
        _ => false,
    }
}

fn expression(expr: vb_core::ExprIdx, slot: SlotIdx, parts: &WorkflowParts) -> bool {
    parts
        .expressions
        .get(usize::from(expr.get()))
        .is_some_and(|program| {
            program.ops.iter().any(|op| match *op {
                vb_core::ExprOp::LoadSlot(read) => read == slot,
                vb_core::ExprOp::LoadAccessor(accessor) => parts
                    .accessors
                    .get(usize::from(accessor.get()))
                    .is_some_and(|accessor| accessor.root == slot),
                _ => false,
            })
        })
}
