use vb_core::replay::ReplayAction;
use vb_core::{CompiledNodeKind as K, SlotIdx, StepIdx};

pub(super) fn add_owned_required(
    kind: &K,
    output: Option<SlotIdx>,
    action: &ReplayAction,
    slots: &mut Vec<SlotIdx>,
) {
    add_next_kind(kind, action, slots);
    add_reduce_kind(kind, output, action, slots);
    add_repeat_kind(kind, slots);
    add_collect_kind(kind, output, slots);
}

fn add_next_kind(kind: &K, action: &ReplayAction, slots: &mut Vec<SlotIdx>) {
    let (iterator_slot, done) = match kind {
        K::ForEachNext {
            iterator_slot,
            done,
            ..
        }
        | K::ReduceNext {
            iterator_slot,
            done,
            ..
        } => (*iterator_slot, *done),
        _ => return,
    };
    add_next_required(iterator_slot, done, action, slots);
}

fn add_reduce_kind(
    kind: &K,
    output: Option<SlotIdx>,
    action: &ReplayAction,
    slots: &mut Vec<SlotIdx>,
) {
    if let K::ReduceStart {
        input,
        accumulator,
        done,
        ..
    } = kind
    {
        add_reduce_start_required(*input, *accumulator, output, *done, action, slots);
    }
}

fn add_repeat_kind(kind: &K, slots: &mut Vec<SlotIdx>) {
    if let K::RepeatCheck { attempt_slot, .. } = kind
        && !slots.contains(attempt_slot)
    {
        slots.push(*attempt_slot);
    }
}

fn add_collect_kind(kind: &K, output: Option<SlotIdx>, slots: &mut Vec<SlotIdx>) {
    match kind {
        K::CollectStart { source, .. } => add_collect_start_required(*source, output, slots),
        K::CollectNext { collector_slot, .. } if !slots.contains(collector_slot) => {
            slots.push(*collector_slot);
        }
        _ => {}
    }
}

fn add_collect_start_required(source: SlotIdx, output: Option<SlotIdx>, slots: &mut Vec<SlotIdx>) {
    let collector = output.unwrap_or(source);
    if !slots.contains(&collector) {
        slots.push(collector);
    }
}

fn add_next_required(
    iterator_slot: SlotIdx,
    done: StepIdx,
    action: &ReplayAction,
    slots: &mut Vec<SlotIdx>,
) {
    if continued_to(action, done) {
        slots.clear();
    } else {
        slots.push(iterator_slot);
    }
}

fn add_reduce_start_required(
    input: SlotIdx,
    accumulator: SlotIdx,
    output: Option<SlotIdx>,
    done: StepIdx,
    action: &ReplayAction,
    slots: &mut Vec<SlotIdx>,
) {
    slots.clear();
    slots.push(accumulator);
    if continued_to(action, done) {
        return;
    }
    slots.push(input);
    if let Some(output) = output
        && !slots.contains(&output)
    {
        slots.push(output);
    }
}

fn continued_to(action: &ReplayAction, target: StepIdx) -> bool {
    matches!(action, ReplayAction::Continue(next) if *next == target)
}
