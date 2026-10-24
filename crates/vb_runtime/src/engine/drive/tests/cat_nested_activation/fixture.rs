#![forbid(unsafe_code)]
use super::super::common::{cn, fin, mkwfc, nop};
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow, SlotBranch};

pub(super) fn build_nested_loop_workflow() -> Result<CompiledWorkflow, String> {
    let nodes = vec![
        cn(
            0,
            Some(2),
            None,
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 2,
                body: StepIdx::new(1),
                done: StepIdx::new(7),
            },
        ),
        cn(
            1,
            None,
            None,
            CompiledNodeKind::ChooseSlot {
                branches: Box::from([SlotBranch {
                    condition: SlotIdx::new(5),
                    target: StepIdx::new(6),
                }]),
                otherwise: Some(StepIdx::new(2)),
            },
        ),
        cn(
            2,
            Some(3),
            None,
            CompiledNodeKind::RepeatStart {
                max_attempts: 2,
                body: StepIdx::new(3),
                done: StepIdx::new(6),
            },
        ),
        nop(3, 4),
        nop(4, 5),
        cn(
            5,
            None,
            Some(3),
            CompiledNodeKind::RepeatCheck {
                attempt_slot: SlotIdx::new(3),
                done: StepIdx::new(6),
            },
        ),
        cn(
            6,
            Some(1),
            None,
            CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(2),
                body: StepIdx::new(1),
                done: StepIdx::new(7),
            },
        ),
        fin(7, 0),
    ];
    mkwfc(nodes, 6, vec![])
}
