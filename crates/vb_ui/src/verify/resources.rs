#![forbid(unsafe_code)]
//! Resource bounds panel -- displays contract limits and computed worst-case resource usage.

use vb_core::workflow::{CompiledNodeKind, ResourceContract, WorkflowParts};

/// Whether a resource metric is within its contracted bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResourceStatus {
    /// Computed value is strictly below the contract limit.
    WithinBounds,
    /// Computed value equals the contract limit exactly.
    AtLimit,
    /// Computed value exceeds the contract limit.
    ExceedsLimit,
}

/// One resource metric comparing contract limit to computed usage.
#[derive(Debug, Clone)]
pub struct ResourceMetric {
    /// Human-readable label for this metric.
    pub label: &'static str,
    /// Contract-declared limit.
    pub contract_value: u64,
    /// Computed worst-case from the workflow.
    pub computed_value: u64,
    /// Status relative to the contract limit.
    pub status: ResourceStatus,
}

/// Computed worst-case resource bounds derived by walking the workflow nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceBounds {
    /// Number of runtime slots declared by the workflow.
    pub slot_count: u16,
    /// Total number of compiled nodes in the workflow.
    pub node_count: u32,
    /// Number of Do (action dispatch) nodes.
    pub do_node_count: u32,
    /// Maximum action payload size from the resource contract.
    pub max_action_payload: u32,
    /// Maximum result size from the resource contract.
    pub max_result_size: u32,
    /// Retry budget: do_node_count * max_retry_attempts from contract.
    pub retry_budget: u32,
    /// Estimated peak frame usage from loop/parallel constructs.
    pub estimated_peak_frames: u32,
}

/// Walk the `WorkflowParts` nodes and compute worst-case resource bounds.
///
/// Counts Do nodes for action dispatch pressure, sums loop/parallel nesting
/// depth for peak frame estimation, and derives retry budget from the
/// contract's `max_retry_attempts`.
#[must_use]
pub fn compute_resource_bounds(parts: &WorkflowParts) -> ResourceBounds {
    let contract = &parts.resource_contract;
    let node_count = u32::try_from(parts.nodes.len()).unwrap_or(u32::MAX);

    let mut do_node_count: u32 = 0;
    let mut peak_frames: u32 = 1;

    // Walk all nodes to count Do nodes and measure loop/parallel nesting depth.
    for node in &parts.nodes {
        match &node.kind {
            CompiledNodeKind::Do { .. } => {
                do_node_count = do_node_count.saturating_add(1);
            }
            CompiledNodeKind::ForEachStart { .. }
            | CompiledNodeKind::ForEachNext { .. }
            | CompiledNodeKind::ForEachJoin { .. }
            | CompiledNodeKind::TogetherStart { .. }
            | CompiledNodeKind::TogetherBranch { .. }
            | CompiledNodeKind::TogetherJoin { .. }
            | CompiledNodeKind::RepeatStart { .. }
            | CompiledNodeKind::RepeatAttempt { .. }
            | CompiledNodeKind::RepeatCheck { .. }
            | CompiledNodeKind::RepeatFinish { .. }
            | CompiledNodeKind::CollectStart { .. }
            | CompiledNodeKind::CollectPage { .. }
            | CompiledNodeKind::CollectNext { .. }
            | CompiledNodeKind::CollectFinish { .. }
            | CompiledNodeKind::ReduceStart { .. }
            | CompiledNodeKind::ReduceNext { .. }
            | CompiledNodeKind::ReduceFinish { .. } => {
                peak_frames = peak_frames.saturating_add(1);
            }
            _ => {}
        }
    }

    // TogetherStart can spawn multiple branches; estimate peak from fanout.
    let fanout_contribution = count_together_branches(&parts.nodes);
    if fanout_contribution > 0 {
        peak_frames = peak_frames.saturating_add(fanout_contribution);
    }

    // ForEachStart with limit contributes iterations as additional frames.
    let foreach_frames = count_foreach_iterations(&parts.nodes);
    if foreach_frames > 0 {
        peak_frames = peak_frames.saturating_add(foreach_frames);
    }

    let retry_budget = do_node_count.saturating_mul(u32::from(contract.max_retry_attempts));

    ResourceBounds {
        slot_count: parts.slot_count,
        node_count,
        do_node_count,
        max_action_payload: contract.max_ipc_payload_bytes,
        max_result_size: contract.max_output_bytes,
        retry_budget,
        estimated_peak_frames: peak_frames,
    }
}

/// Count the total number of branches declared in TogetherStart nodes.
fn count_together_branches(nodes: &[vb_core::workflow::CompiledNode]) -> u32 {
    let mut total: u32 = 0;
    for node in nodes {
        if let CompiledNodeKind::TogetherStart { branches, .. } = &node.kind {
            total = total.saturating_add(u32::try_from(branches.len()).unwrap_or(u32::MAX));
        }
    }
    total
}

/// Sum the iteration limits from ForEachStart nodes as a frame pressure estimate.
fn count_foreach_iterations(nodes: &[vb_core::workflow::CompiledNode]) -> u32 {
    let mut total: u32 = 0;
    for node in nodes {
        if let CompiledNodeKind::ForEachStart { limit, .. } = &node.kind {
            total = total.saturating_add(*limit);
        }
    }
    total
}

/// Panel of resource metrics for UI display.
pub struct ResourceBoundsPanel {
    metrics: Vec<ResourceMetric>,
}

