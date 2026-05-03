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
}

impl WholeWorkflowBudget {
    /// Walks the compiled IR starting from `entry` and computes the four
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
        let max_total_steps = count_total_steps(nodes, entry, node_count)?;

        let mut max_fanout: u16 = 0;
        let mut max_nesting_depth: u16 = 0;
        compute_fanout_and_depth(
            nodes,
            entry,
            &mut visited,
            node_count,
            0,
            &mut max_fanout,
            &mut max_nesting_depth,
        )?;

        let max_total_slots = u64::from(contract.max_slots);

        Ok(Self {
            max_total_steps,
            max_total_slots,
            max_fanout,
            max_nesting_depth,
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
}

impl BoundednessPolicy {
    /// Conservative default policy.
    pub const DEFAULT: Self = Self {
        max_total_steps: 1_000_000,
        max_total_slots: 65_535,
        max_fanout: 64,
        max_nesting_depth: 8,
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
        }
    }
}

impl std::error::Error for BudgetError {}

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
    let mut total: u64 = 0;

    // Phase 1: Walk the IR and compute worst-case steps. Loop headers trigger
    // a body sub-count that is multiplied by the iteration limit.
    let mut stack: Vec<StepIdx> = Vec::new();
    stack.push(entry);

    while let Some(current) = stack.pop() {
        let idx = current.as_usize();
        if idx >= node_count {
            return Err(WorkflowError::StepOutOfBounds { step: current });
        }
        if visited.get(idx).copied() == Some(true) {
            continue;
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
                limit,
                body,
                done,
                ..
            } => {
                let body_count =
                    count_body_region_nodes(nodes, *body, *done, &mut visited, node_count)?;
                let iter_count = u64::from(*limit).max(1);
                total = total.saturating_add(body_count.saturating_mul(iter_count));
                stack.push(*done);
            }
            CompiledNodeKind::CollectStart {
                limit,
                body,
                done,
                ..
            } => {
                let body_count =
                    count_body_region_nodes(nodes, *body, *done, &mut visited, node_count)?;
                let iter_count = u64::from(*limit).max(1);
                total = total.saturating_add(body_count.saturating_mul(iter_count));
                stack.push(*done);
            }
            CompiledNodeKind::ReduceStart { body, done, .. } => {
                let body_count =
                    count_body_region_nodes(nodes, *body, *done, &mut visited, node_count)?;
                // ReduceStart has no explicit limit. Use MAX_LIST_ITEMS_PER_VALUE
                // as a conservative upper bound for the input list size.
                let iter_count =
                    u64::try_from(crate::limits::MAX_LIST_ITEMS_PER_VALUE).unwrap_or(u64::MAX);
                total = total.saturating_add(body_count.saturating_mul(iter_count));
                stack.push(*done);
            }
            CompiledNodeKind::RepeatStart {
                max_attempts,
                body,
                done,
            } => {
                let body_count =
                    count_body_region_nodes(nodes, *body, *done, &mut visited, node_count)?;
                let iter_count = u64::from(*max_attempts).max(1);
                total = total.saturating_add(body_count.saturating_mul(iter_count));
                stack.push(*done);
            }
            _ => {
                push_successor_targets(&node.kind, &mut stack);
                if let Some(next) = node.next {
                    stack.push(next);
                }
            }
        }
    }
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
) -> Result<u64, WorkflowError> {
    let done_idx = done.as_usize();
    let mut region_visited: Vec<bool> = vec![false; node_count];
    let mut stack: Vec<StepIdx> = Vec::new();
    stack.push(body);

    let mut count: u64 = 0;
    while let Some(current) = stack.pop() {
        let idx = current.as_usize();
        if idx >= node_count {
            return Err(WorkflowError::StepOutOfBounds { step: current });
        }
        // Stop at the done exit node -- it's not part of the body
        if idx == done_idx {
            continue;
        }
        if global_visited.get(idx).copied() == Some(true) {
            continue;
        }
        if region_visited.get(idx).copied() == Some(true) {
            continue;
        }
        let Some(flag) = region_visited.get_mut(idx) else {
            return Err(WorkflowError::StepOutOfBounds { step: current });
        };
        *flag = true;

        count = count.saturating_add(1);

        let node = match nodes.get(idx) {
            Some(n) => n,
            None => return Err(WorkflowError::StepOutOfBounds { step: current }),
        };

        // Recursively handle nested loop headers within the body region
        match &node.kind {
            CompiledNodeKind::ForEachStart {
                limit,
                body: inner_body,
                done: inner_done,
                ..
            } => {
                let inner_body_count = count_body_region_nodes(
                    nodes,
                    *inner_body,
                    *inner_done,
                    global_visited,
                    node_count,
                )?;
                let iter_count = u64::from(*limit).max(1);
                count = count.saturating_add(inner_body_count.saturating_mul(iter_count));
                // Continue walking from inner_done within the outer body
                stack.push(*inner_done);
            }
            CompiledNodeKind::CollectStart {
                limit,
                body: inner_body,
                done: inner_done,
                ..
            } => {
                let inner_body_count = count_body_region_nodes(
                    nodes,
                    *inner_body,
                    *inner_done,
                    global_visited,
                    node_count,
                )?;
                let iter_count = u64::from(*limit).max(1);
                count = count.saturating_add(inner_body_count.saturating_mul(iter_count));
                stack.push(*inner_done);
            }
            CompiledNodeKind::ReduceStart {
                body: inner_body,
                done: inner_done,
                ..
            } => {
                let inner_body_count = count_body_region_nodes(
                    nodes,
                    *inner_body,
                    *inner_done,
                    global_visited,
                    node_count,
                )?;
                let iter_count =
                    u64::try_from(crate::limits::MAX_LIST_ITEMS_PER_VALUE).unwrap_or(u64::MAX);
                count = count.saturating_add(inner_body_count.saturating_mul(iter_count));
                stack.push(*inner_done);
            }
            CompiledNodeKind::RepeatStart {
                max_attempts,
                body: inner_body,
                done: inner_done,
            } => {
                let inner_body_count = count_body_region_nodes(
                    nodes,
                    *inner_body,
                    *inner_done,
                    global_visited,
                    node_count,
                )?;
                let iter_count = u64::from(*max_attempts).max(1);
                count = count.saturating_add(inner_body_count.saturating_mul(iter_count));
                stack.push(*inner_done);
            }
            _ => {
                push_successor_targets(&node.kind, &mut stack);
                if let Some(next) = node.next {
                    stack.push(next);
                }
            }
        }
    }
    Ok(count)
}

