#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

//! Whole-workflow budget computation and boundedness policy enforcement.

use crate::ids::{RunId, StepIdx, TenantId};
use crate::workflow::{
    CompiledNodeKind, CompiledWorkflow, NodeContinuation, ResourceContract, WorkflowError,
};
use thiserror::Error;

/// Computed budget for an entire workflow, derived by walking the IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WholeWorkflowBudget {
    /// Sum of all step budgets across all branches.
    pub max_total_steps: u64,
    /// Maximum slot count across all paths.
    pub max_total_slots: u64,
    /// Maximum concurrent branches (fanout).
    pub max_fanout: u16,
    /// Conservative structural nesting bound over reachable owners.
    pub max_nesting_depth: u16,
    /// Maximum executable step count per workflow admission.
    pub max_steps_executable: u32,
    /// Maximum action tickets (Do nodes) in the workflow.
    pub max_action_tickets: u32,
    /// Maximum parallel in-flight actions.
    pub max_parallel_in_flight: u16,
    /// Maximum retries per action.
    pub max_retries_per_action: u16,
    /// Maximum gather pages across all CollectStart nodes.
    pub max_gather_pages: u32,
    /// Maximum gather items across all CollectStart nodes.
    pub max_gather_items: u32,
    /// Maximum for-each loop iterations.
    pub max_for_each_iterations: u32,
    /// Maximum together branches in any TogetherStart.
    pub max_together_branches: u16,
    /// Maximum repeat attempts in any RepeatStart.
    pub max_repeat_attempts: u16,
    /// Maximum run time in seconds.
    pub max_run_time_seconds: u64,
    /// Maximum result bytes.
    pub max_result_bytes: u32,
    /// Maximum total slots written.
    pub max_total_slots_written: u32,
    /// Maximum timer entries reserved for waits, asks, retries, and repeat checks.
    pub max_timer_entries: u32,
    /// Maximum trace events reserved for deterministic execution.
    pub max_trace_events: u64,
    /// Maximum journal batch bytes required by reachable journal-producing operations.
    pub max_journal_batch_bytes: u32,
    /// Maximum queue entries required by reachable suspension/admission operations.
    pub max_queue_depth: u32,
    /// Maximum IPC payload bytes required by reachable IPC operations.
    pub max_ipc_payload_bytes: u32,
    /// Maximum blob bytes required by reachable blob/resource operations.
    pub max_blob_bytes: u64,
    /// Maximum input bytes required by reachable input operations.
    pub max_input_bytes: u32,
    /// Maximum simultaneously open gates, counted as reachable Ask nodes.
    /// AskResume nodes are resumes, not new gates, and are not counted.
    pub max_open_gates: u32,
    /// Maximum signal subscriptions, counted as reachable WaitEvent nodes.
    /// WaitUntil nodes are timer waits, not signal subscriptions.
    pub max_signal_subscriptions: u32,
    /// Maximum incident emissions over the run. No IR node emits incidents,
    /// so this is bounded by total step executions: at most one incident
    /// per executed step.
    pub max_incident_emissions: u64,
    /// Maximum total engine transitions over the run. This is the run total
    /// that `max_transitions_per_tick` (per-tick, from the contract) is not:
    /// total steps times the retry factor, since a retry body re-executes.
    pub max_transitions: u64,
    /// Maximum cumulative journal bytes over the run: the per-batch cap
    /// times total steps. Batches hold at least one event each and journal
    /// events are bounded by trace events, which equal total steps.
    pub max_journal_bytes_total: u64,
    /// Maximum value-store entries over the run: at most one new top-level
    /// entry per executed step, plus input slots and the constant pool.
    /// Deeply nested action/input payloads are bounded at runtime by the
    /// byte caps and the remaining-budget counter, not by this bound.
    pub max_value_store_entries: u64,
    /// Maximum cost units over the run. Cost model: one unit per action
    /// ticket issued, and a retry re-issues the ticket, so this equals
    /// the action ticket bound times the retry factor, mirroring the
    /// transition bound: a verifier that counted distinct tickets only
    /// would under-approximate every retried action.
    pub max_cost_units: u64,
}

impl WholeWorkflowBudget {
    pub const MAX_STEPS_EXECUTABLE: u32 = 32_768;

    /// Walks the compiled IR starting from `entry` and computes all
    /// budget dimensions.
    pub fn compute(
        nodes: &[crate::workflow::CompiledNode],
        entry: StepIdx,
        contract: &ResourceContract,
    ) -> Result<Self, WorkflowError> {
        Self::compute_budget_local(nodes, entry, contract).map_err(WorkflowError::from)
    }

    /// Internal budget traversal path with a narrow error type. This keeps Kani
    /// from exploring unrelated `WorkflowError::Expression(CoreError)` drops.
    #[cfg_attr(kani, allow(unreachable_code))]
    pub(crate) fn compute_budget_local(
        nodes: &[crate::workflow::CompiledNode],
        entry: StepIdx,
        contract: &ResourceContract,
    ) -> Result<Self, BudgetTraversalError> {
        let node_count = nodes.len();
        if entry.as_usize() >= node_count {
            return Err(BudgetTraversalError::EntryOutOfBounds { entry });
        }

        if let Some(budget) = compute_small_linear_budget(nodes, entry, contract)? {
            return Ok(budget);
        }

        #[cfg(kani)]
        return Err(BudgetTraversalError::StepOutOfBounds { step: entry });

        #[cfg(not(kani))]
        {
            let mut visited: Vec<bool> = vec![false; node_count];
            let mut in_path: Vec<u16> = bounded_tracking_vec(node_count);
            let max_total_steps = count_total_steps(nodes, entry, node_count)?;
            check_total_steps_representable(max_total_steps)?;

            let mut max_fanout: u16 = 0;
            let mut max_action_tickets: u32 = 0;
            let mut max_parallel_in_flight: u16 = 0;
            let mut max_gather_pages: u32 = 0;
            let mut max_gather_items: u32 = 0;
            let mut max_for_each_iterations: u32 = 0;
            let mut max_together_branches: u16 = 0;
            let mut max_repeat_attempts: u16 = 0;
            let mut max_timer_entries: u32 = 0;
            let mut max_open_gates: u32 = 0;
            let mut max_signal_subscriptions: u32 = 0;
            compute_reachable_metrics(
                nodes,
                entry,
                &mut visited,
                &mut in_path,
                node_count,
                &mut max_fanout,
                &mut max_action_tickets,
                &mut max_parallel_in_flight,
                &mut max_gather_pages,
                &mut max_gather_items,
                &mut max_for_each_iterations,
                &mut max_together_branches,
                &mut max_repeat_attempts,
                &mut max_timer_entries,
                &mut max_open_gates,
                &mut max_signal_subscriptions,
            )?;

            let max_nesting_depth = compute_structural_depth(nodes, &visited)?;
            let max_total_slots = u64::from(contract.max_slots);

            // Phase 0 executes at most one step per runtime tick, so steps bound time.
            let max_run_time_seconds = max_total_steps;

            let derived = derive_budget_dimensions(max_total_steps, max_action_tickets, contract)?;

            Ok(Self {
                max_total_steps,
                max_total_slots,
                max_fanout,
                max_nesting_depth,
                max_steps_executable: Self::MAX_STEPS_EXECUTABLE,
                max_action_tickets,
                max_parallel_in_flight,
                max_retries_per_action: contract.max_retry_attempts,
                max_gather_pages,
                max_gather_items,
                max_for_each_iterations,
                max_together_branches,
                max_repeat_attempts,
                max_run_time_seconds,
                max_result_bytes: contract.max_output_bytes,
                max_total_slots_written: u32::from(contract.max_slots),
                max_timer_entries,
                max_trace_events: max_total_steps,
                max_journal_batch_bytes: contract.max_journal_batch_bytes,
                max_queue_depth: contract.max_queue_depth,
                max_ipc_payload_bytes: contract.max_ipc_payload_bytes,
                max_blob_bytes: contract.max_blob_bytes,
                max_input_bytes: contract.max_input_bytes,
                max_open_gates,
                max_signal_subscriptions,
                max_incident_emissions: derived.max_incident_emissions,
                max_transitions: derived.max_transitions,
                max_journal_bytes_total: derived.max_journal_bytes_total,
                max_value_store_entries: derived.max_value_store_entries,
                max_cost_units: derived.max_cost_units,
            })
        }
    }
}

fn check_total_steps_representable(total: u64) -> Result<(), BudgetTraversalError> {
    if total > u64::from(u32::MAX) {
        return Err(BudgetTraversalError::StepCountOverflow { actual: total });
    }
    Ok(())
}

/// Dimensions derived from the step and ticket bounds plus the contract.
/// Shared by the general and small-linear compute paths so both agree.
struct DerivedBudgetDimensions {
    max_transitions: u64,
    max_journal_bytes_total: u64,
    max_value_store_entries: u64,
    max_incident_emissions: u64,
    max_cost_units: u64,
}

fn derive_budget_dimensions(
    max_total_steps: u64,
    max_action_tickets: u32,
    contract: &ResourceContract,
) -> Result<DerivedBudgetDimensions, BudgetTraversalError> {
    let retry_factor = u64::from(contract.max_retry_attempts)
        .checked_add(1)
        .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
    let max_transitions = max_total_steps
        .checked_mul(retry_factor)
        .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
    let max_journal_bytes_total = u64::from(contract.max_journal_batch_bytes)
        .checked_mul(max_total_steps)
        .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
    let max_value_store_entries = max_total_steps
        .checked_add(u64::from(contract.max_slots))
        .and_then(|value| value.checked_add(u64::from(contract.max_constants)))
        .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
    let max_cost_units = u64::from(max_action_tickets)
        .checked_mul(retry_factor)
        .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
    Ok(DerivedBudgetDimensions {
        max_transitions,
        max_journal_bytes_total,
        max_value_store_entries,
        max_incident_emissions: max_total_steps,
        max_cost_units,
    })
}

/// Budget-local traversal failures. Deliberately excludes expression/core error
/// variants so proof harnesses do not pay for unrelated destructor graphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BudgetTraversalError {
    EntryOutOfBounds {
        entry: StepIdx,
    },
    StepOutOfBounds {
        step: StepIdx,
    },
    StepCountOverflow {
        actual: u64,
    },
    /// `u16` nesting depth overflowed at `compute_child_depth`; carries the
    /// actual pre-overflow depth so diagnostics can report the real value
    /// instead of the lossy `u64::MAX` sentinel previously produced here.
    DepthOverflow {
        depth: u16,
    },
    JumpCycle {
        step: StepIdx,
        target: StepIdx,
    },
    BackwardScopeExit {
        from: StepIdx,
        to: StepIdx,
    },
}

impl From<BudgetTraversalError> for WorkflowError {
    fn from(error: BudgetTraversalError) -> Self {
        match error {
            BudgetTraversalError::EntryOutOfBounds { entry } => Self::EntryOutOfBounds { entry },
            BudgetTraversalError::StepOutOfBounds { step } => Self::StepOutOfBounds { step },
            BudgetTraversalError::StepCountOverflow { actual } => {
                Self::StepCountOverflow { actual }
            }
            BudgetTraversalError::DepthOverflow { depth } => Self::DepthOverflow { depth },
            BudgetTraversalError::JumpCycle { step, target } => Self::JumpCycle { step, target },
            BudgetTraversalError::BackwardScopeExit { from, to } => Self::BackwardEdge { from, to },
        }
    }
}

