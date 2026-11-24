#![forbid(unsafe_code)]
//! Semantic projection of a compiled workflow, keyed by stable step labels so
//! that diffs survive step renumbering.

use std::collections::{BTreeMap, BTreeSet};

use vb_core::action::ActionContract;
use vb_core::workflow::ErrorRouting;
use vb_core::{CompiledNode, CompiledNodeKind, CompiledWorkflow, StepIdx};

use crate::action_specs::{
    action_idempotency_name, action_retry_safety_name, action_side_effect_name,
};
use crate::commands_workflow::{collect_kind_edges, node_kind_label};

/// Comparable facts extracted from one compiled workflow.
pub(super) struct WorkflowModel {
    pub(super) name: String,
    pub(super) digest: String,
    /// Hash of the compiled content without digest, source identity or source symbols.
    pub(super) content: String,
    pub(super) step_count: u16,
    pub(super) steps: BTreeSet<String>,
    pub(super) edges: BTreeSet<String>,
    pub(super) contract: BTreeMap<String, String>,
    pub(super) secrets: BTreeMap<String, String>,
    pub(super) actions: BTreeMap<String, String>,
    pub(super) retries: BTreeMap<String, String>,
}

/// Lower-case hex rendering of a workflow digest.
pub(super) fn digest_hex(digest: vb_core::WorkflowDigest) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Builds the comparable model; `contracts` are the action contracts that
/// govern this workflow (persisted ones for a stored baseline).
pub(super) fn build(
    workflow: &CompiledWorkflow,
    contracts: &[ActionContract],
) -> Result<WorkflowModel, String> {
    let keys = step_keys(workflow);
    Ok(WorkflowModel {
        name: workflow.name().to_owned(),
        digest: digest_hex(workflow.digest()),
        content: content_fingerprint(workflow)?,
        step_count: workflow.node_count(),
        steps: keys.iter().cloned().collect(),
        edges: edges(workflow, &keys),
        contract: contract_fields(workflow)?,
        secrets: secrets(workflow),
        actions: actions(workflow, &keys, &action_summaries(contracts)),
        retries: retries(workflow, &keys),
    })
}

/// Hash of everything that executes, excluding the digest, the source-semantic
/// identity and source symbols, which change with source spelling alone.
fn content_fingerprint(workflow: &CompiledWorkflow) -> Result<String, String> {
    let mut parts = workflow.to_parts();
    parts.digest = vb_core::WorkflowDigest::from_bytes([0; 32]);
    parts.semantic_source_identity = None;
    parts.source_metadata = None;
    let bytes = postcard::to_allocvec(&parts)
        .map_err(|error| format!("compiled content not serializable: {error}"))?;
    Ok(digest_hex(vb_core::WorkflowDigest::from_bytes(
        *blake3::hash(&bytes).as_bytes(),
    )))
}

/// One label per node: `<step id>:<node kind>`, numbered when still ambiguous.
fn step_keys(workflow: &CompiledWorkflow) -> Vec<String> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    (0..workflow.node_count())
        .map(|index| {
            let step = StepIdx::new(index);
            let base = workflow
                .step_name(step)
                .map_or_else(|| format!("#{index}"), str::to_owned);
            let kind = workflow
                .node(step)
                .map_or("missing", |node| node_kind_label(&node.kind));
            let label = format!("{base}:{kind}");
            let count = seen.entry(label.clone()).or_insert(0);
            *count = count.saturating_add(1);
            if *count == 1 {
                label
            } else {
                format!("{label}#{count}")
            }
        })
        .collect()
}

fn key_of(keys: &[String], index: u16) -> &str {
    keys.get(usize::from(index)).map_or("?", String::as_str)
}

fn edges(workflow: &CompiledWorkflow, keys: &[String]) -> BTreeSet<String> {
    (0..workflow.node_count())
        .filter_map(|index| workflow.node(StepIdx::new(index)).map(|node| (index, node)))
        .flat_map(|(index, node)| node_edges(index, node, keys))
        .collect()
}

fn node_edges(index: u16, node: &CompiledNode, keys: &[String]) -> Vec<String> {
    let mut found = collect_kind_edges(index, &node.kind);
    if let Some(next) = node.routing.next.step() {
        found.push((index, next.get(), "next".to_owned()));
    }
    if let ErrorRouting::Handler { handler, .. } = node.routing.error {
        found.push((index, handler.get(), "on_error".to_owned()));
    }
    found
        .into_iter()
        .map(|(from, to, label)| {
            let label = if label.is_empty() { "goto" } else { &label };
            format!("{} -> {} [{label}]", key_of(keys, from), key_of(keys, to))
        })
        .collect()
}

fn contract_fields(workflow: &CompiledWorkflow) -> Result<BTreeMap<String, String>, String> {
    let value = serde_json::to_value(workflow.resource_contract())
        .map_err(|error| format!("resource contract not serializable: {error}"))?;
    Ok(value
        .as_object()
        .map(|fields| {
            fields
                .iter()
                .map(|(name, field)| (name.clone(), field.to_string()))
                .collect()
        })
        .unwrap_or_default())
}

fn secrets(workflow: &CompiledWorkflow) -> BTreeMap<String, String> {
    let Some(metadata) = workflow.source_metadata() else {
        return BTreeMap::new();
    };
    let name = |id| metadata.symbol(id).unwrap_or("<unresolved>").to_owned();
    metadata
        .secret_requirements()
        .iter()
        .map(|binding| (name(binding.alias()), name(binding.requirement())))
        .collect()
}

fn action_summaries(contracts: &[ActionContract]) -> BTreeMap<u16, String> {
    contracts
        .iter()
        .map(|contract| {
            let id = contract.id.get();
            let summary = format!(
                "{} (id {id}): idempotency={}, retry_safety={}, side_effect={}, timeout_ms={}, input_slots={}, output_slots={}, max_input_bytes={}, max_output_bytes={}",
                contract.name.as_str(),
                action_idempotency_name(contract.idempotency),
                action_retry_safety_name(contract.retry_safety),
                action_side_effect_name(contract.side_effect),
                contract.timeout_ms,
                contract.input_slot_count,
                contract.output_slot_count,
                contract.max_input_bytes,
                contract.max_output_bytes,
            );
            (id, summary)
        })
        .collect()
}

fn actions(
    workflow: &CompiledWorkflow,
    keys: &[String],
    summaries: &BTreeMap<u16, String>,
) -> BTreeMap<String, String> {
    (0..workflow.node_count())
        .filter_map(|index| match workflow.node(StepIdx::new(index))?.kind {
            CompiledNodeKind::Do { action, .. } => {
                let id = action.get();
                let summary = summaries
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| format!("action id {id} (no registered contract)"));
                Some((key_of(keys, index).to_owned(), summary))
            }
            _ => None,
        })
        .collect()
}

fn retries(workflow: &CompiledWorkflow, keys: &[String]) -> BTreeMap<String, String> {
    (0..workflow.node_count())
        .filter_map(|index| {
            let policy = workflow.node(StepIdx::new(index))?.retry?;
            let text = format!(
                "max_attempts={} delay_ms={}",
                policy.max_attempts, policy.delay_ms
            );
            Some((key_of(keys, index).to_owned(), text))
        })
        .collect()
}
