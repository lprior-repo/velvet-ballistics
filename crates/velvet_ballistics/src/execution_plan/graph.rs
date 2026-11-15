use serde_json::{Value, json};
use vb_core::{CompiledNodeKind as K, CompiledWorkflow, StepIdx};
mod edges;

pub(super) fn graph(workflow: &CompiledWorkflow) -> Result<Value, String> {
    let nodes: Vec<_> = (0..workflow.node_count()).filter_map(|step| {
        let id = StepIdx::new(step);
        workflow.node(id).map(|node| {
            let kind = serde_json::to_value(&node.kind).map_err(|error| error.to_string())?;
            let label = kind.as_str().or_else(|| kind.as_object().and_then(|map| map.keys().next().map(String::as_str))).ok_or("node kind cannot be represented")?;
            Ok::<_, String>(json!({"step": step, "name": workflow.step_name(id), "kind": label,
                "parameters": kind, "output_slot": node.output.slot().map(vb_core::SlotIdx::get),
                "next_step": node.routing.next.step().map(StepIdx::get), "on_error_step": node.routing.error.handler().map(StepIdx::get), "retry": node.retry}))
        })
    }).collect::<Result<_, _>>()?;
    let edges: Vec<_> = (0..workflow.node_count())
        .filter_map(|step| workflow.node(StepIdx::new(step)))
        .flat_map(edges::targets)
        .collect();
    Ok(json!({"nodes": nodes, "edges": edges}))
}

pub(super) fn actions(workflow: &CompiledWorkflow) -> Result<Value, String> {
    let actions = (0..workflow.node_count())
        .filter_map(|step| {
            let node = workflow.node(StepIdx::new(step))?;
            let K::Do { action, input } = node.kind else {
                return None;
            };
            Some(action_report(step, action.get(), input.get(), node.retry))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!(actions))
}

fn action_report(
    step: u16,
    action: u16,
    input: u16,
    retry: Option<vb_core::workflow::CompiledRetryPolicy>,
) -> Result<Value, String> {
    let contracts = crate::action_specs::cli_action_specs()
        .iter()
        .map(crate::action_specs::action_contract)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let contract = contracts
        .iter()
        .find(|contract| contract.id.get() == action)
        .ok_or("action contract unavailable")?;
    Ok(
        json!({"step": step, "action": action, "input_slot": input, "retry": retry,
        "contract": {"name": contract.name.as_str(), "idempotency": crate::action_specs::action_idempotency_name(contract.idempotency),
            "retry_safety": crate::action_specs::action_retry_safety_name(contract.retry_safety),
            "side_effect": crate::action_specs::action_side_effect_name(contract.side_effect),
            "timeout_ms": contract.timeout_ms, "input_slot_count": contract.input_slot_count,
            "output_slot_count": contract.output_slot_count, "max_input_bytes": contract.max_input_bytes, "max_output_bytes": contract.max_output_bytes}}),
    )
}

pub(super) fn suspensions(workflow: &CompiledWorkflow) -> Value {
    let points: Vec<_> = (0..workflow.node_count())
        .filter_map(|step| workflow.node(StepIdx::new(step)))
        .flat_map(node_suspensions)
        .collect();
    json!(points)
}

fn suspension_kind(kind: &K) -> Option<&'static str> {
    Some(match kind {
        K::Do { .. } => "action",
        K::WaitUntil { .. } => "timer",
        K::WaitEvent { .. } => "event",
        K::Ask { .. } => "ask",
        K::ForEachConcurrentStart { .. }
        | K::TogetherConcurrentStart { .. }
        | K::FirstSuccessStart { .. }
        | K::ConcurrentGroupJoin { .. } => "concurrent_coordination",
        _ => return None,
    })
}

fn node_suspensions(node: &vb_core::CompiledNode) -> impl Iterator<Item = Value> + '_ {
    let retry_timer = matches!(node.kind, K::Do { .. })
        && node
            .retry
            .is_some_and(|policy| policy.max_attempts > 1 && policy.delay_ms > 0);
    suspension_kind(&node.kind)
        .into_iter()
        .chain(retry_timer.then_some("retry_timer"))
        .map(|kind| json!({"step": node.id.get(), "kind": kind}))
}

#[cfg(test)]
mod tests {
    #[test]
    fn retry_check_is_synchronous_control_flow() {
        let node = vb_core::workflow::node(
            vb_core::StepIdx::new(0),
            vb_core::CompiledNodeKind::RetryCheck {
                policy_slot: vb_core::SlotIdx::new(0),
                body: vb_core::StepIdx::new(1),
                exhausted: vb_core::StepIdx::new(2),
            },
            None,
            None,
        );
        assert_eq!(
            super::node_suspensions(&node).collect::<Vec<_>>(),
            Vec::<serde_json::Value>::new()
        );
    }
}