impl ResourceBoundsPanel {
    /// Build a resource bounds panel from a contract and computed resource bounds.
    #[must_use]
    pub fn new(contract: &ResourceContract, bounds: &ResourceBounds) -> Self {
        let mut metrics = Vec::new();

        let node_count_u64 = u64::from(bounds.node_count);
        let slot_count_u64 = u64::from(bounds.slot_count);
        let do_count_u64 = u64::from(bounds.do_node_count);
        let retry_budget_u64 = u64::from(bounds.retry_budget);
        let peak_u64 = u64::from(bounds.estimated_peak_frames);

        // Node count vs max_steps
        let max_steps_u64 = u64::from(contract.max_steps);
        metrics.push(ResourceMetric {
            label: "node_count / max_steps",
            contract_value: max_steps_u64,
            computed_value: node_count_u64,
            status: classify(node_count_u64, max_steps_u64),
        });

        // Slot count vs max_slots
        let max_slots_u64 = u64::from(contract.max_slots);
        metrics.push(ResourceMetric {
            label: "slot_count / max_slots",
            contract_value: max_slots_u64,
            computed_value: slot_count_u64,
            status: classify(slot_count_u64, max_slots_u64),
        });

        // Estimated worst-case action payload: do_node_count * max_ipc_payload_bytes
        let payload_limit = u64::from(bounds.max_action_payload);
        let estimated_payload = do_count_u64.saturating_mul(payload_limit);
        metrics.push(ResourceMetric {
            label: "estimated_action_payload / max_ipc_payload_bytes",
            contract_value: payload_limit,
            computed_value: estimated_payload,
            status: classify(estimated_payload, payload_limit),
        });

        // Estimated worst-case result size: do_node_count * max_output_bytes
        let result_limit = u64::from(bounds.max_result_size);
        let estimated_result = do_count_u64.saturating_mul(result_limit);
        metrics.push(ResourceMetric {
            label: "estimated_result_size / max_output_bytes",
            contract_value: result_limit,
            computed_value: estimated_result,
            status: classify(estimated_result, result_limit),
        });

        // Retry budget: do_node_count * max_retry_attempts
        metrics.push(ResourceMetric {
            label: "retry_budget",
            contract_value: contract.max_step_budget_per_tick,
            computed_value: retry_budget_u64,
            status: classify(retry_budget_u64, contract.max_step_budget_per_tick),
        });

        // Fanout: do_node_count vs max_fanout
        let fanout_u64 = u64::from(contract.max_fanout);
        metrics.push(ResourceMetric {
            label: "do_node_count / max_fanout",
            contract_value: fanout_u64,
            computed_value: do_count_u64,
            status: classify(do_count_u64, fanout_u64),
        });

        // Estimated peak frames vs queue depth
        let queue_u64 = u64::from(contract.max_queue_depth);
        metrics.push(ResourceMetric {
            label: "estimated_peak_frames / max_queue_depth",
            contract_value: queue_u64,
            computed_value: peak_u64,
            status: classify(peak_u64, queue_u64),
        });

        // Collect items vs max_collect_items -- node_count as rough proxy.
        let collect_u64 = u64::from(contract.max_collect_items);
        metrics.push(ResourceMetric {
            label: "node_count / max_collect_items",
            contract_value: collect_u64,
            computed_value: node_count_u64,
            status: classify(node_count_u64, collect_u64),
        });

        Self { metrics }
    }

    /// Returns all metrics in order.
    #[must_use]
    pub fn metrics(&self) -> &[ResourceMetric] {
        &self.metrics
    }

    /// Returns only metrics that are at limit or exceeding.
    #[must_use]
    pub fn worst_case_metrics(&self) -> Vec<&ResourceMetric> {
        self.metrics
            .iter()
            .filter(|m| m.status != ResourceStatus::WithinBounds)
            .collect()
    }

    /// True when all metrics are strictly within bounds.
    #[must_use]
    pub fn all_within_bounds(&self) -> bool {
        self.metrics
            .iter()
            .all(|m| m.status == ResourceStatus::WithinBounds)
    }
}

