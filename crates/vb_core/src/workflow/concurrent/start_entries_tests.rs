use super::*;
use crate::ids::{BranchCount, FanoutLimit};
use crate::workflow::ContractResource;
use crate::workflow::concurrent_start_entries::validate;

fn check_entries(
    entries: Box<[StepIdx]>,
    expected: WorkflowError,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    if let Some(node0) = parts.nodes.get_mut(0) {
        node0.kind = K::TogetherConcurrentStart {
            branches: entries,
            join: StepIdx::new(2),
        };
    } else {
        return Err("fixture missing node 0".into());
    }
    let node0 = parts.nodes.get(0).ok_or("fixture missing node 0")?;
    let result = validate(&parts, node0);
    assert_eq!(result, Err(expected));
    Ok(())
}

#[test]
fn branch_entry_hostile_matrix() -> Result<(), Box<dyn std::error::Error>> {
    check_entries(Box::new([]), WorkflowError::EmptyBranchTable)?;
    check_entries(
        Box::new([StepIdx::new(1), StepIdx::new(1)]),
        WorkflowError::LoopRegionUnpaired {
            control: StepIdx::new(0),
        },
    )?;
    check_entries(
        Box::new([StepIdx::new(2)]),
        WorkflowError::LoopRegionCrossing {
            control: StepIdx::new(0),
            from: StepIdx::new(0),
            to: StepIdx::new(2),
        },
    )?;
    check_entries(
        Box::new([StepIdx::new(4)]),
        WorkflowError::StepOutOfBounds {
            step: StepIdx::new(4),
        },
    )?;
    Ok(())
}

fn check_foreach(
    limit: u32,
    cap: u16,
    expected: Result<(), WorkflowError>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = build_parts();
    if let Some(node0) = parts.nodes.get_mut(0) {
        node0.kind = K::ForEachConcurrentStart {
            input: SlotIdx::new(0),
            item_slot: SlotIdx::new(0),
            limit: FanoutLimit::new(limit),
            at_once: BranchCount::new(cap),
            body: StepIdx::new(1),
            join: StepIdx::new(2),
        };
    } else {
        return Err("fixture missing node 0".into());
    }
    let node0 = parts.nodes.get(0).ok_or("fixture missing node 0")?;
    let actual = validate(&parts, node0);
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn foreach_capacity_matrix() -> Result<(), Box<dyn std::error::Error>> {
    check_foreach(0, 1, Ok(()))?;
    check_foreach(
        0,
        0,
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::BranchCount,
        }),
    )?;
    check_foreach(
        0,
        65,
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::Fanout,
        }),
    )?;
    check_foreach(
        u32::MAX,
        1,
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::ListItems,
        }),
    )?;
    Ok(())
}
