#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

//! Nesting-depth computation for budget traversal.
//!
//! Each depth-increasing node kind (loop headers, collect/reduce phases,
//! repeat attempts, together branches) increments the current depth by one.
//! The function guards against u16 overflow and records the maximum depth
//! seen so far.

use crate::workflow::CompiledNodeKind;

use super::traversal::BudgetTraversalError;

/// Computes the child depth for a node kind.
///
/// Depth-increasing node kinds (loops, collects, reduces, repeats, together)
/// return `current_depth + 1`, clamped to `u16::MAX`. All other kinds pass
/// `current_depth` through unchanged. On overflow the `max_nesting_depth`
/// output is not mutated and an error is returned.
pub(super) fn compute_child_depth(
    kind: &CompiledNodeKind,
    current_depth: u16,
    max_nesting_depth: &mut u16,
) -> Result<u16, BudgetTraversalError> {
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
                .ok_or(BudgetTraversalError::StepCountOverflow { actual: u64::MAX })?;
            if new_depth > *max_nesting_depth {
                *max_nesting_depth = new_depth;
            }
            Ok(new_depth)
        }
        _ => Ok(current_depth),
    }
}

// =========================================================================
// Tests for `compute_child_depth` — the private depth-increment function.
//
// Because this function is private (no `pub` qualifier) and the public API
// (`WholeWorkflowBudget::compute`) always seeds `current_depth` at 0, the
// u16::MAX overflow path cannot be reached through the public surface
// without constructing a 65 535-node workflow.  The unit tests below call
// the private function directly to prove the overflow guard is exercised
// and that `u16::MAX - 1` still advances to `u16::MAX` cleanly.
// =========================================================================

#[cfg(test)]
mod test {
    use crate::ids::{ConstIdx, SlotIdx, StepIdx};

    use super::*;

    /// B-BUDGET-TRAV-DEPTH-001: u16::MAX + ForEachStart overflows to
    /// `StepCountOverflow { actual: u64::MAX }` rather than wrapping.
    #[test]
    fn compute_child_depth_u16_max_overflows() {
        let kind = CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: 1,
            body: StepIdx::new(0),
            done: StepIdx::new(0),
        };
        let mut max_nesting_depth: u16 = 0;

        let result = compute_child_depth(&kind, u16::MAX, &mut max_nesting_depth);

        assert!(
            result.is_err(),
            "compute_child_depth(u16::MAX, ForEachStart) must overflow"
        );
        match result {
            Err(BudgetTraversalError::StepCountOverflow { actual }) => {
                assert_eq!(
                    actual,
                    u64::MAX,
                    "overflow must carry the sentinel u64::MAX as `actual`"
                );
                // max_nesting_depth must NOT be mutated on error.
                assert_eq!(
                    max_nesting_depth, 0,
                    "max_nesting_depth must remain unchanged after overflow"
                );
            }
            other => panic!("expected StepCountOverflow, got {:?}", other),
        }
    }

    /// B-BUDGET-TRAV-DEPTH-002: u16::MAX - 1 + ForEachStart → Ok(u16::MAX)
    /// and correctly records the new depth into `max_nesting_depth`.
    #[test]
    fn compute_child_depth_u16_max_minus_one_succeeds() {
        let kind = CompiledNodeKind::ForEachStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(1),
            limit: 1,
            body: StepIdx::new(0),
            done: StepIdx::new(0),
        };
        let mut max_nesting_depth: u16 = 0;

        let result = compute_child_depth(&kind, u16::MAX - 1, &mut max_nesting_depth);

        assert!(
            result.is_ok(),
            "u16::MAX - 1 must successfully increment to u16::MAX"
        );
        let new_depth = result.unwrap();
        assert_eq!(
            new_depth,
            u16::MAX,
            "depth should be u16::MAX after incrementing u16::MAX - 1"
        );
        assert_eq!(
            max_nesting_depth,
            u16::MAX,
            "max_nesting_depth must be updated to u16::MAX"
        );
    }

    /// B-BUDGET-TRAV-DEPTH-003: non-depth-increasing node types pass
    /// `current_depth` through unchanged regardless of its value.
    #[test]
    fn compute_child_depth_nop_passthrough() {
        let kind = CompiledNodeKind::Nop;
        let mut max_nesting_depth: u16 = 0;

        let result = compute_child_depth(&kind, u16::MAX, &mut max_nesting_depth);

        assert!(result.is_ok(), "Nop must return Ok at any depth");
        assert_eq!(result.unwrap(), u16::MAX);
        assert_eq!(max_nesting_depth, 0, "max_nesting_depth unchanged for Nop");
    }

    /// B-BUDGET-TRAV-DEPTH-004: Every depth-increasing node kind overflows
    /// at u16::MAX and succeeds at u16::MAX - 1.
    #[test]
    fn compute_child_depth_all_depth_increasing_kinds() {
        let kinds: Vec<CompiledNodeKind> = vec![
            // ForEachStart
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 1,
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // ForEachNext
            CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(0),
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // CollectStart
            CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 1,
                page_size: 1,
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // CollectPage
            CompiledNodeKind::CollectPage {
                collector_slot: SlotIdx::new(0),
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // CollectNext
            CompiledNodeKind::CollectNext {
                collector_slot: SlotIdx::new(0),
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // ReduceStart
            CompiledNodeKind::ReduceStart {
                input: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                initial: ConstIdx::new(0),
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // ReduceNext
            CompiledNodeKind::ReduceNext {
                iterator_slot: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // RepeatStart
            CompiledNodeKind::RepeatStart {
                max_attempts: 1,
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // RepeatAttempt
            CompiledNodeKind::RepeatAttempt {
                attempt_slot: SlotIdx::new(0),
                body: StepIdx::new(0),
                done: StepIdx::new(0),
            },
            // TogetherStart
            CompiledNodeKind::TogetherStart {
                branches: vec![StepIdx::new(0)].into_boxed_slice(),
                join: StepIdx::new(1),
            },
            // TogetherBranch
            CompiledNodeKind::TogetherBranch {
                branch: 0,
                entry: StepIdx::new(0),
                join: StepIdx::new(1),
                accumulator: SlotIdx::new(0),
            },
        ];

        for kind in &kinds {
            let mut max_nesting_depth: u16 = 0;
            let result = compute_child_depth(kind, u16::MAX, &mut max_nesting_depth);
            assert!(
                result.is_err(),
                "depth-increasing kind {:?} must overflow at u16::MAX",
                kind
            );
            match result {
                Err(BudgetTraversalError::StepCountOverflow { actual }) => {
                    assert_eq!(actual, u64::MAX);
                    assert_eq!(max_nesting_depth, 0);
                }
                other => panic!("expected StepCountOverflow for {:?}, got {:?}", kind, other),
            }

            // Re-test: u16::MAX - 1 should succeed for every kind.
            let mut max_nesting_depth2: u16 = 0;
            let result = compute_child_depth(kind, u16::MAX - 1, &mut max_nesting_depth2);
            assert!(
                result.is_ok(),
                "depth-increasing kind {:?} must succeed at u16::MAX - 1",
                kind
            );
            assert_eq!(result.unwrap(), u16::MAX);
            assert_eq!(max_nesting_depth2, u16::MAX);
        }
    }
}
