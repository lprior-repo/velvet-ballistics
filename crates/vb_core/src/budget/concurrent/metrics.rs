use crate::budget::BudgetTraversalError;
use crate::workflow::{CompiledNode, CompiledNodeKind as K};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Metrics {
    pub(super) steps: u64,
    pub(super) actions: u64,
    pub(super) timers: u64,
    pub(super) gates: u64,
    pub(super) signals: u64,
    pub(super) pages: u64,
    pub(super) items: u64,
    pub(super) iterations: u64,
    pub(super) fanout: u16,
    pub(super) together: u16,
    pub(super) repeat: u16,
    pub(super) depth: u16,
    pub(super) parallel: u16,
}

pub(super) fn overflow() -> BudgetTraversalError {
    BudgetTraversalError::StepCountOverflow { actual: u64::MAX }
}

pub(super) fn add(a: u64, b: u64) -> Result<u64, BudgetTraversalError> {
    a.checked_add(b).ok_or(overflow())
}

pub(super) fn mul(a: u64, b: u64) -> Result<u64, BudgetTraversalError> {
    a.checked_mul(b).ok_or(overflow())
}

fn branch_count(len: usize) -> Result<u16, BudgetTraversalError> {
    let v = u64::try_from(len).map_err(|_| overflow())?;
    u16::try_from(v).map_err(|_| BudgetTraversalError::StepCountOverflow { actual: v })
}

impl Metrics {
    fn zero() -> Self {
        Self::default()
    }

    fn resource_metrics(kind: &K, m: &mut Self) -> Result<(), BudgetTraversalError> {
        match kind {
            K::CollectStart {
                limit, page_size, ..
            } => {
                let it = u64::from(limit.get());
                m.items = it;
                let denom = u64::from(*page_size).max(1);
                m.pages = it.div_ceil(denom);
            }
            K::ForEachStart { limit, .. } => {
                m.iterations = u64::from(limit.get());
            }
            K::ForEachConcurrentStart { limit, at_once, .. } => {
                let ao = at_once.get();
                if ao == 0 {
                    return Err(BudgetTraversalError::StepCountOverflow { actual: 0 });
                }
                m.iterations = u64::from(limit.get());
            }
            K::RepeatStart { max_attempts, .. } => {
                m.repeat = max_attempts.get();
            }
            _ => {}
        }
        Ok(())
    }

    fn shape_metrics(kind: &K, m: &mut Self) -> Result<(), BudgetTraversalError> {
        match kind {
            K::TogetherStart { branches, .. }
            | K::TogetherConcurrentStart { branches, .. }
            | K::FirstSuccessStart { branches, .. } => {
                let bc = branch_count(branches.len())?;
                m.fanout = bc;
                m.together = bc;
            }
            K::ForEachConcurrentStart { at_once, .. } => {
                let ao = at_once.get();
                m.fanout = ao;
            }
            K::Choose { branches, .. } => {
                m.fanout = branch_count(branches.len())?;
            }
            K::ChooseSlot { branches, .. } => m.fanout = branch_count(branches.len())?,
            _ => {}
        }
        Ok(())
    }

    pub(super) fn leaf(node: &CompiledNode) -> Result<Self, BudgetTraversalError> {
        let mut m = Self::zero();
        m.steps = 1;
        match node.kind {
            K::Do { .. } => m.actions = 1,
            K::WaitUntil { .. } | K::RetryCheck { .. } | K::RepeatCheck { .. } => m.timers = 1,
            K::Ask { .. } => {
                m.timers = 1;
                m.gates = 1;
            }
            K::WaitEvent { .. } => {
                m.timers = 1;
                m.signals = 1;
            }
            _ => {}
        }
        m.timers = add(
            m.timers,
            u64::from(node.retry.is_some_and(|p| p.delay_ms > 0)),
        )?;
        Self::resource_metrics(&node.kind, &mut m)?;
        Self::shape_metrics(&node.kind, &mut m)?;
        Ok(m)
    }

    pub(super) fn combine(self, other: Self) -> Result<Self, BudgetTraversalError> {
        Ok(Self {
            steps: add(self.steps, other.steps)?,
            actions: add(self.actions, other.actions)?,
            timers: add(self.timers, other.timers)?,
            gates: add(self.gates, other.gates)?,
            signals: add(self.signals, other.signals)?,
            pages: add(self.pages, other.pages)?,
            items: add(self.items, other.items)?,
            iterations: add(self.iterations, other.iterations)?,
            fanout: self.fanout.max(other.fanout),
            together: self.together.max(other.together),
            repeat: self.repeat.max(other.repeat),
            depth: self.depth.max(other.depth),
            parallel: self.parallel.max(other.parallel),
        })
    }

    pub(super) fn scale(self, n: u64) -> Result<Self, BudgetTraversalError> {
        Ok(Self {
            steps: mul(self.steps, n)?,
            actions: mul(self.actions, n)?,
            timers: mul(self.timers, n)?,
            gates: mul(self.gates, n)?,
            signals: mul(self.signals, n)?,
            pages: mul(self.pages, n)?,
            items: mul(self.items, n)?,
            iterations: mul(self.iterations, n)?,
            fanout: self.fanout,
            together: self.together,
            repeat: self.repeat,
            depth: self.depth,
            parallel: self.parallel,
        })
    }

    pub(super) fn maximum(self, other: Self) -> Self {
        Self {
            steps: self.steps.max(other.steps),
            actions: self.actions.max(other.actions),
            timers: self.timers.max(other.timers),
            gates: self.gates.max(other.gates),
            signals: self.signals.max(other.signals),
            pages: self.pages.max(other.pages),
            items: self.items.max(other.items),
            iterations: self.iterations.max(other.iterations),
            fanout: self.fanout.max(other.fanout),
            together: self.together.max(other.together),
            repeat: self.repeat.max(other.repeat),
            depth: self.depth.max(other.depth),
            parallel: self.parallel.max(other.parallel),
        }
    }

    pub(super) fn nested(mut self) -> Result<Self, BudgetTraversalError> {
        let next = add(u64::from(self.depth), 1)?;
        self.depth = u16::try_from(next)
            .map_err(|_| BudgetTraversalError::DepthOverflow { depth: self.depth })?;
        Ok(self)
    }
}
