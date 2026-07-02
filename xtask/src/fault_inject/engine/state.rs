#![forbid(unsafe_code)]

//! Internal engine-state primitives used by [`super::run_fault_injection`].

use crate::fault_inject::prng::SplitMix64;
use crate::fault_inject::report::{FaultOutcome, JournalOutcome};
use crate::fault_inject::types::{BudgetKind, FaultError};

/// Mutable state threaded through the deterministic simulator.
#[derive(Debug, Clone)]
pub(crate) struct SimulationState {
    pub(super) seed: u64,
    pub(crate) prng: SplitMix64,
    pub(crate) events_applied: u32,
    pub(crate) runtime_steps: u32,
    pub(crate) journal_entries: Vec<JournalOutcome>,
    pub(crate) outcomes: Vec<FaultOutcome>,
    pub(crate) recovery_required: bool,
    pub(crate) next_seq: u64,
}

impl SimulationState {
    pub(crate) fn new(seed: u64) -> Self {
        Self {
            seed,
            prng: SplitMix64::new(seed),
            events_applied: 0,
            runtime_steps: 0,
            journal_entries: Vec::new(),
            outcomes: Vec::new(),
            recovery_required: false,
            next_seq: 0,
        }
    }

    pub(crate) fn alloc_seq(&mut self) -> Result<u64, FaultError> {
        let next = self.next_seq.checked_add(1).ok_or_else(|| {
            FaultError::InvalidConfig("journal sequence counter overflowed".to_owned())
        })?;
        self.next_seq = next;
        Ok(next)
    }

    pub(crate) fn consume_steps(&mut self, steps: u32, limit: u32) -> Result<(), FaultError> {
        let observed = self.runtime_steps.saturating_add(steps);
        if observed > limit {
            return Err(FaultError::BudgetExceeded {
                budget_kind: BudgetKind::RuntimeSteps,
                observed,
                limit,
            });
        }
        self.runtime_steps = observed;
        Ok(())
    }

    pub(crate) fn consume_fault(&mut self, limit: u32) -> Result<(), FaultError> {
        let observed = self.events_applied.saturating_add(1);
        if observed > limit {
            return Err(FaultError::BudgetExceeded {
                budget_kind: BudgetKind::Faults,
                observed,
                limit,
            });
        }
        self.events_applied = observed;
        Ok(())
    }
}