fn compute_small_linear_budget(
    nodes: &[crate::workflow::CompiledNode],
    entry: StepIdx,
    contract: &ResourceContract,
) -> Result<Option<WholeWorkflowBudget>, BudgetTraversalError> {
    if nodes.len() > 2 || !small_linear_domain(nodes) {
        return Ok(None);
    }
    let metrics = small_linear_metrics(nodes, entry)?;
    let derived = derive_budget_dimensions(metrics.steps, metrics.actions, contract)?;
    Ok(Some(WholeWorkflowBudget {
        max_total_steps: metrics.steps,
        max_total_slots: u64::from(contract.max_slots),
        max_fanout: 0,
        max_nesting_depth: 0,
        max_steps_executable: WholeWorkflowBudget::MAX_STEPS_EXECUTABLE,
        max_action_tickets: metrics.actions,
        max_parallel_in_flight: 0,
        max_retries_per_action: contract.max_retry_attempts,
        max_gather_pages: 0,
        max_gather_items: 0,
        max_for_each_iterations: 0,
        max_together_branches: 0,
        max_repeat_attempts: 0,
        max_run_time_seconds: metrics.steps,
        max_result_bytes: contract.max_output_bytes,
        max_total_slots_written: u32::from(contract.max_slots),
        max_timer_entries: metrics.timers,
        max_trace_events: metrics.steps,
        max_journal_batch_bytes: contract.max_journal_batch_bytes,
        max_queue_depth: contract.max_queue_depth,
        max_ipc_payload_bytes: contract.max_ipc_payload_bytes,
        max_blob_bytes: contract.max_blob_bytes,
        max_input_bytes: contract.max_input_bytes,
        max_open_gates: metrics.gates,
        max_signal_subscriptions: metrics.signals,
        max_incident_emissions: derived.max_incident_emissions,
        max_transitions: derived.max_transitions,
        max_journal_bytes_total: derived.max_journal_bytes_total,
        max_value_store_entries: derived.max_value_store_entries,
        max_cost_units: derived.max_cost_units,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SmallLinearMetrics {
    steps: u64,
    actions: u32,
    timers: u32,
    gates: u32,
    signals: u32,
}

fn small_linear_domain(nodes: &[crate::workflow::CompiledNode]) -> bool {
    match nodes {
        [] => false,
        [first] => first.id == StepIdx::new(0) && small_linear_node(first, 1),
        [first, second] => {
            first.id == StepIdx::new(0)
                && second.id == StepIdx::new(1)
                && small_linear_node(first, 2)
                && small_linear_node(second, 2)
        }
        _ => false,
    }
}

fn small_linear_node(node: &crate::workflow::CompiledNode, node_count: usize) -> bool {
    small_linear_next(node.routing.next.step(), node_count)
        && small_linear_next(node.routing.error.handler(), node_count)
        && matches!(
            node.kind,
            CompiledNodeKind::Nop
                | CompiledNodeKind::Do { .. }
                | CompiledNodeKind::WaitUntil { .. }
                | CompiledNodeKind::WaitEvent { .. }
                | CompiledNodeKind::Ask { .. }
                | CompiledNodeKind::Finish { .. }
        )
}

fn small_linear_next(next: Option<StepIdx>, node_count: usize) -> bool {
    match next {
        Some(step) => step.as_usize() < node_count,
        None => true,
    }
}

fn small_linear_metrics(
    nodes: &[crate::workflow::CompiledNode],
    entry: StepIdx,
) -> Result<SmallLinearMetrics, BudgetTraversalError> {
    let first_idx = entry.as_usize();
    let first = node_at_position(nodes, first_idx, entry)?;
    let first_metrics = small_linear_node_metrics(first);
    match first.routing.next.step() {
        Some(next) if next.as_usize() != first_idx => {
            let second = node_at_position(nodes, next.as_usize(), next)?;
            Ok(first_metrics.add(small_linear_node_metrics(second)))
        }
        _ => Ok(first_metrics),
    }
}

fn small_linear_node_metrics(node: &crate::workflow::CompiledNode) -> SmallLinearMetrics {
    match node.kind {
        CompiledNodeKind::Do { .. } => SmallLinearMetrics {
            steps: 1,
            actions: 1,
            timers: 0,
            gates: 0,
            signals: 0,
        },
        CompiledNodeKind::WaitEvent { .. } => SmallLinearMetrics {
            steps: 1,
            actions: 0,
            timers: 1,
            gates: 0,
            signals: 1,
        },
        CompiledNodeKind::Ask { .. } => SmallLinearMetrics {
            steps: 1,
            actions: 0,
            timers: 1,
            gates: 1,
            signals: 0,
        },
        CompiledNodeKind::WaitUntil { .. } => SmallLinearMetrics {
            steps: 1,
            actions: 0,
            timers: 1,
            gates: 0,
            signals: 0,
        },
        _ => SmallLinearMetrics {
            steps: 1,
            actions: 0,
            timers: 0,
            gates: 0,
            signals: 0,
        },
    }
}

impl SmallLinearMetrics {
    const fn add(self, other: Self) -> Self {
        Self {
            steps: self.steps.saturating_add(other.steps),
            actions: self.actions.saturating_add(other.actions),
            timers: self.timers.saturating_add(other.timers),
            gates: self.gates.saturating_add(other.gates),
            signals: self.signals.saturating_add(other.signals),
        }
    }
}

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

/// Budget validation failures.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BudgetError {
    #[error("total steps exceeded: {actual} > {limit}")]
    TotalStepsExceeded { actual: u64, limit: u64 },
    #[error("total slots exceeded: {actual} > {limit}")]
    TotalSlotsExceeded { actual: u64, limit: u64 },
    #[error("fanout exceeded: {actual} > {limit}")]
    FanoutExceeded { actual: u16, limit: u16 },
    #[error("nesting depth exceeded: {actual} > {limit}")]
    NestingDepthExceeded { actual: u16, limit: u16 },
    #[error("parallel exceeded: {actual} > {limit}")]
    ParallelExceeded { actual: u16, limit: u16 },
    #[error("action tickets exceeded: {actual} > {limit}")]
    ActionTicketsExceeded { actual: u32, limit: u32 },
    #[error("run time exceeded: {actual} > {limit}")]
    RunTimeExceeded { actual: u64, limit: u64 },
    #[error("result bytes exceeded: {actual} > {limit}")]
    ResultBytesExceeded { actual: u32, limit: u32 },
    #[error("steps executable exceeded: {actual} > {limit}")]
    StepsExecutableExceeded { actual: u32, limit: u32 },
    #[error("timer entries exceeded: {actual} > {limit}")]
    TimerEntriesExceeded { actual: u32, limit: u32 },
    #[error("trace events exceeded: {actual} > {limit}")]
    TraceEventsExceeded { actual: u64, limit: u64 },
    #[error("journal batch bytes exceeded: {actual} > {limit}")]
    JournalBatchBytesExceeded { actual: u32, limit: u32 },
    #[error("queue depth exceeded: {actual} > {limit}")]
    QueueDepthExceeded { actual: u32, limit: u32 },
    #[error("ipc payload bytes exceeded: {actual} > {limit}")]
    IpcPayloadBytesExceeded { actual: u32, limit: u32 },
    #[error("blob bytes exceeded: {actual} > {limit}")]
    BlobBytesExceeded { actual: u64, limit: u64 },
    #[error("input bytes exceeded: {actual} > {limit}")]
    InputBytesExceeded { actual: u32, limit: u32 },
    #[error("open gates exceeded: {actual} > {limit}")]
    OpenGatesExceeded { actual: u32, limit: u32 },
    #[error("signal subscriptions exceeded: {actual} > {limit}")]
    SignalSubscriptionsExceeded { actual: u32, limit: u32 },
    #[error("incident emissions exceeded: {actual} > {limit}")]
    IncidentEmissionsExceeded { actual: u64, limit: u64 },
    #[error("transitions exceeded: {actual} > {limit}")]
    TransitionsExceeded { actual: u64, limit: u64 },
    #[error("cumulative journal bytes exceeded: {actual} > {limit}")]
    JournalBytesTotalExceeded { actual: u64, limit: u64 },
    #[error("value-store entries exceeded: {actual} > {limit}")]
    ValueStoreEntriesExceeded { actual: u64, limit: u64 },
    #[error("cost units exceeded: {actual} > {limit}")]
    CostUnitsExceeded { actual: u64, limit: u64 },
}

impl BudgetError {
    /// Classifies which budget dimension failed its policy bound.
    ///
    /// The mapping is exhaustive: adding a variant here fails compilation
    /// until its dimension is classified.
    #[must_use]
    pub const fn dimension(&self) -> BudgetDimension {
        match self {
            Self::TotalStepsExceeded { .. } => BudgetDimension::TotalSteps,
            Self::TotalSlotsExceeded { .. } => BudgetDimension::TotalSlots,
            Self::FanoutExceeded { .. } => BudgetDimension::Fanout,
            Self::NestingDepthExceeded { .. } => BudgetDimension::NestingDepth,
            Self::ParallelExceeded { .. } => BudgetDimension::Parallel,
            Self::ActionTicketsExceeded { .. } => BudgetDimension::ActionTickets,
            Self::RunTimeExceeded { .. } => BudgetDimension::RunTime,
            Self::ResultBytesExceeded { .. } => BudgetDimension::ResultBytes,
            Self::StepsExecutableExceeded { .. } => BudgetDimension::StepsExecutable,
            Self::TimerEntriesExceeded { .. } => BudgetDimension::TimerEntries,
            Self::TraceEventsExceeded { .. } => BudgetDimension::TraceEvents,
            Self::JournalBatchBytesExceeded { .. } => BudgetDimension::JournalBatchBytes,
            Self::QueueDepthExceeded { .. } => BudgetDimension::QueueDepth,
            Self::IpcPayloadBytesExceeded { .. } => BudgetDimension::IpcPayloadBytes,
            Self::BlobBytesExceeded { .. } => BudgetDimension::BlobBytes,
            Self::InputBytesExceeded { .. } => BudgetDimension::InputBytes,
            Self::OpenGatesExceeded { .. } => BudgetDimension::OpenGates,
            Self::SignalSubscriptionsExceeded { .. } => BudgetDimension::SignalSubscriptions,
            Self::IncidentEmissionsExceeded { .. } => BudgetDimension::IncidentEmissions,
            Self::TransitionsExceeded { .. } => BudgetDimension::Transitions,
            Self::JournalBytesTotalExceeded { .. } => BudgetDimension::JournalBytesTotal,
            Self::ValueStoreEntriesExceeded { .. } => BudgetDimension::ValueStoreEntries,
            Self::CostUnitsExceeded { .. } => BudgetDimension::CostUnits,
        }
    }
}

/// Budget dimension that exceeded its policy bound.
///
/// Canonical classification backing `WorkflowError::BudgetPolicyExceeded`.
/// The canonical names are frozen: diagnostics and admission mapping
/// match on them, so renaming a string is a behavior change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetDimension {
    /// Total step count bound.
    TotalSteps,
    /// Total slot count bound.
    TotalSlots,
    /// Branch fanout bound.
    Fanout,
    /// Structural nesting bound.
    NestingDepth,
    /// Parallel in-flight bound.
    Parallel,
    /// Action ticket bound.
    ActionTickets,
    /// Run time bound.
    RunTime,
    /// Result byte bound.
    ResultBytes,
    /// Executable step bound.
    StepsExecutable,
    /// Timer entry bound.
    TimerEntries,
    /// Trace event bound.
    TraceEvents,
    /// Journal batch byte bound.
    JournalBatchBytes,
    /// Queue depth bound.
    QueueDepth,
    /// IPC payload byte bound.
    IpcPayloadBytes,
    /// Blob byte bound.
    BlobBytes,
    /// Input byte bound.
    InputBytes,
    /// Open gate bound.
    OpenGates,
    /// Signal subscription bound.
    SignalSubscriptions,
    /// Incident emission bound.
    IncidentEmissions,
    /// Transition count bound.
    Transitions,
    /// Cumulative journal byte bound.
    JournalBytesTotal,
    /// Value-store entry bound.
    ValueStoreEntries,
    /// Cost unit bound.
    CostUnits,
}

impl BudgetDimension {
    /// Canonical dimension name. Frozen: diagnostics match on these strings.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TotalSteps => "max_total_steps",
            Self::TotalSlots => "max_total_slots",
            Self::Fanout => "max_fanout",
            Self::NestingDepth => "max_nesting_depth",
            Self::Parallel => "max_parallel_in_flight",
            Self::ActionTickets => "max_action_tickets",
            Self::RunTime => "max_run_time_seconds",
            Self::ResultBytes => "max_result_bytes",
            Self::StepsExecutable => "max_steps_executable",
            Self::TimerEntries => "max_timer_entries",
            Self::TraceEvents => "max_trace_events",
            Self::JournalBatchBytes => "max_journal_batch_bytes",
            Self::QueueDepth => "max_queue_depth",
            Self::IpcPayloadBytes => "max_ipc_payload_bytes",
            Self::BlobBytes => "max_blob_bytes",
            Self::InputBytes => "max_input_bytes",
            Self::OpenGates => "max_open_gates",
            Self::SignalSubscriptions => "max_signal_subscriptions",
            Self::IncidentEmissions => "max_incident_emissions",
            Self::Transitions => "max_transitions",
            Self::JournalBytesTotal => "max_journal_bytes_total",
            Self::ValueStoreEntries => "max_value_store_entries",
            Self::CostUnits => "max_cost_units",
        }
    }
}

