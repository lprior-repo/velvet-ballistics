use serde_json::{Value, json};
use vb_core::{CompiledNode, CompiledNodeKind as K, StepIdx};

pub(super) fn targets(node: &CompiledNode) -> Vec<Value> {
    let mut targets = Vec::new();
    let mut add = |label: &str, to: StepIdx| {
        targets.push(json!({"from": node.id.get(), "to": to.get(), "label": label}))
    };
    if let Some(to) = node.routing.next.step() {
        add("next", to);
    }
    if let Some(to) = node.routing.error.handler() {
        add("on_error", to);
    }
    choice_targets(&node.kind, &mut add);
    loop_targets(&node.kind, &mut add);
    parallel_targets(&node.kind, &mut add);
    terminal_targets(&node.kind, &mut add);
    targets
}

fn choice_targets(kind: &K, add: &mut impl FnMut(&str, StepIdx)) {
    match kind {
        K::Choose {
            branches,
            otherwise,
        } => {
            choose(
                branches.iter().map(|branch| branch.target),
                otherwise.step(),
                add,
            );
        }
        K::ChooseSlot {
            branches,
            otherwise,
        } => {
            choose(
                branches.iter().map(|branch| branch.target),
                otherwise.step(),
                add,
            );
        }
        _ => {}
    }
}

fn choose(
    targets: impl Iterator<Item = StepIdx>,
    otherwise: Option<StepIdx>,
    add: &mut impl FnMut(&str, StepIdx),
) {
    targets.for_each(|target| add("branch", target));
    if let Some(to) = otherwise {
        add("otherwise", to);
    }
}

fn loop_targets(kind: &K, add: &mut impl FnMut(&str, StepIdx)) {
    match kind {
        K::ForEachStart { body, done, .. }
        | K::ForEachNext { body, done, .. }
        | K::CollectStart { body, done, .. }
        | K::CollectPage { body, done, .. }
        | K::CollectNext { body, done, .. }
        | K::ReduceStart { body, done, .. }
        | K::ReduceNext { body, done, .. }
        | K::RepeatStart { body, done, .. }
        | K::RepeatAttempt { body, done, .. } => {
            add("body", *body);
            add("done", *done);
        }
        _ => {}
    }
}

fn parallel_targets(kind: &K, add: &mut impl FnMut(&str, StepIdx)) {
    match kind {
        K::TogetherStart { branches, join }
        | K::TogetherConcurrentStart { branches, join }
        | K::FirstSuccessStart { branches, join } => {
            branches.iter().for_each(|to| add("branch", *to));
            add("join", *join);
        }
        K::TogetherBranch { entry, join, .. } => {
            add("entry", *entry);
            add("join", *join);
        }
        K::ForEachConcurrentStart { body, join, .. } => {
            add("body", *body);
            add("join", *join);
        }
        _ => {}
    }
}

fn terminal_targets(kind: &K, add: &mut impl FnMut(&str, StepIdx)) {
    match kind {
        K::RepeatCheck { done, .. } => add("done", *done),
        K::RetryCheck {
            body, exhausted, ..
        } => {
            add("body", *body);
            add("exhausted", *exhausted);
        }
        K::ErrorHandler { body, handler, .. } => {
            add("body", *body);
            add("handler", *handler);
        }
        K::Jump { target } => add("target", *target),
        _ => {}
    }
}
