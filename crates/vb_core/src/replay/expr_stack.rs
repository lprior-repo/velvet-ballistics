#![forbid(unsafe_code)]
//! Bounded expression evaluation stack for replay.

use crate::ids::StepIdx;
use crate::value::SlotValue;

use super::ReplayError;

pub struct ReplayExprStack {
    values: [SlotValue; crate::limits::MAX_EXPRESSION_STACK_USIZE],
    pub(crate) len: u8,
    capacity: u8,
}

impl ReplayExprStack {
    #[cfg(test)]
    pub(crate) fn new(capacity: u8) -> Result<Self, ReplayError> {
        if usize::from(capacity) <= crate::limits::MAX_EXPRESSION_STACK_USIZE {
            Ok(Self {
                values: [SlotValue::Null; crate::limits::MAX_EXPRESSION_STACK_USIZE],
                len: 0,
                capacity,
            })
        } else {
            Err(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })
        }
    }

    pub(crate) fn push(&mut self, value: SlotValue) -> Result<(), ReplayError> {
        if self.len >= self.capacity {
            return Err(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            });
        }
        let index = usize::from(self.len);
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

    pub(crate) fn pop(&mut self) -> Result<SlotValue, ReplayError> {
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
        self.values
            .get(usize::from(self.len))
            .copied()
            .ok_or(ReplayError::ExpressionEvalFailed {
                step: StepIdx::ZERO,
            })
    }
}
