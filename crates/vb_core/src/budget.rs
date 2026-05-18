#![forbid(unsafe_code)]

//! Whole-workflow budget computation and boundedness policy enforcement.

use crate::ids::{RunId, StepIdx};
use crate::workflow::{CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowError};
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
    /// Maximum loop nesting depth.
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
}

impl WholeWorkflowBudget {
    /// Walks the compiled IR starting from `entry` and computes all
    /// budget dimensions.
    pub fn compute(
        nodes: &[crate::workflow::CompiledNode],
        entry: StepIdx,
        contract: &ResourceContract,
    ) -> Result<Self, WorkflowError> {
        let node_count = nodes.len();
        if entry.as_usize() >= node_count {
            return Err(WorkflowError::EntryOutOfBounds { entry });
        }

        let mut visited: Vec<bool> = vec![false; node_count];
        let mut in_path: std::collections::HashSet<u16> = std::collections::HashSet::new();
        let max_total_steps = count_total_steps(nodes, entry, node_count)?;

        let mut max_fanout: u16 = 0;
        let mut max_nesting_depth: u16 = 0;
        let mut max_action_tickets: u32 = 0;
        let mut max_parallel_in_flight: u16 = 0;
        let mut max_gather_pages: u32 = 0;
        let mut max_gather_items: u32 = 0;
        let mut max_for_each_iterations: u32 = 0;
        let mut max_together_branches: u16 = 0;
        let mut max_repeat_attempts: u16 = 0;
        compute_fanout_and_depth(
            nodes,
            entry,
            &mut visited,
            &mut in_path,
            node_count,
            0,
            &mut max_fanout,
            &mut max_nesting_depth,
            &mut max_action_tickets,
            &mut max_parallel_in_flight,
            &mut max_gather_pages,
            &mut max_gather_items,
            &mut max_for_each_iterations,
            &mut max_together_branches,
            &mut max_repeat_attempts,
        )?;

        let max_total_slots = u64::from(contract.max_slots);

        // Phase 0 executes at most one step per runtime tick, so steps bound time.
        let max_run_time_seconds = max_total_steps;

        Ok(Self {
            max_total_steps,
            max_total_slots,
            max_fanout,
            max_nesting_depth,
            max_steps_executable: match u32::try_from(max_total_steps) {
                Ok(value) => value,
                Err(_) => {
                    return Err(WorkflowError::StepCountOverflow {
                        actual: max_total_steps,
                    });
                }
            },
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
        })
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
        Ok(())
    }
}

/// Budget validation failures.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum BudgetError {
    /// Total step count exceeded the policy limit.
    #[error("total steps {actual} exceeded policy limit {limit}")]
    TotalStepsExceeded {
        /// Actual total steps computed.
        actual: u64,
        /// Policy limit.
        limit: u64,
    },
    /// Total slot count exceeded the policy limit.
    #[error("total slots {actual} exceeded policy limit {limit}")]
    TotalSlotsExceeded {
        /// Actual total slots computed.
        actual: u64,
        /// Policy limit.
        limit: u64,
    },
    /// Fanout exceeded the policy limit.
    #[error("fanout {actual} exceeded policy limit {limit}")]
    FanoutExceeded {
        /// Actual fanout computed.
        actual: u16,
        /// Policy limit.
        limit: u16,
    },
    /// Nesting depth exceeded the policy limit.
    #[error("nesting depth {actual} exceeded policy limit {limit}")]
    NestingDepthExceeded {
        /// Actual nesting depth computed.
        actual: u16,
        /// Policy limit.
        limit: u16,
    },
    /// Parallel in-flight exceeded the policy limit.
    #[error("parallel in-flight {actual} exceeded policy limit {limit}")]
    ParallelExceeded {
        /// Actual parallel in-flight computed.
        actual: u16,
        /// Policy limit.
        limit: u16,
    },
    /// Action tickets exceeded the policy limit.
    #[error("action tickets {actual} exceeded policy limit {limit}")]
    ActionTicketsExceeded {
        /// Actual action tickets computed.
        actual: u32,
        /// Policy limit.
        limit: u32,
    },
    /// Run time exceeded the policy limit.
    #[error("run time {actual} exceeded policy limit {limit}")]
    RunTimeExceeded {
        /// Actual run time computed.
        actual: u64,
        /// Policy limit.
        limit: u64,
    },
    /// Result bytes exceeded the policy limit.
    #[error("result bytes {actual} exceeded policy limit {limit}")]
    ResultBytesExceeded {
        /// Actual result bytes computed.
        actual: u32,
        /// Policy limit.
        limit: u32,
    },
    /// Steps executable exceeded the policy limit.
    #[error("steps executable {actual} exceeded policy limit {limit}")]
    StepsExecutableExceeded {
        /// Actual steps executable computed.
        actual: u32,
        /// Policy limit.
        limit: u32,
    },
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
    pub max_queue_depth: u32,
    pub max_journal_batch_bytes: u32,
    /// Maximum step budget per runtime tick (from ResourceContract).
    pub max_step_budget_per_tick: u64,
    /// Maximum transitions per runtime tick.
    pub max_transitions_per_tick: u64,
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
    pub max_active_runs: u64,
    pub max_queue_depth: u64,
    pub max_journal_batch_bytes: u64,
    /// Maximum step budget per tick capacity.
    pub max_step_budget_per_tick: u64,
    /// Maximum transitions per tick capacity.
    pub max_transitions_per_tick: u64,
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
    pub max_active_runs: u64,
    pub max_queue_depth: u64,
    pub max_journal_batch_bytes: u64,
    /// Current step budget per tick usage.
    pub max_step_budget_per_tick: u64,
    /// Current transitions per tick usage.
    pub max_transitions_per_tick: u64,
}

