use crate::ids::StepIdx;
use crate::workflow::{CompiledNode, CompiledNodeKind};
use std::collections::HashMap;

use super::{
    BudgetTraversalError, find_node_position, remove_tracked_step, visit_node_for_total_steps,
};

struct Frame {
    entry: StepIdx,
    visited: Vec<bool>,
    stack: Vec<StepIdx>,
    jump_edges: Vec<(u16, u16)>,
    in_path: Vec<u16>,
    total: u64,
}

impl Frame {
    fn new(entry: StepIdx, node_count: usize) -> Self {
        Self {
            entry,
            visited: vec![false; node_count],
            stack: vec![entry],
            jump_edges: Vec::new(),
            in_path: Vec::new(),
            total: 0,
        }
    }

    fn pending_target(
        &self,
        nodes: &[CompiledNode],
        current: StepIdx,
        cache: &HashMap<StepIdx, u64>,
    ) -> Result<Option<StepIdx>, BudgetTraversalError> {
        let position = find_node_position(nodes, current, nodes.len())?;
        if self.visited.get(position).copied() == Some(true) {
            return Ok(None);
        }
        let node = nodes
            .get(position)
            .ok_or(BudgetTraversalError::StepOutOfBounds { step: current })?;
        match &node.kind {
            CompiledNodeKind::Choose {
                branches,
                otherwise,
            } => Ok(otherwise
                .iter()
                .copied()
                .chain(branches.iter().map(|branch| branch.target))
                .find(|target| !cache.contains_key(target))),
            CompiledNodeKind::ChooseSlot {
                branches,
                otherwise,
            } => Ok(otherwise
                .iter()
                .copied()
                .chain(branches.iter().map(|branch| branch.target))
                .find(|target| !cache.contains_key(target))),
            _ => Ok(None),
        }
    }

    fn visit(
        &mut self,
        nodes: &[CompiledNode],
        current: StepIdx,
        cache: &HashMap<StepIdx, u64>,
    ) -> Result<(), BudgetTraversalError> {
        remove_tracked_step(&mut self.in_path, current.get());
        self.total = visit_node_for_total_steps(
            nodes,
            current,
            nodes.len(),
            &mut self.visited,
            &mut self.jump_edges,
            &mut self.in_path,
            self.total,
            &mut self.stack,
            cache,
        )?;
        Ok(())
    }
}
// vb-8eocj.7.2.1: bead-linked cold analysis uses explicit suspend/complete
// cases to preserve per-invocation visited state. This is outside runtime
// execution.
pub(super) fn count(
    nodes: &[CompiledNode],
    entry: StepIdx,
    node_count: usize,
) -> Result<u64, BudgetTraversalError> {
    // Active entries are distinct; each fresh entry is computed once.
    // Visited state belongs to each frame, keeping state bounded by input graph keys.

    let mut frames = Vec::new();
    frames.push(Frame::new(entry, node_count));
    let mut cache = HashMap::new();

    loop {
        let Some(mut frame) = frames.pop() else {
            return Err(BudgetTraversalError::StepOutOfBounds { step: entry });
        };

        if let Some(current) = frame.stack.pop() {
            let target = frame.pending_target(nodes, current, &cache)?;
            if let Some(target) = target {
                // Cycle check: is target an entry of any frame in the call chain?
                let cycle = target == frame.entry || frames.iter().any(|f| f.entry == target);
                if cycle {
                    return Err(BudgetTraversalError::JumpCycle {
                        step: current,
                        target,
                    });
                }
                // Push current back onto parent frame's stack
                frame.stack.push(current);
                // Suspend parent, push child
                frames.push(frame);
                frames.push(Frame::new(target, node_count));
            } else {
                // No child needed, visit current (already popped)
                frame.visit(nodes, current, &cache)?;
                frames.push(frame);
            }
        } else {
            // Frame's stack is empty - this frame is done
            if frames.is_empty() {
                return Ok(frame.total);
            }
            let _previous = cache.insert(frame.entry, frame.total);
        }
    }
}
