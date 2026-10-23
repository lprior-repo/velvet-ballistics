use vb_core::workflow::{CompiledNodeKind, CompiledWorkflow, SlotBranch};

use super::super::common::{cn, fin, mkwfc, nop};
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx};
use vb_core::value::ConstValue;

pub(super) enum Family {
    ForEach,
    Reduce,
    Collect,
    Repeat,
}

impl std::fmt::Debug for Family {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Family::ForEach => write!(f, "ForEach"),
            Family::Reduce => write!(f, "Reduce"),
            Family::Collect => write!(f, "Collect"),
            Family::Repeat => write!(f, "Repeat"),
        }
    }
}

impl Clone for Family {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for Family {}

impl PartialEq for Family {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Eq for Family {}

pub(super) const FAMILIES: [Family; 4] = [
    Family::ForEach,
    Family::Reduce,
    Family::Collect,
    Family::Repeat,
];

pub(super) fn build(family: Family, productive: bool) -> Result<CompiledWorkflow, String> {
    let owner = match family {
        Family::ForEach => cn(
            0,
            Some(2),
            None,
            CompiledNodeKind::ForEachStart {
                input: SlotIdx::new(0),
                item_slot: SlotIdx::new(1),
                limit: 2,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        ),
        Family::Reduce => cn(
            0,
            Some(3),
            None,
            CompiledNodeKind::ReduceStart {
                input: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                initial: ConstIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        ),
        Family::Collect => cn(
            0,
            Some(1),
            None,
            CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 2,
                page_size: 1,
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        ),
        Family::Repeat => {
            let max = if productive { 2u16 } else { 1u16 };
            cn(
                0,
                Some(0),
                None,
                CompiledNodeKind::RepeatStart {
                    max_attempts: max,
                    body: StepIdx::new(1),
                    done: StepIdx::new(4),
                },
            )
        }
    };

    let body = cn(
        1,
        None,
        None,
        CompiledNodeKind::ChooseSlot {
            branches: vec![SlotBranch {
                condition: SlotIdx::new(5),
                target: StepIdx::new(3),
            }]
            .into_boxed_slice(),
            otherwise: Some(StepIdx::new(2)),
        },
    );

    let otherwise = nop(2, 3);

    let latch = match family {
        Family::ForEach => cn(
            3,
            Some(1),
            None,
            CompiledNodeKind::ForEachNext {
                iterator_slot: SlotIdx::new(2),
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        ),
        Family::Reduce => cn(
            3,
            Some(0),
            None,
            CompiledNodeKind::ReduceNext {
                iterator_slot: SlotIdx::new(3),
                accumulator: SlotIdx::new(1),
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        ),
        Family::Collect => cn(
            3,
            None,
            None,
            CompiledNodeKind::CollectNext {
                collector_slot: SlotIdx::new(1),
                body: StepIdx::new(1),
                done: StepIdx::new(4),
            },
        ),
        Family::Repeat => cn(
            3,
            None,
            Some(1),
            CompiledNodeKind::RepeatCheck {
                attempt_slot: SlotIdx::new(0),
                done: StepIdx::new(4),
            },
        ),
    };

    let done = fin(4, 0);

    let nodes = vec![owner, body, otherwise, latch, done];
    let constants = vec![ConstValue::I64(42)];

    mkwfc(nodes, 6, constants)
}