/// Exact budget reservation associated with a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AggregateReservation {
    pub run: RunId,
    pub requested: AggregateResourceBudget,
}

/// Aggregate resource-accounting failure.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
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
            max_queue_depth: contract.max_queue_depth,
            max_journal_batch_bytes: contract.max_journal_batch_bytes,
            max_step_budget_per_tick: contract.max_step_budget_per_tick,
            max_transitions_per_tick: contract.max_transitions_per_tick,
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
            "max_step_budget_per_tick",
            self.max_step_budget_per_tick,
            capacity.max_step_budget_per_tick,
        )?;
        check_capacity(
            "max_transitions_per_tick",
            self.max_transitions_per_tick,
            capacity.max_transitions_per_tick,
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
        )
    }
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
        "max_queue_depth",
        u64::from(budget.max_queue_depth),
        u64::from(u32::MAX),
    )?;
    check_policy(
        "max_journal_batch_bytes",
        u64::from(budget.max_journal_batch_bytes),
        u64::from(u32::MAX),
    )
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
fn count_total_steps(
    nodes: &[crate::workflow::CompiledNode],
    entry: StepIdx,
    node_count: usize,
) -> Result<u64, WorkflowError> {
    let mut visited: Vec<bool> = vec![false; node_count];
    let mut jump_edges: std::collections::HashSet<(u16, u16)> = std::collections::HashSet::new();
    let mut in_path: std::collections::HashSet<u16> = std::collections::HashSet::new();
    let mut total: u64 = 0;

    let mut stack: Vec<StepIdx> = Vec::new();
    stack.push(entry);

    while let Some(current) = stack.pop() {
        let current_u16 = current.get();
        in_path.remove(&current_u16);
        total = visit_node_for_total_steps(
            nodes,
            current,
            node_count,
            &mut visited,
            &mut jump_edges,
            &mut in_path,
            total,
            &mut stack,
        )?;
    }
    Ok(total)
}

fn find_node_position(
    nodes: &[crate::workflow::CompiledNode],
    step: StepIdx,
    node_count: usize,
) -> Result<usize, WorkflowError> {
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

    Err(WorkflowError::StepOutOfBounds { step })
}

fn node_at_position(
    nodes: &[crate::workflow::CompiledNode],
    position: usize,
    step: StepIdx,
) -> Result<&crate::workflow::CompiledNode, WorkflowError> {
    match nodes.get(position) {
        Some(node) => Ok(node),
        None => Err(WorkflowError::StepOutOfBounds { step }),
    }
}

