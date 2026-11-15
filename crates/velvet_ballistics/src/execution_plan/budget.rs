use serde::Serialize;
use vb_core::WholeWorkflowBudget;

#[derive(Serialize)]
pub(super) struct BudgetReport {
    pub(super) status: &'static str,
    #[serde(flatten)]
    pub(super) computed: BudgetValues,
}

#[derive(Serialize)]
pub(super) struct BudgetValues {
    #[serde(flatten)]
    work: WorkBudget,
    #[serde(flatten)]
    primitives: PrimitiveBudget,
    #[serde(flatten)]
    transport: TransportBudget,
    #[serde(flatten)]
    lifecycle: LifecycleBudget,
}

impl From<WholeWorkflowBudget> for BudgetValues {
    fn from(budget: WholeWorkflowBudget) -> Self {
        Self {
            work: budget.into(),
            primitives: budget.into(),
            transport: budget.into(),
            lifecycle: budget.into(),
        }
    }
}

#[derive(Serialize)]
struct WorkBudget {
    max_total_steps: u64,
    max_total_slots: u64,
    max_fanout: u16,
    max_nesting_depth: u16,
    max_steps_executable: u32,
    max_action_tickets: u32,
    max_parallel_in_flight: u16,
    max_retries_per_action: u16,
}

impl From<WholeWorkflowBudget> for WorkBudget {
    fn from(budget: WholeWorkflowBudget) -> Self {
        let WholeWorkflowBudget {
            max_total_steps,
            max_total_slots,
            max_fanout,
            max_nesting_depth,
            max_steps_executable,
            max_action_tickets,
            max_parallel_in_flight,
            max_retries_per_action,
            ..
        } = budget;
        Self {
            max_total_steps,
            max_total_slots,
            max_fanout,
            max_nesting_depth,
            max_steps_executable,
            max_action_tickets,
            max_parallel_in_flight,
            max_retries_per_action,
        }
    }
}

#[derive(Serialize)]
struct PrimitiveBudget {
    max_gather_pages: u32,
    max_gather_items: u32,
    max_for_each_iterations: u32,
    max_together_branches: u16,
    max_repeat_attempts: u16,
    max_run_time_seconds: u64,
    max_result_bytes: u32,
    max_total_slots_written: u32,
}

impl From<WholeWorkflowBudget> for PrimitiveBudget {
    fn from(budget: WholeWorkflowBudget) -> Self {
        let WholeWorkflowBudget {
            max_gather_pages,
            max_gather_items,
            max_for_each_iterations,
            max_together_branches,
            max_repeat_attempts,
            max_run_time_seconds,
            max_result_bytes,
            max_total_slots_written,
            ..
        } = budget;
        Self {
            max_gather_pages,
            max_gather_items,
            max_for_each_iterations,
            max_together_branches,
            max_repeat_attempts,
            max_run_time_seconds,
            max_result_bytes,
            max_total_slots_written,
        }
    }
}

#[derive(Serialize)]
struct TransportBudget {
    max_timer_entries: u32,
    max_trace_events: u64,
    max_journal_batch_bytes: u32,
    max_queue_depth: u32,
    max_ipc_payload_bytes: u32,
    max_blob_bytes: u64,
    max_input_bytes: u32,
}

impl From<WholeWorkflowBudget> for TransportBudget {
    fn from(budget: WholeWorkflowBudget) -> Self {
        let WholeWorkflowBudget {
            max_timer_entries,
            max_trace_events,
            max_journal_batch_bytes,
            max_queue_depth,
            max_ipc_payload_bytes,
            max_blob_bytes,
            max_input_bytes,
            ..
        } = budget;
        Self {
            max_timer_entries,
            max_trace_events,
            max_journal_batch_bytes,
            max_queue_depth,
            max_ipc_payload_bytes,
            max_blob_bytes,
            max_input_bytes,
        }
    }
}

#[derive(Serialize)]
struct LifecycleBudget {
    max_open_gates: u32,
    max_signal_subscriptions: u32,
    max_incident_emissions: u64,
    max_transitions: u64,
    max_journal_bytes_total: u64,
    max_value_store_entries: u64,
    max_cost_units: u64,
}

impl From<WholeWorkflowBudget> for LifecycleBudget {
    fn from(budget: WholeWorkflowBudget) -> Self {
        let WholeWorkflowBudget {
            max_open_gates,
            max_signal_subscriptions,
            max_incident_emissions,
            max_transitions,
            max_journal_bytes_total,
            max_value_store_entries,
            max_cost_units,
            ..
        } = budget;
        Self {
            max_open_gates,
            max_signal_subscriptions,
            max_incident_emissions,
            max_transitions,
            max_journal_bytes_total,
            max_value_store_entries,
            max_cost_units,
        }
    }
}
