//! Action-contract inference for AI context output.
//!
//! Builds a list of action IDs from both compiled-IR Do nodes and journal action events,
//! then emits a stub contract JSON for each unique action.

#![forbid(unsafe_code)]

use serde_json::Value;

fn push_unique_u32(mut values: Vec<u32>, value: u32) -> Vec<u32> {
    if !values.contains(&value) {
        values.push(value);
    }
    values
}

pub(super) fn ai_action_contracts(
    events: &[vb_storage::JournalEvent],
    workflow_actions: Option<&Value>,
) -> Value {
    let workflow_ids = workflow_actions
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_u64().and_then(|raw| u32::try_from(raw).ok()));
    let event_ids = events.iter().filter_map(|event| match event {
        vb_storage::JournalEvent::ActionScheduled { action, .. }
        | vb_storage::JournalEvent::ActionCompletedEvent { action, .. }
        | vb_storage::JournalEvent::ActionFailedEvent { action, .. } => {
            Some(u32::from(action.get()))
        }
        _ => None,
    });
    Value::Array(
        workflow_ids
            .chain(event_ids)
            .fold(Vec::<u32>::new(), push_unique_u32)
            .into_iter()
            .map(inferred_action_contract_json)
            .collect(),
    )
}

fn inferred_action_contract_json(action: u32) -> Value {
    serde_json::json!({
        "action": action,
        "contract_status": "inferred_from_compiled_ir_and_journal",
        "contract": {
            "id": action,
            "source": "compiled_ir_do_node_or_action_event",
            "input_slot_count": null,
            "output_slot_count": null,
            "max_input_bytes": null,
            "max_output_bytes": null,
            "timeout_ms": null,
            "idempotency": "unknown_not_embedded",
            "side_effect": "unknown_not_embedded",
            "retry_safety": "unknown_not_embedded",
            "required_capabilities": []
        }
    })
}