/// Visits a single node during step counting and updates the total and stack.
#[allow(clippy::too_many_arguments)]
fn visit_node_for_total_steps(
    nodes: &[crate::workflow::CompiledNode],
    current: StepIdx,
    node_count: usize,
    visited: &mut [bool],
    jump_edges: &mut std::collections::HashSet<(u16, u16)>,
    in_path: &mut std::collections::HashSet<u16>,
    mut total: u64,
    stack: &mut Vec<StepIdx>,
) -> Result<u64, WorkflowError> {
    let idx = find_node_position(nodes, current, node_count)?;
    if visited.get(idx).copied() == Some(true) {
        return Ok(total);
    }
    let Some(flag) = visited.get_mut(idx) else {
        return Err(WorkflowError::StepOutOfBounds { step: current });
    };
    *flag = true;

    let node = node_at_position(nodes, idx, current)?;

    total = match total.checked_add(1) {
        Some(v) => v,
        None => return Err(WorkflowError::StepOutOfBounds { step: current }),
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
            .map_err(|e| {
                let actual = match e {
                    BudgetError::TotalStepsExceeded { actual, .. } => actual,
                    _ => u64::MAX,
                };
                WorkflowError::StepCountOverflow { actual }
            })?;
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
            .map_err(|e| {
                let actual = match e {
                    BudgetError::TotalStepsExceeded { actual, .. } => actual,
                    _ => u64::MAX,
                };
                WorkflowError::StepCountOverflow { actual }
            })?;
        }
        CompiledNodeKind::ReduceStart { body, done, .. } => {
            let iter_count = match u64::try_from(crate::limits::MAX_LIST_ITEMS_PER_VALUE) {
                Ok(value) => value,
                Err(_) => return Err(WorkflowError::StepCountOverflow { actual: u64::MAX }),
            };
            total = count_and_push_loop_body(
                nodes, *body, *done, iter_count, visited, node_count, total, stack,
            )
            .map_err(|e| {
                let actual = match e {
                    BudgetError::TotalStepsExceeded { actual, .. } => actual,
                    _ => u64::MAX,
                };
                WorkflowError::StepCountOverflow { actual }
            })?;
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
            .map_err(|e| {
                let actual = match e {
                    BudgetError::TotalStepsExceeded { actual, .. } => actual,
                    _ => u64::MAX,
                };
                WorkflowError::StepCountOverflow { actual }
            })?;
        }
        CompiledNodeKind::Jump { target } => {
            let from = current.get();
            let to = target.get();
            if in_path.contains(&to) {
                return Err(WorkflowError::JumpCycle {
                    step: current,
                    target: *target,
                });
            }
            if !jump_edges.insert((from, to)) {
                return Err(WorkflowError::JumpCycle {
                    step: current,
                    target: *target,
                });
            }
            in_path.insert(to);
            stack.push(*target);
        }
        _ => {
            push_successor_targets(&node.kind, stack);
            if let Some(next) = node.next {
                stack.push(next);
            }
        }
    }
    Ok(total)
}

/// Counts body region steps for a loop header and adds multiplied iterations to total.
#[inline]
#[allow(clippy::too_many_arguments)]
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
    let body_count = count_body_region_nodes(nodes, body, done, visited, node_count)?;
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
        && node.next.is_none()
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
fn count_body_region_nodes(
    nodes: &[crate::workflow::CompiledNode],
    body: StepIdx,
    done: StepIdx,
    global_visited: &mut [bool],
    node_count: usize,
) -> Result<u64, BudgetError> {
    let mut region_visited: Vec<bool> = vec![false; node_count];
    let mut stack: Vec<StepIdx> = Vec::new();
    stack.push(body);

    let mut count: u64 = 0;
    while let Some(current) = stack.pop() {
        count = visit_body_region_node(
            nodes,
            current,
            done,
            node_count,
            global_visited,
            &mut region_visited,
            &mut stack,
            count,
        )?;
    }
    let body_span = done.get().saturating_sub(body.get()).saturating_sub(1);
    Ok(count.max(u64::from(body_span)))
}

