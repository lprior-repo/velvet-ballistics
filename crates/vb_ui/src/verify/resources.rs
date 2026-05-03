//! Resource bounds panel -- displays contract limits and computed worst-case resource usage.

use vb_core::workflow::ResourceContract;

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

/// Panel of resource metrics for UI display.
pub struct ResourceBoundsPanel {
    metrics: Vec<ResourceMetric>,
}

impl ResourceBoundsPanel {
    /// Build a resource bounds panel from a contract and computed workflow statistics.
    ///
    /// - `node_count`: number of compiled nodes
    /// - `slot_count`: declared slot count from the workflow
    /// - `action_count`: number of Do action nodes
    #[must_use]
    pub fn from_contract(
        contract: &ResourceContract,
        node_count: usize,
        slot_count: u16,
        action_count: usize,
    ) -> Self {
        let mut metrics = Vec::new();

        // Node count vs max_steps
        let node_count_u64 = u64::try_from(node_count).unwrap_or(u64::MAX);
        metrics.push(ResourceMetric {
            label: "node_count / max_steps",
            contract_value: u64::from(contract.max_steps),
            computed_value: node_count_u64,
            status: classify(node_count_u64, u64::from(contract.max_steps)),
        });

        // Slot count vs max_slots
        let slot_count_u64 = u64::from(slot_count);
        metrics.push(ResourceMetric {
            label: "slot_count / max_slots",
            contract_value: u64::from(contract.max_slots),
            computed_value: slot_count_u64,
            status: classify(slot_count_u64, u64::from(contract.max_slots)),
        });

        // Action count as a proxy for max_input_bytes / max_output_bytes usage.
        // We compare action_count against max_retry_attempts as a loose bound
        // on action dispatch pressure (each Do node may be retried).
        let action_count_u64 = u64::try_from(action_count).unwrap_or(u64::MAX);
        metrics.push(ResourceMetric {
            label: "action_count / max_retry_attempts",
            contract_value: u64::from(contract.max_retry_attempts),
            computed_value: action_count_u64,
            status: classify(action_count_u64, u64::from(contract.max_retry_attempts)),
        });

        // Nodes + slot_count as a proxy for step budget usage.
        // step_budget is max_step_budget_per_tick; we compare total nodes as a
        // lower-bound proxy.
        let budget_proxy = node_count_u64.saturating_add(slot_count_u64);
        metrics.push(ResourceMetric {
            label: "node+slot proxy / max_step_budget_per_tick",
            contract_value: contract.max_step_budget_per_tick,
            computed_value: budget_proxy,
            status: classify(budget_proxy, contract.max_step_budget_per_tick),
        });

        // Fanout: we use action_count as a proxy for fanout pressure.
        let fanout_u64 = u64::from(contract.max_fanout);
        metrics.push(ResourceMetric {
            label: "action_count / max_fanout",
            contract_value: fanout_u64,
            computed_value: action_count_u64,
            status: classify(action_count_u64, fanout_u64),
        });

        // Collect items vs max_collect_items -- node_count as a rough proxy.
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

    fn contract_with(steps: u16, slots: u16) -> ResourceContract {
        ResourceContract {
            max_steps: steps,
            max_slots: slots,
            ..ResourceContract::DEFAULT
        }
    }

    #[test]
    fn test_all_within_bounds() {
        let contract = contract_with(100, 50);
        let panel = ResourceBoundsPanel::from_contract(&contract, 10, 5, 2);
        assert!(panel.all_within_bounds());
        assert!(panel.worst_case_metrics().is_empty());
    }

    #[test]
    fn test_at_limit() {
        let contract = contract_with(10, 50);
        let panel = ResourceBoundsPanel::from_contract(&contract, 10, 5, 2);
        // node_count equals max_steps -> AtLimit
        let node_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "node_count / max_steps");
        assert!(node_metric.is_some());
        let metric = node_metric.unwrap_or_else(|| panic!("metric missing"));
        assert_eq!(metric.status, ResourceStatus::AtLimit);
        assert_eq!(metric.computed_value, 10);
        assert_eq!(metric.contract_value, 10);
        assert!(!panel.all_within_bounds());
    }

    #[test]
    fn test_exceeds_limit() {
        let contract = contract_with(5, 2);
        let panel = ResourceBoundsPanel::from_contract(&contract, 10, 5, 1);
        // node_count (10) > max_steps (5)
        let node_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "node_count / max_steps");
        assert!(node_metric.is_some());
        let metric = node_metric.unwrap_or_else(|| panic!("metric missing"));
        assert_eq!(metric.status, ResourceStatus::ExceedsLimit);

        // slot_count (5) > max_slots (2)
        let slot_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "slot_count / max_slots");
        assert!(slot_metric.is_some());
        let slot = slot_metric.unwrap_or_else(|| panic!("metric missing"));
        assert_eq!(slot.status, ResourceStatus::ExceedsLimit);
    }

    #[test]
    fn test_worst_case_filters_within_bounds() {
        let contract = contract_with(100, 50);
        let panel = ResourceBoundsPanel::from_contract(&contract, 10, 5, 2);
        // All within bounds, worst_case should be empty
        assert!(panel.worst_case_metrics().is_empty());
    }

    #[test]
    fn test_worst_case_includes_at_limit() {
        let contract = contract_with(10, 50);
        let panel = ResourceBoundsPanel::from_contract(&contract, 10, 5, 2);
        let worst = panel.worst_case_metrics();
        assert!(!worst.is_empty());
        assert!(
            worst
                .iter()
                .all(|m| m.status != ResourceStatus::WithinBounds)
        );
    }

    #[test]
    fn test_metrics_count() {
        let contract = ResourceContract::DEFAULT;
        let panel = ResourceBoundsPanel::from_contract(&contract, 10, 5, 2);
        assert_eq!(panel.metrics().len(), 6);
    }

    #[test]
    fn test_default_contract_large_limits() {
        // Default contract has max_steps = 10_000, max_slots = 1_024,
        // max_retry_attempts = 3, max_fanout = 64.
        let panel = ResourceBoundsPanel::from_contract(&ResourceContract::DEFAULT, 50, 100, 2);
        assert!(panel.all_within_bounds());
    }

    #[test]
    fn test_zero_slots_within_bounds() {
        let contract = contract_with(100, 10);
        let panel = ResourceBoundsPanel::from_contract(&contract, 5, 0, 0);
        assert!(panel.all_within_bounds());
    }

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
    fn test_classify_zero_within() {
        assert_eq!(classify(0, 0), ResourceStatus::AtLimit);
    }

    #[test]
    fn test_action_count_exceeds_retry() {
        let mut contract = ResourceContract::DEFAULT;
        contract.max_retry_attempts = 2;
        let panel = ResourceBoundsPanel::from_contract(&contract, 10, 5, 5);
        let action_metric = panel
            .metrics()
            .iter()
            .find(|m| m.label == "action_count / max_retry_attempts");
        assert!(action_metric.is_some());
        let metric = action_metric.unwrap_or_else(|| panic!("metric missing"));
        assert_eq!(metric.status, ResourceStatus::ExceedsLimit);
    }
}
