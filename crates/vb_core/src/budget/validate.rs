#![forbid(unsafe_code)]
//! Boundedness policy validation: policy limits, the default policy,
//! and the per-dimension checks a computed budget must satisfy.

use super::{BudgetError, WholeWorkflowBudget};

/// Policy limits that a computed budget must satisfy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundednessPolicy {
    /// Maximum allowed total steps.
    pub max_total_steps: u64,
    /// Maximum allowed total slots.
    pub max_total_slots: u64,
    /// Maximum allowed fanout.
    pub max_fanout: u16,
    /// Maximum allowed nesting depth.
    pub max_nesting_depth: u16,
    /// Absolute maximum action tickets.
    pub absolute_max_action_tickets: u32,
    /// Absolute maximum parallel in-flight.
    pub absolute_max_parallel: u16,
    /// Absolute maximum run time in seconds.
    pub absolute_max_run_time_seconds: u64,
    /// Absolute maximum result bytes.
    pub absolute_max_result_bytes: u32,
    /// Absolute maximum steps executable.
    pub absolute_max_steps_executable: u32,
    /// Absolute maximum timer entries.
    pub absolute_max_timer_entries: u32,
    /// Absolute maximum trace events.
    pub absolute_max_trace_events: u64,
    /// Absolute maximum journal batch bytes.
    pub absolute_max_journal_batch_bytes: u32,
    /// Absolute maximum queue depth.
    pub absolute_max_queue_depth: u32,
    /// Absolute maximum IPC payload bytes.
    pub absolute_max_ipc_payload_bytes: u32,
    /// Absolute maximum blob bytes.
    pub absolute_max_blob_bytes: u64,
    /// Absolute maximum input bytes.
    pub absolute_max_input_bytes: u32,
    /// Absolute maximum open gates.
    pub absolute_max_open_gates: u32,
    /// Absolute maximum signal subscriptions.
    pub absolute_max_signal_subscriptions: u32,
    /// Absolute maximum incident emissions.
    pub absolute_max_incident_emissions: u64,
    /// Absolute maximum total transitions over the run.
    pub absolute_max_transitions: u64,
    /// Absolute maximum cumulative journal bytes over the run.
    pub absolute_max_journal_bytes_total: u64,
    /// Absolute maximum value-store entries over the run.
    pub absolute_max_value_store_entries: u64,
    /// Absolute maximum cost units over the run.
    pub absolute_max_cost_units: u64,
}

impl BoundednessPolicy {
    /// Conservative default policy.
    pub const DEFAULT: Self = Self {
        max_total_steps: 1_000_000,
        max_total_slots: 65_535,
        max_fanout: 64,
        max_nesting_depth: 8,
        absolute_max_action_tickets: 100_000,
        absolute_max_parallel: 256,
        absolute_max_run_time_seconds: 2_592_000,
        absolute_max_result_bytes: 262_144,
        absolute_max_steps_executable: 1_000_000,
        absolute_max_timer_entries: 1_000_000,
        absolute_max_trace_events: 1_000_000,
        absolute_max_journal_batch_bytes: 1_048_576,
        absolute_max_queue_depth: 1_024,
        absolute_max_ipc_payload_bytes: 1_048_576,
        absolute_max_blob_bytes: 16_777_216,
        absolute_max_input_bytes: 1_048_576,
        absolute_max_open_gates: 1_000_000,
        absolute_max_signal_subscriptions: 1_000_000,
        absolute_max_incident_emissions: 1_000_000,
        absolute_max_transitions: 10_000_000,
        absolute_max_journal_bytes_total: 1_048_576_000_000,
        absolute_max_value_store_entries: 2_000_000,
        absolute_max_cost_units: 100_000,
    };

    /// Validates the computed budget against this policy. Returns the first
    /// violation encountered.
    pub fn validate(&self, budget: &WholeWorkflowBudget) -> Result<(), BudgetError> {
        if budget.max_total_steps > self.max_total_steps {
            return Err(BudgetError::TotalStepsExceeded {
                actual: budget.max_total_steps,
                limit: self.max_total_steps,
            });
        }
        if budget.max_total_slots > self.max_total_slots {
            return Err(BudgetError::TotalSlotsExceeded {
                actual: budget.max_total_slots,
                limit: self.max_total_slots,
            });
        }
        if budget.max_fanout > self.max_fanout {
            return Err(BudgetError::FanoutExceeded {
                actual: budget.max_fanout,
                limit: self.max_fanout,
            });
        }
        if budget.max_nesting_depth > self.max_nesting_depth {
            return Err(BudgetError::NestingDepthExceeded {
                actual: budget.max_nesting_depth,
                limit: self.max_nesting_depth,
            });
        }
        if budget.max_action_tickets > self.absolute_max_action_tickets {
            return Err(BudgetError::ActionTicketsExceeded {
                actual: budget.max_action_tickets,
                limit: self.absolute_max_action_tickets,
            });
        }
        if budget.max_parallel_in_flight > self.absolute_max_parallel {
            return Err(BudgetError::ParallelExceeded {
                actual: budget.max_parallel_in_flight,
                limit: self.absolute_max_parallel,
            });
        }
        if budget.max_run_time_seconds > self.absolute_max_run_time_seconds {
            return Err(BudgetError::RunTimeExceeded {
                actual: budget.max_run_time_seconds,
                limit: self.absolute_max_run_time_seconds,
            });
        }
        if budget.max_result_bytes > self.absolute_max_result_bytes {
            return Err(BudgetError::ResultBytesExceeded {
                actual: budget.max_result_bytes,
                limit: self.absolute_max_result_bytes,
            });
        }
        if budget.max_steps_executable > self.absolute_max_steps_executable {
            return Err(BudgetError::StepsExecutableExceeded {
                actual: budget.max_steps_executable,
                limit: self.absolute_max_steps_executable,
            });
        }
        validate_extended_budget(self, budget)?;
        validate_budget_dimensions(self, budget)?;
        Ok(())
    }
}

