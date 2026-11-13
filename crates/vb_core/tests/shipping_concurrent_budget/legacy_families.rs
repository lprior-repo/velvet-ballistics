use super::legacy_fixture::inner_legacy;
use super::{K, WorkflowError};
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx};
use vb_core::workflow::CompiledNode;

fn set(nodes: &mut [CompiledNode], index: usize, kind: K) -> Result<(), WorkflowError> {
    nodes.get_mut(index).ok_or(WorkflowError::EmptyNodes)?.kind = kind;
    Ok(())
}

pub(super) fn collect() -> Result<Vec<CompiledNode>, WorkflowError> {
    let mut nodes = inner_legacy()?;
    set(&mut nodes, 1, collect_start())?;
    set(&mut nodes, 3, collect_next())?;
    set(&mut nodes, 4, collect_finish())?;
    Ok(nodes)
}

pub(super) fn reduce() -> Result<Vec<CompiledNode>, WorkflowError> {
    let mut nodes = inner_legacy()?;
    set(&mut nodes, 1, reduce_start())?;
    set(&mut nodes, 3, reduce_next())?;
    set(&mut nodes, 4, reduce_finish())?;
    Ok(nodes)
}

pub(super) fn repeat() -> Result<Vec<CompiledNode>, WorkflowError> {
    let mut nodes = inner_legacy()?;
    set(&mut nodes, 1, repeat_start())?;
    set(&mut nodes, 3, repeat_check())?;
    nodes
        .get_mut(3)
        .ok_or(WorkflowError::EmptyNodes)?
        .routing
        .next = Some(StepIdx::new(2)).into();
    set(&mut nodes, 4, repeat_finish())?;
    Ok(nodes)
}
fn collect_start() -> K {
    K::CollectStart {
        source: SlotIdx::new(0),
        limit: 5.into(),
        page_size: 2,
        body: StepIdx::new(2),
        done: StepIdx::new(4),
    }
}
fn collect_next() -> K {
    K::CollectNext {
        collector_slot: SlotIdx::new(3),
        body: StepIdx::new(2),
        done: StepIdx::new(4),
    }
}
fn collect_finish() -> K {
    K::CollectFinish {
        collector_slot: SlotIdx::new(3),
    }
}
fn reduce_start() -> K {
    K::ReduceStart {
        limit: 5,
        input: SlotIdx::new(0),
        accumulator: SlotIdx::new(2),
        initial: ConstIdx::new(0),
        body: StepIdx::new(2),
        done: StepIdx::new(4),
    }
}
fn reduce_next() -> K {
    K::ReduceNext {
        iterator_slot: SlotIdx::new(3),
        accumulator: SlotIdx::new(2),
        body: StepIdx::new(2),
        done: StepIdx::new(4),
    }
}
fn reduce_finish() -> K {
    K::ReduceFinish {
        accumulator: SlotIdx::new(2),
    }
}
fn repeat_start() -> K {
    K::RepeatStart {
        max_attempts: 5.into(),
        body: StepIdx::new(2),
        done: StepIdx::new(4),
    }
}
fn repeat_check() -> K {
    K::RepeatCheck {
        attempt_slot: SlotIdx::new(3),
        done: StepIdx::new(4),
    }
}
fn repeat_finish() -> K {
    K::RepeatFinish {
        result: SlotIdx::new(0),
    }
}
