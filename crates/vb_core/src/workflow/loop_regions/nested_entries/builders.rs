#![forbid(unsafe_code)]

use super::support::*;

pub(super) fn start_kind(family: Family, body: u16, done: u16) -> CompiledNodeKind {
    match family {
        Family::ForEach => CompiledNodeKind::ForEachStart {
            input: slot(0),
            item_slot: slot(3),
            limit: 2,
            body: step(body),
            done: step(done),
        },
        Family::Collect => CompiledNodeKind::CollectStart {
            source: slot(0),
            limit: 2,
            page_size: 1,
            body: step(body),
            done: step(done),
        },
        Family::Reduce => reduce_start(body, done),
        Family::Repeat => repeat_start(body, done),
    }
}

fn reduce_start(body: u16, done: u16) -> CompiledNodeKind {
    CompiledNodeKind::ReduceStart {
        input: slot(0),
        accumulator: slot(4),
        initial: ConstIdx::new(0),
        body: step(body),
        done: step(done),
    }
}

fn repeat_start(body: u16, done: u16) -> CompiledNodeKind {
    CompiledNodeKind::RepeatStart {
        max_attempts: 2,
        body: step(body),
        done: step(done),
    }
}

pub(super) fn control_kind(family: Family, state: u16, body: u16, done: u16) -> CompiledNodeKind {
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
        Family::Reduce => reduce_control(state, body, done),
        Family::Repeat => CompiledNodeKind::RepeatCheck {
            attempt_slot: slot(state),
            done: step(done),
        },
    }
}

fn reduce_control(state: u16, body: u16, done: u16) -> CompiledNodeKind {
    CompiledNodeKind::ReduceNext {
        iterator_slot: slot(state),
        accumulator: slot(4),
        body: step(body),
        done: step(done),
    }
}

pub(super) fn install_pair(
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
    owner.output = Some(slot(state));
    owner.next = None;
    let latch = edit(parts, control)?;
    latch.kind = control_kind(family, state, body, done);
    latch.next = matches!(family, Family::Repeat).then_some(step(body));
    Ok(())
}

pub(super) fn outer_parts(count: u16, control: u16, done: u16) -> TestResult<WorkflowParts> {
    let mut parts = linear_parts(count)?;
    install_pair(&mut parts, Family::ForEach, 1, 2, control, done, 1)?;
    Ok(parts)
}
