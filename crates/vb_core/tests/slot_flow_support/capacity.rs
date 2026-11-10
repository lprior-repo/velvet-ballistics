use super::*;
use vb_core::workflow::{InputSlotKind, SlotBranch, SlotFlowFailure, validate_slot_flow};

fn capacity_rejected(p: vb_core::workflow::WorkflowParts) -> TestResult {
    match validate_slot_flow(&p) {
        Err(error) => assert_eq!(error.reason, SlotFlowFailure::AnalysisCapacity),
        Ok(()) => return Err("analysis accepted excessive resource demand".into()),
    }
    Ok(())
}

#[test]
fn dense_state_capacity_is_checked_before_allocating_node_states() -> TestResult {
    let nodes = (0_u16..4096)
        .map(|raw| {
            if raw == 4095 {
                finish(raw, 0)
            } else {
                node(raw, K::Nop, None, raw.checked_add(1))
            }
        })
        .collect();
    let mut p = parts(nodes, u16::MAX);
    declare(&mut p, 0, InputSlotKind::I64);
    capacity_rejected(p)
}

#[test]
fn excessive_choice_fanout_is_rejected_before_transfer_expansion() -> TestResult {
    let count = usize::from(vb_core::limits::MAX_FANOUT)
        .checked_add(1)
        .ok_or("fanout overflow")?;
    let mut p = parts(
        vec![
            node(
                0,
                K::ChooseSlot {
                    branches: vec![
                        SlotBranch {
                            condition: slot(0),
                            target: step(1)
                        };
                        count
                    ]
                    .into_boxed_slice(),
                    otherwise: Some(step(1)),
                },
                None,
                None,
            ),
            finish(1, 0),
        ],
        1,
    );
    declare(&mut p, 0, InputSlotKind::Bool);
    capacity_rejected(p)
}
