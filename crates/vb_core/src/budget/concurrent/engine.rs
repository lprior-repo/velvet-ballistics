use std::collections::{HashMap, HashSet};

use crate::budget::BudgetTraversalError;
use crate::ids::StepIdx;
use crate::workflow::CompiledNode;

use super::compose;
use super::metrics::Metrics;
use super::scope;
use super::scope::{Boundary, Key};

struct Frame {
    key: Key,
    visited: HashSet<StepIdx>,
    stack: Vec<StepIdx>,
    metrics: Metrics,
}

impl Frame {
    fn new(key: Key) -> Self {
        Frame {
            key,
            visited: HashSet::new(),
            stack: vec![key.entry],
            metrics: Metrics::default(),
        }
    }
}

fn run_step(
    frame: &mut Frame,
    nodes: &[CompiledNode],
    cache: &HashMap<Key, Metrics>,
) -> Result<Option<Key>, BudgetTraversalError> {
    let Some(current) = frame.stack.pop() else {
        return Ok(None);
    };
    if scope::stops(nodes, frame.key.boundary, current)? || frame.visited.contains(&current) {
        return Ok(None);
    }
    let node = scope::node_at(nodes, current)?;
    scope::validate_start(nodes, node)?;
    let keys = scope::dependencies(node, frame.key.boundary, frame.key.active);
    if let Some(key) = keys.iter().find(|key| !cache.contains_key(key)) {
        frame.stack.push(current);
        return Ok(Some(*key));
    }
    frame.visited.insert(current);
    frame.metrics =
        frame
            .metrics
            .combine(compose::summarize(node, &keys, cache, frame.key.active)?)?;
    push_targets(frame, nodes, node)?;
    Ok(None)
}

fn push_targets(
    frame: &mut Frame,
    nodes: &[CompiledNode],
    node: &CompiledNode,
) -> Result<(), BudgetTraversalError> {
    for target in scope::targets(nodes, node, frame.key.boundary)? {
        if target <= node.id {
            return Err(BudgetTraversalError::JumpCycle {
                step: node.id,
                target,
            });
        }
        scope::node_at(nodes, target)?;
        frame.stack.push(target);
    }
    Ok(())
}

fn schedule(
    frame: Frame,
    key: Key,
    frames: &mut Vec<Frame>,
    limit: usize,
) -> Result<(), BudgetTraversalError> {
    if key == frame.key || frames.iter().any(|active| active.key == key) || frames.len() >= limit {
        return Err(BudgetTraversalError::JumpCycle {
            step: frame.key.entry,
            target: key.entry,
        });
    }
    frames.push(frame);
    frames.push(Frame::new(key));
    Ok(())
}

pub(super) fn analyze(
    nodes: &[CompiledNode],
    entry: StepIdx,
) -> Result<Metrics, BudgetTraversalError> {
    scope::node_at(nodes, entry)?;
    let mut frames = vec![Frame::new(Key {
        entry,
        boundary: Boundary::Run,
        active: true,
    })];
    let mut cache = HashMap::new();
    loop {
        let mut frame = frames
            .pop()
            .ok_or(BudgetTraversalError::StepOutOfBounds { step: entry })?;
        if let Some(key) = run_step(&mut frame, nodes, &cache)? {
            schedule(frame, key, &mut frames, nodes.len())?;
        } else if !frame.stack.is_empty() {
            frames.push(frame);
        } else if frames.is_empty() {
            return Ok(frame.metrics);
        } else {
            let _previous = cache.insert(frame.key, frame.metrics);
        }
    }
}
