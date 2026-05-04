#![forbid(unsafe_code)]

//! Whole-workflow budget computation and boundedness policy enforcement.

use crate::ids::StepIdx;
use crate::workflow::{CompiledNodeKind, ResourceContract, WorkflowError};
use std::fmt;

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

        Ok(Self {
            max_total_steps,
            max_total_slots,
            max_fanout,
            max_nesting_depth,
            max_steps_executable: u32::try_from(max_total_steps).unwrap_or(u32::MAX),
            max_action_tickets,
            max_parallel_in_flight,
            max_retries_per_action: contract.max_retry_attempts,
            max_gather_pages,
            max_gather_items,
            max_for_each_iterations,
            max_together_branches,
            max_repeat_attempts,
            max_run_time_seconds: 0,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetError {
    /// Total step count exceeded the policy limit.
    TotalStepsExceeded {
        /// Actual total steps computed.
        actual: u64,
        /// Policy limit.
        limit: u64,
    },
    /// Total slot count exceeded the policy limit.
    TotalSlotsExceeded {
        /// Actual total slots computed.
        actual: u64,
        /// Policy limit.
        limit: u64,
    },
    /// Fanout exceeded the policy limit.
    FanoutExceeded {
        /// Actual fanout computed.
        actual: u16,
        /// Policy limit.
        limit: u16,
    },
    /// Nesting depth exceeded the policy limit.
    NestingDepthExceeded {
        /// Actual nesting depth computed.
        actual: u16,
        /// Policy limit.
        limit: u16,
    },
    /// Parallel in-flight exceeded the policy limit.
    ParallelExceeded {
        /// Actual parallel in-flight computed.
        actual: u16,
        /// Policy limit.
        limit: u16,
    },
    /// Action tickets exceeded the policy limit.
    ActionTicketsExceeded {
        /// Actual action tickets computed.
        actual: u32,
        /// Policy limit.
        limit: u32,
    },
    /// Run time exceeded the policy limit.
    RunTimeExceeded {
        /// Actual run time computed.
        actual: u64,
        /// Policy limit.
        limit: u64,
    },
    /// Result bytes exceeded the policy limit.
    ResultBytesExceeded {
        /// Actual result bytes computed.
        actual: u32,
        /// Policy limit.
        limit: u32,
    },
    /// Steps executable exceeded the policy limit.
    StepsExecutableExceeded {
        /// Actual steps executable computed.
        actual: u32,
        /// Policy limit.
        limit: u32,
    },
}

impl fmt::Display for BudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TotalStepsExceeded { actual, limit } => {
                write!(f, "total steps exceeded: {actual} > {limit}")
            }
            Self::TotalSlotsExceeded { actual, limit } => {
                write!(f, "total slots exceeded: {actual} > {limit}")
            }
            Self::FanoutExceeded { actual, limit } => {
                write!(f, "fanout exceeded: {actual} > {limit}")
            }
            Self::NestingDepthExceeded { actual, limit } => {
                write!(f, "nesting depth exceeded: {actual} > {limit}")
            }
            Self::ParallelExceeded { actual, limit } => {
                write!(f, "parallel exceeded: {actual} > {limit}")
            }
            Self::ActionTicketsExceeded { actual, limit } => {
                write!(f, "action tickets exceeded: {actual} > {limit}")
            }
            Self::RunTimeExceeded { actual, limit } => {
                write!(f, "run time exceeded: {actual} > {limit}")
            }
            Self::ResultBytesExceeded { actual, limit } => {
                write!(f, "result bytes exceeded: {actual} > {limit}")
            }
            Self::StepsExecutableExceeded { actual, limit } => {
                write!(f, "steps executable exceeded: {actual} > {limit}")
            }
        }
    }
}

impl std::error::Error for BudgetError {}

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
    let mut jump_edges: std::collections::HashSet<(u16, u16)> =
        std::collections::HashSet::new();
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
    let idx = current.as_usize();
    if idx >= node_count {
        return Err(WorkflowError::StepOutOfBounds { step: current });
    }
    if visited.get(idx).copied() == Some(true) {
        return Ok(total);
    }
    let Some(flag) = visited.get_mut(idx) else {
        return Err(WorkflowError::StepOutOfBounds { step: current });
    };
    *flag = true;

    let node = match nodes.get(idx) {
        Some(n) => n,
        None => return Err(WorkflowError::StepOutOfBounds { step: current }),
    };

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
            let iter_count =
                u64::try_from(crate::limits::MAX_LIST_ITEMS_PER_VALUE).unwrap_or(u64::MAX);
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
    stack.push(done);
    Ok(total)
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
    let done_idx = done.as_usize();
    let mut region_visited: Vec<bool> = vec![false; node_count];
    let mut stack: Vec<StepIdx> = Vec::new();
    stack.push(body);

    let mut count: u64 = 0;
    while let Some(current) = stack.pop() {
        count = visit_body_region_node(
            nodes,
            current,
            done_idx,
            node_count,
            global_visited,
            &mut region_visited,
            &mut stack,
            count,
        )?;
    }
    Ok(count)
}