impl core::fmt::Display for BudgetDimension {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Aggregate whole-run budget required for runtime admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AggregateResourceBudget {
    pub max_steps_executable: u32,
    pub max_action_tickets: u32,
    pub max_parallel_in_flight: u16,
    pub max_retries_per_action: u16,
    pub max_gather_pages: u32,
    pub max_gather_items: u32,
    pub max_for_each_iterations: u32,
    pub max_together_branches: u16,
    pub max_repeat_attempts: u16,
    pub max_run_time_seconds: u64,
    pub max_result_bytes: u32,
    pub max_total_slots_written: u32,
    pub max_timer_entries: u32,
    pub max_trace_events: u64,
    pub max_queue_depth: u32,
    pub max_journal_batch_bytes: u32,
    pub max_ipc_payload_bytes: u32,
    pub max_blob_bytes: u64,
    pub max_input_bytes: u32,
    /// Maximum step budget per runtime tick (from ResourceContract).
    pub max_step_budget_per_tick: u64,
    /// Maximum transitions per runtime tick.
    pub max_transitions_per_tick: u64,
    /// Maximum simultaneously open gates.
    pub max_open_gates: u32,
    /// Maximum signal subscriptions.
    pub max_signal_subscriptions: u32,
    /// Maximum incident emissions over the run.
    pub max_incident_emissions: u64,
    /// Maximum total transitions over the run.
    pub max_transitions: u64,
    /// Maximum cumulative journal bytes over the run.
    pub max_journal_bytes_total: u64,
    /// Maximum value-store entries over the run.
    pub max_value_store_entries: u64,
    /// Maximum cost units over the run.
    pub max_cost_units: u64,
}

/// Shard-local aggregate admission capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateResourceCapacity {
    pub max_steps_executable: u64,
    pub max_action_tickets: u64,
    pub max_parallel_in_flight: u32,
    pub max_gather_pages: u64,
    pub max_gather_items: u64,
    pub max_result_bytes: u64,
    pub max_total_slots_written: u64,
    pub max_timer_entries: u64,
    pub max_trace_events: u64,
    pub max_active_runs: u64,
    pub max_queue_depth: u64,
    pub max_journal_batch_bytes: u64,
    pub max_ipc_payload_bytes: u64,
    pub max_blob_bytes: u64,
    pub max_input_bytes: u64,
    /// Maximum step budget per tick capacity.
    pub max_step_budget_per_tick: u64,
    /// Maximum transitions per tick capacity.
    pub max_transitions_per_tick: u64,
    /// Maximum open gates capacity.
    pub max_open_gates: u64,
    /// Maximum signal subscriptions capacity.
    pub max_signal_subscriptions: u64,
    /// Maximum incident emissions capacity.
    pub max_incident_emissions: u64,
    /// Maximum total transitions capacity.
    pub max_transitions: u64,
    /// Maximum cumulative journal bytes capacity.
    pub max_journal_bytes_total: u64,
    /// Maximum value-store entries capacity.
    pub max_value_store_entries: u64,
    /// Maximum cost units capacity.
    pub max_cost_units: u64,
    /// Maximum aggregate cost units attributable to a single tenant.
    /// Enforced by [`check_tenant_quota`] at tenant-scoped admission;
    /// per-tenant usage is recorded in [`ShardUsageSnapshot`].
    pub tenant_quota_cost_units: u64,
    /// Maximum aggregate cost units shard-wide. Distinct from
    /// `max_cost_units` capacity: both cap summed cost, and both must hold.
    pub global_quota_cost_units: u64,
}

/// Active shard aggregate usage snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AggregateResourceUsage {
    pub max_steps_executable: u64,
    pub max_action_tickets: u64,
    pub max_parallel_in_flight: u64,
    pub max_gather_pages: u64,
    pub max_gather_items: u64,
    pub max_result_bytes: u64,
    pub max_total_slots_written: u64,
    pub max_timer_entries: u64,
    pub max_trace_events: u64,
    pub max_active_runs: u64,
    pub max_queue_depth: u64,
    pub max_journal_batch_bytes: u64,
    pub max_ipc_payload_bytes: u64,
    pub max_blob_bytes: u64,
    pub max_input_bytes: u64,
    /// Current step budget per tick usage.
    pub max_step_budget_per_tick: u64,
    /// Current transitions per tick usage.
    pub max_transitions_per_tick: u64,
    /// Current open gates usage.
    pub max_open_gates: u64,
    /// Current signal subscriptions usage.
    pub max_signal_subscriptions: u64,
    /// Current incident emissions usage.
    pub max_incident_emissions: u64,
    /// Current total transitions usage.
    pub max_transitions: u64,
    /// Current cumulative journal bytes usage.
    pub max_journal_bytes_total: u64,
    /// Current value-store entries usage.
    pub max_value_store_entries: u64,
    /// Current cost units usage.
    pub max_cost_units: u64,
}

/// Shard usage snapshot: aggregate usage plus per-tenant cost usage.
///
/// The aggregate field mirrors [`AggregateResourceUsage`]; the tenant
/// entries record admitted cost per caller-supplied [`TenantId`], so
/// tenant-scoped admission sums from this snapshot instead of trusting a
/// caller-supplied sum. The tenant identity is opaque: this type assigns
/// it no meaning and performs no authentication.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShardUsageSnapshot {
    /// Active shard aggregate usage.
    pub aggregate: AggregateResourceUsage,
    /// Admitted cost units per tenant, at most one entry per tenant.
    tenant_costs: Box<[(TenantId, u64)]>,
}

impl ShardUsageSnapshot {
    /// Sums the recorded cost for one tenant across the snapshot.
    ///
    /// A tenant with no entry has zero recorded cost.
    pub fn tenant_cost_usage(&self, tenant: TenantId) -> Result<u64, AggregateBudgetError> {
        let mut total = 0u64;
        for (entry_tenant, cost) in self.tenant_costs.iter() {
            if *entry_tenant == tenant {
                total = total
                    .checked_add(*cost)
                    .ok_or(AggregateBudgetError::Overflow {
                        resource: "tenant_quota_cost_units",
                    })?;
            }
        }
        Ok(total)
    }

    /// Returns the recorded per-tenant cost entries.
    #[must_use]
    pub fn tenant_entries(&self) -> &[(TenantId, u64)] {
        &self.tenant_costs
    }

    /// Records admitted cost for a tenant, merging into its entry.
    pub fn record_tenant_cost(
        &mut self,
        tenant: TenantId,
        cost: u64,
    ) -> Result<(), AggregateBudgetError> {
        let mut entries: Vec<(TenantId, u64)> = self.tenant_costs.to_vec();
        for entry in entries.iter_mut() {
            if entry.0 == tenant {
                entry.1 = entry
                    .1
                    .checked_add(cost)
                    .ok_or(AggregateBudgetError::Overflow {
                        resource: "tenant_quota_cost_units",
                    })?;
                self.tenant_costs = entries.into_boxed_slice();
                return Ok(());
            }
        }
        entries.push((tenant, cost));
        self.tenant_costs = entries.into_boxed_slice();
        Ok(())
    }
}

/// Exact budget reservation associated with a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateReservation {
    pub run: RunId,
    pub requested: AggregateResourceBudget,
}

/// Aggregate resource-accounting failure.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AggregateBudgetError {
    /// Workflow budget validation failed.
    #[cfg(not(kani))]
    #[error("workflow budget error: {0}")]
    WorkflowBudget(#[source] WorkflowError),
    /// Workflow budget validation failed (Kani stub).
    #[cfg(kani)]
    #[error("workflow budget error")]
    WorkflowBudget,
    /// A policy-defined resource limit was exceeded.
    #[error("policy exceeded: {resource} {actual} > {limit}")]
    PolicyExceeded {
        /// Resource name.
        resource: &'static str,
        /// Actual value.
        actual: u64,
        /// Policy limit.
        limit: u64,
    },
    /// Requested capacity exceeds available.
    #[error("capacity exceeded: {resource} requested {requested}, available {available}")]
    CapacityExceeded {
        /// Resource name.
        resource: &'static str,
        /// Requested amount.
        requested: u64,
        /// Available amount.
        available: u64,
    },
    /// Arithmetic overflow.
    #[error("overflow: {resource}")]
    Overflow {
        /// Resource name.
        resource: &'static str,
    },
    /// Arithmetic underflow.
    #[error("underflow: {resource}")]
    Underflow {
        /// Resource name.
        resource: &'static str,
    },
    /// Invalid capacity configuration.
    #[error("invalid capacity: {resource}")]
    InvalidCapacity {
        /// Resource name.
        resource: &'static str,
    },
    /// Reservation not found.
    #[error("reservation not found: run {run:?}")]
    ReservationNotFound {
        /// Run identifier.
        run: RunId,
    },
    /// Step ceiling exceeded per tick.
    #[error("step ceiling exceeded: {requested} > {limit}")]
    StepCeilingExceeded {
        /// Requested steps.
        requested: u64,
        /// Tick limit.
        limit: u64,
    },
    /// Per-tick transition ceiling exceeded.
    #[error("per-tick ceiling exceeded: {requested} > {limit}")]
    PerTickCeilingExceeded {
        /// Requested transitions.
        requested: u64,
        /// Tick limit.
        limit: u64,
    },
}

#[cfg(kani)]
impl Drop for AggregateBudgetError {
    fn drop(&mut self) {}
}

impl AggregateResourceBudget {
    pub fn from_workflow(workflow: &CompiledWorkflow) -> Result<Self, AggregateBudgetError> {
        let parts = workflow.to_parts();
        let budget = WholeWorkflowBudget::compute(
            &parts.nodes,
            workflow.entry(),
            &workflow.resource_contract(),
        )
        .map_err(map_workflow_budget_error)?;
        let aggregate = Self::from_whole_workflow_budget(budget, workflow.resource_contract())?;
        validate_step_ceilings(&aggregate)?;
        Ok(aggregate)
    }

    pub fn from_whole_workflow_budget(
        budget: WholeWorkflowBudget,
        contract: ResourceContract,
    ) -> Result<Self, AggregateBudgetError> {
        Ok(Self {
            max_steps_executable: budget.max_steps_executable,
            max_action_tickets: budget.max_action_tickets,
            max_parallel_in_flight: budget.max_parallel_in_flight,
            max_retries_per_action: budget.max_retries_per_action,
            max_gather_pages: budget.max_gather_pages,
            max_gather_items: budget.max_gather_items,
            max_for_each_iterations: budget.max_for_each_iterations,
            max_together_branches: budget.max_together_branches,
            max_repeat_attempts: budget.max_repeat_attempts,
            max_run_time_seconds: budget.max_run_time_seconds,
            max_result_bytes: budget.max_result_bytes,
            max_total_slots_written: budget.max_total_slots_written,
            max_timer_entries: budget.max_timer_entries,
            max_trace_events: budget.max_trace_events,
            max_queue_depth: budget.max_queue_depth,
            max_journal_batch_bytes: budget.max_journal_batch_bytes,
            max_ipc_payload_bytes: budget.max_ipc_payload_bytes,
            max_blob_bytes: budget.max_blob_bytes,
            max_input_bytes: budget.max_input_bytes,
            max_step_budget_per_tick: contract.max_step_budget_per_tick,
            max_transitions_per_tick: contract.max_transitions_per_tick,
            max_open_gates: budget.max_open_gates,
            max_signal_subscriptions: budget.max_signal_subscriptions,
            max_incident_emissions: budget.max_incident_emissions,
            max_transitions: budget.max_transitions,
            max_journal_bytes_total: budget.max_journal_bytes_total,
            max_value_store_entries: budget.max_value_store_entries,
            max_cost_units: budget.max_cost_units,
        })
    }
}

#[cfg(not(kani))]
fn map_workflow_budget_error(error: WorkflowError) -> AggregateBudgetError {
    AggregateBudgetError::WorkflowBudget(error)
}

#[cfg(kani)]
fn map_workflow_budget_error(_error: WorkflowError) -> AggregateBudgetError {
    AggregateBudgetError::WorkflowBudget
}

