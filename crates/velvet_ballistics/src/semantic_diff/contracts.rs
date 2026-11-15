use std::collections::BTreeSet;
use vb_core::{ActionContract, CompiledWorkflow};

pub(super) struct Comparison {
    pub(super) report: serde_json::Value,
    pub(super) changed: Option<bool>,
    pub(super) complete: bool,
}

pub(super) fn compare(
    before: &CompiledWorkflow,
    after: &CompiledWorkflow,
    persisted: Option<&[ActionContract]>,
) -> Result<Comparison, String> {
    let before_ids = action_ids(before);
    let after_ids = action_ids(after);
    let registry = current_contracts()?;
    let comparisons: Vec<_> = before_ids
        .union(&after_ids)
        .map(|id| compare_action(*id, &before_ids, &after_ids, persisted, &registry))
        .collect();
    let complete = comparisons.iter().all(|(_, changed)| changed.is_some());
    let changed = known_change(&comparisons, complete);
    let entries: Vec<_> = comparisons.into_iter().map(|(entry, _)| entry).collect();
    let status = evidence_status(&entries, complete);
    Ok(Comparison {
        report: serde_json::json!({"status": status, "changed": changed, "comparisons": entries}),
        changed,
        complete,
    })
}

fn current_contracts() -> Result<Vec<ActionContract>, String> {
    crate::action_specs::cli_action_specs()
        .iter()
        .map(crate::action_specs::action_contract)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

fn compare_action(
    id: u16,
    before_ids: &BTreeSet<u16>,
    after_ids: &BTreeSet<u16>,
    persisted: Option<&[ActionContract]>,
    registry: &[ActionContract],
) -> (serde_json::Value, Option<bool>) {
    let before = declaration(persisted, before_ids, id);
    let after = declaration(Some(registry), after_ids, id);
    match (before, after) {
        (Some(before), Some(after)) => (
            serde_json::json!({"action": id, "status": "available", "changed": before != after, "before": before, "after": after}),
            Some(before != after),
        ),
        _ => (
            serde_json::json!({"action": id, "status": "unavailable", "changed": null, "reason": "historical or current action declaration unavailable"}),
            None,
        ),
    }
}

fn known_change(comparisons: &[(serde_json::Value, Option<bool>)], complete: bool) -> Option<bool> {
    if comparisons
        .iter()
        .any(|(_, changed)| *changed == Some(true))
    {
        Some(true)
    } else if complete {
        Some(false)
    } else {
        None
    }
}

fn evidence_status(entries: &[serde_json::Value], complete: bool) -> &'static str {
    if complete {
        "available"
    } else if entries
        .iter()
        .any(|entry| entry.get("status").and_then(serde_json::Value::as_str) == Some("available"))
    {
        "partial"
    } else {
        "unavailable"
    }
}

fn action_ids(workflow: &CompiledWorkflow) -> BTreeSet<u16> {
    (0..workflow.node_count())
        .filter_map(
            |step| match workflow.node(vb_core::StepIdx::new(step))?.kind {
                vb_core::CompiledNodeKind::Do { action, .. } => Some(action.get()),
                _ => None,
            },
        )
        .collect()
}

fn declaration<'a>(
    contracts: Option<&'a [ActionContract]>,
    ids: &BTreeSet<u16>,
    id: u16,
) -> Option<Option<&'a ActionContract>> {
    if !ids.contains(&id) {
        return Some(None);
    }
    contracts?
        .iter()
        .find(|contract| contract.id.get() == id)
        .map(Some)
}
