#![forbid(unsafe_code)]
//! Bounded expression evaluation stack for replay.

use crate::engine::expr_eval::taints::OperandTaints;
use crate::ids::StepIdx;
use crate::value::{SlotValue, Taint};

use super::ReplayError;

pub struct ReplayExprStack {
    values: [SlotValue; crate::limits::MAX_EXPRESSION_STACK_USIZE],
    pub(crate) len: u8,
    capacity: u8,
    taints: OperandTaints,
}

impl ReplayExprStack {
    #[cfg(test)]
    pub(crate) fn new(capacity: u8) -> Result<Self, ReplayError> {
        if usize::from(capacity) <= crate::limits::MAX_EXPRESSION_STACK_USIZE {
            Ok(Self {
                values: [SlotValue::Null; crate::limits::MAX_EXPRESSION_STACK_USIZE],
                len: 0,
                capacity,
                taints: OperandTaints::new(),
            })
        } else {
            Err(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })
        }
    }

    pub(crate) fn result_taint(&self) -> Result<Taint, ReplayError> {
        self.len
            .checked_sub(1)
            .and_then(|index| self.taints.peek(usize::from(index)))
            .ok_or(ReplayError::Internal {
                reason: "expression result taint missing",
            })
    }
    pub(crate) fn reset_operand_taint(&mut self) {
        self.taints.reset();
    }
    pub(crate) fn join_operand_taint(&mut self, taint: Taint) {
        self.taints.join(taint);
    }
    pub(crate) fn push(&mut self, value: SlotValue) -> Result<(), ReplayError> {
        let result = self.push_tainted(value, self.taints.pending());
        self.taints.reset();
        result
    }
    pub(crate) fn pop(&mut self) -> Result<SlotValue, ReplayError> {
        let (value, taint) = self.pop_tainted()?;
        self.taints.join(taint);
        Ok(value)
    }
    pub(crate) fn push_tainted(
        &mut self,
        value: SlotValue,
        taint: Taint,
    ) -> Result<(), ReplayError> {
        if self.len >= self.capacity {
            return Err(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            });
        }
        let index = usize::from(self.len);
        self.taints
            .put(index, taint)
            .ok_or(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })?;
        *self
            .values
            .get_mut(index)
            .ok_or(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })? = value;
        self.len = self
            .len
            .checked_add(1)
            .ok_or(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })?;
        Ok(())
    }

    pub(crate) fn pop_tainted(&mut self) -> Result<(SlotValue, Taint), ReplayError> {
        if self.len == 0 {
            return Err(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            });
        }
        self.len = self
            .len
            .checked_sub(1)
            .ok_or(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })?;
        let value = self.values.get(usize::from(self.len)).copied().ok_or(
            ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            },
        )?;
        let taint = self
            .taints
            .take(usize::from(self.len))
            .ok_or(ReplayError::Internal {
                reason: "expression stack taint missing",
            })?;
        Ok((value, taint))
    }
}