impl AggregateResourceUsage {
    pub fn try_add_budget(
        &self,
        budget: &AggregateResourceBudget,
    ) -> Result<Self, AggregateBudgetError> {
        Ok(Self {
            max_steps_executable: add_dim(
                self.max_steps_executable,
                u64::from(budget.max_steps_executable),
                "max_steps_executable",
            )?,
            max_action_tickets: add_dim(
                self.max_action_tickets,
                u64::from(budget.max_action_tickets),
                "max_action_tickets",
            )?,
            max_parallel_in_flight: add_dim(
                self.max_parallel_in_flight,
                u64::from(budget.max_parallel_in_flight),
                "max_parallel_in_flight",
            )?,
            max_gather_pages: add_dim(
                self.max_gather_pages,
                u64::from(budget.max_gather_pages),
                "max_gather_pages",
            )?,
            max_gather_items: add_dim(
                self.max_gather_items,
                u64::from(budget.max_gather_items),
                "max_gather_items",
            )?,
            max_result_bytes: add_dim(
                self.max_result_bytes,
                u64::from(budget.max_result_bytes),
                "max_result_bytes",
            )?,
            max_total_slots_written: add_dim(
                self.max_total_slots_written,
                u64::from(budget.max_total_slots_written),
                "max_total_slots_written",
            )?,
            max_timer_entries: add_dim(
                self.max_timer_entries,
                u64::from(budget.max_timer_entries),
                "max_timer_entries",
            )?,
            max_trace_events: add_dim(
                self.max_trace_events,
                budget.max_trace_events,
                "max_trace_events",
            )?,
            max_active_runs: add_dim(self.max_active_runs, 1, "max_active_runs")?,
            max_queue_depth: add_dim(
                self.max_queue_depth,
                u64::from(budget.max_queue_depth),
                "max_queue_depth",
            )?,
            max_journal_batch_bytes: add_dim(
                self.max_journal_batch_bytes,
                u64::from(budget.max_journal_batch_bytes),
                "max_journal_batch_bytes",
            )?,
            max_ipc_payload_bytes: add_dim(
                self.max_ipc_payload_bytes,
                u64::from(budget.max_ipc_payload_bytes),
                "max_ipc_payload_bytes",
            )?,
            max_blob_bytes: add_dim(self.max_blob_bytes, budget.max_blob_bytes, "max_blob_bytes")?,
            max_input_bytes: add_dim(
                self.max_input_bytes,
                u64::from(budget.max_input_bytes),
                "max_input_bytes",
            )?,
            max_step_budget_per_tick: add_dim(
                self.max_step_budget_per_tick,
                budget.max_step_budget_per_tick,
                "max_step_budget_per_tick",
            )?,
            max_transitions_per_tick: add_dim(
                self.max_transitions_per_tick,
                budget.max_transitions_per_tick,
                "max_transitions_per_tick",
            )?,
            max_open_gates: add_dim(
                self.max_open_gates,
                u64::from(budget.max_open_gates),
                "max_open_gates",
            )?,
            max_signal_subscriptions: add_dim(
                self.max_signal_subscriptions,
                u64::from(budget.max_signal_subscriptions),
                "max_signal_subscriptions",
            )?,
            max_incident_emissions: add_dim(
                self.max_incident_emissions,
                budget.max_incident_emissions,
                "max_incident_emissions",
            )?,
            max_transitions: add_dim(
                self.max_transitions,
                budget.max_transitions,
                "max_transitions",
            )?,
            max_journal_bytes_total: add_dim(
                self.max_journal_bytes_total,
                budget.max_journal_bytes_total,
                "max_journal_bytes_total",
            )?,
            max_value_store_entries: add_dim(
                self.max_value_store_entries,
                budget.max_value_store_entries,
                "max_value_store_entries",
            )?,
            max_cost_units: add_dim(self.max_cost_units, budget.max_cost_units, "max_cost_units")?,
        })
    }

    pub fn try_subtract_budget(
        &self,
        budget: &AggregateResourceBudget,
    ) -> Result<Self, AggregateBudgetError> {
        Ok(Self {
            max_steps_executable: sub_dim(
                self.max_steps_executable,
                u64::from(budget.max_steps_executable),
                "max_steps_executable",
            )?,
            max_action_tickets: sub_dim(
                self.max_action_tickets,
                u64::from(budget.max_action_tickets),
                "max_action_tickets",
            )?,
            max_parallel_in_flight: sub_dim(
                self.max_parallel_in_flight,
                u64::from(budget.max_parallel_in_flight),
                "max_parallel_in_flight",
            )?,
            max_gather_pages: sub_dim(
                self.max_gather_pages,
                u64::from(budget.max_gather_pages),
                "max_gather_pages",
            )?,
            max_gather_items: sub_dim(
                self.max_gather_items,
                u64::from(budget.max_gather_items),
                "max_gather_items",
            )?,
            max_result_bytes: sub_dim(
                self.max_result_bytes,
                u64::from(budget.max_result_bytes),
                "max_result_bytes",
            )?,
            max_total_slots_written: sub_dim(
                self.max_total_slots_written,
                u64::from(budget.max_total_slots_written),
                "max_total_slots_written",
            )?,
            max_timer_entries: sub_dim(
                self.max_timer_entries,
                u64::from(budget.max_timer_entries),
                "max_timer_entries",
            )?,
            max_trace_events: sub_dim(
                self.max_trace_events,
                budget.max_trace_events,
                "max_trace_events",
            )?,
            max_active_runs: sub_dim(self.max_active_runs, 1, "max_active_runs")?,
            max_queue_depth: sub_dim(
                self.max_queue_depth,
                u64::from(budget.max_queue_depth),
                "max_queue_depth",
            )?,
            max_journal_batch_bytes: sub_dim(
                self.max_journal_batch_bytes,
                u64::from(budget.max_journal_batch_bytes),
                "max_journal_batch_bytes",
            )?,
            max_ipc_payload_bytes: sub_dim(
                self.max_ipc_payload_bytes,
                u64::from(budget.max_ipc_payload_bytes),
                "max_ipc_payload_bytes",
            )?,
            max_blob_bytes: sub_dim(self.max_blob_bytes, budget.max_blob_bytes, "max_blob_bytes")?,
            max_input_bytes: sub_dim(
                self.max_input_bytes,
                u64::from(budget.max_input_bytes),
                "max_input_bytes",
            )?,
            max_step_budget_per_tick: sub_dim(
                self.max_step_budget_per_tick,
                budget.max_step_budget_per_tick,
                "max_step_budget_per_tick",
            )?,
            max_transitions_per_tick: sub_dim(
                self.max_transitions_per_tick,
                budget.max_transitions_per_tick,
                "max_transitions_per_tick",
            )?,
            max_open_gates: sub_dim(
                self.max_open_gates,
                u64::from(budget.max_open_gates),
                "max_open_gates",
            )?,
            max_signal_subscriptions: sub_dim(
                self.max_signal_subscriptions,
                u64::from(budget.max_signal_subscriptions),
                "max_signal_subscriptions",
            )?,
            max_incident_emissions: sub_dim(
                self.max_incident_emissions,
                budget.max_incident_emissions,
                "max_incident_emissions",
            )?,
            max_transitions: sub_dim(
                self.max_transitions,
                budget.max_transitions,
                "max_transitions",
            )?,
            max_journal_bytes_total: sub_dim(
                self.max_journal_bytes_total,
                budget.max_journal_bytes_total,
                "max_journal_bytes_total",
            )?,
            max_value_store_entries: sub_dim(
                self.max_value_store_entries,
                budget.max_value_store_entries,
                "max_value_store_entries",
            )?,
            max_cost_units: sub_dim(self.max_cost_units, budget.max_cost_units, "max_cost_units")?,
        })
    }

    pub fn fits_within(
        &self,
        capacity: &AggregateResourceCapacity,
    ) -> Result<(), AggregateBudgetError> {
        check_capacity(
            "max_steps_executable",
            self.max_steps_executable,
            capacity.max_steps_executable,
        )?;
        check_capacity(
            "max_action_tickets",
            self.max_action_tickets,
            capacity.max_action_tickets,
        )?;
        check_capacity(
            "max_parallel_in_flight",
            self.max_parallel_in_flight,
            u64::from(capacity.max_parallel_in_flight),
        )?;
        check_capacity(
            "max_gather_pages",
            self.max_gather_pages,
            capacity.max_gather_pages,
        )?;
        check_capacity(
            "max_gather_items",
            self.max_gather_items,
            capacity.max_gather_items,
        )?;
        check_capacity(
            "max_result_bytes",
            self.max_result_bytes,
            capacity.max_result_bytes,
        )?;
        check_capacity(
            "max_total_slots_written",
            self.max_total_slots_written,
            capacity.max_total_slots_written,
        )?;
        check_capacity(
            "max_timer_entries",
            self.max_timer_entries,
            capacity.max_timer_entries,
        )?;
        check_capacity(
            "max_trace_events",
            self.max_trace_events,
            capacity.max_trace_events,
        )?;
        check_capacity(
            "max_active_runs",
            self.max_active_runs,
            capacity.max_active_runs,
        )?;
        check_capacity(
            "max_queue_depth",
            self.max_queue_depth,
            capacity.max_queue_depth,
        )?;
        check_capacity(
            "max_journal_batch_bytes",
            self.max_journal_batch_bytes,
            capacity.max_journal_batch_bytes,
        )?;
        check_capacity(
            "max_ipc_payload_bytes",
            self.max_ipc_payload_bytes,
            capacity.max_ipc_payload_bytes,
        )?;
        check_capacity(
            "max_blob_bytes",
            self.max_blob_bytes,
            capacity.max_blob_bytes,
        )?;
        check_capacity(
            "max_input_bytes",
            self.max_input_bytes,
            capacity.max_input_bytes,
        )?;
        check_capacity(
            "max_step_budget_per_tick",
            self.max_step_budget_per_tick,
            capacity.max_step_budget_per_tick,
        )?;
        check_capacity(
            "max_transitions_per_tick",
            self.max_transitions_per_tick,
            capacity.max_transitions_per_tick,
        )?;
        check_capacity(
            "max_open_gates",
            self.max_open_gates,
            capacity.max_open_gates,
        )?;
        check_capacity(
            "max_signal_subscriptions",
            self.max_signal_subscriptions,
            capacity.max_signal_subscriptions,
        )?;
        check_capacity(
            "max_incident_emissions",
            self.max_incident_emissions,
            capacity.max_incident_emissions,
        )?;
        check_capacity(
            "max_transitions",
            self.max_transitions,
            capacity.max_transitions,
        )?;
        check_capacity(
            "max_journal_bytes_total",
            self.max_journal_bytes_total,
            capacity.max_journal_bytes_total,
        )?;
        check_capacity(
            "max_value_store_entries",
            self.max_value_store_entries,
            capacity.max_value_store_entries,
        )?;
        check_capacity(
            "max_cost_units",
            self.max_cost_units,
            capacity.max_cost_units,
        )?;
        check_capacity(
            "global_quota_cost_units",
            self.max_cost_units,
            capacity.global_quota_cost_units,
        )
    }