/// Pushes all successor StepIdx targets from a node kind onto the stack,
/// excluding the `next` field which is handled separately.
#[allow(clippy::match_same_arms)]
fn push_successor_targets(kind: &CompiledNodeKind, stack: &mut Vec<StepIdx>) {
    match kind {
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
        | CompiledNodeKind::Finish { .. } => {}
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            for branch in branches.as_ref() {
                stack.push(branch.target);
            }
            if let Some(fallback) = *otherwise {
                stack.push(fallback);
            }
        }
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            for branch in branches.as_ref() {
                stack.push(branch.target);
            }
            if let Some(fallback) = *otherwise {
                stack.push(fallback);
            }
        }
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
        } => {
            stack.push(*body);
            stack.push(*done);
        }
        CompiledNodeKind::RepeatCheck { done, .. } => {
            stack.push(*done);
        }
        CompiledNodeKind::TogetherStart { branches, join } => {
            for branch in branches.as_ref() {
                stack.push(*branch);
            }
            stack.push(*join);
        }
        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            stack.push(*entry);
            stack.push(*join);
        }
        CompiledNodeKind::TogetherJoin { .. } => {}
        CompiledNodeKind::WaitEvent { .. } => {}
        CompiledNodeKind::ErrorHandler { body, handler } => {
            stack.push(*body);
            stack.push(*handler);
        }
        CompiledNodeKind::Jump { target } => {
            stack.push(*target);
        }
    }
}

