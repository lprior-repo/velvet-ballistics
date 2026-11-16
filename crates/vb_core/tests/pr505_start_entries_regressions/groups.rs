use super::fixtures::{self as f, Group, TestResult};
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNodeKind as K, CompiledWorkflow, ContractResource, WorkflowError};

#[test]
fn valid_singleton_and_distinct_groups_use_the_concurrent_constructor() -> TestResult {
    for kind in [Group::Together, Group::FirstSuccess] {
        for lanes in [1, 2, 3] {
            f::accepts(f::group(kind, lanes)?)?;
        }
    }
    Ok(())
}

#[test]
fn legacy_facade_is_not_an_acceptance_control_for_concurrent_graphs() -> TestResult {
    for kind in [Group::Together, Group::FirstSuccess] {
        assert_eq!(CompiledWorkflow::try_from_parts(f::group(kind, 1)?).map(|_| ()),
            Err(WorkflowError::ConcurrentProfileRequired { step: StepIdx::new(0) }));
    }
    Ok(())
}

#[test]
fn branch_count_at_fanout_is_accepted_and_next_count_is_rejected() -> TestResult {
    for kind in [Group::Together, Group::FirstSuccess] {
        let mut accepted = f::group(kind, 2)?;
        accepted.resource_contract.max_fanout = 2;
        f::accepts(accepted)?;
        let mut rejected = f::group(kind, 3)?;
        rejected.resource_contract.max_fanout = 2;
        f::rejects(rejected, WorkflowError::ResourceContractExceeded { resource: ContractResource::Fanout });
    }
    Ok(())
}

#[test]
fn empty_branch_table_is_a_typed_rejection() -> TestResult {
    for kind in [Group::Together, Group::FirstSuccess] {
        let mut parts = f::group(kind, 1)?;
        f::entries(&mut parts, &[])?;
        f::rejects(parts, WorkflowError::EmptyBranchTable);
    }
    Ok(())
}

#[test]
fn entries_at_start_join_and_node_count_keep_distinct_errors() -> TestResult {
    for kind in [Group::Together, Group::FirstSuccess] {
        for (entry, expected) in [
            (0, f::crossing(0)), (4, f::crossing(4)),
            (6, WorkflowError::StepOutOfBounds { step: StepIdx::new(6) }),
        ] {
            let mut parts = f::group(kind, 3)?;
            f::entries(&mut parts, &[entry])?;
            f::rejects(parts, expected);
        }
    }
    Ok(())
}

#[test]
fn nonadjacent_duplicate_names_the_start_control() -> TestResult {
    for kind in [Group::Together, Group::FirstSuccess] {
        let mut parts = f::group(kind, 3)?;
        f::entries(&mut parts, &[1, 2, 1])?;
        f::rejects(parts, WorkflowError::LoopRegionUnpaired { control: StepIdx::new(0) });
    }
    Ok(())
}

#[test]
fn later_invalid_entry_wins_over_an_earlier_duplicate() -> TestResult {
    for kind in [Group::Together, Group::FirstSuccess] {
        for (entry, expected) in [
            (6, WorkflowError::StepOutOfBounds { step: StepIdx::new(6) }),
            (4, f::crossing(4)),
        ] {
            let mut parts = f::group(kind, 3)?;
            f::entries(&mut parts, &[1, 1, entry])?;
            f::rejects(parts, expected);
        }
    }
    Ok(())
}

#[test]
fn common_output_bounds_precede_pair_and_entry_failures() -> TestResult {
    for kind in [Group::Together, Group::FirstSuccess] {
        let mut parts = f::group(kind, 2)?;
        f::entries(&mut parts, &[0])?;
        f::join(&mut parts, 5)?;
        f::node_mut(&mut parts, 0)?.output = Some(SlotIdx::new(1)).into();
        f::rejects(parts, WorkflowError::SlotOutOfBounds { slot: SlotIdx::new(1) });
    }
    Ok(())
}

#[test]
fn pair_bounds_precede_invalid_start_entries() -> TestResult {
    for kind in [Group::Together, Group::FirstSuccess] {
        let mut parts = f::group(kind, 2)?;
        f::entries(&mut parts, &[0])?;
        f::join(&mut parts, 5)?;
        f::rejects(parts, WorkflowError::StepOutOfBounds { step: StepIdx::new(5) });
    }
    Ok(())
}

#[test]
fn complete_graph_with_valid_start_entries_rejects_bad_lane_shape() -> TestResult {
    for kind in [Group::Together, Group::FirstSuccess] {
        let mut parts = f::group(kind, 2)?;
        f::node_mut(&mut parts, 1)?.output = Some(SlotIdx::new(0)).into();
        f::rejects(parts, WorkflowError::LoopRegionUnpaired { control: StepIdx::new(1) });
    }
    Ok(())
}

#[test]
fn complete_graph_with_valid_start_entries_rejects_lane_result_at_slot_count() -> TestResult {
    for kind in [Group::Together, Group::FirstSuccess] {
        let mut parts = f::group(kind, 2)?;
        f::node_mut(&mut parts, 1)?.kind = K::ConcurrentLaneEnd {
            owner: StepIdx::new(0), result: Some(SlotIdx::new(1)),
        };
        f::rejects(parts, WorkflowError::SlotOutOfBounds { slot: SlotIdx::new(1) });
    }
    Ok(())
}