    /// Checks if this usage satisfies a boundedness policy.
    /// Returns `Ok(())` if all usage dimensions are within policy limits,
    /// or `Err(AggregateBudgetError::PolicyExceeded)` if any dimension exceeds.
    pub fn check_policy(&self, policy: &BoundednessPolicy) -> Result<(), AggregateBudgetError> {
        check_policy(
            "max_steps_executable",
            self.max_steps_executable,
            u64::from(policy.absolute_max_steps_executable),
        )?;
        check_policy(
            "max_action_tickets",
            self.max_action_tickets,
            u64::from(policy.absolute_max_action_tickets),
        )?;
        check_policy(
            "max_parallel_in_flight",
            self.max_parallel_in_flight,
            u64::from(policy.absolute_max_parallel),
        )?;
        check_policy(
            "max_result_bytes",
            self.max_result_bytes,
            u64::from(policy.absolute_max_result_bytes),
        )?;
        check_policy(
            "max_timer_entries",
            self.max_timer_entries,
            u64::from(policy.absolute_max_timer_entries),
        )?;
        check_policy(
            "max_trace_events",
            self.max_trace_events,
            policy.absolute_max_trace_events,
        )?;
        check_policy(
            "max_journal_batch_bytes",
            self.max_journal_batch_bytes,
            u64::from(policy.absolute_max_journal_batch_bytes),
        )?;
        check_policy(
            "max_queue_depth",
            self.max_queue_depth,
            u64::from(policy.absolute_max_queue_depth),
        )?;
        check_policy(
            "max_ipc_payload_bytes",
            self.max_ipc_payload_bytes,
            u64::from(policy.absolute_max_ipc_payload_bytes),
        )?;
        check_policy(
            "max_blob_bytes",
            self.max_blob_bytes,
            policy.absolute_max_blob_bytes,
        )?;
        check_policy(
            "max_input_bytes",
            self.max_input_bytes,
            u64::from(policy.absolute_max_input_bytes),
        )?;
        check_policy(
            "max_open_gates",
            self.max_open_gates,
            u64::from(policy.absolute_max_open_gates),
        )?;
        check_policy(
            "max_signal_subscriptions",
            self.max_signal_subscriptions,
            u64::from(policy.absolute_max_signal_subscriptions),
        )?;
        check_policy(
            "max_incident_emissions",
            self.max_incident_emissions,
            policy.absolute_max_incident_emissions,
        )?;
        check_policy(
            "max_transitions",
            self.max_transitions,
            policy.absolute_max_transitions,
        )?;
        check_policy(
            "max_journal_bytes_total",
            self.max_journal_bytes_total,
            policy.absolute_max_journal_bytes_total,
        )?;
        check_policy(
            "max_value_store_entries",
            self.max_value_store_entries,
            policy.absolute_max_value_store_entries,
        )?;
        check_policy(
            "max_cost_units",
            self.max_cost_units,
            policy.absolute_max_cost_units,
        )
    }
}

/// Checks a tenant-scoped cost quota: the tenant's already-admitted cost
/// plus the requested run's cost must not exceed the tenant quota.
/// This is the explicit-sum path: callers that hold a [`ShardUsageSnapshot`]
/// should sum from it with [`ShardUsageSnapshot::tenant_cost_usage`]
/// instead of passing a sum computed elsewhere.
pub fn check_tenant_quota(
    tenant_cost_usage: u64,
    requested_cost: u64,
    tenant_quota: u64,
) -> Result<(), AggregateBudgetError> {
    let total =
        tenant_cost_usage
            .checked_add(requested_cost)
            .ok_or(AggregateBudgetError::Overflow {
                resource: "tenant_quota_cost_units",
            })?;
    check_capacity("tenant_quota_cost_units", total, tenant_quota)
}

/// Checks a shard-wide cost quota: the shard's already-admitted cost
/// plus the requested run's cost must not exceed the global quota.
/// The shard usage snapshot carries no summed global cost, so
/// global-scoped admission passes the shard's summed cost explicitly.
pub fn check_global_quota(
    global_cost_usage: u64,
    requested_cost: u64,
    global_quota: u64,
) -> Result<(), AggregateBudgetError> {
    let total =
        global_cost_usage
            .checked_add(requested_cost)
            .ok_or(AggregateBudgetError::Overflow {
                resource: "global_quota_cost_units",
            })?;
    check_capacity("global_quota_cost_units", total, global_quota)
}

pub fn validate_aggregate_budget(
    budget: &AggregateResourceBudget,
    policy: &BoundednessPolicy,
) -> Result<(), AggregateBudgetError> {
    check_policy(
        "max_steps_executable",
        u64::from(budget.max_steps_executable),
        u64::from(policy.absolute_max_steps_executable),
    )?;
    check_policy(
        "max_action_tickets",
        u64::from(budget.max_action_tickets),
        u64::from(policy.absolute_max_action_tickets),
    )?;
    check_policy(
        "max_parallel_in_flight",
        u64::from(budget.max_parallel_in_flight),
        u64::from(policy.absolute_max_parallel),
    )?;
    check_policy(
        "max_retries_per_action",
        u64::from(budget.max_retries_per_action),
        u64::from(u16::MAX),
    )?;
    check_policy(
        "max_gather_pages",
        u64::from(budget.max_gather_pages),
        u64::from(u32::MAX),
    )?;
    check_policy(
        "max_gather_items",
        u64::from(budget.max_gather_items),
        u64::from(u32::MAX),
    )?;
    check_policy(
        "max_for_each_iterations",
        u64::from(budget.max_for_each_iterations),
        u64::from(u32::MAX),
    )?;
    check_policy(
        "max_together_branches",
        u64::from(budget.max_together_branches),
        u64::from(policy.max_fanout),
    )?;
    check_policy(
        "max_repeat_attempts",
        u64::from(budget.max_repeat_attempts),
        u64::from(u16::MAX),
    )?;
    check_policy(
        "max_run_time_seconds",
        budget.max_run_time_seconds,
        policy.absolute_max_run_time_seconds,
    )?;
    check_policy(
        "max_result_bytes",
        u64::from(budget.max_result_bytes),
        u64::from(policy.absolute_max_result_bytes),
    )?;
    check_policy(
        "max_total_slots_written",
        u64::from(budget.max_total_slots_written),
        policy.max_total_slots,
    )?;
    check_policy(
        "max_timer_entries",
        u64::from(budget.max_timer_entries),
        u64::from(policy.absolute_max_timer_entries),
    )?;
    check_policy(
        "max_trace_events",
        budget.max_trace_events,
        policy.absolute_max_trace_events,
    )?;
    check_policy(
        "max_queue_depth",
        u64::from(budget.max_queue_depth),
        u64::from(policy.absolute_max_queue_depth),
    )?;
    check_policy(
        "max_journal_batch_bytes",
        u64::from(budget.max_journal_batch_bytes),
        u64::from(policy.absolute_max_journal_batch_bytes),
    )?;
    check_policy(
        "max_ipc_payload_bytes",
        u64::from(budget.max_ipc_payload_bytes),
        u64::from(policy.absolute_max_ipc_payload_bytes),
    )?;
    check_policy(
        "max_blob_bytes",
        budget.max_blob_bytes,
        policy.absolute_max_blob_bytes,
    )?;
    check_policy(
        "max_input_bytes",
        u64::from(budget.max_input_bytes),
        u64::from(policy.absolute_max_input_bytes),
    )?;
    check_policy(
        "max_open_gates",
        u64::from(budget.max_open_gates),
        u64::from(policy.absolute_max_open_gates),
    )?;
    check_policy(
        "max_signal_subscriptions",
        u64::from(budget.max_signal_subscriptions),
        u64::from(policy.absolute_max_signal_subscriptions),
    )?;
    check_policy(
        "max_incident_emissions",
        budget.max_incident_emissions,
        policy.absolute_max_incident_emissions,
    )?;
    check_policy(
        "max_transitions",
        budget.max_transitions,
        policy.absolute_max_transitions,
    )?;
    check_policy(
        "max_journal_bytes_total",
        budget.max_journal_bytes_total,
        policy.absolute_max_journal_bytes_total,
    )?;
    check_policy(
        "max_value_store_entries",
        budget.max_value_store_entries,
        policy.absolute_max_value_store_entries,
    )?;
    check_policy(
        "max_cost_units",
        budget.max_cost_units,
        policy.absolute_max_cost_units,
    )
}

/// Outcome of a remaining-budget exhaustion, for the caller's policy to act
/// on. Releasable dimensions (open gates, signal subscriptions) suspend:
/// a slot frees when a gate closes or a subscription drops, so the run can
/// resume. Monotonic dimensions (transitions, journal bytes, value-store
/// entries, incidents, cost) never replenish within a run, so exhaustion
/// fails. Callers may override this default mapping; the error stays typed
/// either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetExhaustionOutcome {
    Suspend,
    Fail,
}

/// Per-dimension remaining-budget exhaustion for one admitted run.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RemainingBudgetError {
    #[error("open gates exhausted")]
    OpenGatesExhausted,
    #[error("signal subscriptions exhausted")]
    SignalSubscriptionsExhausted,
    #[error("incident emissions exhausted")]
    IncidentEmissionsExhausted,
    #[error("transitions exhausted")]
    TransitionsExhausted,
    #[error("journal bytes exhausted: requested {requested}, remaining {remaining}")]
    JournalBytesExhausted { requested: u64, remaining: u64 },
    #[error("value-store entries exhausted: requested {requested}, remaining {remaining}")]
    ValueStoreEntriesExhausted { requested: u64, remaining: u64 },
    #[error("cost units exhausted: requested {requested}, remaining {remaining}")]
    CostUnitsExhausted { requested: u64, remaining: u64 },
    #[error("release above admitted limit: {resource}")]
    ReleaseAboveLimit { resource: &'static str },
}

impl RemainingBudgetError {
    /// Default suspension/failure mapping for this exhaustion.
    #[must_use]
    pub const fn outcome(&self) -> BudgetExhaustionOutcome {
        match self {
            Self::OpenGatesExhausted | Self::SignalSubscriptionsExhausted => {
                BudgetExhaustionOutcome::Suspend
            }
            _ => BudgetExhaustionOutcome::Fail,
        }
    }
}

/// Remaining-budget counters for one admitted run, covering the §77.18
/// dimensions added by this batch. Created from the admitted aggregate
/// budget; every consume is checked, never saturating or truncating.
/// Gates and signal subscriptions are releasable (close/unsubscribe);
/// transitions, journal bytes, value-store entries, incidents, and cost
/// are monotonic within a run. Whole-run release is shard-level
/// accounting via `AggregateResourceUsage::try_subtract_budget`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemainingBudget {
    limits: AggregateResourceBudget,
    remaining_open_gates: u32,
    remaining_signal_subscriptions: u32,
    remaining_incident_emissions: u64,
    remaining_transitions: u64,
    remaining_journal_bytes_total: u64,
    remaining_value_store_entries: u64,
    remaining_cost_units: u64,
}

impl RemainingBudget {
    /// Creates counters with every dimension fully remaining.
    #[must_use]
    pub const fn from_budget(budget: &AggregateResourceBudget) -> Self {
        Self {
            limits: *budget,
            remaining_open_gates: budget.max_open_gates,
            remaining_signal_subscriptions: budget.max_signal_subscriptions,
            remaining_incident_emissions: budget.max_incident_emissions,
            remaining_transitions: budget.max_transitions,
            remaining_journal_bytes_total: budget.max_journal_bytes_total,
            remaining_value_store_entries: budget.max_value_store_entries,
            remaining_cost_units: budget.max_cost_units,
        }
    }

    /// The admitted budget these counters enforce.
    #[must_use]
    pub const fn limits(&self) -> AggregateResourceBudget {
        self.limits
    }

    /// Remaining open-gate slots.
    #[must_use]
    pub const fn remaining_open_gates(&self) -> u32 {
        self.remaining_open_gates
    }

    /// Remaining signal-subscription slots.
    #[must_use]
    pub const fn remaining_signal_subscriptions(&self) -> u32 {
        self.remaining_signal_subscriptions
    }

    /// Remaining incident emissions.
    #[must_use]
    pub const fn remaining_incident_emissions(&self) -> u64 {
        self.remaining_incident_emissions
    }

    /// Remaining total transitions.
    #[must_use]
    pub const fn remaining_transitions(&self) -> u64 {
        self.remaining_transitions
    }

    /// Remaining cumulative journal bytes.
    #[must_use]
    pub const fn remaining_journal_bytes_total(&self) -> u64 {
        self.remaining_journal_bytes_total
    }

    /// Remaining value-store entries.
    #[must_use]
    pub const fn remaining_value_store_entries(&self) -> u64 {
        self.remaining_value_store_entries
    }

    /// Remaining cost units.
    #[must_use]
    pub const fn remaining_cost_units(&self) -> u64 {
        self.remaining_cost_units
    }

    /// Opens one gate, consuming one open-gate slot.
    pub fn try_open_gate(&mut self) -> Result<(), RemainingBudgetError> {
        match self.remaining_open_gates.checked_sub(1) {
            Some(next) => {
                self.remaining_open_gates = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::OpenGatesExhausted),
        }
    }