/// Visits a single node in a body region during step counting.
#[allow(clippy::too_many_arguments)]
fn visit_body_region_node(
    nodes: &[crate::workflow::CompiledNode],
    current: StepIdx,
    done_idx: usize,
    node_count: usize,
    global_visited: &mut [bool],
    region_visited: &mut [bool],
    stack: &mut Vec<StepIdx>,
    mut count: u64,
) -> Result<u64, BudgetError> {
    let idx = current.as_usize();
    if idx >= node_count {
        return Err(WorkflowError::StepOutOfBounds { step: current }.into());
    }
    if idx == done_idx {
        return Ok(count);
    }
    if global_visited.get(idx).copied() == Some(true) {
        return Ok(count);
    }
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

    let node = match nodes.get(idx) {
        Some(n) => n,
        None => return Err(WorkflowError::StepOutOfBounds { step: current }.into()),
    };

    match &node.kind {
        CompiledNodeKind::ForEachStart {
            limit, body, done, ..
        } => {
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
        CompiledNodeKind::CollectStart {
            limit, body, done, ..
        } => {
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
        CompiledNodeKind::ReduceStart { body, done, .. } => {
            let iter = u64::try_from(crate::limits::MAX_LIST_ITEMS_PER_VALUE).unwrap_or(u64::MAX);
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
        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
            ..
        } => {
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
        _ => unreachable!("no-successor variants handled by early return"),
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
fn push_together_start_successors(
    branches: &[StepIdx],
    join: StepIdx,
    stack: &mut Vec<StepIdx>,
) {
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

/// Converts a usize branch count to u16, saturating at u16::MAX on overflow.
fn branch_count_to_u16(count: usize) -> u16 {
    u16::try_from(count).unwrap_or(u16::MAX)
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
    let idx = current.as_usize();
    if idx >= node_count {
        return Err(WorkflowError::StepOutOfBounds { step: current });
    }
    if visited.get(idx).copied() == Some(true) {
        return Ok(());
    }
    let Some(flag) = visited.get_mut(idx) else {
        return Err(WorkflowError::StepOutOfBounds { step: current });
    };
    *flag = true;

    let node = match nodes.get(idx) {
        Some(n) => n,
        None => return Err(WorkflowError::StepOutOfBounds { step: current }),
    };

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

    let child_depth = compute_child_depth(&node.kind, current_depth, max_nesting_depth);
    update_fanout(&node.kind, max_fanout);
    update_workflow_metrics(
        &node.kind,
        max_action_tickets,
        max_parallel_in_flight,
        max_gather_pages,
        max_gather_items,
        max_for_each_iterations,
        max_together_branches,
        max_repeat_attempts,
    );

    let mut targets: Vec<StepIdx> = Vec::new();
    push_successor_targets(&node.kind, &mut targets);
    if let Some(next) = node.next {
        targets.push(next);
    }

    for target in targets {
        let target_idx = target.as_usize();
        if target_idx < node_count {
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

/// Returns the depth to pass to children, updating max_nesting_depth if this
/// node is a nesting construct.
fn compute_child_depth(
    kind: &CompiledNodeKind,
    current_depth: u16,
    max_nesting_depth: &mut u16,
) -> u16 {
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
        | CompiledNodeKind::TogetherStart { .. } => {
            let new_depth = current_depth.saturating_add(1);
            if new_depth > *max_nesting_depth {
                *max_nesting_depth = new_depth;
            }
            new_depth
        }
        _ => current_depth,
    }
}

/// Updates max_fanout based on branching node kinds.
fn update_fanout(kind: &CompiledNodeKind, max_fanout: &mut u16) {
    match kind {
        CompiledNodeKind::TogetherStart { branches, .. } => {
            let branch_count = branch_count_to_u16(branches.len());
            if branch_count > *max_fanout {
                *max_fanout = branch_count;
            }
        }
        CompiledNodeKind::ChooseSlot { branches, .. } => {
            let branch_count = branch_count_to_u16(branches.len());
            if branch_count > *max_fanout {
                *max_fanout = branch_count;
            }
        }
        CompiledNodeKind::Choose { branches, .. } => {
            let branch_count = branch_count_to_u16(branches.len());
            if branch_count > *max_fanout {
                *max_fanout = branch_count;
            }
        }
        _ => {}
    }
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
) {
    match kind {
        CompiledNodeKind::Do { .. } => {
            *max_action_tickets = max_action_tickets.saturating_add(1);
        }
        CompiledNodeKind::TogetherStart { branches, .. } => {
            let branch_count = branch_count_to_u16(branches.len());
            if branch_count > *max_parallel_in_flight {
                *max_parallel_in_flight = branch_count;
            }
            if branch_count > *max_together_branches {
                *max_together_branches = branch_count;
            }
        }
        CompiledNodeKind::CollectStart { limit, .. } => {
            *max_gather_pages = max_gather_pages.saturating_add(1);
            *max_gather_items = max_gather_items.saturating_add(*limit);
        }
        CompiledNodeKind::ForEachStart { limit, .. } => {
            *max_for_each_iterations = max_for_each_iterations.saturating_add(*limit);
        }
        CompiledNodeKind::RepeatStart { max_attempts, .. } => {
            *max_repeat_attempts = (*max_repeat_attempts).max(*max_attempts);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests;