/// Visits a single node in a body region during step counting.
#[allow(clippy::too_many_arguments)]
fn visit_body_region_node(
    nodes: &[crate::workflow::CompiledNode],
    current: StepIdx,
    done: StepIdx,
    node_count: usize,
    global_visited: &mut [bool],
    region_visited: &mut [bool],
    stack: &mut Vec<StepIdx>,
    mut count: u64,
) -> Result<u64, BudgetError> {
    if current == done {
        return Ok(count);
    }
    let idx = find_node_position(nodes, current, node_count)?;
    if region_visited.get(idx).copied() == Some(true) {
        return Ok(count);
    }
    let Some(flag) = region_visited.get_mut(idx) else {
        return Err(WorkflowError::StepOutOfBounds { step: current }.into());
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
                    nodes,
                    *body,
                    *done,
                    u64::from(*limit).max(1),
                    global_visited,
                    node_count,
                    count,
                    stack,
                )?;
            }
        }
        CompiledNodeKind::CollectStart {
            limit, body, done, ..
        } => {
            if *body != current {
                count = count_nested_for_region(
                    nodes,
                    *body,
                    *done,
                    u64::from(*limit).max(1),
                    global_visited,
                    node_count,
                    count,
                    stack,
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
                count = count_nested_for_region(
                    nodes,
                    *body,
                    *done,
                    iter,
                    global_visited,
                    node_count,
                    count,
                    stack,
                )?;
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
                    nodes,
                    *body,
                    *done,
                    u64::from(*max_attempts).max(1),
                    global_visited,
                    node_count,
                    count,
                    stack,
                )?;
            }
        }
        _ => {
            push_successor_targets(&node.kind, stack);
            if let Some(next) = node.next {
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
    nodes: &[crate::workflow::CompiledNode],
    body: StepIdx,
    done: StepIdx,
    iter_count: u64,
    global_visited: &mut [bool],
    node_count: usize,
    count: u64,
    stack: &mut Vec<StepIdx>,
) -> Result<u64, BudgetError> {
    let body_count = count_body_region_nodes(nodes, body, done, global_visited, node_count)?;
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

fn branch_count_to_u16(count: usize) -> Result<u16, WorkflowError> {
    u16::try_from(count).map_err(|_| WorkflowError::StepCountOverflow {
        actual: u64::try_from(count).unwrap_or(u64::MAX),
    })
}

/// Computes max fanout and max nesting depth via a DFS walk.
#[allow(clippy::too_many_arguments)]
fn compute_fanout_and_depth(
    nodes: &[crate::workflow::CompiledNode],
    current: StepIdx,
    visited: &mut [bool],
    in_path: &mut std::collections::HashSet<u16>,
    node_count: usize,
    current_depth: u16,
    max_fanout: &mut u16,
    max_nesting_depth: &mut u16,
    max_action_tickets: &mut u32,
    max_parallel_in_flight: &mut u16,
    max_gather_pages: &mut u32,
    max_gather_items: &mut u32,
    max_for_each_iterations: &mut u32,
    max_together_branches: &mut u16,
    max_repeat_attempts: &mut u16,
) -> Result<(), WorkflowError> {
    let idx = find_node_position(nodes, current, node_count)?;
    if visited.get(idx).copied() == Some(true) {
        return Ok(());
    }
    let Some(flag) = visited.get_mut(idx) else {
        return Err(WorkflowError::StepOutOfBounds { step: current });
    };
    *flag = true;

    let node = node_at_position(nodes, idx, current)?;

    let current_u16 = current.get();
    in_path.insert(current_u16);

    if let CompiledNodeKind::Jump { target } = &node.kind {
        let target_u16 = target.get();
        if in_path.contains(&target_u16) {
            in_path.remove(&current_u16);
            return Err(WorkflowError::JumpCycle {
                step: current,
                target: *target,
            });
        }
    }

    let child_depth = compute_child_depth(&node.kind, current_depth, max_nesting_depth)?;
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
    )?;

    let mut targets: Vec<StepIdx> = Vec::new();
    push_successor_targets(&node.kind, &mut targets);
    if let Some(next) = node.next {
        targets.push(next);
    }

    for target in targets {
        if find_node_position(nodes, target, node_count).is_ok() {
            compute_fanout_and_depth(
                nodes,
                target,
                visited,
                in_path,
                node_count,
                child_depth,
                max_fanout,
                max_nesting_depth,
                max_action_tickets,
                max_parallel_in_flight,
                max_gather_pages,
                max_gather_items,
                max_for_each_iterations,
                max_together_branches,
                max_repeat_attempts,
            )?;
        }
    }

    Ok(())
}

fn compute_child_depth(
    kind: &CompiledNodeKind,
    current_depth: u16,
    max_nesting_depth: &mut u16,
) -> Result<u16, WorkflowError> {
    match kind {
        CompiledNodeKind::ForEachStart { .. }
        | CompiledNodeKind::ForEachNext { .. }
        | CompiledNodeKind::CollectStart { .. }
        | CompiledNodeKind::CollectPage { .. }
        | CompiledNodeKind::CollectNext { .. }
        | CompiledNodeKind::ReduceStart { .. }
        | CompiledNodeKind::ReduceNext { .. }
        | CompiledNodeKind::RepeatStart { .. }
        | CompiledNodeKind::RepeatAttempt { .. }
        | CompiledNodeKind::TogetherStart { .. }
        | CompiledNodeKind::TogetherBranch { .. } => {
            let new_depth = current_depth
                .checked_add(1)
                .ok_or(WorkflowError::StepCountOverflow { actual: u64::MAX })?;
            if new_depth > *max_nesting_depth {
                *max_nesting_depth = new_depth;
            }
            Ok(new_depth)
        }
        _ => Ok(current_depth),
    }
}

fn update_fanout(kind: &CompiledNodeKind, max_fanout: &mut u16) -> Result<(), WorkflowError> {
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
) -> Result<(), WorkflowError> {
    match kind {
        CompiledNodeKind::Do { .. } => {
            *max_action_tickets = max_action_tickets
                .checked_add(1)
                .ok_or(WorkflowError::StepCountOverflow { actual: u64::MAX })?;
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
        CompiledNodeKind::CollectStart { limit, .. } => {
            *max_gather_pages = max_gather_pages
                .checked_add(1)
                .ok_or(WorkflowError::StepCountOverflow { actual: u64::MAX })?;
            *max_gather_items = max_gather_items
                .checked_add(*limit)
                .ok_or(WorkflowError::StepCountOverflow { actual: u64::MAX })?;
        }
        CompiledNodeKind::ForEachStart { limit, .. } => {
            *max_for_each_iterations = max_for_each_iterations
                .checked_add(*limit)
                .ok_or(WorkflowError::StepCountOverflow { actual: u64::MAX })?;
        }
        CompiledNodeKind::RepeatStart { max_attempts, .. } => {
            *max_repeat_attempts = (*max_repeat_attempts).max(*max_attempts);
        }
        _ => {}
    }
    Ok(())
}

// Kani harnesses for PO-RUST-002-BUDGET-KANI: add_dim/sub_dim panic-freedom and correctness.
// Uses a minimal local error enum to avoid transitive inclusion of AggregateBudgetError
// -> WorkflowError -> Capability (Vec<Capability> causes deep drop_in_place unwind).
// add_dim/sub_dim are pure checked_add/checked_sub — we prove these directly.
#[cfg(kani)]
mod kani_harnesses {
    fn same_static_str(left: &'static str, right: &'static str) -> bool {
        left.len() == right.len() && core::ptr::eq(left.as_ptr(), right.as_ptr())
    }

    /// Minimal error enum mirroring the Overflow/Underflow variants.
    /// No transitive types — just &'static str for resource naming.
    enum LocalError {
        Overflow { resource: &'static str },
        Underflow { resource: &'static str },
    }

    /// Standalone add_dim — identical logic to budget.rs, no panics.
    fn add_dim(current: u64, requested: u64, resource: &'static str) -> Result<u64, LocalError> {
        current
            .checked_add(requested)
            .ok_or(LocalError::Overflow { resource })
    }

    /// Standalone sub_dim — identical logic to budget.rs, no panics.
    fn sub_dim(current: u64, requested: u64, resource: &'static str) -> Result<u64, LocalError> {
        current
            .checked_sub(requested)
            .ok_or(LocalError::Underflow { resource })
    }

    /// K-B1: add_dim is panic-free for bounded concrete inputs.
    #[kani::proof]
    fn add_dim_no_panic() {
        let vals: &[u64] = &[0, 1, 100, u64::MAX / 2, u64::MAX - 1, u64::MAX];
        let mut call_count = 0u64;
        for &current in vals {
            for &requested in vals {
                let result = add_dim(current, requested, "cpu");
                match &result {
                    Ok(_) => call_count += 1,
                    Err(_) => call_count += 1,
                }
                core::mem::forget(result);
            }
        }
        let total = (vals.len() as u64) * (vals.len() as u64);
        kani::cover!(call_count == total, "full cartesian product");
    }

    /// K-B2: sub_dim is panic-free for bounded concrete inputs.
    #[kani::proof]
    fn sub_dim_no_panic() {
        let vals: &[u64] = &[0, 1, 100, u64::MAX / 2, u64::MAX - 1, u64::MAX];
        let mut call_count = 0u64;
        for &current in vals {
            for &requested in vals {
                let result = sub_dim(current, requested, "disk");
                match &result {
                    Ok(_) => call_count += 1,
                    Err(_) => call_count += 1,
                }
                core::mem::forget(result);
            }
        }
        let total = (vals.len() as u64) * (vals.len() as u64);
        kani::cover!(call_count == total, "full cartesian product");
    }

    /// K-B3: add_dim(MAX, MAX) returns Err(Overflow).
    #[kani::proof]
    fn add_dim_max_plus_max_overflow() {
        let result = add_dim(u64::MAX, u64::MAX, "cpu");
        match result {
            Err(LocalError::Overflow { resource: "cpu" }) => {}
            Ok(_) => assert!(false, "MAX+MAX must overflow"),
            Err(_) => assert!(false, "only Overflow valid here"),
        }
    }

    /// K-B4: add_dim(0, 0) returns Ok(0).
    #[kani::proof]
    fn add_dim_zero_plus_zero() {
        let result = add_dim(0, 0, "cpu");
        match result {
            Ok(v) => assert!(v == 0, "0+0=0"),
            Err(_) => assert!(false, "0+0 cannot overflow"),
        }
    }

    /// K-B5: add_dim(1, MAX) returns Err(Overflow).
    #[kani::proof]
    fn add_dim_one_plus_max_overflow() {
        let result = add_dim(1, u64::MAX, "cpu");
        match result {
            Err(LocalError::Overflow { .. }) => {}
            Ok(_) => assert!(false, "1+MAX must overflow"),
            Err(_) => assert!(false, "only Overflow valid"),
        }
    }

    /// K-B6: sub_dim(0, 1) returns Err(Underflow).
    #[kani::proof]
    fn sub_dim_zero_minus_one_underflow() {
        let result = sub_dim(0, 1, "disk");
        match result {
            Err(LocalError::Underflow { .. }) => {}
            Ok(_) => assert!(false, "0-1 must underflow"),
            Err(_) => assert!(false, "only Underflow valid"),
        }
    }

    /// K-B7: sub_dim(100, 50) returns Ok(50).
    #[kani::proof]
    fn sub_dim_hundred_minus_fifty() {
        let result = sub_dim(100, 50, "disk");
        match result {
            Ok(v) => assert!(v == 50, "100-50=50"),
            Err(_) => assert!(false, "100-50 cannot underflow"),
        }
    }

    /// K-B8: add_dim non-overflow case (100+200=300).
    #[kani::proof]
    fn add_dim_non_overflow() {
        let result = add_dim(100, 200, "mem");
        match result {
            Ok(v) => assert!(v == 300, "100+200=300"),
            Err(_) => assert!(false, "100+200 cannot overflow"),
        }
    }

    /// K-B9: sub_dim non-underflow case (200-100=100).
    #[kani::proof]
    fn sub_dim_non_underflow() {
        let result = sub_dim(200, 100, "net");
        match result {
            Ok(v) => assert!(v == 100, "200-100=100"),
            Err(_) => assert!(false, "200-100 cannot underflow"),
        }
    }

    /// PO-010: aggregate usage addition rejects overflow and preserves exact sums.
    #[kani::proof]
    fn aggregate_usage_try_add_budget_rejects_overflow_and_sums_fields() {
        let usage = super::AggregateResourceUsage {
            max_steps_executable: 10,
            max_action_tickets: 20,
            max_parallel_in_flight: 30,
            max_gather_pages: 40,
            max_gather_items: 50,
            max_result_bytes: 60,
            max_total_slots_written: 70,
            max_active_runs: 80,
            max_queue_depth: 90,
            max_journal_batch_bytes: 100,
            max_step_budget_per_tick: 110,
            max_transitions_per_tick: 120,
        };
        let budget = super::AggregateResourceBudget {
            max_steps_executable: 1,
            max_action_tickets: 2,
            max_parallel_in_flight: 3,
            max_retries_per_action: 4,
            max_gather_pages: 5,
            max_gather_items: 6,
            max_for_each_iterations: 7,
            max_together_branches: 8,
            max_repeat_attempts: 9,
            max_run_time_seconds: 10,
            max_result_bytes: 11,
            max_total_slots_written: 12,
            max_queue_depth: 13,
            max_journal_batch_bytes: 14,
            max_step_budget_per_tick: 15,
            max_transitions_per_tick: 16,
        };

        let summed = usage.try_add_budget(&budget);
        match &summed {
            Ok(next) => {
                assert!(next.max_steps_executable == 11);
                assert!(next.max_action_tickets == 22);
                assert!(next.max_parallel_in_flight == 33);
                assert!(next.max_gather_pages == 45);
                assert!(next.max_gather_items == 56);
                assert!(next.max_result_bytes == 71);
                assert!(next.max_total_slots_written == 82);
                assert!(next.max_active_runs == 81);
                assert!(next.max_queue_depth == 103);
                assert!(next.max_journal_batch_bytes == 114);
                assert!(next.max_step_budget_per_tick == 125);
                assert!(next.max_transitions_per_tick == 136);
            }
            Err(_) => assert!(false),
        }
        core::mem::forget(summed);

        let overflowing = super::AggregateResourceUsage {
            max_steps_executable: u64::MAX,
            max_action_tickets: usage.max_action_tickets,
            max_parallel_in_flight: usage.max_parallel_in_flight,
            max_gather_pages: usage.max_gather_pages,
            max_gather_items: usage.max_gather_items,
            max_result_bytes: usage.max_result_bytes,
            max_total_slots_written: usage.max_total_slots_written,
            max_active_runs: usage.max_active_runs,
            max_queue_depth: usage.max_queue_depth,
            max_journal_batch_bytes: usage.max_journal_batch_bytes,
            max_step_budget_per_tick: usage.max_step_budget_per_tick,
            max_transitions_per_tick: usage.max_transitions_per_tick,
        };
        let result = overflowing.try_add_budget(&budget);
        match &result {
            Err(super::AggregateBudgetError::Overflow { resource }) => {
                assert!(same_static_str(resource, "max_steps_executable"));
            }
            Ok(_) => assert!(false),
            Err(_) => assert!(false),
        }
        core::mem::forget(result);
    }

    /// PO-011: aggregate capacity rejection reports exact resource/request/available.
    #[kani::proof]
    fn aggregate_usage_fits_within_rejects_over_capacity_fields() {
        let usage = super::AggregateResourceUsage {
            max_steps_executable: 11,
            max_action_tickets: 20,
            max_parallel_in_flight: 30,
            max_gather_pages: 40,
            max_gather_items: 50,
            max_result_bytes: 60,
            max_total_slots_written: 70,
            max_active_runs: 80,
            max_queue_depth: 90,
            max_journal_batch_bytes: 100,
            max_step_budget_per_tick: 110,
            max_transitions_per_tick: 120,
        };
        let capacity = super::AggregateResourceCapacity {
            max_steps_executable: 10,
            max_action_tickets: 20,
            max_parallel_in_flight: 30,
            max_gather_pages: 40,
            max_gather_items: 50,
            max_result_bytes: 60,
            max_total_slots_written: 70,
            max_active_runs: 80,
            max_queue_depth: 90,
            max_journal_batch_bytes: 100,
            max_step_budget_per_tick: 110,
            max_transitions_per_tick: 120,
        };

        let result = usage.fits_within(&capacity);
        match &result {
            Err(super::AggregateBudgetError::CapacityExceeded {
                resource,
                requested,
                available,
            }) => {
                assert!(same_static_str(resource, "max_steps_executable"));
                assert!(*requested == 11);
                assert!(*available == 10);
            }
            Ok(()) => assert!(false),
            Err(_) => assert!(false),
        }
        core::mem::forget(result);
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod vb_qi37_2_4_state8_tests;