    /// Closes one gate, releasing its slot. Releasing past the admitted
    /// limit is a caller bug, not silent replenishment.
    pub fn release_gate(&mut self) -> Result<(), RemainingBudgetError> {
        if self.remaining_open_gates >= self.limits.max_open_gates {
            return Err(RemainingBudgetError::ReleaseAboveLimit {
                resource: "max_open_gates",
            });
        }
        match self.remaining_open_gates.checked_add(1) {
            Some(next) => {
                self.remaining_open_gates = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::ReleaseAboveLimit {
                resource: "max_open_gates",
            }),
        }
    }

    /// Subscribes to one signal, consuming one subscription slot.
    pub fn try_subscribe_signal(&mut self) -> Result<(), RemainingBudgetError> {
        match self.remaining_signal_subscriptions.checked_sub(1) {
            Some(next) => {
                self.remaining_signal_subscriptions = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::SignalSubscriptionsExhausted),
        }
    }

    /// Drops one signal subscription, releasing its slot.
    pub fn release_signal(&mut self) -> Result<(), RemainingBudgetError> {
        if self.remaining_signal_subscriptions >= self.limits.max_signal_subscriptions {
            return Err(RemainingBudgetError::ReleaseAboveLimit {
                resource: "max_signal_subscriptions",
            });
        }
        match self.remaining_signal_subscriptions.checked_add(1) {
            Some(next) => {
                self.remaining_signal_subscriptions = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::ReleaseAboveLimit {
                resource: "max_signal_subscriptions",
            }),
        }
    }

    /// Emits one incident, consuming one emission.
    pub fn try_emit_incident(&mut self) -> Result<(), RemainingBudgetError> {
        match self.remaining_incident_emissions.checked_sub(1) {
            Some(next) => {
                self.remaining_incident_emissions = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::IncidentEmissionsExhausted),
        }
    }

    /// Consumes one total transition.
    pub fn try_consume_transition(&mut self) -> Result<(), RemainingBudgetError> {
        match self.remaining_transitions.checked_sub(1) {
            Some(next) => {
                self.remaining_transitions = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::TransitionsExhausted),
        }
    }

    /// Appends `bytes` journal bytes. No partial append: an overrun
    /// consumes nothing and returns the typed exhaustion.
    pub fn try_append_journal_bytes(&mut self, bytes: u64) -> Result<(), RemainingBudgetError> {
        match self.remaining_journal_bytes_total.checked_sub(bytes) {
            Some(next) => {
                self.remaining_journal_bytes_total = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::JournalBytesExhausted {
                requested: bytes,
                remaining: self.remaining_journal_bytes_total,
            }),
        }
    }

    /// Creates `count` value-store entries.
    pub fn try_create_value_store_entries(
        &mut self,
        count: u64,
    ) -> Result<(), RemainingBudgetError> {
        match self.remaining_value_store_entries.checked_sub(count) {
            Some(next) => {
                self.remaining_value_store_entries = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::ValueStoreEntriesExhausted {
                requested: count,
                remaining: self.remaining_value_store_entries,
            }),
        }
    }

    /// Spends `units` cost units.
    pub fn try_spend_cost_units(&mut self, units: u64) -> Result<(), RemainingBudgetError> {
        match self.remaining_cost_units.checked_sub(units) {
            Some(next) => {
                self.remaining_cost_units = next;
                Ok(())
            }
            None => Err(RemainingBudgetError::CostUnitsExhausted {
                requested: units,
                remaining: self.remaining_cost_units,
            }),
        }
    }
}

/// Validates step ceiling dimensions (max_step_budget_per_tick and
/// max_transitions_per_tick) against hard limits.
pub fn validate_step_ceilings(
    budget: &AggregateResourceBudget,
) -> Result<(), AggregateBudgetError> {
    // Hard limit for step budget per tick - derived from MAX_STEPS_PER_TICK if defined,
    // otherwise use a conservative upper bound.
    const HARD_MAX_STEP_BUDGET_PER_TICK: u64 = 1_000_000;
    const HARD_MAX_TRANSITIONS_PER_TICK: u64 = 1_000_000;

    if budget.max_step_budget_per_tick == 0 {
        return Err(AggregateBudgetError::StepCeilingExceeded {
            requested: 0,
            limit: HARD_MAX_STEP_BUDGET_PER_TICK,
        });
    }
    if budget.max_step_budget_per_tick > HARD_MAX_STEP_BUDGET_PER_TICK {
        return Err(AggregateBudgetError::StepCeilingExceeded {
            requested: budget.max_step_budget_per_tick,
            limit: HARD_MAX_STEP_BUDGET_PER_TICK,
        });
    }

    if budget.max_transitions_per_tick == 0 {
        return Err(AggregateBudgetError::PerTickCeilingExceeded {
            requested: 0,
            limit: HARD_MAX_TRANSITIONS_PER_TICK,
        });
    }
    if budget.max_transitions_per_tick > HARD_MAX_TRANSITIONS_PER_TICK {
        return Err(AggregateBudgetError::PerTickCeilingExceeded {
            requested: budget.max_transitions_per_tick,
            limit: HARD_MAX_TRANSITIONS_PER_TICK,
        });
    }

    Ok(())
}

fn add_dim(
    current: u64,
    requested: u64,
    resource: &'static str,
) -> Result<u64, AggregateBudgetError> {
    current
        .checked_add(requested)
        .ok_or(AggregateBudgetError::Overflow { resource })
}

fn sub_dim(
    current: u64,
    requested: u64,
    resource: &'static str,
) -> Result<u64, AggregateBudgetError> {
    current
        .checked_sub(requested)
        .ok_or(AggregateBudgetError::Underflow { resource })
}

fn check_capacity(
    resource: &'static str,
    requested: u64,
    available: u64,
) -> Result<(), AggregateBudgetError> {
    if requested > available {
        Err(AggregateBudgetError::CapacityExceeded {
            resource,
            requested,
            available,
        })
    } else {
        Ok(())
    }
}

fn check_policy(
    resource: &'static str,
    actual: u64,
    limit: u64,
) -> Result<(), AggregateBudgetError> {
    if actual > limit {
        Err(AggregateBudgetError::PolicyExceeded {
            resource,
            actual,
            limit,
        })
    } else {
        Ok(())
    }
}

impl From<WorkflowError> for BudgetError {
    fn from(_err: WorkflowError) -> Self {
        BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        }
    }
}

impl From<BudgetTraversalError> for BudgetError {
    fn from(_err: BudgetTraversalError) -> Self {
        BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        }
    }
}

/// Counts the worst-case total number of runtime steps by performing a DFS walk
/// from the entry node. Unlike a naive unique-node count, this function accounts
/// for loop iteration limits: when a loop header (ForEachStart, CollectStart,
/// RepeatStart, ReduceStart) is encountered, the body subgraph step count is
/// multiplied by the iteration limit and added once for the header itself.
///
/// The algorithm works in two phases:
/// 1. **Body counting phase**: A DFS walk counts unique nodes in each loop body
///    region (nodes reachable from `body` but not from `done`). This avoids
///    infinite recursion from back-edges.
/// 2. **Worst-case multiplication**: Loop body counts are multiplied by the
///    declared iteration limits and summed with non-loop node counts.
mod conditional_counter;

fn count_total_steps(
    nodes: &[crate::workflow::CompiledNode],
    entry: StepIdx,
    node_count: usize,
) -> Result<u64, BudgetTraversalError> {
    conditional_counter::count(nodes, entry, node_count)
}

fn find_node_position(
    nodes: &[crate::workflow::CompiledNode],
    step: StepIdx,
    node_count: usize,
) -> Result<usize, BudgetTraversalError> {
    let direct_idx = step.as_usize();
    if direct_idx < node_count
        && let Some(node) = nodes.get(direct_idx)
        && node.id == step
    {
        return Ok(direct_idx);
    }

    for (position, node) in nodes.iter().enumerate() {
        if node.id == step {
            return Ok(position);
        }
    }

    if direct_idx < node_count {
        return Ok(direct_idx);
    }

    Err(BudgetTraversalError::StepOutOfBounds { step })
}

fn node_at_position(
    nodes: &[crate::workflow::CompiledNode],
    position: usize,
    step: StepIdx,
) -> Result<&crate::workflow::CompiledNode, BudgetTraversalError> {
    match nodes.get(position) {
        Some(node) => Ok(node),
        None => Err(BudgetTraversalError::StepOutOfBounds { step }),
    }
}

/// Visits a single node during step counting and updates the total and stack.
#[allow(clippy::too_many_arguments)]
fn visit_node_for_total_steps(
    nodes: &[crate::workflow::CompiledNode],
    current: StepIdx,
    node_count: usize,
    visited: &mut [bool],
    jump_edges: &mut Vec<(u16, u16)>,
    in_path: &mut Vec<u16>,
    mut total: u64,
    stack: &mut Vec<StepIdx>,
    cache: &std::collections::HashMap<StepIdx, u64>,
) -> Result<u64, BudgetTraversalError> {
    let idx = find_node_position(nodes, current, node_count)?;
    if visited.get(idx).copied() == Some(true) {
        return Ok(total);
    }
    let Some(flag) = visited.get_mut(idx) else {
        return Err(BudgetTraversalError::StepOutOfBounds { step: current });
    };
    *flag = true;

    let node = node_at_position(nodes, idx, current)?;

    total = match total.checked_add(1) {
        Some(v) => v,
        None => return Err(BudgetTraversalError::StepOutOfBounds { step: current }),
    };

    match &node.kind {
        CompiledNodeKind::ForEachStart {
            limit, body, done, ..
        } => {
            total = count_and_push_loop_body(
                nodes,
                *body,
                *done,
                u64::from(*limit),
                visited,
                node_count,
                total,
                stack,
            )
            .map_err(map_loop_body_budget_error)?;
        }
        CompiledNodeKind::CollectStart {
            limit, body, done, ..
        } => {
            total = count_and_push_loop_body(
                nodes,
                *body,
                *done,
                u64::from(*limit),
                visited,
                node_count,
                total,
                stack,
            )
            .map_err(map_loop_body_budget_error)?;
        }
        CompiledNodeKind::ReduceStart { body, done, .. } => {
            let iter_count = match u64::try_from(crate::limits::MAX_LIST_ITEMS_PER_VALUE) {
                Ok(value) => value,
                Err(_) => return Err(BudgetTraversalError::StepCountOverflow { actual: u64::MAX }),
            };
            total = count_and_push_loop_body(
                nodes, *body, *done, iter_count, visited, node_count, total, stack,
            )
            .map_err(map_loop_body_budget_error)?;
        }
        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
        } => {
            total = count_and_push_loop_body(
                nodes,
                *body,
                *done,
                u64::from(*max_attempts),
                visited,
                node_count,
                total,
                stack,
            )
            .map_err(map_loop_body_budget_error)?;
        }
        CompiledNodeKind::Jump { target } => {
            let from = current.get();
            let to = target.get();
            if tracked_steps_contain(in_path, to) {
                return Err(BudgetTraversalError::JumpCycle {
                    step: current,
                    target: *target,
                });
            }
            if !insert_tracked_jump_edge(jump_edges, (from, to), node_count)? {
                return Err(BudgetTraversalError::JumpCycle {
                    step: current,
                    target: *target,
                });
            }
            insert_tracked_step(in_path, to, node_count)?;
            stack.push(*target);
        }
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            total = add_conditional_max_steps(branches, *otherwise, total, cache)?;
        }
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            total = add_conditional_slot_max_steps(branches, *otherwise, total, cache)?;
        }
        _ => {
            push_successor_targets(&node.kind, stack);
            if let Some(next) = node.routing.next.step() {
                stack.push(next);
            }
        }
    }
    Ok(total)
}

fn map_loop_body_budget_error(error: BudgetError) -> BudgetTraversalError {
    let actual = match error {
        BudgetError::TotalStepsExceeded { actual, .. } => actual,
        _ => u64::MAX,
    };
    BudgetTraversalError::StepCountOverflow { actual }
}

fn add_conditional_max_steps(
    branches: &[crate::workflow::ExprBranch],
    otherwise: Option<StepIdx>,
    total: u64,
    cache: &std::collections::HashMap<StepIdx, u64>,
) -> Result<u64, BudgetTraversalError> {
    let mut max_branch = match otherwise {
        Some(target) => cache
            .get(&target)
            .copied()
            .ok_or(BudgetTraversalError::StepOutOfBounds { step: target })?,
        None => 0,
    };
    for branch in branches {
        let branch_steps =
            cache
                .get(&branch.target)
                .copied()
                .ok_or(BudgetTraversalError::StepOutOfBounds {
                    step: branch.target,
                })?;
        max_branch = max_branch.max(branch_steps);
    }
    checked_step_add(total, max_branch)
}

