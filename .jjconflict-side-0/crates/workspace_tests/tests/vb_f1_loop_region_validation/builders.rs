#![forbid(unsafe_code)]

use super::{Family, TestResult, edit, linear_parts, slot, step};
use vb_core::workflow::{NodeContinuation, NodeOutput};
use vb_core::{CompiledNodeKind, ConstIdx, WorkflowParts};

pub(crate) fn start_kind(family: Family, body: u16, done: u16) -> CompiledNodeKind {
    match family {
        Family::ForEach => CompiledNodeKind::ForEachStart {
            input: slot(0),
            item_slot: slot(3),
            limit: 2.into(),
            body: step(body),
            done: step(done),
        },
        Family::Collect => CompiledNodeKind::CollectStart {
            source: slot(0),
            limit: 2.into(),
            page_size: 1,
            body: step(body),
            done: step(done),
        },
        Family::Reduce => CompiledNodeKind::ReduceStart {
            limit: 65_535,
            input: slot(0),
            accumulator: slot(2),
            initial: ConstIdx::new(0),
            body: step(body),
            done: step(done),
        },
        Family::Repeat => CompiledNodeKind::RepeatStart {
            max_attempts: 2.into(),
            body: step(body),
            done: step(done),
        },
    }
}

pub(crate) fn control_kind(family: Family, state: u16, body: u16, done: u16) -> CompiledNodeKind {
    match family {
        Family::ForEach => CompiledNodeKind::ForEachNext {
            iterator_slot: slot(state),
            body: step(body),
            done: step(done),
        },
        Family::Collect => CompiledNodeKind::CollectNext {
            collector_slot: slot(state),
            body: step(body),
            done: step(done),
        },
        Family::Reduce => CompiledNodeKind::ReduceNext {
            iterator_slot: slot(state),
            accumulator: slot(2),
            body: step(body),
            done: step(done),
        },
        Family::Repeat => CompiledNodeKind::RepeatCheck {
            attempt_slot: slot(state),
            done: step(done),
        },
    }
}

pub(crate) fn install_pair(
    parts: &mut WorkflowParts,
    family: Family,
    start: u16,
    body: u16,
    control: u16,
    done: u16,
    state: u16,
) -> TestResult {
    let owner = edit(parts, start)?;
    owner.kind = start_kind(family, body, done);
    owner.output = NodeOutput::Slot(slot(state));
    owner.routing.next = NodeContinuation::End;
    let latch = edit(parts, control)?;
    latch.kind = control_kind(family, state, body, done);
    latch.output = match family {
        Family::ForEach => NodeOutput::Slot(slot(3)),
        Family::Reduce => NodeOutput::Slot(slot(0)),
        Family::Collect | Family::Repeat => NodeOutput::Discard,
    };
    latch.routing.next = if matches!(family, Family::Repeat) {
        NodeContinuation::Step(step(body))
    } else {
        NodeContinuation::End
    };
    Ok(())
}

pub(crate) fn paired(family: Family) -> TestResult<WorkflowParts> {
    let mut parts = linear_parts(6)?;
    install_pair(&mut parts, family, 1, 2, 4, 5, 1)?;
    Ok(parts)
}