/// Converts a usize branch count to u16, saturating at u16::MAX on overflow.
fn branch_count_to_u16(count: usize) -> u16 {
    u16::try_from(count).unwrap_or(u16::MAX)
}

/// Computes max fanout and max nesting depth via a DFS walk.
fn compute_fanout_and_depth(
    nodes: &[crate::workflow::CompiledNode],
    current: StepIdx,
    visited: &mut [bool],
    node_count: usize,
    current_depth: u16,
    max_fanout: &mut u16,
    max_nesting_depth: &mut u16,
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

    let child_depth = compute_child_depth(&node.kind, current_depth, max_nesting_depth);
    update_fanout(&node.kind, max_fanout);

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
                node_count,
                child_depth,
                max_fanout,
                max_nesting_depth,
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

#[cfg(test)]
mod tests {
    use super::{BoundednessPolicy, BudgetError, WholeWorkflowBudget};
    use crate::engine::StepBudget;
    use crate::ids::{ExprIdx, SlotIdx, StepIdx};
    use crate::workflow::{
        CompiledNode, CompiledNodeKind, ExprBranch, ResourceContract, SlotBranch, WorkflowError,
    };

    #[test]
    fn budget_simple_linear_workflow() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ];
        let contract = test_contract(3, 1);
        let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
            .ok()
            .filter(|b| {
                b.max_total_steps == 3
                    && b.max_fanout == 0
                    && b.max_nesting_depth == 0
            });

        assert!(budget.is_some(), "linear workflow budget mismatch");
    }

    #[test]
    fn budget_branching_workflow() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ChooseSlot {
                    branches: vec![
                        SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(1),
                        },
                        SlotBranch {
                            condition: SlotIdx::new(0),
                            target: StepIdx::new(2),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise: Some(StepIdx::new(3)),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ];
        let contract = test_contract(4, 1);
        let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
            .ok()
            .filter(|b| b.max_total_steps == 4 && b.max_fanout == 2);

        assert!(budget.is_some(), "branching workflow budget mismatch");
    }

    #[test]
    fn budget_nested_loop_depth() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 10,
                    body: StepIdx::new(1),
                    done: StepIdx::new(4),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(2),
                    item_slot: SlotIdx::new(3),
                    limit: 10,
                    body: StepIdx::new(2),
                    done: StepIdx::new(3),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: Some(SlotIdx::new(4)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachJoin {
                    output: SlotIdx::new(4),
                },
            },
            CompiledNode {
                id: StepIdx::new(4),
                output: Some(SlotIdx::new(5)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachJoin {
                    output: SlotIdx::new(5),
                },
            },
            CompiledNode {
                id: StepIdx::new(5),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(4),
                },
            },
        ];
        let contract = test_contract(6, 6);
        let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
            .ok()
            .filter(|b| b.max_nesting_depth == 2);

        assert!(budget.is_some(), "nested loop depth mismatch");
    }

    #[test]
    fn budget_rejects_excessive_steps() {
        let budget = WholeWorkflowBudget {
            max_total_steps: 3,
            max_total_slots: 10,
            max_fanout: 1,
            max_nesting_depth: 0,
        };
        let policy = BoundednessPolicy {
            max_total_steps: 2,
            max_total_slots: 10,
            max_fanout: 64,
            max_nesting_depth: 8,
        };

        match policy.validate(&budget) {
            Err(BudgetError::TotalStepsExceeded { actual: 3, limit: 2 }) => {}
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn budget_rejects_excessive_fanout() {
        let budget = WholeWorkflowBudget {
            max_total_steps: 1,
            max_total_slots: 10,
            max_fanout: 3,
            max_nesting_depth: 0,
        };
        let policy = BoundednessPolicy {
            max_total_steps: 1_000_000,
            max_total_slots: 65_535,
            max_fanout: 2,
            max_nesting_depth: 8,
        };

        match policy.validate(&budget) {
            Err(BudgetError::FanoutExceeded { actual: 3, limit: 2 }) => {}
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn budget_accepts_within_policy() {
        let budget = WholeWorkflowBudget {
            max_total_steps: 10,
            max_total_slots: 100,
            max_fanout: 4,
            max_nesting_depth: 2,
        };
        let result = BoundednessPolicy::DEFAULT.validate(&budget);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn budget_rejects_excessive_nesting_depth() {
        let budget = WholeWorkflowBudget {
            max_total_steps: 1,
            max_total_slots: 10,
            max_fanout: 1,
            max_nesting_depth: 10,
        };
        let policy = BoundednessPolicy {
            max_total_steps: 1_000_000,
            max_total_slots: 65_535,
            max_fanout: 64,
            max_nesting_depth: 4,
        };

        match policy.validate(&budget) {
            Err(BudgetError::NestingDepthExceeded { actual: 10, limit: 4 }) => {}
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn budget_rejects_excessive_total_slots() {
        let budget = WholeWorkflowBudget {
            max_total_steps: 1,
            max_total_slots: 200_000,
            max_fanout: 1,
            max_nesting_depth: 0,
        };
        let policy = BoundednessPolicy {
            max_total_steps: 1_000_000,
            max_total_slots: 65_535,
            max_fanout: 64,
            max_nesting_depth: 8,
        };

        match policy.validate(&budget) {
            Err(BudgetError::TotalSlotsExceeded {
                actual: 200_000,
                limit: 65_535,
            }) => {}
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn budget_default_policy_accepts_reasonable_budget() {
        let budget = WholeWorkflowBudget {
            max_total_steps: 500_000,
            max_total_slots: 10_000,
            max_fanout: 32,
            max_nesting_depth: 4,
        };
        let result = BoundednessPolicy::DEFAULT.validate(&budget);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn budget_compute_rejects_entry_out_of_bounds() {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }];
        let contract = test_contract(1, 0);
        let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(5), &contract);

        match result {
            Err(WorkflowError::EntryOutOfBounds { entry }) if entry == StepIdx::new(5) => {}
            other => panic!("unexpected result: {other:?}"),
        }
    }

    #[test]
    fn budget_together_start_fanout() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::TogetherStart {
                    branches: vec![StepIdx::new(1), StepIdx::new(2), StepIdx::new(3)]
                        .into_boxed_slice(),
                    join: StepIdx::new(4),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(4),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ];
        let contract = test_contract(5, 1);
        let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
            .ok()
            .filter(|b| b.max_fanout == 3 && b.max_total_steps == 5);

        assert!(budget.is_some(), "together start fanout mismatch");
    }

    const fn test_contract(max_steps: u16, max_slots: u16) -> ResourceContract {
        ResourceContract {
            max_steps,
            max_slots,
            max_constants: 1,
            max_accessors: 0,
            max_expressions: 0,
            max_expr_stack: 0,
            max_step_budget_per_tick: 1,
            max_input_bytes: 1,
            max_output_bytes: 1,
            max_blob_bytes: 1,
            max_ipc_payload_bytes: 1,
            max_retry_attempts: 0,
            max_fanout: 64,
            max_collect_items: 0,
            max_queue_depth: 1,
            max_journal_batch_bytes: 1,
        }
    }

    #[test]
    fn budget_single_node_workflow() {
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }];
        let contract = test_contract(1, 1);
        let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
            .ok()
            .filter(|b| {
                b.max_total_steps == 1
                    && b.max_fanout == 0
                    && b.max_nesting_depth == 0
            });

        assert!(budget.is_some(), "single-node workflow budget mismatch");
    }

    #[test]
    fn budget_error_display_formatting() {
        let err = BudgetError::TotalStepsExceeded {
            actual: 5,
            limit: 3,
        };
        assert_eq!(format!("{err}"), "total steps exceeded: 5 > 3");

        let err = BudgetError::TotalSlotsExceeded {
            actual: 200,
            limit: 100,
        };
        assert_eq!(format!("{err}"), "total slots exceeded: 200 > 100");

        let err = BudgetError::FanoutExceeded {
            actual: 10,
            limit: 4,
        };
        assert_eq!(format!("{err}"), "fanout exceeded: 10 > 4");

        let err = BudgetError::NestingDepthExceeded {
            actual: 16,
            limit: 8,
        };
        assert_eq!(format!("{err}"), "nesting depth exceeded: 16 > 8");
    }

    #[test]
    fn budget_step_count_overflow_detected() {
        // Construct a workflow where a node's next points out of bounds,
        // verifying error propagation through the count path.
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(99)), // out of bounds
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }];
        let contract = test_contract(1, 0);
        let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
        match result {
            Err(WorkflowError::StepOutOfBounds { .. }) => {}
            other => panic!("expected StepOutOfBounds, got {other:?}"),
        }
    }

    #[test]
    fn budget_empty_nodes_rejected() {
        let nodes: Vec<CompiledNode> = vec![];
        let contract = test_contract(0, 0);
        let result = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract);
        match result {
            Err(WorkflowError::EntryOutOfBounds { .. }) => {}
            other => panic!("expected EntryOutOfBounds for empty nodes, got {other:?}"),
        }
    }

    #[test]
    fn budget_choose_fanout_counted() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Choose {
                    branches: vec![
                        ExprBranch {
                            condition: ExprIdx::new(0),
                            target: StepIdx::new(1),
                        },
                        ExprBranch {
                            condition: ExprIdx::new(0),
                            target: StepIdx::new(2),
                        },
                    ]
                    .into_boxed_slice(),
                    otherwise: None,
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ];
        let contract = test_contract(3, 1);
        let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
            .ok()
            .filter(|b| b.max_fanout == 2 && b.max_total_steps == 3);

        assert!(budget.is_some(), "choose fanout budget mismatch");
    }

    // =========================================================================
    // Security regression tests: loop-aware step counting
    // =========================================================================

    /// Verifies that a ForEachStart loop multiplies body steps by the limit.
    /// Workflow: ForEachStart(limit=5, body=1, done=2) -> Nop -> Finish
    /// Expected: 1 (header) + 5 * 1 (body * iterations) + 1 (Finish) = 7
    #[test]
    fn budget_foreach_loop_multiplies_body_steps() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 5,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ];
        let contract = test_contract(3, 3);
        let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
            .ok()
            .filter(|b| b.max_total_steps == 7);

        assert!(
            budget.is_some(),
            "for-each loop should multiply body steps by limit"
        );
    }

    /// Verifies that a RepeatStart loop multiplies body steps by max_attempts.
    /// Workflow: RepeatStart(max=3, body=1, done=2) -> Nop -> Finish
    /// Expected: 1 (header) + 3 * 1 (body * attempts) + 1 (Finish) = 5
    #[test]
    fn budget_repeat_loop_multiplies_body_steps() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::RepeatStart {
                    max_attempts: 3,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ];
        let contract = test_contract(3, 3);
        let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
            .ok()
            .filter(|b| b.max_total_steps == 5);

        assert!(
            budget.is_some(),
            "repeat loop should multiply body steps by max_attempts"
        );
    }

    /// Verifies nested loop step counting multiplies correctly.
    /// Outer ForEachStart(limit=10, body=1, done=4)
    ///   Inner ForEachStart(limit=10, body=2, done=3)
    ///     Nop (node 2)
    ///   ForEachJoin (node 3)
    /// ForEachJoin (node 4)
    ///
    /// Inner body: 1 (Nop), multiplied by 10 = 10. Inner loop = 1 + 10 + 1 (Join) = 12.
    /// Outer body region: 1 (inner header) + 10 (inner body*iter) + 1 (inner Join) = 12.
    ///   Wait, inner header counted as 1 in body, then inner body_count = 1, * 10 = 10.
    ///   Then inner done (ForEachJoin) = 1. Total body region = 1 + 10 + 1 = 12.
    /// Outer body * 10 = 12 * 10 = 120. Outer header = 1.
    /// Outer done (ForEachJoin) = 1.
    /// Total = 1 + 120 + 1 = 122.
    #[test]
    fn budget_nested_loop_multiplies_correctly() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 10,
                    body: StepIdx::new(1),
                    done: StepIdx::new(4),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(2),
                    item_slot: SlotIdx::new(3),
                    limit: 10,
                    body: StepIdx::new(2),
                    done: StepIdx::new(3),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: Some(SlotIdx::new(4)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachJoin {
                    output: SlotIdx::new(4),
                },
            },
            CompiledNode {
                id: StepIdx::new(4),
                output: Some(SlotIdx::new(5)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachJoin {
                    output: SlotIdx::new(5),
                },
            },
        ];
        let contract = test_contract(5, 6);
        let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
            .ok()
            .filter(|b| b.max_total_steps == 122 && b.max_nesting_depth == 2);

        assert!(
            budget.is_some(),
            "nested loop should multiply step counts at each nesting level"
        );
    }

    /// Security regression: a workflow with loops that previously appeared as a
    /// small step count (unique nodes only) now correctly reports the worst-case
    /// multiplied count, which should exceed the default policy.
    ///
    /// ForEachStart(limit=1000, body=1, done=2) -> Nop -> Finish
    /// Old: 3 steps (passed policy).
    /// New: 1 + 1000*1 + 1 = 1002 steps (still under 1M policy, but realistic).
    #[test]
    fn budget_large_loop_counted_realistically() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::ForEachStart {
                    input: SlotIdx::new(0),
                    item_slot: SlotIdx::new(1),
                    limit: 1000,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ];
        let contract = test_contract(3, 3);
        let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
            .ok()
            .filter(|b| b.max_total_steps == 1002);

        assert!(
            budget.is_some(),
            "large loop should count 1002 steps not 3"
        );

        // The default policy (1M steps) should accept this
        let budget_val = budget.as_ref().unwrap();
        assert!(
            BoundednessPolicy::DEFAULT.validate(budget_val).is_ok(),
            "1002 steps should be within default policy"
        );
    }

    /// Verifies that a CollectStart loop multiplies body steps by the limit.
    #[test]
    fn budget_collect_loop_multiplies_body_steps() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::CollectStart {
                    source: SlotIdx::new(0),
                    limit: 5,
                    page_size: 2,
                    body: StepIdx::new(1),
                    done: StepIdx::new(2),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ];
        let contract = test_contract(3, 3);
        let budget = WholeWorkflowBudget::compute(&nodes, StepIdx::new(0), &contract)
            .ok()
            .filter(|b| b.max_total_steps == 7);

        assert!(
            budget.is_some(),
            "collect loop should multiply body steps by limit"
        );
    }

    /// Verifies that StepBudget clamps values above MAX_STEP_BUDGET.
    #[test]
    fn step_budget_clamps_above_max() {
        let budget = StepBudget::new(crate::limits::MAX_STEP_BUDGET + 100);
        assert_eq!(
            budget.remaining(),
            crate::limits::MAX_STEP_BUDGET,
            "budget should be clamped to MAX_STEP_BUDGET"
        );
    }

    /// Verifies that StepBudget::MAX equals MAX_STEP_BUDGET.
    #[test]
    fn step_budget_max_equals_limit() {
        assert_eq!(
            StepBudget::MAX.remaining(),
            crate::limits::MAX_STEP_BUDGET,
            "StepBudget::MAX should equal MAX_STEP_BUDGET"
        );
    }

    /// Verifies that StepBudget zero budget exhausts immediately.
    #[test]
    fn step_budget_zero_exhausts_immediately() {
        let mut budget = StepBudget::new(0);
        let result = budget.try_take();
        assert!(
            result.is_ok() && result.as_ref().map_err(|_| "").unwrap() == &false,
            "zero budget should return Ok(false) immediately"
        );
    }
}