fn add_conditional_slot_max_steps(
    branches: &[crate::workflow::SlotBranch],
    otherwise: Option<StepIdx>,
    total: u64,
    cache: &std::collections::HashMap<StepIdx, u64>,
) -> Result<u64, BudgetTraversalError> {
    let mut max_branch = match otherwise {
        Some(target) => cache
            .get(&target)
            .copied()
            .ok_or(BudgetTraversalError::StepOutOfBounds { step: target })?,
        None => 0,
    };
    for branch in branches {
        let branch_steps =
            cache
                .get(&branch.target)
                .copied()
                .ok_or(BudgetTraversalError::StepOutOfBounds {
                    step: branch.target,
                })?;
        max_branch = max_branch.max(branch_steps);
    }
    checked_step_add(total, max_branch)
}

fn checked_step_add(left: u64, right: u64) -> Result<u64, BudgetTraversalError> {
    match left.checked_add(right) {
        Some(value) => Ok(value),
        None => Err(BudgetTraversalError::StepCountOverflow { actual: u64::MAX }),
    }
}

/// Counts body region steps for a loop header and adds multiplied iterations to total.
#[inline]
#[allow(clippy::too_many_arguments, unused_variables)]
fn count_and_push_loop_body(
    nodes: &[crate::workflow::CompiledNode],
    body: StepIdx,
    done: StepIdx,
    iter_count: u64,
    visited: &mut [bool],
    node_count: usize,
    mut total: u64,
    stack: &mut Vec<StepIdx>,
) -> Result<u64, BudgetError> {
    let body_count = count_body_region_nodes(nodes, body, done, node_count)?;
    let iter_count = iter_count.max(1);
    let product = body_count
        .checked_mul(iter_count)
        .ok_or(BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        })?;
    total = total
        .checked_add(product)
        .ok_or(BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        })?;
    push_done_continuation(nodes, done, node_count, stack)?;
    Ok(total)
}

fn push_done_continuation(
    nodes: &[crate::workflow::CompiledNode],
    done: StepIdx,
    node_count: usize,
    stack: &mut Vec<StepIdx>,
) -> Result<(), BudgetError> {
    let done_idx = find_node_position(nodes, done, node_count)?;
    if let Some(node) = nodes.get(done_idx)
        && matches!(node.routing.next, NodeContinuation::End)
        && let Some(next_idx) = done_idx.checked_add(1)
        && next_idx < nodes.len()
        && let Some(next_node) = nodes.get(next_idx)
    {
        stack.push(next_node.id);
    }
    stack.push(done);
    Ok(())
}

/// Counts the worst-case total steps in a loop body region: all nodes reachable
/// from `body` that are not at or past `done` (the loop exit). Nested loop
/// headers within the body are recursively multiplied by their iteration limits.
mod region_counter;

fn count_body_region_nodes(
    nodes: &[crate::workflow::CompiledNode],
    body: StepIdx,
    done: StepIdx,
    node_count: usize,
) -> Result<u64, BudgetError> {
    region_counter::count(nodes, body, done, node_count)
}

/// Visits a single node in a body region during step counting.
#[allow(clippy::too_many_arguments)]
fn visit_body_region_node(
    nodes: &[crate::workflow::CompiledNode],
    current: StepIdx,
    done: StepIdx,
    node_count: usize,
    region_visited: &mut [bool],
    stack: &mut Vec<StepIdx>,
    mut count: u64,
    cache: &std::collections::HashMap<(StepIdx, StepIdx), u64>,
) -> Result<u64, BudgetError> {
    if current == done {
        return Ok(count);
    }
    let idx = find_node_position(nodes, current, node_count)?;
    if region_visited.get(idx).copied() == Some(true) {
        return Ok(count);
    }
    let Some(flag) = region_visited.get_mut(idx) else {
        return Err(BudgetTraversalError::StepOutOfBounds { step: current }.into());
    };
    *flag = true;

    count = count
        .checked_add(1)
        .ok_or(BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        })?;

    let node = node_at_position(nodes, idx, current)?;

    match &node.kind {
        CompiledNodeKind::ForEachStart {
            limit, body, done, ..
        } => {
            if *body != current {
                count = count_nested_for_region(
                    *body,
                    *done,
                    u64::from(*limit).max(1),
                    count,
                    stack,
                    cache,
                )?;
            }
        }
        CompiledNodeKind::CollectStart {
            limit, body, done, ..
        } => {
            if *body != current {
                count = count_nested_for_region(
                    *body,
                    *done,
                    u64::from(*limit).max(1),
                    count,
                    stack,
                    cache,
                )?;
            }
        }
        CompiledNodeKind::ReduceStart { body, done, .. } => {
            let iter = match u64::try_from(crate::limits::MAX_LIST_ITEMS_PER_VALUE) {
                Ok(value) => value,
                Err(_) => {
                    return Err(BudgetError::TotalStepsExceeded {
                        actual: u64::MAX,
                        limit: u64::MAX,
                    });
                }
            };
            if *body != current {
                count = count_nested_for_region(*body, *done, iter, count, stack, cache)?;
            }
        }
        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
            ..
        } => {
            if *body != current {
                count = count_nested_for_region(
                    *body,
                    *done,
                    u64::from(*max_attempts).max(1),
                    count,
                    stack,
                    cache,
                )?;
            }
        }
        _ => {
            push_successor_targets(&node.kind, stack);
            if let Some(next) = node.routing.next.step() {
                stack.push(next);
            }
        }
    }
    Ok(count)
}

/// Counts a nested loop body within a region and adds multiplied iterations.
#[inline]
#[allow(clippy::too_many_arguments)]
fn count_nested_for_region(
    body: StepIdx,
    done: StepIdx,
    iter_count: u64,
    count: u64,
    stack: &mut Vec<StepIdx>,
    cache: &std::collections::HashMap<(StepIdx, StepIdx), u64>,
) -> Result<u64, BudgetError> {
    let body_count = cache
        .get(&(body, done))
        .copied()
        .ok_or(BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        })?;
    stack.push(done);
    let product = body_count
        .checked_mul(iter_count)
        .ok_or(BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        })?;
    count
        .checked_add(product)
        .ok_or(BudgetError::TotalStepsExceeded {
            actual: u64::MAX,
            limit: u64::MAX,
        })
}

/// Pushes all successor StepIdx targets from a node kind onto the stack,
/// excluding the `next` field which is handled separately.
fn push_successor_targets(kind: &CompiledNodeKind, stack: &mut Vec<StepIdx>) {
    if node_kind_has_no_successors(kind) {
        return;
    }
    match kind {
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => push_slot_choose_successors(branches, *otherwise, stack),
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => push_expr_choose_successors(branches, *otherwise, stack),
        CompiledNodeKind::ForEachStart { body, done, .. }
        | CompiledNodeKind::ForEachNext { body, done, .. }
        | CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::CollectPage { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. }
        | CompiledNodeKind::RepeatStart { body, done, .. }
        | CompiledNodeKind::RepeatAttempt { body, done, .. }
        | CompiledNodeKind::RetryCheck {
            body,
            exhausted: done,
            ..
        } => push_loop_successors(*body, *done, stack),
        CompiledNodeKind::RepeatCheck { done, .. } => push_repeat_check_successors(*done, stack),
        CompiledNodeKind::TogetherStart { branches, join } => {
            push_together_start_successors(branches, *join, stack)
        }
        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            push_together_branch_successors(*entry, *join, stack)
        }
        CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            push_error_handler_successors(*body, *handler, stack)
        }
        CompiledNodeKind::Jump { target } => stack.push(*target),
        _ => {}
    }
}

/// Returns true if the node kind has no successor targets.
#[inline]
fn node_kind_has_no_successors(kind: &CompiledNodeKind) -> bool {
    matches!(
        kind,
        CompiledNodeKind::Nop
            | CompiledNodeKind::SetConst { .. }
            | CompiledNodeKind::Copy { .. }
            | CompiledNodeKind::EvalExpr { .. }
            | CompiledNodeKind::BuildObject { .. }
            | CompiledNodeKind::BuildList { .. }
            | CompiledNodeKind::Do { .. }
            | CompiledNodeKind::ForEachJoin { .. }
            | CompiledNodeKind::CollectFinish { .. }
            | CompiledNodeKind::ReduceFinish { .. }
            | CompiledNodeKind::RepeatFinish { .. }
            | CompiledNodeKind::WaitUntil { .. }
            | CompiledNodeKind::Ask { .. }
            | CompiledNodeKind::AskResume { .. }
            | CompiledNodeKind::Finish { .. }
            | CompiledNodeKind::TogetherJoin { .. }
            | CompiledNodeKind::WaitEvent { .. }
    )
}

/// Push Choose successors: all branch targets + optional fallback.
fn push_expr_choose_successors(
    branches: &[crate::workflow::ExprBranch],
    otherwise: Option<StepIdx>,
    stack: &mut Vec<StepIdx>,
) {
    for branch in branches {
        stack.push(branch.target);
    }
    if let Some(fallback) = otherwise {
        stack.push(fallback);
    }
}

/// Push ChooseSlot successors: all slot branch targets + optional fallback.
fn push_slot_choose_successors(
    branches: &[crate::workflow::SlotBranch],
    otherwise: Option<StepIdx>,
    stack: &mut Vec<StepIdx>,
) {
    for branch in branches {
        stack.push(branch.target);
    }
    if let Some(fallback) = otherwise {
        stack.push(fallback);
    }
}

/// Push loop successors: body + done targets.
fn push_loop_successors(body: StepIdx, done: StepIdx, stack: &mut Vec<StepIdx>) {
    stack.push(body);
    stack.push(done);
}

/// Push RepeatCheck successor: done target only.
fn push_repeat_check_successors(done: StepIdx, stack: &mut Vec<StepIdx>) {
    stack.push(done);
}

/// Push TogetherStart successors: all branch targets + join.
fn push_together_start_successors(branches: &[StepIdx], join: StepIdx, stack: &mut Vec<StepIdx>) {
    for branch in branches {
        stack.push(*branch);
    }
    stack.push(join);
}

/// Push TogetherBranch successors: entry + join.
fn push_together_branch_successors(entry: StepIdx, join: StepIdx, stack: &mut Vec<StepIdx>) {
    stack.push(entry);
    stack.push(join);
}

/// Push ErrorHandler successors: body + handler.
fn push_error_handler_successors(body: StepIdx, handler: StepIdx, stack: &mut Vec<StepIdx>) {
    stack.push(body);
    stack.push(handler);
}

fn branch_count_to_u16(count: usize) -> Result<u16, BudgetTraversalError> {
    match u16::try_from(count) {
        Ok(value) => Ok(value),
        Err(_) => Err(BudgetTraversalError::StepCountOverflow {
            actual: usize_to_u64_saturating(count),
        }),
    }
}

fn usize_to_u64_saturating(value: usize) -> u64 {
    u64::try_from(value).map_or(u64::MAX, core::convert::identity)
}

fn bounded_tracking_vec<T>(node_count: usize) -> Vec<T> {
    Vec::with_capacity(node_count)
}

fn tracked_steps_contain(steps: &[u16], step: u16) -> bool {
    steps.iter().copied().any(|candidate| candidate == step)
}

fn insert_tracked_step(
    steps: &mut Vec<u16>,
    step: u16,
    limit: usize,
) -> Result<bool, BudgetTraversalError> {
    if tracked_steps_contain(steps, step) {
        return Ok(false);
    }
    if steps.len() >= limit {
        return Err(BudgetTraversalError::StepCountOverflow { actual: u64::MAX });
    }
    steps.push(step);
    Ok(true)
}

fn remove_tracked_step(steps: &mut Vec<u16>, step: u16) {
    if let Some(position) = steps.iter().position(|candidate| *candidate == step) {
        steps.remove(position);
    }
}

fn insert_tracked_jump_edge(
    edges: &mut Vec<(u16, u16)>,
    edge: (u16, u16),
    limit: usize,
) -> Result<bool, BudgetTraversalError> {
    if edges.iter().copied().any(|candidate| candidate == edge) {
        return Ok(false);
    }
    if edges.len() >= limit {
        return Err(BudgetTraversalError::StepCountOverflow { actual: u64::MAX });
    }
    edges.push(edge);
    Ok(true)
}

