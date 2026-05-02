#![forbid(unsafe_code)]

//! Whole-workflow budget computation and boundedness policy enforcement.

use crate::ids::StepIdx;
use crate::workflow::{CompiledNodeKind, ResourceContract, WorkflowError};

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
        let max_total_steps = count_total_steps(nodes, entry, &mut visited, node_count)?;

        visited.fill(false);
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

/// Counts the total number of reachable steps in the workflow by performing a
/// DFS walk from the entry node. Each node is counted once.
fn count_total_steps(
    nodes: &[crate::workflow::CompiledNode],
    entry: StepIdx,
    visited: &mut [bool],
    node_count: usize,
) -> Result<u64, WorkflowError> {
    let mut stack: Vec<StepIdx> = Vec::new();
    stack.push(entry);

    let mut total: u64 = 0;
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
        total = total.saturating_add(1);

        let node = match nodes.get(idx) {
            Some(n) => n,
            None => return Err(WorkflowError::StepOutOfBounds { step: current }),
        };
        push_successor_targets(&node.kind, &mut stack);
        if let Some(next) = node.next {
            stack.push(next);
        }
    }
    Ok(total)
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
    use crate::ids::{SlotIdx, StepIdx};
    use crate::workflow::{
        CompiledNode, CompiledNodeKind, ResourceContract, SlotBranch, WorkflowError,
    };

    #[test]
    fn budget_simple_linear_workflow() {
        let nodes = vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: Some(StepIdx::new(2)),
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
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
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: None,
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
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: Some(SlotIdx::new(4)),
                next: None,
                kind: CompiledNodeKind::ForEachJoin {
                    output: SlotIdx::new(4),
                },
            },
            CompiledNode {
                id: StepIdx::new(4),
                output: Some(SlotIdx::new(5)),
                next: None,
                kind: CompiledNodeKind::ForEachJoin {
                    output: SlotIdx::new(5),
                },
            },
            CompiledNode {
                id: StepIdx::new(5),
                output: None,
                next: None,
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
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(2),
                output: None,
                next: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(3),
                output: None,
                next: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(4),
                output: None,
                next: None,
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
}
