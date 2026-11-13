use super::super::{budget::Budget, facts};
use super::layout::Layout;
use super::store::Store;
use super::{Fact, FlowResult, capacity, check, dispatch, invalid, layout, slots, writers};
use crate::ids::StepIdx;
use crate::workflow::{WorkflowParts, concurrent_regions::ConcurrentGroupRegion};

pub(super) struct Solver<'a> {
    pub(super) parts: &'a WorkflowParts,
    pub(super) layout: Layout<'a>,
    pub(super) store: Store,
    pub(super) results: Vec<Option<Fact>>,
    pub(super) budget: Budget,
}

impl<'a> Solver<'a> {
    pub(super) fn new(
        parts: &'a WorkflowParts,
        regions: &'a [ConcurrentGroupRegion],
    ) -> FlowResult<Self> {
        let mut budget = Budget::new(parts)?;
        budget.charge(parts.nodes.len().checked_mul(4).ok_or_else(capacity)?)?;
        let layout = layout::build(parts, regions)?;
        let store = Store::new(parts, &layout)?;
        writers::validate(parts, &layout, &store, &mut budget)?;
        let results = slots(regions.len(), None)?;
        let mut solver = Self {
            parts,
            layout,
            store,
            results,
            budget,
        };
        solver.seed()?;
        Ok(solver)
    }

    fn seed(&mut self) -> FlowResult<()> {
        self.budget.scratch(1)?;
        let facts = facts::seed(self.parts)?;
        self.store
            .propagate(0, self.parts.entry, facts, &mut self.budget)
    }

    fn process(&mut self, frame: usize, step: StepIdx) -> FlowResult<()> {
        let parts = self.parts;
        let node = parts
            .nodes
            .get(step.as_usize())
            .ok_or_else(|| invalid(step))?;
        self.budget.transfer(parts, node)?;
        let input = self.store.snapshot(frame, step)?;
        dispatch::step(self, frame, node, &input)
    }

    fn run(mut self) -> FlowResult<()> {
        let bound = bound(self.parts, self.layout.regions.len())?;
        for _ in 0..bound {
            let Some((frame, step)) = self.store.pop()? else {
                return check::validate(&mut self);
            };
            self.process(frame, step)?;
        }
        Err(capacity())
    }
}

fn bound(parts: &WorkflowParts, groups: usize) -> FlowResult<usize> {
    let height = usize::from(parts.slot_count)
        .checked_mul(10)
        .and_then(|n| n.checked_add(1))
        .ok_or_else(capacity)?;
    let groups = groups
        .checked_mul(11)
        .and_then(|n| n.checked_add(1))
        .ok_or_else(capacity)?;
    // Incoming slots have ten growing bits; each group adds one observation and ten bits.
    parts
        .nodes
        .len()
        .checked_mul(height)
        .and_then(|n| n.checked_add(groups))
        .ok_or_else(capacity)
}

pub(super) fn validate<'a>(
    parts: &'a WorkflowParts,
    regions: &'a [ConcurrentGroupRegion],
) -> FlowResult<()> {
    Solver::new(parts, regions)?.run()
}