enum ReachableVisit {
    Enter(StepIdx),
    Exit(u16),
}

/// Computes once-only resource metrics and reachability via a DFS walk.
#[allow(clippy::too_many_arguments)]
fn compute_reachable_metrics(
    nodes: &[crate::workflow::CompiledNode],
    current: StepIdx,
    visited: &mut [bool],
    in_path: &mut Vec<u16>,
    node_count: usize,
    max_fanout: &mut u16,
    max_action_tickets: &mut u32,
    max_parallel_in_flight: &mut u16,
    max_gather_pages: &mut u32,
    max_gather_items: &mut u32,
    max_for_each_iterations: &mut u32,
    max_together_branches: &mut u16,
    max_repeat_attempts: &mut u16,
    max_timer_entries: &mut u32,
    max_open_gates: &mut u32,
    max_signal_subscriptions: &mut u32,
) -> Result<(), BudgetTraversalError> {
    // vb-8eocj.7.2.1 Explicit DFS event stack preserves the original recursive
    // in-path set semantics so jump cycle detection and backtracking remain
    // identical while the stack grows linearly with node count.
    let mut stack: Vec<ReachableVisit> = Vec::new();
    stack.push(ReachableVisit::Enter(current));

    while let Some(event) = stack.pop() {
        match event {
            ReachableVisit::Enter(step) => {
                let idx = find_node_position(nodes, step, node_count)?;
                if visited.get(idx).copied() == Some(true) {
                    continue;
                }
                let Some(flag) = visited.get_mut(idx) else {
                    return Err(BudgetTraversalError::StepOutOfBounds { step });
                };
                *flag = true;

                let node = node_at_position(nodes, idx, step)?;
                let step_u16 = step.get();
                insert_tracked_step(in_path, step_u16, node_count)?;

                if let CompiledNodeKind::Jump { target } = &node.kind {
                    let target_u16 = target.get();
                    if tracked_steps_contain(in_path, target_u16) {
                        remove_tracked_step(in_path, step_u16);
                        return Err(BudgetTraversalError::JumpCycle {
                            step,
                            target: *target,
                        });
                    }
                }

                update_fanout(&node.kind, max_fanout)?;
                update_workflow_metrics(
                    &node.kind,
                    max_action_tickets,
                    max_parallel_in_flight,
                    max_gather_pages,
                    max_gather_items,
                    max_for_each_iterations,
                    max_together_branches,
                    max_repeat_attempts,
                    max_timer_entries,
                    max_open_gates,
                    max_signal_subscriptions,
                )?;

                let mut targets: Vec<StepIdx> = Vec::new();
                push_successor_targets(&node.kind, &mut targets);
                if let Some(next) = node.routing.next.step() {
                    targets.push(next);
                }

                stack.push(ReachableVisit::Exit(step_u16));
                for target in targets.iter().rev() {
                    if find_node_position(nodes, *target, node_count).is_ok() {
                        stack.push(ReachableVisit::Enter(*target));
                    }
                }
            }
            ReachableVisit::Exit(step_u16) => {
                remove_tracked_step(in_path, step_u16);
            }
        }
    }

    Ok(())
}

/// Cold structural upper bound for vb-8eocj.3.6, independent of DFS order.
/// Reachable owner spans are half-open: an exit retires before a start there.
/// Alternate-entry IR can conservatively overlap mutually exclusive owners;
/// canonical source scopes have their exact lexical nesting depth.
fn compute_structural_depth(
    nodes: &[crate::workflow::CompiledNode],
    visited: &[bool],
) -> Result<u16, BudgetTraversalError> {
    let mut owners = reachable_nesting_owners(nodes, visited)?;
    owners.sort_unstable_by_key(|(node, _)| node.id);
    let mut active_exits = bounded_tracking_vec(owners.len());
    let mut max_depth = 0;
    for (node, exit) in owners {
        active_exits.retain(|end| *end > node.id);
        let depth = u16::try_from(active_exits.len())
            .map_err(|_| BudgetTraversalError::DepthOverflow { depth: u16::MAX })?;
        compute_child_depth(&node.kind, depth, &mut max_depth)?;
        active_exits.push(exit);
    }
    Ok(max_depth)
}

fn reachable_nesting_owners<'a>(
    nodes: &'a [crate::workflow::CompiledNode],
    visited: &[bool],
) -> Result<Vec<(&'a crate::workflow::CompiledNode, StepIdx)>, BudgetTraversalError> {
    let mut owners = bounded_tracking_vec(nodes.len());
    for (index, node) in nodes.iter().enumerate() {
        if visited.get(index).copied() != Some(true) {
            continue;
        }
        let Some(exit) = nesting_scope_exit(&node.kind) else {
            continue;
        };
        find_node_position(nodes, exit, nodes.len())?;
        if exit <= node.id {
            return Err(BudgetTraversalError::BackwardScopeExit {
                from: node.id,
                to: exit,
            });
        }
        owners.push((node, exit));
    }
    Ok(owners)
}

fn nesting_scope_exit(kind: &CompiledNodeKind) -> Option<StepIdx> {
    match kind {
        CompiledNodeKind::ForEachStart { done, .. }
        | CompiledNodeKind::CollectStart { done, .. }
        | CompiledNodeKind::ReduceStart { done, .. }
        | CompiledNodeKind::RepeatStart { done, .. } => Some(*done),
        CompiledNodeKind::TogetherStart { join, .. } => Some(*join),
        _ => None,
    }
}

fn compute_child_depth(
    kind: &CompiledNodeKind,
    current_depth: u16,
    max_nesting_depth: &mut u16,
) -> Result<u16, BudgetTraversalError> {
    match kind {
        CompiledNodeKind::ForEachStart { .. }
        | CompiledNodeKind::CollectStart { .. }
        | CompiledNodeKind::ReduceStart { .. }
        | CompiledNodeKind::RepeatStart { .. }
        | CompiledNodeKind::TogetherStart { .. } => {
            let new_depth =
                current_depth
                    .checked_add(1)
                    .ok_or(BudgetTraversalError::DepthOverflow {
                        depth: current_depth,
                    })?;
            if new_depth > *max_nesting_depth {
                *max_nesting_depth = new_depth;
            }
            Ok(new_depth)
        }
        _ => Ok(current_depth),
    }
}

fn update_fanout(
    kind: &CompiledNodeKind,
    max_fanout: &mut u16,
) -> Result<(), BudgetTraversalError> {
    match kind {
        CompiledNodeKind::TogetherStart { branches, .. } => {
            let branch_count = branch_count_to_u16(branches.len())?;
            if branch_count > *max_fanout {
                *max_fanout = branch_count;
            }
        }
        CompiledNodeKind::ChooseSlot { branches, .. } => {
            let branch_count = branch_count_to_u16(branches.len())?;
            if branch_count > *max_fanout {
                *max_fanout = branch_count;
            }
        }
        CompiledNodeKind::Choose { branches, .. } => {
            let branch_count = branch_count_to_u16(branches.len())?;
            if branch_count > *max_fanout {
                *max_fanout = branch_count;
            }
        }
        _ => {}
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn update_workflow_metrics(
    kind: &CompiledNodeKind,
    max_action_tickets: &mut u32,
    max_parallel_in_flight: &mut u16,
    max_gather_pages: &mut u32,
    max_gather_items: &mut u32,
    max_for_each_iterations: &mut u32,
    max_together_branches: &mut u16,
    max_repeat_attempts: &mut u16,
    max_timer_entries: &mut u32,
    max_open_gates: &mut u32,
    max_signal_subscriptions: &mut u32,
) -> Result<(), BudgetTraversalError> {
    match kind {
        CompiledNodeKind::Do { .. } => {
            *max_action_tickets = max_action_tickets
                .checked_add(1)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        }
        CompiledNodeKind::TogetherStart { branches, .. } => {
            let branch_count = branch_count_to_u16(branches.len())?;
            if branch_count > *max_parallel_in_flight {
                *max_parallel_in_flight = branch_count;
            }
            if branch_count > *max_together_branches {
                *max_together_branches = branch_count;
            }
        }
        CompiledNodeKind::CollectStart {
            limit, page_size, ..
        } => {
            let per_page = u64::from(*page_size).max(1);
            let pages = u64::from(*limit).div_ceil(per_page);
            let pages = u32::try_from(pages)
                .map_err(|_| BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
            *max_gather_pages = max_gather_pages
                .checked_add(pages)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
            *max_gather_items = max_gather_items
                .checked_add(*limit)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        }
        CompiledNodeKind::ForEachStart { limit, .. } => {
            *max_for_each_iterations = max_for_each_iterations
                .checked_add(*limit)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        }
        CompiledNodeKind::RepeatStart { max_attempts, .. } => {
            *max_repeat_attempts = (*max_repeat_attempts).max(*max_attempts);
        }
        CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::RetryCheck { .. }
        | CompiledNodeKind::RepeatCheck { .. } => {
            *max_timer_entries = max_timer_entries
                .checked_add(1)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        }
        _ => {}
    }
    match kind {
        CompiledNodeKind::Ask { .. } => {
            *max_open_gates = max_open_gates
                .checked_add(1)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        }
        CompiledNodeKind::WaitEvent { .. } => {
            *max_signal_subscriptions = max_signal_subscriptions
                .checked_add(1)
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
        }
        _ => {}
    }
    Ok(())
}

mod tests_and_verification;

#[cfg(test)]
#[path = "budget/tests.rs"]
mod tests;

#[cfg(test)]
mod vb_qi37_2_4_state8_tests;

#[cfg(test)]
mod depth_overflow_tests {
    //! Regression tests for CB-004 / `vb-lk26g`:
    //! `compute_child_depth` must return `BudgetTraversalError::DepthOverflow`
    //! carrying the actual pre-overflow depth value, not the lossy
    //! `StepCountOverflow { actual: u64::MAX }` sentinel it used to produce.

    use super::{BudgetTraversalError, CompiledNodeKind, compute_child_depth};

    #[test]
    fn compute_child_depth_returns_depth_overflow_carrying_actual_depth() {
        let kind = CompiledNodeKind::ForEachStart {
            input: crate::ids::SlotIdx::new(0),
            item_slot: crate::ids::SlotIdx::new(1),
            limit: 1,
            body: crate::ids::StepIdx::new(0),
            done: crate::ids::StepIdx::new(0),
        };
        let mut max_nesting_depth: u16 = 0;
        let err = compute_child_depth(&kind, u16::MAX, &mut max_nesting_depth)
            .expect_err("u16::MAX + 1 must overflow");
        match err {
            BudgetTraversalError::DepthOverflow { depth } => {
                assert_eq!(
                    depth,
                    u16::MAX,
                    "DepthOverflow must carry the actual pre-overflow depth"
                );
            }
            other => {
                let msg = format!(
                    "expected BudgetTraversalError::DepthOverflow {{ depth: u16::MAX }}, got {other:?}"
                );
                debug_assert!(false, "{msg}");
            }
        }
    }

    #[test]
    fn compute_child_depth_does_not_emit_step_count_overflow_at_u16_max() {
        let kind = CompiledNodeKind::RepeatStart {
            max_attempts: 1,
            body: crate::ids::StepIdx::new(0),
            done: crate::ids::StepIdx::new(0),
        };
        let mut max_nesting_depth: u16 = 0;
        let err = compute_child_depth(&kind, u16::MAX, &mut max_nesting_depth)
            .expect_err("u16::MAX + 1 must overflow");
        assert!(
            !matches!(err, BudgetTraversalError::StepCountOverflow { .. }),
            "depth overflow must not be misclassified as StepCountOverflow (CB-004)"
        );
    }

    #[test]
    fn depth_overflow_converts_to_workflow_error_carrying_actual_depth() {
        let err: crate::workflow::WorkflowError =
            BudgetTraversalError::DepthOverflow { depth: 42 }.into();
        match err {
            crate::workflow::WorkflowError::DepthOverflow { depth } => {
                assert_eq!(depth, 42, "From conversion must preserve depth");
            }
            other => {
                let msg =
                    format!("expected WorkflowError::DepthOverflow {{ depth: 42 }}, got {other:?}");
                debug_assert!(false, "{msg}");
            }
        }
    }
}
