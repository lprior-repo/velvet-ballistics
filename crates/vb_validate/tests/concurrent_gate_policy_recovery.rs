#[path = "../../vb_core/tests/concurrent_together_support/mod.rs"]
mod support;

use vb_core::StepIdx;
use vb_core::execution_lanes::GroupMode;
use vb_core::workflow::{CompiledWorkflow, WorkflowError, validate_concurrent_structure};
use vb_validate::ValidationError;
use vb_validate::gates::validate_gate_11_loop_body_graph;

#[test]
fn public_structure_rejects_nested_together_with_constructor_policy_error() {
    let parts = support::pair(GroupMode::TogetherOrdered, GroupMode::TogetherOrdered);
    let expected = WorkflowError::NestedTogether {
        outer: StepIdx::new(0),
        inner: StepIdx::new(1),
    };
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(parts.clone()).map(|_| ()),
        Err(expected.clone())
    );
    assert_eq!(validate_concurrent_structure(&parts), Err(expected));
}

#[test]
fn gate_11_rejects_deep_nested_together_through_other_group_modes() {
    for middle in [GroupMode::FirstSuccess, GroupMode::ForEachOrdered] {
        let parts = support::deep(middle);
        let expected = WorkflowError::NestedTogether {
            outer: StepIdx::new(0),
            inner: StepIdx::new(2),
        };
        assert_eq!(
            CompiledWorkflow::try_from_concurrent_parts(parts.clone()).map(|_| ()),
            Err(expected.clone())
        );
        assert_eq!(
            validate_gate_11_loop_body_graph(&parts),
            Err(ValidationError::NodeKindConstraintViolation {
                node_index: 0,
                detail: expected.to_string(),
            })
        );
    }
}

#[test]
fn public_structure_and_gate_11_accept_legal_nested_modes() -> Result<(), Box<dyn std::error::Error>>
{
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
            CompiledWorkflow::try_from_concurrent_parts(parts.clone())?;
            assert_eq!(validate_concurrent_structure(&parts), Ok(()));
            assert_eq!(validate_gate_11_loop_body_graph(&parts), Ok(()));
        }
    }
    Ok(())
}
