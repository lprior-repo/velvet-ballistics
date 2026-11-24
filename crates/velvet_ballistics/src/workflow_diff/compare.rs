#![forbid(unsafe_code)]
//! Dimension-by-dimension comparison of two workflow models.

use std::collections::{BTreeMap, BTreeSet};

use super::model::WorkflowModel;

/// One detected semantic difference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Change {
    pub(super) category: &'static str,
    pub(super) message: String,
}

fn change(category: &'static str, message: String) -> Change {
    Change { category, message }
}

/// Every semantic difference between `old` and `new`, grouped by dimension.
pub(super) fn compare(old: &WorkflowModel, new: &WorkflowModel) -> Vec<Change> {
    let mut changes = Vec::new();
    if old.name != new.name {
        changes.push(change(
            "workflow_name",
            format!("workflow name changed: {} -> {}", old.name, new.name),
        ));
    }
    if old.step_count != new.step_count {
        changes.push(change(
            "step_count",
            format!(
                "step count changed: {} -> {}",
                old.step_count, new.step_count
            ),
        ));
    }
    changes.extend(set_diff("steps", "step", &old.steps, &new.steps));
    changes.extend(set_diff("control_flow", "edge", &old.edges, &new.edges));
    changes.extend(map_diff(
        "resource_contract",
        "limit",
        &old.contract,
        &new.contract,
    ));
    changes.extend(map_diff(
        "secret_usage",
        "secret",
        &old.secrets,
        &new.secrets,
    ));
    changes.extend(map_diff(
        "action_contract",
        "step",
        &old.actions,
        &new.actions,
    ));
    changes.extend(map_diff("retry_policy", "step", &old.retries, &new.retries));
    changes
}

fn set_diff(
    category: &'static str,
    subject: &str,
    old: &BTreeSet<String>,
    new: &BTreeSet<String>,
) -> Vec<Change> {
    let removed = old
        .difference(new)
        .map(|item| change(category, format!("{subject} removed: {item}")));
    let added = new
        .difference(old)
        .map(|item| change(category, format!("{subject} added: {item}")));
    removed.chain(added).collect()
}

fn map_diff(
    category: &'static str,
    subject: &str,
    old: &BTreeMap<String, String>,
    new: &BTreeMap<String, String>,
) -> Vec<Change> {
    let keys: BTreeSet<&String> = old.keys().chain(new.keys()).collect();
    keys.into_iter()
        .filter_map(|key| {
            let message = match (old.get(key), new.get(key)) {
                (Some(before), Some(after)) if before != after => {
                    format!("{subject} {key} changed: {before} -> {after}")
                }
                (Some(before), None) => format!("{subject} {key} removed (was {before})"),
                (None, Some(after)) => format!("{subject} {key} added ({after})"),
                _ => return None,
            };
            Some(change(category, message))
        })
        .collect()
}
