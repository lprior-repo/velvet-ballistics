#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

use super::budget_error::BudgetError;
use super::types::WholeWorkflowBudget;

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
}

impl BoundednessPolicy {
    /// Conservative default policy.
    ///
    /// `max_total_steps` matches the master spec
    /// `velvet-ballistics-MASTER.md` §13 line 479 (Steps | 1000).
    /// This is a production extension field; the spec's `BoundednessPolicy`
    /// (master §65 line 3241-3247) defines 5 `absolute_max_*` fields. We
    /// add `max_total_steps` to bound the per-workflow cap below the
    /// `absolute_max_run_time_seconds` budget.
    pub const DEFAULT: Self = Self {
        max_total_steps: 1_000,
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
        Ok(())
    }
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
    validate_u32_budget(
        "journal",
        budget.max_journal_batch_bytes,
        policy.absolute_max_journal_batch_bytes,
    )?;
    validate_u32_budget(
        "queue",
        budget.max_queue_depth,
        policy.absolute_max_queue_depth,
    )?;
    validate_u32_budget(
        "ipc",
        budget.max_ipc_payload_bytes,
        policy.absolute_max_ipc_payload_bytes,
    )?;
    validate_u64_budget(
        "blob",
        budget.max_blob_bytes,
        policy.absolute_max_blob_bytes,
    )?;
    validate_u32_budget(
        "input",
        budget.max_input_bytes,
        policy.absolute_max_input_bytes,
    )
}

fn validate_u32_budget(kind: &'static str, actual: u32, limit: u32) -> Result<(), BudgetError> {
    if actual <= limit {
        return Ok(());
    }
    match kind {
        "journal" => Err(BudgetError::JournalBatchBytesExceeded { actual, limit }),
        "queue" => Err(BudgetError::QueueDepthExceeded { actual, limit }),
        "ipc" => Err(BudgetError::IpcPayloadBytesExceeded { actual, limit }),
        _ => Err(BudgetError::InputBytesExceeded { actual, limit }),
    }
}

fn validate_u64_budget(kind: &'static str, actual: u64, limit: u64) -> Result<(), BudgetError> {
    if actual <= limit {
        return Ok(());
    }
    match kind {
        "blob" => Err(BudgetError::BlobBytesExceeded { actual, limit }),
        _ => Err(BudgetError::TraceEventsExceeded { actual, limit }),
    }
}
