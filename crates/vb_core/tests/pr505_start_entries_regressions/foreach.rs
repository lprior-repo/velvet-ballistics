use super::fixtures::{self as f, TestResult};
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{ContractResource, WorkflowError};

#[test]
fn foreach_limit_and_parallelism_accept_inclusive_boundaries() -> TestResult {
    for limit in [0, 1, 65_535] {
        for at_once in [1, 2] {
            f::accepts(f::foreach(0, 1, limit, at_once))?;
        }
    }
    Ok(())
}

#[test]
fn foreach_limit_above_the_integer_bound_is_typed() {
    f::rejects(
        f::foreach(0, 1, 65_536, 1),
        WorkflowError::ResourceContractExceeded {
            resource: ContractResource::ListItems,
        },
    );
}

#[test]
fn foreach_zero_and_over_fanout_parallelism_are_distinct() {
    for (parallelism, resource) in [
        (0, ContractResource::BranchCount),
        (3, ContractResource::Fanout),
    ] {
        f::rejects(
            f::foreach(0, 1, 1, parallelism),
            WorkflowError::ResourceContractExceeded { resource },
        );
    }
}

#[test]
fn foreach_slot_equality_and_first_slot_error_are_exact() {
    for (input, item, expected) in [(3, 1, 3), (0, 3, 3), (4, 3, 4)] {
        f::rejects(
            f::foreach(input, item, 1, 1),
            WorkflowError::SlotOutOfBounds {
                slot: SlotIdx::new(expected),
            },
        );
    }
}

#[test]
fn foreach_slot_and_limit_errors_precede_parallelism() {
    f::rejects(
        f::foreach(4, 3, 65_536, 0),
        WorkflowError::SlotOutOfBounds {
            slot: SlotIdx::new(4),
        },
    );
    f::rejects(
        f::foreach(0, 3, 65_536, 0),
        WorkflowError::SlotOutOfBounds {
            slot: SlotIdx::new(3),
        },
    );
    f::rejects(
        f::foreach(0, 1, 65_536, 0),
        WorkflowError::ResourceContractExceeded {
            resource: ContractResource::ListItems,
        },
    );
}

#[test]
fn foreach_body_at_start_join_and_node_count_is_typed() -> TestResult {
    for (body, expected) in [
        (0, f::crossing(0)),
        (2, f::crossing(2)),
        (
            4,
            WorkflowError::StepOutOfBounds {
                step: StepIdx::new(4),
            },
        ),
    ] {
        let mut parts = f::foreach(0, 1, 1, 1);
        f::body(&mut parts, body)?;
        f::rejects(parts, expected);
    }
    Ok(())
}

#[test]
fn foreach_limit_precedes_bad_body() -> TestResult {
    let mut parts = f::foreach(0, 1, 65_536, 1);
    f::body(&mut parts, 4)?;
    f::rejects(
        parts,
        WorkflowError::ResourceContractExceeded {
            resource: ContractResource::ListItems,
        },
    );
    Ok(())
}

#[test]
fn foreach_parallelism_precedes_bad_body() -> TestResult {
    for (parallelism, resource) in [
        (0, ContractResource::BranchCount),
        (3, ContractResource::Fanout),
    ] {
        let mut parts = f::foreach(0, 1, 1, parallelism);
        f::body(&mut parts, 4)?;
        f::rejects(parts, WorkflowError::ResourceContractExceeded { resource });
    }
    Ok(())
}
