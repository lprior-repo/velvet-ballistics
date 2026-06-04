#![forbid(unsafe_code)]

use vb_core::{CompiledNodeKind, StepIdx};

pub(super) fn kind_edge_targets(kind: &CompiledNodeKind) -> Vec<(String, StepIdx)> {
    match kind {
        CompiledNodeKind::Choose { .. } | CompiledNodeKind::ChooseSlot { .. } => {
            choice_edge_targets(kind)
        }
        CompiledNodeKind::ForEachStart { .. }
        | CompiledNodeKind::ForEachNext { .. }
        | CompiledNodeKind::CollectStart { .. }
        | CompiledNodeKind::CollectPage { .. }
        | CompiledNodeKind::CollectNext { .. }
        | CompiledNodeKind::ReduceStart { .. }
        | CompiledNodeKind::ReduceNext { .. }
        | CompiledNodeKind::RepeatStart { .. }
        | CompiledNodeKind::RepeatAttempt { .. } => body_done_edge_targets(kind),
        CompiledNodeKind::TogetherStart { branches, join } => parallel_edge_targets(branches, join),
        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            labeled_pair("entry", entry, "join", join)
        }
        CompiledNodeKind::RepeatCheck { done, .. } => labeled_single("done", done),
        CompiledNodeKind::RetryCheck {
            body, exhausted, ..
        } => labeled_pair("body", body, "exhausted", exhausted),
        CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            labeled_pair("body", body, "handler", handler)
        }
        CompiledNodeKind::Jump { target } => labeled_single("target", target),
        _ => Vec::new(),
    }
}

fn choice_edge_targets(kind: &CompiledNodeKind) -> Vec<(String, StepIdx)> {
    match kind {
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => branch_edges(branches.iter().map(|branch| branch.target), otherwise),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => branch_edges(branches.iter().map(|branch| branch.target), otherwise),
        _ => Vec::new(),
    }
}

fn body_done_edge_targets(kind: &CompiledNodeKind) -> Vec<(String, StepIdx)> {
    match kind {
        CompiledNodeKind::ForEachStart { body, done, .. }
        | CompiledNodeKind::ForEachNext { body, done, .. }
        | CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::CollectPage { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. }
        | CompiledNodeKind::RepeatStart { body, done, .. }
        | CompiledNodeKind::RepeatAttempt { body, done, .. } => {
            labeled_pair("body", body, "done", done)
        }
        _ => Vec::new(),
    }
}

fn parallel_edge_targets(branches: &[StepIdx], join: &StepIdx) -> Vec<(String, StepIdx)> {
    branches
        .iter()
        .enumerate()
        .map(|(index, target)| (format!("branch_{index}"), *target))
        .chain(std::iter::once((String::from("join"), *join)))
        .collect()
}

fn branch_edges<'a>(
    targets: impl Iterator<Item = StepIdx> + 'a,
    otherwise: &'a Option<StepIdx>,
) -> Vec<(String, StepIdx)> {
    targets
        .enumerate()
        .map(|(index, target)| (format!("branch_{index}"), target))
        .chain(
            otherwise
                .iter()
                .map(|target| (String::from("otherwise"), *target)),
        )
        .collect()
}

fn labeled_pair(
    first_label: &'static str,
    first_target: &StepIdx,
    second_label: &'static str,
    second_target: &StepIdx,
) -> Vec<(String, StepIdx)> {
    vec![
        (String::from(first_label), *first_target),
        (String::from(second_label), *second_target),
    ]
}

fn labeled_single(label: &'static str, target: &StepIdx) -> Vec<(String, StepIdx)> {
    vec![(String::from(label), *target)]
}
