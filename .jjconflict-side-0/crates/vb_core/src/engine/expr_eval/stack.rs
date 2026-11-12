#![forbid(unsafe_code)]
//! Expression evaluation stack.

use super::taints::OperandTaints;
use crate::errors::EngineError;
use crate::limits::MAX_EXPRESSION_STACK_USIZE;
use crate::value::{SlotValue, Taint};

pub(super) struct ExprStack {
    values: [SlotValue; MAX_EXPRESSION_STACK_USIZE],
    len: u8,
    capacity: u8,
    taints: OperandTaints,
}

impl ExprStack {
    pub(super) fn new(capacity: u8) -> Result<Self, EngineError> {
        if usize::from(capacity) <= MAX_EXPRESSION_STACK_USIZE {
            Ok(Self {
                values: [SlotValue::Null; MAX_EXPRESSION_STACK_USIZE],
                len: 0,
                capacity,
                taints: OperandTaints::new(),
            })
        } else {
            Err(EngineError::ExpressionStackOverflow { max: capacity })
        }
    }

    pub(super) const fn len(&self) -> u8 {
        self.len
    }

    pub(super) fn result_taint(&self) -> Result<Taint, EngineError> {
        self.len
            .checked_sub(1)
            .and_then(|index| self.taints.peek(usize::from(index)))
            .ok_or(EngineError::InternalInvariantViolation {
                reason: "expression result taint missing".into(),
            })
    }
    pub(super) fn pending_operand_taint(&self) -> Taint {
        self.taints.pending()
    }
    pub(super) fn reset_operand_taint(&mut self) {
        self.taints.reset();
    }
    pub(super) fn join_operand_taint(&mut self, taint: Taint) {
        self.taints.join(taint);
    }
    pub(super) fn push(&mut self, value: SlotValue) -> Result<(), EngineError> {
        let result = self.push_tainted(value, self.taints.pending());
        self.taints.reset();
        result
    }
    pub(super) fn pop(&mut self) -> Result<SlotValue, EngineError> {
        let (value, taint) = self.pop_tainted()?;
        self.taints.join(taint);
        Ok(value)
    }
    pub(super) fn push_tainted(
        &mut self,
        value: SlotValue,
        taint: Taint,
    ) -> Result<(), EngineError> {
        if self.len >= self.capacity {
            return Err(EngineError::ExpressionStackOverflow { max: self.capacity });
        }
        let index = usize::from(self.len);
        self.taints
            .put(index, taint)
            .ok_or(EngineError::ExpressionStackOverflow { max: self.capacity })?;
        *self
            .values
            .get_mut(index)
            .ok_or(EngineError::ExpressionStackOverflow { max: self.capacity })? = value;
        self.len = self
            .len
            .checked_add(1)
            .ok_or(EngineError::ExpressionStackOverflow { max: self.capacity })?;
        Ok(())
    }

    pub(super) fn pop_tainted(&mut self) -> Result<(SlotValue, Taint), EngineError> {
        if self.len == 0 {
            return Err(EngineError::ExpressionStackUnderflow);
        }
        self.len = self
            .len
            .checked_sub(1)
            .ok_or(EngineError::ExpressionStackUnderflow)?;
        let value = self.values.get(usize::from(self.len)).copied().ok_or(
            EngineError::InternalInvariantViolation {
                reason: "expression stack pop index checked by length".into(),
            },
        )?;
        let taint = self.taints.take(usize::from(self.len)).ok_or(
            EngineError::InternalInvariantViolation {
                reason: "expression stack taint missing".into(),
            },
        )?;
        Ok((value, taint))
    }
}

pub(super) fn push_value(stack: &mut ExprStack, value: SlotValue) -> Result<(), EngineError> {
    stack.push(value)
}

pub(super) fn pop_value(stack: &mut ExprStack) -> Result<SlotValue, EngineError> {
    stack.pop()
}

pub(super) fn pop_pair(stack: &mut ExprStack) -> Result<(SlotValue, SlotValue), EngineError> {
    let right = pop_value(stack)?;
    let left = pop_value(stack)?;
    Ok((left, right))
}

#[cfg(test)]
pub(super) fn pop_triple(
    stack: &mut ExprStack,
) -> Result<(SlotValue, SlotValue, SlotValue), EngineError> {
    let right = pop_value(stack)?;
    let mid = pop_value(stack)?;
    let left = pop_value(stack)?;
    Ok((left, mid, right))
}

#[cfg(test)]
pub(super) fn pop_i64_pair(stack: &mut ExprStack) -> Result<(i64, i64), EngineError> {
    let (left, right) = pop_pair(stack)?;
    Ok((expect_i64(left)?, expect_i64(right)?))
}

pub(super) fn expect_bool(value: SlotValue) -> Result<bool, EngineError> {
    match value {
        SlotValue::Bool(value) => Ok(value),
        other => Err(EngineError::TypeMismatch {
            expected: "boolean",
            found: other.type_name(),
        }),
    }
}

pub(super) fn expect_i64(value: SlotValue) -> Result<i64, EngineError> {
    match value {
        SlotValue::I64(value) => Ok(value),
        other => Err(EngineError::TypeMismatch {
            expected: "number",
            found: other.type_name(),
        }),
    }
}

pub(super) fn expect_symbol(value: SlotValue) -> Result<crate::ids::SymbolId, EngineError> {
    match value {
        SlotValue::Symbol(id) => Ok(id),
        other => Err(EngineError::TypeMismatch {
            expected: "text",
            found: other.type_name(),
        }),
    }
}

pub(super) fn expect_list(value: SlotValue) -> Result<crate::ids::ListId, EngineError> {
    match value {
        SlotValue::List(id) => Ok(id),
        other => Err(EngineError::TypeMismatch {
            expected: "list",
            found: other.type_name(),
        }),
    }
}

pub(super) fn expect_object(value: SlotValue) -> Result<crate::ids::ObjectId, EngineError> {
    match value {
        SlotValue::Object(id) => Ok(id),
        other => Err(EngineError::TypeMismatch {
            expected: "object",
            found: other.type_name(),
        }),
    }
}

#[cfg(test)]
#[path = "stack/tests.rs"]
mod tests;
