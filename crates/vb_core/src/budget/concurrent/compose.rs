use super::metrics::{Metrics, add, mul};
use super::scope::Key;
use crate::budget::BudgetTraversalError;
use crate::budget::BudgetTraversalError::StepOutOfBounds;
use crate::workflow::{CompiledNode, CompiledNodeKind as K};
use std::collections::HashMap;

fn children_metrics(
    keys: &[Key],
    cache: &HashMap<Key, Metrics>,
) -> Result<Vec<Metrics>, BudgetTraversalError> {
    let mut out = Vec::with_capacity(keys.len());
    for k in keys {
        let m = cache
            .get(k)
            .copied()
            .ok_or(StepOutOfBounds { step: k.entry })?;
        out.push(m);
    }
    Ok(out)
}

fn fixed_group(children: &[Metrics]) -> Result<Metrics, BudgetTraversalError> {
    let sum = children
        .iter()
        .try_fold(Metrics::default(), |acc, m| acc.combine(*m))?;
    let lanes: Result<u16, BudgetTraversalError> = children.iter().try_fold(0u16, |p, m| {
        let slot: u64 = add(1, u64::from(m.parallel))?;
        let t = add(u64::from(p), slot)?;
        u16::try_from(t).map_err(|_| BudgetTraversalError::StepCountOverflow { actual: t })
    });
    Ok(Metrics {
        parallel: lanes?,
        depth: sum.nested()?.depth,
        ..sum
    })
}

fn choice_group(children: &[Metrics]) -> Metrics {
    children
        .iter()
        .fold(Metrics::default(), |acc, m| acc.maximum(*m))
}

fn scaled(child: Metrics, iterations: u64, capacity: u64) -> Result<Metrics, BudgetTraversalError> {
    let mut run = child.scale(iterations)?;
    run.timers = mul(child.timers, capacity)?;
    run.gates = mul(child.gates, capacity)?;
    run.signals = mul(child.signals, capacity)?;
    run.nested()
}

fn single(children: &[Metrics], node: &CompiledNode) -> Result<Metrics, BudgetTraversalError> {
    if children.len() != 1 {
        return Err(StepOutOfBounds { step: node.id });
    }
    children
        .first()
        .copied()
        .ok_or(StepOutOfBounds { step: node.id })
}

fn concurrent_loop(
    node: &CompiledNode,
    children: &[Metrics],
) -> Result<Metrics, BudgetTraversalError> {
    let K::ForEachConcurrentStart { limit, at_once, .. } = node.kind else {
        return Err(StepOutOfBounds { step: node.id });
    };
    let child = single(children, node)?;
    let effective = u64::from(limit.get().min(u32::from(at_once.get())));
    let mut run = scaled(child, u64::from(limit.get()), effective)?;
    let total = mul(effective, add(1, u64::from(child.parallel))?)?;
    run.parallel = u16::try_from(total)
        .map_err(|_| BudgetTraversalError::StepCountOverflow { actual: total })?;
    Ok(run)
}

fn legacy_iterations(kind: &K) -> Option<u64> {
    match kind {
        K::ForEachStart { limit, .. } | K::CollectStart { limit, .. } => {
            Some(u64::from(limit.get()))
        }
        K::ReduceStart { limit, .. } => Some(u64::from(*limit)),
        K::RepeatStart { max_attempts, .. } => Some(u64::from(max_attempts.get())),
        _ => None,
    }
}

fn composed(node: &CompiledNode, children: &[Metrics]) -> Result<Metrics, BudgetTraversalError> {
    if let Some(iterations) = legacy_iterations(&node.kind) {
        return scaled(single(children, node)?, iterations.max(1), 1);
    }
    match node.kind {
        K::Choose { .. } | K::ChooseSlot { .. } => Ok(choice_group(children)),
        K::TogetherStart { .. }
        | K::TogetherConcurrentStart { .. }
        | K::FirstSuccessStart { .. } => fixed_group(children),
        K::ForEachConcurrentStart { .. } => concurrent_loop(node, children),
        _ => Ok(Metrics::default()),
    }
}

pub(super) fn summarize(
    node: &CompiledNode,
    keys: &[Key],
    cache: &HashMap<Key, Metrics>,
    active: bool,
) -> Result<Metrics, BudgetTraversalError> {
    let kids = children_metrics(keys, cache)?;
    let leaf = Metrics::leaf(node)?;
    let mut child = composed(node, &kids)?;
    // Inactive descendants retain declared caps without accumulating execution work.
    if !active {
        child = child.scale(0)?;
        child.parallel = 0;
        return leaf.scale(0)?.combine(child);
    }
    leaf.combine(child)
}
