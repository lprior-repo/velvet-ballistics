//! Resource bounds panel -- displays contract limits and computed worst-case resource usage.

use vb_core::workflow::{CompiledNodeKind, ResourceContract, WorkflowParts};

/// Whether a resource metric is within its contracted bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    let retry_budget = do_node_count
        .saturating_mul(u32::from(contract.max_retry_attempts));

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
                id: StepIdx::new(i as u16),
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
        let m = node_metric.unwrap_or_else(|| panic!("metric missing"));
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
        assert_eq!(
            node_metric.unwrap_or_else(|| panic!("metric missing")).status,
            ResourceStatus::ExceedsLimit
        );

        let slot_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "slot_count / max_slots");
        assert!(slot_metric.is_some());
        assert_eq!(
            slot_metric.unwrap_or_else(|| panic!("metric missing")).status,
            ResourceStatus::ExceedsLimit
        );
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
        let fanout_metric = worst.iter().find(|m| m.label == "do_node_count / max_fanout");
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
}
