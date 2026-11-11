// Verification artifact: proptest_collect_traversal.rs
// PO: PO-023 (Collect traversal termination)
// Verifier: proptest
// Command: cargo test -p vb_core -- proptest_collect_traversal --test-threads=1

#![cfg(test)]
#![forbid(unsafe_code)]

use crate::ids::StepIdx;
use crate::workflow::CompiledWorkflow;
use proptest::prelude::*;

use super::proptest_collect_budget::{expected_budget_total, make_collect_workflow};

fn bounded_collect_strategy() -> impl Strategy<Value = (u32, u32)> {
    (0..1000u32, 0..1000u32)
}

fn assert_collect_region(limit: u32, page_size: u32) -> Result<(), String> {
    let parts = make_collect_workflow(limit, page_size);
    let compiled = CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())?;
    let region = compiled
        .loop_region(StepIdx::new(2))
        .ok_or("loop region at control step 2 missing")?;
    assert_eq!(region.start(), StepIdx::ZERO);
    assert_eq!(region.body(), StepIdx::new(1));
    assert_eq!(region.control(), StepIdx::new(2));
    assert_eq!(region.done(), StepIdx::new(3));
    Ok(())
}

proptest! {
    #[test]
    fn proptest_collect_traversal((limit, page_size) in bounded_collect_strategy()) {
        assert_collect_region(limit, page_size).map_err(TestCaseError::fail)?;
    }
}

#[test]
fn proptest_collect_traversal_limit_zero() -> Result<(), String> {
    assert_collect_region(0, 1)
}

#[test]
fn proptest_collect_traversal_limit_one() -> Result<(), String> {
    assert_collect_region(1, 1)
}

#[test]
fn proptest_collect_traversal_limit_max() -> Result<(), String> {
    let parts = make_collect_workflow(u32::MAX, u32::MAX);
    match CompiledWorkflow::try_from_parts(parts) {
        Err(crate::workflow::WorkflowError::StepCountOverflow { actual }) => {
            assert_eq!(
                actual,
                expected_budget_total(u32::MAX).map_err(|error| error.to_string())?
            );
        }
        other => return Err(format!("expected StepCountOverflow, got {other:?}")),
    }
    Ok(())
}
