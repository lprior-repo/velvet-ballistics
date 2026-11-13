//! R86 nested Together policy — any depth through any concurrent group mode.
//!
//! The policy is enforced by the checked constructor itself. This suite
//! validates that the constructor rejects nested Together and that other
//! mode combinations still accept.

mod concurrent_together_support;
use concurrent_together_support as support;
use vb_core::execution_lanes::GroupMode;
use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledWorkflow, WorkflowError};

#[test]
fn checked_constructor_rejects_nested_concurrent_together() {
    let parts = support::pair(GroupMode::TogetherOrdered, GroupMode::TogetherOrdered);
    let result = CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ());
    assert_eq!(
        result,
        Err(WorkflowError::NestedTogether {
            outer: StepIdx::new(0),
            inner: StepIdx::new(1),
        })
    );
}

#[test]
fn any_depth_through_first_success_or_foreach_is_rejected() {
    for middle in [GroupMode::FirstSuccess, GroupMode::ForEachOrdered] {
        let parts = support::deep(middle);
        let expected = WorkflowError::NestedTogether {
            outer: StepIdx::new(0),
            inner: StepIdx::new(2),
        };
        assert_eq!(
            CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ()),
            Err(expected)
        );
    }
}

#[test]
fn other_nested_modes_keep_real_c1_acceptance() -> Result<(), Box<dyn std::error::Error>> {
    for outer in [
        GroupMode::TogetherOrdered,
        GroupMode::FirstSuccess,
        GroupMode::ForEachOrdered,
    ] {
        for inner in [
            GroupMode::TogetherOrdered,
            GroupMode::FirstSuccess,
            GroupMode::ForEachOrdered,
        ] {
            if outer == GroupMode::TogetherOrdered && inner == GroupMode::TogetherOrdered {
                continue;
            }
            let parts = support::pair(outer, inner);
            CompiledWorkflow::try_from_concurrent_parts(parts)?;
        }
    }
    Ok(())
}
