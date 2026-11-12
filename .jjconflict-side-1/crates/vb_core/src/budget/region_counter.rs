// vb-8eocj.7.2.1: bead-linked cold analysis uses explicit suspend/complete
// cases to preserve per-invocation visited state. This is outside runtime
// execution.

use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind};
use std::collections::HashMap;

use super::{BudgetError, BudgetTraversalError, find_node_position, visit_body_region_node};

struct Frame {
    body: StepIdx,
    done: StepIdx,
    visited: Vec<bool>,
    stack: Vec<StepIdx>,
    count: u64,
}

impl Frame {
    fn new(body: StepIdx, done: StepIdx, node_count: usize) -> Self {
        Self {
            body,
            done,
            visited: vec![false; node_count],
            stack: vec![body],
            count: 0,
        }
    }

    fn key(&self) -> (StepIdx, StepIdx) {
        (self.body, self.done)
    }

    fn pending_region(
        &self,
        nodes: &[CompiledNode],
        current: StepIdx,
        cache: &HashMap<(StepIdx, StepIdx), u64>,
    ) -> Result<Option<(StepIdx, StepIdx)>, BudgetTraversalError> {
        if current == self.done {
            return Ok(None);
        }
        let position = find_node_position(nodes, current, nodes.len())?;
        if self.visited.get(position).copied() == Some(true) {
            return Ok(None);
        }
        let node = nodes
            .get(position)
            .ok_or(BudgetTraversalError::StepOutOfBounds { step: current })?;

        let (child_body, child_done) = match &node.kind {
            CompiledNodeKind::ForEachStart { body, done, .. }
            | CompiledNodeKind::CollectStart { body, done, .. }
            | CompiledNodeKind::ReduceStart { body, done, .. }
            | CompiledNodeKind::RepeatStart { body, done, .. } => (body, done),
            _ => return Ok(None),
        };

        if *child_body == current || cache.contains_key(&(*child_body, *child_done)) {
            return Ok(None);
        }
        Ok(Some((*child_body, *child_done)))
    }

    fn visit(
        &mut self,
        nodes: &[CompiledNode],
        current: StepIdx,
        cache: &HashMap<(StepIdx, StepIdx), u64>,
    ) -> Result<(), BudgetError> {
        self.count = visit_body_region_node(
            nodes,
            current,
            self.done,
            nodes.len(),
            &mut self.visited,
            &mut self.stack,
            self.count,
            cache,
        )?;
        Ok(())
    }

    fn finish(&self) -> u64 {
        let span = self
            .done
            .get()
            .saturating_sub(self.body.get())
            .saturating_sub(1);
        self.count.max(u64::from(span))
    }
}

pub(super) fn count(
    nodes: &[CompiledNode],
    body: StepIdx,
    done: StepIdx,
    node_count: usize,
) -> Result<u64, BudgetError> {
    let mut frames = Vec::new();
    frames.push(Frame::new(body, done, node_count));
    let mut cache = HashMap::new();

    loop {
        let Some(mut frame) = frames.pop() else {
            return Err(BudgetError::TotalStepsExceeded {
                actual: u64::MAX,
                limit: u64::MAX,
            });
        };

        if let Some(current) = frame.stack.pop() {
            let region = frame.pending_region(nodes, current, &cache)?;
            if let Some((child_body, child_done)) = region {
                if (child_body, child_done) == frame.key()
                    || frames.iter().any(|f| f.key() == (child_body, child_done))
                {
                    return Err(BudgetError::TotalStepsExceeded {
                        actual: u64::MAX,
                        limit: u64::MAX,
                    });
                }
                frame.stack.push(current);
                frames.push(frame);
                frames.push(Frame::new(child_body, child_done, node_count));
            } else {
                frame.visit(nodes, current, &cache)?;
                frames.push(frame);
            }
        } else {
            if frames.is_empty() {
                return Ok(frame.finish());
            }
            let _previous = cache.insert(frame.key(), frame.finish());
        }
    }
}