/// Classify a computed value against its contract limit.
fn classify(computed: u64, limit: u64) -> ResourceStatus {
    if computed > limit {
        ResourceStatus::ExceedsLimit
    } else if computed == limit {
        ResourceStatus::AtLimit
    } else {
        ResourceStatus::WithinBounds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{ActionId, SlotIdx, StepIdx, WorkflowDigest};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

    fn make_parts(kinds: Vec<CompiledNodeKind>) -> WorkflowParts {
        let nodes: Vec<CompiledNode> = kinds
            .into_iter()
            .enumerate()
            .map(|(i, kind)| CompiledNode {
                id: StepIdx::new(u16::try_from(i).unwrap_or(u16::MAX)),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind,
            })
            .collect();
        let count = nodes.len();
        WorkflowParts {
            name: String::from("resources-test").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: (0..count)
                .map(|_| String::from("").into_boxed_str())
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }

    fn make_parts_with_contract(
        kinds: Vec<CompiledNodeKind>,
        contract: ResourceContract,
    ) -> WorkflowParts {
        let mut parts = make_parts(kinds);
        parts.resource_contract = contract;
        parts
    }

    // --- compute_resource_bounds tests ---

    #[test]
    fn test_bounds_empty_workflow() {
        let parts = make_parts(vec![CompiledNodeKind::Nop]);
        let bounds = compute_resource_bounds(&parts);
        assert_eq!(bounds.node_count, 1);
        assert_eq!(bounds.do_node_count, 0);
        assert_eq!(bounds.slot_count, 4);
        assert_eq!(bounds.retry_budget, 0);
        assert_eq!(bounds.estimated_peak_frames, 1);
    }

    #[test]
    fn test_bounds_counts_do_nodes() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Nop,
            CompiledNodeKind::Do {
                action: ActionId::new(2),
                input: SlotIdx::new(1),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let bounds = compute_resource_bounds(&parts);
        assert_eq!(bounds.do_node_count, 2);
        assert_eq!(bounds.node_count, 4);
    }

    #[test]
    fn test_bounds_retry_budget() {
        let parts = make_parts(vec![
            CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let bounds = compute_resource_bounds(&parts);
        // DEFAULT max_retry_attempts = 3
        assert_eq!(bounds.retry_budget, 1 * 3);
    }

    #[test]
    fn test_bounds_foreach_peak_frames() {
        let parts = make_parts(vec![
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(2),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(3),
            },
        ]);
        let bounds = compute_resource_bounds(&parts);
        // Base 1 + 3 loop nodes = 4, + 10 foreach iterations = 14
        assert_eq!(bounds.estimated_peak_frames, 14);
    }

    #[test]
    fn test_bounds_together_peak_frames() {
        let parts = make_parts(vec![
            CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(1), StepIdx::new(2), StepIdx::new(3)]),
                join: StepIdx::new(4),
            },
            CompiledNodeKind::TogetherBranch {
                branch: 0,
                entry: StepIdx::new(1),
                join: StepIdx::new(4),
                accumulator: SlotIdx::new(5),
            },
            CompiledNodeKind::TogetherBranch {
                branch: 1,
                entry: StepIdx::new(2),
                join: StepIdx::new(4),
                accumulator: SlotIdx::new(5),
            },
            CompiledNodeKind::TogetherBranch {
                branch: 2,
                entry: StepIdx::new(3),
                join: StepIdx::new(4),
                accumulator: SlotIdx::new(5),
            },
            CompiledNodeKind::TogetherJoin {
                branch_count: 3,
                accumulator: SlotIdx::new(5),
            },
        ]);
        let bounds = compute_resource_bounds(&parts);
        // Base 1 + 5 together nodes = 6, + 3 branch fanout = 9
        assert_eq!(bounds.estimated_peak_frames, 9);
    }

    #[test]
    fn test_bounds_uses_contract_values() {
        let contract = ResourceContract {
            max_ipc_payload_bytes: 500,
            max_output_bytes: 200,
            max_retry_attempts: 5,
            ..ResourceContract::DEFAULT
        };
        let parts = make_parts_with_contract(
            vec![CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            }],
            contract,
        );
        let bounds = compute_resource_bounds(&parts);
        assert_eq!(bounds.max_action_payload, 500);
        assert_eq!(bounds.max_result_size, 200);
        assert_eq!(bounds.retry_budget, 1 * 5);
    }

    #[test]
    fn test_bounds_repeat_and_collect_nodes_add_frames() {
        let parts = make_parts(vec![
            CompiledNodeKind::RepeatStart {
                max_attempts: 3,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
            CompiledNodeKind::RepeatAttempt {
                attempt_slot: SlotIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
            CompiledNodeKind::RepeatCheck {
                attempt_slot: SlotIdx::new(0),
                done: StepIdx::new(4),
            },
            CompiledNodeKind::RepeatFinish {
                result: SlotIdx::new(0),
            },
            CompiledNodeKind::CollectStart {
                source: SlotIdx::new(1),
                limit: 5,
                page_size: 10,
                body: StepIdx::new(5),
                done: StepIdx::new(8),
            },
            CompiledNodeKind::CollectPage {
                collector_slot: SlotIdx::new(2),
                body: StepIdx::new(5),
                done: StepIdx::new(8),
            },
            CompiledNodeKind::CollectNext {
                collector_slot: SlotIdx::new(2),
                body: StepIdx::new(5),
                done: StepIdx::new(8),
            },
            CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(2),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let bounds = compute_resource_bounds(&parts);
        // Base 1 + 4 repeat nodes + 4 collect nodes = 9
        assert_eq!(bounds.estimated_peak_frames, 9);
        assert_eq!(bounds.node_count, 9);
        assert_eq!(bounds.do_node_count, 0);
    }

    #[test]
    fn test_bounds_no_do_nodes_zero_retry() {
        let parts = make_parts(vec![
            CompiledNodeKind::SetConst {
                value: vb_core::ids::ConstIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let bounds = compute_resource_bounds(&parts);
        assert_eq!(bounds.do_node_count, 0);
        assert_eq!(bounds.retry_budget, 0);
    }

    // --- ResourceBoundsPanel tests ---

    #[test]
    fn test_panel_all_within_bounds() {
        let contract = ResourceContract::DEFAULT;
        let bounds = ResourceBounds {
            slot_count: 4,
            node_count: 10,
            do_node_count: 0,
            max_action_payload: contract.max_ipc_payload_bytes,
            max_result_size: contract.max_output_bytes,
            retry_budget: 0,
            estimated_peak_frames: 3,
        };
        let panel = ResourceBoundsPanel::new(&contract, &bounds);
        assert!(panel.all_within_bounds());
        assert!(panel.worst_case_metrics().is_empty());
    }

    #[test]
    fn test_panel_node_count_at_limit() {
        let contract = ResourceContract {
            max_steps: 10,
            ..ResourceContract::DEFAULT
        };
        let bounds = ResourceBounds {
            slot_count: 4,
            node_count: 10,
            do_node_count: 0,
            max_action_payload: contract.max_ipc_payload_bytes,
            max_result_size: contract.max_output_bytes,
            retry_budget: 0,
            estimated_peak_frames: 1,
        };
        let panel = ResourceBoundsPanel::new(&contract, &bounds);
        assert!(!panel.all_within_bounds());
        let node_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "node_count / max_steps");
        assert!(node_metric.is_some());
        let Some(m) = node_metric else {
            assert!(false, "metric missing");
            return;
        };
        assert_eq!(m.status, ResourceStatus::AtLimit);
    }

    #[test]
    fn test_panel_exceeds_limit() {
        let contract = ResourceContract {
            max_steps: 5,
            max_slots: 2,
            ..ResourceContract::DEFAULT
        };
        let bounds = ResourceBounds {
            slot_count: 5,
            node_count: 10,
            do_node_count: 3,
            max_action_payload: contract.max_ipc_payload_bytes,
            max_result_size: contract.max_output_bytes,
            retry_budget: 9,
            estimated_peak_frames: 2,
        };
        let panel = ResourceBoundsPanel::new(&contract, &bounds);
        let node_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "node_count / max_steps");
        assert!(node_metric.is_some());
        let Some(node_metric) = node_metric else {
            assert!(false, "metric missing");
            return;
        };
        assert_eq!(node_metric.status, ResourceStatus::ExceedsLimit);

        let slot_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "slot_count / max_slots");
        assert!(slot_metric.is_some());
        let Some(slot_metric) = slot_metric else {
            assert!(false, "metric missing");
            return;
        };
        assert_eq!(slot_metric.status, ResourceStatus::ExceedsLimit);
    }

    #[test]
    fn test_panel_metrics_count() {
        let contract = ResourceContract::DEFAULT;
        let bounds = ResourceBounds {
            slot_count: 4,
            node_count: 10,
            do_node_count: 2,
            max_action_payload: contract.max_ipc_payload_bytes,
            max_result_size: contract.max_output_bytes,
            retry_budget: 6,
            estimated_peak_frames: 3,
        };
        let panel = ResourceBoundsPanel::new(&contract, &bounds);
        assert_eq!(panel.metrics().len(), 8);
    }

    // --- classify tests ---

    #[test]
    fn test_classify_within() {
        assert_eq!(classify(5, 10), ResourceStatus::WithinBounds);
    }

    #[test]
    fn test_classify_at_limit() {
        assert_eq!(classify(10, 10), ResourceStatus::AtLimit);
    }

    #[test]
    fn test_classify_exceeds() {
        assert_eq!(classify(11, 10), ResourceStatus::ExceedsLimit);
    }

    #[test]
    fn test_classify_zero_equals_zero_is_at_limit() {
        assert_eq!(classify(0, 0), ResourceStatus::AtLimit);
    }

    // --- Integration: compute + panel together ---

    #[test]
    fn test_compute_then_panel_default_contract() {
        let parts = make_parts(vec![
            CompiledNodeKind::Nop,
            CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let bounds = compute_resource_bounds(&parts);
        let panel = ResourceBoundsPanel::new(&parts.resource_contract, &bounds);
        // With 1 Do node, estimated_payload = 1 * limit (AtLimit), so not all within bounds.
        assert!(!panel.all_within_bounds());
        assert_eq!(bounds.do_node_count, 1);
        assert_eq!(bounds.node_count, 3);
        // The "at limit" metrics should be the payload/result size metrics.
        let worst = panel.worst_case_metrics();
        assert!(!worst.is_empty());
        assert!(worst.iter().all(|m| m.status == ResourceStatus::AtLimit));
    }

    #[test]
    fn test_compute_then_panel_tight_contract() {
        let contract = ResourceContract {
            max_steps: 2,
            ..ResourceContract::DEFAULT
        };
        let parts = make_parts_with_contract(
            vec![
                CompiledNodeKind::Nop,
                CompiledNodeKind::Nop,
                CompiledNodeKind::Nop,
            ],
            contract,
        );
        let bounds = compute_resource_bounds(&parts);
        let panel = ResourceBoundsPanel::new(&contract, &bounds);
        assert!(!panel.all_within_bounds());
        let worst = panel.worst_case_metrics();
        assert!(!worst.is_empty());
    }

    #[test]
    fn test_compute_reduce_nodes_add_frames() {
        let parts = make_parts(vec![
            CompiledNodeKind::ReduceStart {
                input: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                initial: vb_core::ids::ConstIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
            CompiledNodeKind::ReduceNext {
                iterator_slot: SlotIdx::new(2),
                accumulator: SlotIdx::new(1),
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
            CompiledNodeKind::ReduceFinish {
                accumulator: SlotIdx::new(1),
            },
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        ]);
        let bounds = compute_resource_bounds(&parts);
        // Base 1 + 3 reduce nodes = 4
        assert_eq!(bounds.estimated_peak_frames, 4);
    }

    #[test]
    fn test_compute_multiple_foreach_loops() {
        let parts = make_parts(vec![
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 5,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(2),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(3),
            },
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(4),
                item_slot: SlotIdx::new(5),
                limit: 3,
                body: StepIdx::new(4),
                done: StepIdx::new(5),
            },
            CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(6),
                body: StepIdx::new(4),
                done: StepIdx::new(5),
            },
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(7),
            },
        ]);
        let bounds = compute_resource_bounds(&parts);
        // Base 1 + 6 loop nodes = 7, + 5 + 3 = 15 iterations total = 15
        assert_eq!(bounds.estimated_peak_frames, 15);
    }

    // --- Additional edge-case tests ---

    #[test]
    fn test_bounds_no_nodes_zero_everything() {
        let parts = WorkflowParts {
            name: String::from("zero-nodes").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: Vec::new().into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 0,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let bounds = compute_resource_bounds(&parts);
        assert_eq!(bounds.node_count, 0);
        assert_eq!(bounds.do_node_count, 0);
        assert_eq!(bounds.retry_budget, 0);
        assert_eq!(bounds.estimated_peak_frames, 1);
        assert_eq!(bounds.slot_count, 0);
    }

    #[test]
    fn test_bounds_retry_budget_with_multiple_do_nodes() {
        let contract = ResourceContract {
            max_retry_attempts: 10,
            ..ResourceContract::DEFAULT
        };
        let parts = make_parts_with_contract(
            vec![
                CompiledNodeKind::Do {
                    action: ActionId::new(1),
                    input: SlotIdx::new(0),
                },
                CompiledNodeKind::Do {
                    action: ActionId::new(2),
                    input: SlotIdx::new(1),
                },
                CompiledNodeKind::Do {
                    action: ActionId::new(3),
                    input: SlotIdx::new(2),
                },
                CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            ],
            contract,
        );
        let bounds = compute_resource_bounds(&parts);
        assert_eq!(bounds.do_node_count, 3);
        assert_eq!(bounds.retry_budget, 30); // 3 * 10
    }

    #[test]
    fn test_resource_bounds_clone_and_eq() {
        let a = ResourceBounds {
            slot_count: 4,
            node_count: 10,
            do_node_count: 2,
            max_action_payload: 1024,
            max_result_size: 512,
            retry_budget: 6,
            estimated_peak_frames: 3,
        };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn test_panel_worst_case_metrics_mixed() {
        let contract = ResourceContract {
            max_steps: 10,
            max_slots: 4,
            max_ipc_payload_bytes: 1024,
            max_output_bytes: 512,
            max_retry_attempts: 3,
            max_fanout: 2,
            max_queue_depth: 5,
            max_collect_items: 20,
            max_step_budget_per_tick: 100,
            ..ResourceContract::DEFAULT
        };
        // node_count=5 < max_steps=10 -> WithinBounds
        // do_node_count=3 > max_fanout=2 -> ExceedsLimit
        let bounds = ResourceBounds {
            slot_count: 4,
            node_count: 5,
            do_node_count: 3,
            max_action_payload: 1024,
            max_result_size: 512,
            retry_budget: 9,
            estimated_peak_frames: 1,
        };
        let panel = ResourceBoundsPanel::new(&contract, &bounds);
        assert!(!panel.all_within_bounds());
        let worst = panel.worst_case_metrics();
        assert!(!worst.is_empty());
        // At least the fanout metric should be ExceedsLimit.
        let fanout_metric = worst
            .iter()
            .find(|m| m.label == "do_node_count / max_fanout");
        assert!(fanout_metric.is_some());
        let fm = fanout_metric.ok_or("missing").ok();
        if let Some(m) = fm {
            assert_eq!(m.status, ResourceStatus::ExceedsLimit);
        }
    }

    #[test]
    fn test_resource_status_ordering() {
        assert_ne!(ResourceStatus::WithinBounds, ResourceStatus::AtLimit);
        assert_ne!(ResourceStatus::AtLimit, ResourceStatus::ExceedsLimit);
        assert_ne!(ResourceStatus::WithinBounds, ResourceStatus::ExceedsLimit);
    }

    #[test]
    fn test_classify_large_values() {
        assert_eq!(classify(u64::MAX, u64::MAX), ResourceStatus::AtLimit);
        assert_eq!(classify(0, u64::MAX), ResourceStatus::WithinBounds);
        assert_eq!(classify(u64::MAX, 0), ResourceStatus::ExceedsLimit);
    }

    #[test]
    fn test_panel_metrics_labels() {
        let contract = ResourceContract::DEFAULT;
        let bounds = ResourceBounds {
            slot_count: 4,
            node_count: 10,
            do_node_count: 2,
            max_action_payload: contract.max_ipc_payload_bytes,
            max_result_size: contract.max_output_bytes,
            retry_budget: 6,
            estimated_peak_frames: 3,
        };
        let panel = ResourceBoundsPanel::new(&contract, &bounds);
        let labels: Vec<&str> = panel.metrics().iter().map(|m| m.label).collect();
        assert!(labels.contains(&"node_count / max_steps"));
        assert!(labels.contains(&"slot_count / max_slots"));
        assert!(labels.contains(&"retry_budget"));
        assert!(labels.contains(&"estimated_peak_frames / max_queue_depth"));
    }

    #[test]
    fn test_compute_bounds_together_with_zero_branches() {
        let parts = make_parts(vec![
            CompiledNodeKind::TogetherStart {
                branches: Box::new([]),
                join: StepIdx::new(1),
            },
            CompiledNodeKind::TogetherJoin {
                branch_count: 0,
                accumulator: SlotIdx::new(0),
            },
        ]);
        let bounds = compute_resource_bounds(&parts);
        // Base 1 + 2 together nodes = 3, + 0 fanout = 3
        assert_eq!(bounds.estimated_peak_frames, 3);
    }

    // --- Requested tests ---

    /// Test 1: Empty workflow (zero nodes) produces zero counts and passes the
    /// panel when contract limits are nonzero.
    #[test]
    fn test_empty_workflow_resource_bounds() {
        let parts = WorkflowParts {
            name: String::from("empty").into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: Vec::new().into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 0,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Vec::new().into_boxed_slice(),
        };
        let bounds = compute_resource_bounds(&parts);
        assert_eq!(bounds.node_count, 0);
        assert_eq!(bounds.do_node_count, 0);
        assert_eq!(bounds.retry_budget, 0);
        assert_eq!(bounds.estimated_peak_frames, 1);
        assert_eq!(bounds.slot_count, 0);

        // Panel should show all metrics within bounds for a truly empty workflow.
        // With zero do_node_count, payload and result are 0 (well under limits).
        // With zero node_count, node_count/max_steps is 0 < 10_000.
        // However slot_count=0 == 0 is AtLimit only if max_slots were 0, but
        // DEFAULT max_slots=1024 so 0 < 1024 is WithinBounds.
        // One edge: node_count=0 vs max_collect_items -- 0 < 1024 WithinBounds.
        let panel = ResourceBoundsPanel::new(&parts.resource_contract, &bounds);
        assert!(panel.all_within_bounds());
    }

    /// Test 2: Single Do node action payload estimation -- verify the panel
    /// computes estimated_action_payload as 1 * max_ipc_payload_bytes.
    #[test]
    fn test_single_do_node_action_payload_estimation() {
        let contract = ResourceContract {
            max_ipc_payload_bytes: 2048,
            ..ResourceContract::DEFAULT
        };
        let parts = make_parts_with_contract(
            vec![CompiledNodeKind::Do {
                action: ActionId::new(0),
                input: SlotIdx::new(0),
            }],
            contract,
        );
        let bounds = compute_resource_bounds(&parts);
        assert_eq!(bounds.do_node_count, 1);
        assert_eq!(bounds.max_action_payload, 2048);

        let panel = ResourceBoundsPanel::new(&contract, &bounds);
        let payload_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "estimated_action_payload / max_ipc_payload_bytes");
        let Some(metric) = payload_metric else {
            return;
        };
        // estimated_payload = 1 * 2048 = 2048, contract_value = 2048 => AtLimit
        assert_eq!(metric.computed_value, 2048);
        assert_eq!(metric.contract_value, 2048);
        assert_eq!(metric.status, ResourceStatus::AtLimit);
    }

    /// Test 3: Multiple Do nodes produce total payload that exceeds the single
    /// payload limit in the panel (do_node_count * max_ipc_payload_bytes > limit).
    #[test]
    fn test_multiple_do_nodes_total_payload() {
        let contract = ResourceContract {
            max_ipc_payload_bytes: 512,
            ..ResourceContract::DEFAULT
        };
        let bounds = ResourceBounds {
            slot_count: 1,
            node_count: 5,
            do_node_count: 3,
            max_action_payload: 512,
            max_result_size: 256,
            retry_budget: 9,
            estimated_peak_frames: 1,
        };
        let panel = ResourceBoundsPanel::new(&contract, &bounds);

        let payload_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "estimated_action_payload / max_ipc_payload_bytes");
        let Some(metric) = payload_metric else {
            return;
        };
        // estimated_payload = 3 * 512 = 1536, contract limit = 512 => ExceedsLimit
        assert_eq!(metric.computed_value, 1536);
        assert_eq!(metric.contract_value, 512);
        assert_eq!(metric.status, ResourceStatus::ExceedsLimit);
    }

    /// Test 4: TogetherStart fanout calculation -- verify multiple TogetherStart
    /// nodes contribute their branch counts cumulatively.
    #[test]
    fn test_together_start_fanout_calculation() {
        let parts = make_parts(vec![
            // First TogetherStart with 2 branches
            CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(1), StepIdx::new(2)]),
                join: StepIdx::new(3),
            },
            CompiledNodeKind::Nop,
            CompiledNodeKind::Nop,
            CompiledNodeKind::TogetherJoin {
                branch_count: 2,
                accumulator: SlotIdx::new(0),
            },
            // Second TogetherStart with 4 branches
            CompiledNodeKind::TogetherStart {
                branches: Box::new([
                    StepIdx::new(5),
                    StepIdx::new(6),
                    StepIdx::new(7),
                    StepIdx::new(8),
                ]),
                join: StepIdx::new(9),
            },
            CompiledNodeKind::Nop,
            CompiledNodeKind::Nop,
            CompiledNodeKind::Nop,
            CompiledNodeKind::Nop,
            CompiledNodeKind::TogetherJoin {
                branch_count: 4,
                accumulator: SlotIdx::new(1),
            },
        ]);
        let bounds = compute_resource_bounds(&parts);
        // Only TogetherStart and TogetherJoin nodes increment peak_frames (Nop does not).
        // 2 TogetherStart + 2 TogetherJoin = 4 together nodes. Base 1 + 4 = 5.
        // Fanout: first TogetherStart has 2 branches, second has 4 => 5 + 2 + 4 = 11
        assert_eq!(bounds.estimated_peak_frames, 11);
    }

    /// Test 5: ForEachStart iteration limit contributes to peak frames and the
    /// panel reports the result against max_queue_depth.
    #[test]
    fn test_foreach_start_iteration_limit() {
        let contract = ResourceContract {
            max_queue_depth: 20,
            ..ResourceContract::DEFAULT
        };
        let parts = make_parts_with_contract(
            vec![
                CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 25,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
                CompiledNodeKind::ForEachNext {
                    iterator_slot: SlotIdx::new(2),
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
                CompiledNodeKind::ForEachJoin {
                    output: SlotIdx::new(3),
                },
            ],
            contract,
        );
        let bounds = compute_resource_bounds(&parts);
        // Base 1 + 3 loop nodes = 4, + 25 iterations = 29
        assert_eq!(bounds.estimated_peak_frames, 29);

        // Panel: peak=29 > max_queue_depth=20 => ExceedsLimit
        let panel = ResourceBoundsPanel::new(&contract, &bounds);
        let peak_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "estimated_peak_frames / max_queue_depth");
        let Some(metric) = peak_metric else {
            return;
        };
        assert_eq!(metric.computed_value, 29);
        assert_eq!(metric.contract_value, 20);
        assert_eq!(metric.status, ResourceStatus::ExceedsLimit);
    }

    /// Test 6: ResourceBoundsPanel with all metrics strictly within bounds
    /// using a generous contract so every computed value is safely below.
    #[test]
    fn test_panel_all_metrics_passing() {
        let contract = ResourceContract {
            max_steps: 10_000,
            max_slots: 1_000,
            max_ipc_payload_bytes: 100_000,
            max_output_bytes: 50_000,
            max_retry_attempts: 10,
            max_fanout: 100,
            max_queue_depth: 500,
            max_collect_items: 10_000,
            max_step_budget_per_tick: 10_000,
            ..ResourceContract::DEFAULT
        };
        // Use do_node_count=0 so payload and result estimations are 0 (well under limits).
        let bounds = ResourceBounds {
            slot_count: 2,
            node_count: 5,
            do_node_count: 0,
            max_action_payload: 100_000,
            max_result_size: 50_000,
            retry_budget: 0,
            estimated_peak_frames: 4,
        };
        let panel = ResourceBoundsPanel::new(&contract, &bounds);
        assert!(panel.all_within_bounds());
        assert!(panel.worst_case_metrics().is_empty());

        // Spot-check individual metrics
        for metric in panel.metrics() {
            assert_eq!(
                metric.status,
                ResourceStatus::WithinBounds,
                "metric '{}' should be WithinBounds but is {:?}",
                metric.label,
                metric.status
            );
        }
    }

    /// Test 7: ResourceBoundsPanel with exactly one metric at limit while
    /// all others remain within bounds.
    #[test]
    fn test_panel_one_metric_at_limit() {
        // Use slot_count == max_slots for a single AtLimit metric,
        // with do_node_count=0 so payload/result estimations stay at 0 (WithinBounds).
        let contract = ResourceContract {
            max_fanout: 64,
            max_slots: 5,
            max_ipc_payload_bytes: 10_000,
            max_output_bytes: 10_000,
            max_step_budget_per_tick: 100,
            max_collect_items: 10_000,
            max_queue_depth: 100,
            max_steps: 10_000,
            ..ResourceContract::DEFAULT
        };
        let bounds = ResourceBounds {
            slot_count: 5, // == max_slots => AtLimit
            node_count: 10,
            do_node_count: 0, // payload=0, result=0 => WithinBounds
            max_action_payload: 10_000,
            max_result_size: 10_000,
            retry_budget: 0,
            estimated_peak_frames: 2,
        };
        let panel = ResourceBoundsPanel::new(&contract, &bounds);
        assert!(!panel.all_within_bounds());

        let worst = panel.worst_case_metrics();
        let exceeds_count = worst
            .iter()
            .filter(|m| m.status == ResourceStatus::ExceedsLimit)
            .count();
        assert_eq!(exceeds_count, 0, "no metric should exceed the limit");

        let slot_metric = worst.iter().find(|m| m.label == "slot_count / max_slots");
        let Some(sm) = slot_metric else {
            return;
        };
        assert_eq!(sm.status, ResourceStatus::AtLimit);
        assert_eq!(sm.computed_value, 5);
        assert_eq!(sm.contract_value, 5);
    }

    /// Test 8: Zero max_steps causes node_count/max_steps to immediately
    /// exceed the limit (even with a single node).
    #[test]
    fn test_zero_max_steps_immediate_fail() {
        let contract = ResourceContract {
            max_steps: 0,
            ..ResourceContract::DEFAULT
        };
        // Even a minimal single-node workflow exceeds max_steps=0.
        let bounds = ResourceBounds {
            slot_count: 1,
            node_count: 1,
            do_node_count: 0,
            max_action_payload: contract.max_ipc_payload_bytes,
            max_result_size: contract.max_output_bytes,
            retry_budget: 0,
            estimated_peak_frames: 1,
        };
        let panel = ResourceBoundsPanel::new(&contract, &bounds);
        assert!(!panel.all_within_bounds());

        let node_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "node_count / max_steps");
        let Some(nm) = node_metric else {
            return;
        };
        assert_eq!(nm.computed_value, 1);
        assert_eq!(nm.contract_value, 0);
        assert_eq!(nm.status, ResourceStatus::ExceedsLimit);
    }

    // =========================================================================
    // BLACK HAT findings (BH-R01 through BH-R05)
    // =========================================================================

    /// BH-R01 [HIGH]: Payload estimation always reports AtLimit for any
    /// workflow with exactly 1 Do node.
    ///
    /// The metric compares `do_node_count * max_ipc_payload_bytes` against
    /// `max_ipc_payload_bytes`. When do_node_count == 1, computed equals
    /// the contract limit exactly, producing AtLimit rather than WithinBounds.
    /// This means a perfectly healthy single-action workflow always shows up
    /// as "at limit" for payload, which is misleading -- a single payload of
    /// the contracted size should be WithinBounds.
    #[test]
    fn bhr01_single_do_node_payload_always_at_limit() {
        let contract = ResourceContract {
            max_ipc_payload_bytes: 1024,
            ..ResourceContract::DEFAULT
        };
        let bounds = ResourceBounds {
            slot_count: 2,
            node_count: 2,
            do_node_count: 1,
            max_action_payload: 1024,
            max_result_size: 512,
            retry_budget: 3,
            estimated_peak_frames: 1,
        };
        let panel = ResourceBoundsPanel::new(&contract, &bounds);

        let payload_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "estimated_action_payload / max_ipc_payload_bytes");
        let Some(metric) = payload_metric else { return };

        // BLACK HAT [HIGH]: 1 * 1024 = 1024 == 1024 => AtLimit.
        // A single Do node with payload at the contracted limit should arguably
        // be WithinBounds since it's the expected usage pattern.
        assert_eq!(
            metric.computed_value, 1024,
            "computed should be 1 * 1024 = 1024"
        );
        assert_eq!(metric.contract_value, 1024, "contract limit is 1024");
        assert_eq!(
            metric.status,
            ResourceStatus::AtLimit,
            "BLACK HAT [HIGH]: single Do node payload is always AtLimit, never WithinBounds"
        );

        // Verify the same for result size metric.
        let result_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "estimated_result_size / max_output_bytes");
        let Some(result_m) = result_metric else {
            return;
        };
        assert_eq!(
            result_m.status,
            ResourceStatus::AtLimit,
            "BLACK HAT [HIGH]: single Do node result size is also always AtLimit"
        );
    }

    /// BH-R02 [MEDIUM]: retry_budget overflow via saturating_mul.
    ///
    /// With a large number of Do nodes and high max_retry_attempts, the
    /// retry_budget uses saturating_mul which silently clamps to u32::MAX.
    /// This means the reported retry budget may undercount the actual budget
    /// by an unknown amount, and the panel comparison against
    /// max_step_budget_per_tick may be inaccurate.
    #[test]
    fn bhr02_retry_budget_saturation_clamps_silently() {
        let contract = ResourceContract {
            max_retry_attempts: 10,
            max_step_budget_per_tick: u64::MAX,
            ..ResourceContract::DEFAULT
        };
        // Simulate a large do_node_count that would overflow u32.
        let bounds = ResourceBounds {
            slot_count: 4,
            node_count: 100,
            do_node_count: u32::MAX, // maximally large
            max_action_payload: 1024,
            max_result_size: 512,
            retry_budget: u32::MAX.saturating_mul(10), // saturates to u32::MAX
            estimated_peak_frames: 1,
        };

        // The retry_budget saturates at u32::MAX.
        assert_eq!(
            bounds.retry_budget,
            u32::MAX,
            "BLACK HAT [MEDIUM]: retry_budget saturates silently at u32::MAX"
        );

        // The panel comparison still works but the value is not accurate.
        let panel = ResourceBoundsPanel::new(&contract, &bounds);
        let retry_metric = panel.metrics().iter().find(|m| m.label == "retry_budget");
        let Some(metric) = retry_metric else { return };

        // The saturated u32::MAX as u64 vs u64::MAX => WithinBounds,
        // but the real budget would be astronomically larger.
        assert_eq!(
            metric.computed_value,
            u64::from(u32::MAX),
            "saturated value is u32::MAX as u64"
        );
        assert_eq!(metric.contract_value, u64::MAX, "contract allows u64::MAX");
        assert_eq!(
            metric.status,
            ResourceStatus::WithinBounds,
            "saturated budget appears within bounds even though real budget is unknown"
        );
    }

    /// BH-R03 [MEDIUM]: Peak frames overestimation from additive counting.
    ///
    /// The peak_frames calculation adds +1 for each loop/parallel node AND
    /// separately adds the iteration count from ForEachStart. This
    /// double-counts because the base +1 for ForEachStart/ForEachNext/
    /// ForEachJoin already represents the structural presence of those nodes,
    /// and then the iteration limit is added on top as if all iterations run
    /// simultaneously in parallel frames. This produces an inflated estimate
    /// that may cause false resource violations.
    #[test]
    fn bhr03_peak_frames_overestimates_with_nested_loops() {
        // A single ForEachStart with limit=1000 adds:
        // +1 for ForEachStart, +1 for ForEachNext, +1 for ForEachJoin = +3
        // +1000 for the iteration limit
        // = base 1 + 3 + 1000 = 1004
        // But in reality, only one iteration runs at a time, so peak frames
        // should be much lower.
        let parts = make_parts(vec![
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 1000,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(2),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(3),
            },
        ]);
        let bounds = compute_resource_bounds(&parts);

        // BLACK HAT [MEDIUM]: peak_frames = 1 + 3 + 1000 = 1004.
        // This is an overestimate; real peak frames are much lower.
        assert_eq!(
            bounds.estimated_peak_frames, 1004,
            "BLACK HAT [MEDIUM]: peak_frames overestimates by counting iteration limit as parallel frames"
        );

        // With a tight queue depth, this overestimate causes a false violation.
        let contract = ResourceContract {
            max_queue_depth: 500,
            ..ResourceContract::DEFAULT
        };
        let panel = ResourceBoundsPanel::new(&contract, &bounds);
        let peak_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "estimated_peak_frames / max_queue_depth");
        let Some(metric) = peak_metric else { return };

        assert_eq!(
            metric.status,
            ResourceStatus::ExceedsLimit,
            "BLACK HAT [MEDIUM]: overestimated peak_frames causes false resource violation"
        );
    }

    /// BH-R04 [LOW]: classify(0, 0) returns AtLimit but zero-vs-zero could
    /// reasonably be WithinBounds (nothing allocated, nothing allowed).
    /// This is an edge case where the semantics are debatable.
    #[test]
    fn bhr04_zero_vs_zero_classified_as_at_limit() {
        assert_eq!(
            classify(0, 0),
            ResourceStatus::AtLimit,
            "BLACK HAT [LOW]: zero computed vs zero limit is AtLimit, not WithinBounds"
        );
    }

    /// BH-R05 [LOW]: TogetherStart with many branches inflates peak_frames.
    ///
    /// A TogetherStart with N branches adds +1 for TogetherStart, +1 for
    /// TogetherJoin, +1 per TogetherBranch, AND +N for fanout. This is a
    /// 2N+2 count when the true peak is approximately N+2.
    #[test]
    fn bhr05_together_branches_inflate_peak_frames() {
        let parts = make_parts(vec![
            CompiledNodeKind::TogetherStart {
                branches: Box::new([
                    StepIdx::new(1),
                    StepIdx::new(2),
                    StepIdx::new(3),
                    StepIdx::new(4),
                    StepIdx::new(5),
                ]),
                join: StepIdx::new(6),
            },
            CompiledNodeKind::TogetherBranch {
                branch: 0,
                entry: StepIdx::new(1),
                join: StepIdx::new(6),
                accumulator: SlotIdx::new(10),
            },
            CompiledNodeKind::TogetherBranch {
                branch: 1,
                entry: StepIdx::new(2),
                join: StepIdx::new(6),
                accumulator: SlotIdx::new(10),
            },
            CompiledNodeKind::TogetherBranch {
                branch: 2,
                entry: StepIdx::new(3),
                join: StepIdx::new(6),
                accumulator: SlotIdx::new(10),
            },
            CompiledNodeKind::TogetherBranch {
                branch: 3,
                entry: StepIdx::new(4),
                join: StepIdx::new(6),
                accumulator: SlotIdx::new(10),
            },
            CompiledNodeKind::TogetherBranch {
                branch: 4,
                entry: StepIdx::new(5),
                join: StepIdx::new(6),
                accumulator: SlotIdx::new(10),
            },
            CompiledNodeKind::TogetherJoin {
                branch_count: 5,
                accumulator: SlotIdx::new(10),
            },
        ]);
        let bounds = compute_resource_bounds(&parts);

        // Base 1 + 1(TogetherStart) + 5(TogetherBranch) + 1(TogetherJoin) = 8
        // + 5 fanout = 13
        // True peak should be closer to 5+2 = 7 (branches run in parallel,
        // plus start and join).
        assert_eq!(
            bounds.estimated_peak_frames, 13,
            "BLACK HAT [LOW]: peak_frames counts branch nodes AND fanout separately, double-counting"
        );
    }

    // =========================================================================
    // BLACKHAT security-focused tests
    // =========================================================================

    /// BLACKHAT_resources_node_count_truncation [MEDIUM]:
    /// compute_resource_bounds uses `u32::try_from(parts.nodes.len())`
    /// clamping to u32::MAX. While unlikely to be hit in practice, a
    /// workflow with more than ~4 billion nodes would silently report
    /// u32::MAX instead of the real count, causing resource comparison
    /// errors in the panel.
    #[test]
    fn blackhat_resources_node_count_truncation_to_u32_max() {
        // Verify the truncation logic.
        let huge_count: usize = 5_000_000_000;
        let clamped = u32::try_from(huge_count).unwrap_or(u32::MAX);
        assert_eq!(
            clamped,
            u32::MAX,
            "BLACKHAT [MEDIUM]: node count > u32::MAX is clamped to u32::MAX"
        );
    }

    /// BLACKHAT_resources_payload_metric_semantics [LOW]:
    /// The "estimated_action_payload / max_ipc_payload_bytes" metric computes
    /// do_node_count * max_ipc_payload_bytes and compares against
    /// max_ipc_payload_bytes. For any workflow with > 1 Do node, this always
    /// exceeds the limit. The metric's semantics are questionable because it
    /// compares total payload against single-payload limit.
    #[test]
    fn blackhat_resources_two_do_nodes_always_exceed_payload_limit() {
        let contract = ResourceContract {
            max_ipc_payload_bytes: 1024,
            ..ResourceContract::DEFAULT
        };
        let bounds = ResourceBounds {
            slot_count: 2,
            node_count: 3,
            do_node_count: 2,
            max_action_payload: 1024,
            max_result_size: 512,
            retry_budget: 6,
            estimated_peak_frames: 1,
        };
        let panel = ResourceBoundsPanel::new(&contract, &bounds);
        let payload_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "estimated_action_payload / max_ipc_payload_bytes");
        let Some(m) = payload_metric else { return };
        // 2 * 1024 = 2048 > 1024 => ExceedsLimit
        assert_eq!(
            m.status,
            ResourceStatus::ExceedsLimit,
            "BLACKHAT [LOW]: any workflow with >1 Do node always exceeds payload limit"
        );
    }

    /// BLACKHAT_resources_slot_count_u16_to_u64_safe [CONFIRMED-SAFE]:
    /// slot_count is u16 and is converted to u64 for comparison. This
    /// conversion is always safe because u16 fits in u64.
    #[test]
    fn blackhat_resources_slot_count_conversion_safe() {
        let max_slot: u16 = u16::MAX;
        let converted: u64 = u64::from(max_slot);
        assert_eq!(converted, 65535u64, "u16 to u64 conversion is always safe");
    }

    /// BLACKHAT_resources_zero_do_nodes_zero_payload [CONFIRMED-SAFE]:
    /// With zero Do nodes, payload and result estimations are 0, which
    /// should be WithinBounds.
    #[test]
    fn blackhat_resources_zero_do_nodes_zero_payload_within_bounds() {
        let contract = ResourceContract {
            max_ipc_payload_bytes: 1024,
            max_output_bytes: 512,
            ..ResourceContract::DEFAULT
        };
        let bounds = ResourceBounds {
            slot_count: 2,
            node_count: 5,
            do_node_count: 0,
            max_action_payload: 1024,
            max_result_size: 512,
            retry_budget: 0,
            estimated_peak_frames: 1,
        };
        let panel = ResourceBoundsPanel::new(&contract, &bounds);

        let payload_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "estimated_action_payload / max_ipc_payload_bytes");
        let Some(m) = payload_metric else { return };
        assert_eq!(m.computed_value, 0);
        assert_eq!(m.status, ResourceStatus::WithinBounds);

        let result_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "estimated_result_size / max_output_bytes");
        let Some(r) = result_metric else { return };
        assert_eq!(r.computed_value, 0);
        assert_eq!(r.status, ResourceStatus::WithinBounds);
    }

    /// BLACKHAT_resources_foreach_zero_limit [LOW]:
    /// A ForEachStart with limit=0 contributes 0 iteration frames but
    /// still adds +3 structural frames (start+next+join). This is correct
    /// but the test documents the behavior.
    #[test]
    fn blackhat_resources_foreach_zero_limit_contributes_structural_only() {
        let parts = make_parts(vec![
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 0, // zero iterations
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(2),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
            CompiledNodeKind::ForEachJoin {
                output: SlotIdx::new(3),
            },
        ]);
        let bounds = compute_resource_bounds(&parts);
        // Base 1 + 3 structural loop nodes = 4, + 0 iterations = 4
        assert_eq!(
            bounds.estimated_peak_frames, 4,
            "ForEachStart with limit=0 should contribute structural frames only"
        );
    }

    /// BLACKHAT_resources_together_zero_branches [LOW]:
    /// A TogetherStart with zero branches contributes +1 for TogetherStart
    /// and +1 for TogetherJoin but 0 fanout. This is correct behavior.
    #[test]
    fn blackhat_resources_together_zero_branches_no_fanout() {
        let parts = make_parts(vec![
            CompiledNodeKind::TogetherStart {
                branches: Box::new([]),
                join: StepIdx::new(1),
            },
            CompiledNodeKind::TogetherJoin {
                branch_count: 0,
                accumulator: SlotIdx::new(0),
            },
        ]);
        let bounds = compute_resource_bounds(&parts);
        // Base 1 + 2 together nodes = 3, + 0 fanout = 3
        assert_eq!(
            bounds.estimated_peak_frames, 3,
            "TogetherStart with zero branches should have no fanout contribution"
        );
    }
}
