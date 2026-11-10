//! Cold, edge-sensitive assignment and operand analysis of compiled execution.
//!
//! Dynamic values retain runtime type checks. Input presence is a precondition
//! supplied by admission, never inferred from reads or missing producers.

use super::{CompiledNode, WorkflowParts};
use crate::ids::{SlotIdx, StepIdx};
use thiserror::Error;

mod budget;
mod compound;
mod evidence;
mod expression;
mod facts;
mod graph;
mod solve;
mod transfer;

use facts::{Fact, Facts};

/// One reason compiled slot flow cannot be accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotFlowFailure {
    /// A reachable read can observe an absent slot.
    UninitializedRead,
    /// A known possible operand kind is incompatible with the operation.
    IncompatibleType,
    /// Existing Gate 14 constant writers disagree on a slot's kind.
    ConflictingConstantWrites,
    /// A numeric reference or required output is invalid.
    InvalidReference,
    /// An execution variant has no modeled transfer.
    UnsupportedSemantics,
    /// Scratch allocation or the finite convergence bound was exceeded.
    AnalysisCapacity,
}

/// A cold compiled-IR dataflow failure, retaining the failing step and slot.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("slot flow at {step:?}, slot {slot:?}: {reason:?}")]
pub struct SlotFlowError {
    /// Node at which the failure was found.
    pub step: StepIdx,
    /// Operand or destination, when the failure identifies a slot.
    pub slot: Option<SlotIdx>,
    /// Failure classification.
    pub reason: SlotFlowFailure,
}

type FlowResult<T> = Result<T, SlotFlowError>;

fn failure(step: StepIdx, slot: Option<SlotIdx>, reason: SlotFlowFailure) -> SlotFlowError {
    SlotFlowError { step, slot, reason }
}

fn capacity() -> SlotFlowError {
    failure(StepIdx::new(0), None, SlotFlowFailure::AnalysisCapacity)
}

/// Validates all executable slot reads and known operand kinds.
///
/// Structural bounds/topology gates remain required independently. Dynamic
/// action outputs and accessor leaves keep their existing runtime checks.
pub fn validate_slot_flow(parts: &WorkflowParts) -> FlowResult<()> {
    solve::validate(parts)
}

struct Context<'a> {
    parts: &'a WorkflowParts,
    node: &'a CompiledNode,
    state: Facts,
    partial: Facts,
    checking: bool,
}

impl Context<'_> {
    fn error(&self, slot: Option<SlotIdx>, reason: SlotFlowFailure) -> SlotFlowError {
        failure(self.node.id, slot, reason)
    }

    fn read(&self, slot: SlotIdx) -> FlowResult<Fact> {
        let fact = self.peek(slot)?;
        if self.checking && fact.absent {
            return Err(self.error(Some(slot), SlotFlowFailure::UninitializedRead));
        }
        Ok(fact)
    }

    fn peek(&self, slot: SlotIdx) -> FlowResult<Fact> {
        self.state
            .get(slot.as_usize())
            .copied()
            .ok_or_else(|| self.error(Some(slot), SlotFlowFailure::InvalidReference))
    }

    fn require(&self, fact: Fact, allowed: u16) -> FlowResult<()> {
        if self.checking && fact.kinds & !allowed & !Fact::DYNAMIC != 0 {
            return Err(self.error(None, SlotFlowFailure::IncompatibleType));
        }
        Ok(())
    }

    fn typed_read(&self, slot: SlotIdx, allowed: u16) -> FlowResult<Fact> {
        let fact = self.read(slot)?;
        self.require(fact, allowed)?;
        Ok(fact)
    }

    fn write(&mut self, slot: SlotIdx, fact: Fact, partial: bool) -> FlowResult<()> {
        let error = self.error(Some(slot), SlotFlowFailure::InvalidReference);
        *self
            .state
            .get_mut(slot.as_usize())
            .ok_or_else(|| error.clone())? = fact;
        if partial {
            let prior = self.partial.get_mut(slot.as_usize()).ok_or(error)?;
            prior.kinds |= fact.kinds;
        }
        Ok(())
    }

    fn output(&mut self, fact: Fact) -> FlowResult<()> {
        let slot = self
            .node
            .output
            .slot()
            .ok_or_else(|| self.error(None, SlotFlowFailure::InvalidReference))?;
        self.write(slot, fact, true)
    }
}
