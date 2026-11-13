use super::super::{budget::Budget, facts};
use super::layout::Layout;
use super::{Fact, Facts, FlowResult};
use super::{capacity, invalid, scratch, slots};
use crate::ids::StepIdx;
use crate::workflow::WorkflowParts;
use std::collections::VecDeque;

#[derive(Clone, Copy)]
struct Location {
    frame: usize,
    local: usize,
}

struct NodeState {
    step: StepIdx,
    incoming: Option<Facts>,
    queued: bool,
}

struct Frame {
    nodes: Vec<NodeState>,
}

pub(super) struct Store {
    frames: Vec<Frame>,
    locations: Vec<Location>,
    queue: VecDeque<(usize, StepIdx)>,
    slots: usize,
}

fn counts(layout: &Layout<'_>) -> FlowResult<Vec<usize>> {
    let mut result = slots(layout.scopes.len(), 0usize)?;
    for idx in &layout.node_frames {
        let value = result.get_mut(*idx).ok_or_else(capacity)?;
        *value = value.checked_add(1).ok_or_else(capacity)?;
    }
    Ok(result)
}

fn frames(counts: &[usize]) -> FlowResult<Vec<Frame>> {
    let mut out = scratch(counts.len())?;
    for c in counts.iter() {
        let nodes = scratch::<NodeState>(*c)?;
        out.push(Frame { nodes });
    }
    Ok(out)
}

fn assign(
    parts: &WorkflowParts,
    layout: &Layout<'_>,
    frames: &mut [Frame],
) -> FlowResult<Vec<Location>> {
    let mut locations = scratch(parts.nodes.len())?;
    for node in parts.nodes.iter() {
        let frame = layout.frame(node.id)?;
        let target = frames.get_mut(frame).ok_or_else(capacity)?;
        let local = target.nodes.len();
        target.nodes.push(NodeState {
            step: node.id,
            incoming: None,
            queued: false,
        });
        locations.push(Location { frame, local });
    }
    Ok(locations)
}

impl Store {
    pub(super) fn new(parts: &WorkflowParts, layout: &Layout<'_>) -> FlowResult<Self> {
        let frame_counts = counts(layout)?;
        let mut frames = frames(&frame_counts)?;
        let locations = assign(parts, layout, &mut frames)?;
        let mut queue = VecDeque::new();
        queue
            .try_reserve_exact(parts.nodes.len())
            .map_err(|_| capacity())?;
        Ok(Self {
            frames,
            locations,
            queue,
            slots: usize::from(parts.slot_count),
        })
    }

    pub(super) fn frame_of(&self, step: StepIdx) -> FlowResult<usize> {
        let loc = self.location(step)?;
        Ok(loc.frame)
    }

    fn location(&self, step: StepIdx) -> FlowResult<Location> {
        self.locations
            .get(step.as_usize())
            .copied()
            .ok_or_else(|| invalid(step))
    }

    fn record(&self, frame: usize, step: StepIdx) -> FlowResult<&NodeState> {
        let loc = self.location(step)?;
        if loc.frame != frame {
            return Err(invalid(step));
        }
        let state = self
            .frames
            .get(frame)
            .ok_or_else(capacity)?
            .nodes
            .get(loc.local)
            .ok_or_else(capacity)?;
        if state.step != step {
            return Err(invalid(step));
        }
        Ok(state)
    }

    fn record_mut(&mut self, frame: usize, step: StepIdx) -> FlowResult<&mut NodeState> {
        let loc = self.location(step)?;
        if loc.frame != frame {
            return Err(invalid(step));
        }
        let frame = self.frames.get_mut(frame).ok_or_else(capacity)?;
        let state = frame.nodes.get_mut(loc.local).ok_or_else(capacity)?;
        if state.step != step {
            return Err(invalid(step));
        }
        Ok(state)
    }

    pub(super) fn propagate(
        &mut self,
        frame: usize,
        step: StepIdx,
        input: Facts,
        budget: &mut Budget,
    ) -> FlowResult<()> {
        if input.len() != self.slots {
            return Err(invalid(step));
        }
        let changed = {
            let state = self.record_mut(frame, step)?;
            match state.incoming.as_mut() {
                Some(prior) => facts::merge(prior, &input),
                None => {
                    budget.retain()?;
                    state.incoming = Some(input);
                    true
                }
            }
        };
        if changed {
            self.schedule(frame, step)?;
        }
        Ok(())
    }

    pub(super) fn schedule(&mut self, frame: usize, step: StepIdx) -> FlowResult<()> {
        {
            let state = self.record(frame, step)?;
            if state.incoming.is_none() {
                return Err(invalid(step));
            }
            if state.queued {
                return Ok(());
            }
        }
        if self.queue.len() >= self.locations.len() {
            return Err(capacity());
        }
        {
            let state = self.record_mut(frame, step)?;
            state.queued = true;
        }
        self.queue.push_back((frame, step));
        Ok(())
    }

    pub(super) fn pop(&mut self) -> FlowResult<Option<(usize, StepIdx)>> {
        let key = match self.queue.pop_front() {
            Some(k) => k,
            None => return Ok(None),
        };
        {
            let state = self.record_mut(key.0, key.1)?;
            if !state.queued {
                return Err(invalid(key.1));
            }
            state.queued = false;
        }
        Ok(Some(key))
    }

    pub(super) fn snapshot(&self, frame: usize, step: StepIdx) -> FlowResult<Facts> {
        let state = self.record(frame, step)?;
        let slice = state.incoming.as_deref().ok_or_else(|| invalid(step))?;
        facts::copy(slice)
    }

    pub(super) fn state(&self, step: StepIdx) -> FlowResult<Option<&[Fact]>> {
        let frame = self.frame_of(step)?;
        let state = self.record(frame, step)?;
        Ok(state.incoming.as_deref())
    }
    pub(super) fn steps(&self, frame: usize) -> FlowResult<impl Iterator<Item = StepIdx> + '_> {
        let frame = self.frames.get(frame).ok_or_else(capacity)?;
        Ok(frame.nodes.iter().map(|node| node.step))
    }
}
