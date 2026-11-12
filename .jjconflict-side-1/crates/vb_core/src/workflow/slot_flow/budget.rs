use super::{FlowResult, capacity};
use crate::workflow::{CompiledNode, CompiledNodeKind as K, WorkflowParts};

// Cold analysis limits bound dense states and repeated transfer work separately
// from runtime admission. Exhaustion rejects the IR without bypassing analysis.
const MAX_FACT_CELLS: usize = 16_777_216;
const MAX_WORK: usize = 67_108_864;

pub(super) struct Budget {
    work: usize,
    states: usize,
    slots: usize,
}

impl Budget {
    pub(super) fn new(parts: &WorkflowParts) -> FlowResult<Self> {
        if parts.input_slots.len() > usize::from(parts.slot_count) {
            return Err(capacity());
        }
        for node in &parts.nodes {
            edges(node)?;
        }
        Ok(Self {
            work: MAX_WORK,
            states: 0,
            slots: usize::from(parts.slot_count),
        })
    }

    pub(super) fn retain(&mut self) -> FlowResult<()> {
        self.scratch(1)?;
        self.states = self.states.checked_add(1).ok_or_else(capacity)?;
        Ok(())
    }

    pub(super) fn release(&mut self) -> FlowResult<()> {
        self.states = self.states.checked_sub(1).ok_or_else(capacity)?;
        Ok(())
    }

    fn scratch(&self, additional: usize) -> FlowResult<()> {
        let cells = self
            .states
            .checked_add(additional)
            .and_then(|n| n.checked_mul(self.slots))
            .ok_or_else(capacity)?;
        if cells > MAX_FACT_CELLS {
            return Err(capacity());
        }
        Ok(())
    }

    pub(super) fn charge(&mut self, amount: usize) -> FlowResult<()> {
        self.work = self.work.checked_sub(amount).ok_or_else(capacity)?;
        Ok(())
    }

    pub(super) fn transfer(
        &mut self,
        parts: &WorkflowParts,
        node: &CompiledNode,
    ) -> FlowResult<()> {
        let expression = |expr: crate::ids::ExprIdx| -> FlowResult<usize> {
            let Some(program) = parts.expressions.get(expr.as_usize()) else {
                return Ok(0);
            };
            program
                .ops
                .len()
                .checked_mul(
                    crate::limits::MAX_PATH_DEPTH
                        .checked_add(1)
                        .ok_or_else(capacity)?,
                )
                .ok_or_else(capacity)
        };
        let reads = match &node.kind {
            K::BuildObject { fields } => fields.len(),
            K::BuildList { items } => items.len(),
            K::EvalExpr { expr } => expression(*expr)?,
            K::Choose { branches, .. } => branches.iter().try_fold(0usize, |total, b| {
                total
                    .checked_add(expression(b.condition)?)
                    .ok_or_else(capacity)
            })?,
            K::ChooseSlot { branches, .. } => branches.len(),
            _ => 4,
        };
        let edges = edges(node)?;
        // Retained inputs plus live transition results, input, and context copies.
        self.scratch(edges.checked_add(4).ok_or_else(capacity)?)?;
        // Each edge copies two context states, joins partial writes, and may
        // merge into its destination. Terminals still construct one context.
        let scans = edges
            .checked_mul(4)
            .and_then(|n| n.checked_add(1))
            .ok_or_else(capacity)?
            .max(4);
        let work = self
            .slots
            .checked_mul(scans)
            .and_then(|n| {
                reads
                    .checked_mul(edges.max(1))
                    .and_then(|r| n.checked_add(r))
            })
            .ok_or_else(capacity)?;
        self.charge(work)?;
        if node.routing.error.handler().is_some() && node.routing.error.error_slot().is_none() {
            self.charge(parts.nodes.len())?;
        }
        Ok(())
    }
}

fn edges(node: &CompiledNode) -> FlowResult<usize> {
    let branches = match &node.kind {
        K::Choose { branches, .. } => branches.len(),
        K::ChooseSlot { branches, .. } => branches.len(),
        _ => 0,
    };
    if branches > usize::from(crate::limits::MAX_FANOUT) {
        return Err(capacity());
    }
    let count = super::graph::successors(node)?.len();
    count
        .checked_add(usize::from(node.routing.error.handler().is_some()))
        .ok_or_else(capacity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_and_scratch_states_share_the_unchanged_cell_bound() -> FlowResult<()> {
        let mut budget = Budget {
            work: MAX_WORK,
            states: 0,
            slots: 1024,
        };
        for _ in 0..16_380 {
            budget.retain()?;
        }
        budget.scratch(4)?;
        assert_eq!(budget.scratch(5), Err(capacity()));
        budget.release()?;
        budget.scratch(5)?;
        Ok(())
    }

    #[test]
    fn work_exhaustion_fails_closed_at_the_boundary() -> FlowResult<()> {
        let mut budget = Budget {
            work: MAX_WORK,
            states: 0,
            slots: 1,
        };
        budget.charge(MAX_WORK)?;
        assert_eq!(budget.charge(1), Err(capacity()));
        Ok(())
    }
}