fn validate_budget_dimensions(
    policy: &BoundednessPolicy,
    budget: &WholeWorkflowBudget,
) -> Result<(), BudgetError> {
    if budget.max_open_gates > policy.absolute_max_open_gates {
        return Err(BudgetError::OpenGatesExceeded {
            actual: budget.max_open_gates,
            limit: policy.absolute_max_open_gates,
        });
    }
    if budget.max_signal_subscriptions > policy.absolute_max_signal_subscriptions {
        return Err(BudgetError::SignalSubscriptionsExceeded {
            actual: budget.max_signal_subscriptions,
            limit: policy.absolute_max_signal_subscriptions,
        });
    }
    if budget.max_incident_emissions > policy.absolute_max_incident_emissions {
        return Err(BudgetError::IncidentEmissionsExceeded {
            actual: budget.max_incident_emissions,
            limit: policy.absolute_max_incident_emissions,
        });
    }
    if budget.max_transitions > policy.absolute_max_transitions {
        return Err(BudgetError::TransitionsExceeded {
            actual: budget.max_transitions,
            limit: policy.absolute_max_transitions,
        });
    }
    if budget.max_journal_bytes_total > policy.absolute_max_journal_bytes_total {
        return Err(BudgetError::JournalBytesTotalExceeded {
            actual: budget.max_journal_bytes_total,
            limit: policy.absolute_max_journal_bytes_total,
        });
    }
    if budget.max_value_store_entries > policy.absolute_max_value_store_entries {
        return Err(BudgetError::ValueStoreEntriesExceeded {
            actual: budget.max_value_store_entries,
            limit: policy.absolute_max_value_store_entries,
        });
    }
    if budget.max_cost_units > policy.absolute_max_cost_units {
        return Err(BudgetError::CostUnitsExceeded {
            actual: budget.max_cost_units,
            limit: policy.absolute_max_cost_units,
        });
    }
    Ok(())
}

fn validate_extended_budget(
    policy: &BoundednessPolicy,
    budget: &WholeWorkflowBudget,
) -> Result<(), BudgetError> {
    if budget.max_timer_entries > policy.absolute_max_timer_entries {
        return Err(BudgetError::TimerEntriesExceeded {
            actual: budget.max_timer_entries,
            limit: policy.absolute_max_timer_entries,
        });
    }
    if budget.max_trace_events > policy.absolute_max_trace_events {
        return Err(BudgetError::TraceEventsExceeded {
            actual: budget.max_trace_events,
            limit: policy.absolute_max_trace_events,
        });
    }
    validate_payload_budget(policy, budget)
}

fn validate_payload_budget(
    policy: &BoundednessPolicy,
    budget: &WholeWorkflowBudget,
) -> Result<(), BudgetError> {
    if budget.max_journal_batch_bytes > policy.absolute_max_journal_batch_bytes {
        return Err(BudgetError::JournalBatchBytesExceeded {
            actual: budget.max_journal_batch_bytes,
            limit: policy.absolute_max_journal_batch_bytes,
        });
    }
    if budget.max_queue_depth > policy.absolute_max_queue_depth {
        return Err(BudgetError::QueueDepthExceeded {
            actual: budget.max_queue_depth,
            limit: policy.absolute_max_queue_depth,
        });
    }
    if budget.max_ipc_payload_bytes > policy.absolute_max_ipc_payload_bytes {
        return Err(BudgetError::IpcPayloadBytesExceeded {
            actual: budget.max_ipc_payload_bytes,
            limit: policy.absolute_max_ipc_payload_bytes,
        });
    }
    if budget.max_blob_bytes > policy.absolute_max_blob_bytes {
        return Err(BudgetError::BlobBytesExceeded {
            actual: budget.max_blob_bytes,
            limit: policy.absolute_max_blob_bytes,
        });
    }
    if budget.max_input_bytes > policy.absolute_max_input_bytes {
        return Err(BudgetError::InputBytesExceeded {
            actual: budget.max_input_bytes,
            limit: policy.absolute_max_input_bytes,
        });
    }
    Ok(())
}
