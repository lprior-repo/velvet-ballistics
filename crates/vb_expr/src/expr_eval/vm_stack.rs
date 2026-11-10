use crate::{ExprError, ExprResult};
use arrayvec::ArrayVec;
use vb_core::limits::{MAX_EXPRESSION_STACK, MAX_EXPRESSION_STACK_USIZE};
use vb_core::{SlotValue, Taint, value::join_taint};

pub(super) struct VmStack {
    values: ArrayVec<SlotValue, MAX_EXPRESSION_STACK_USIZE>,
    taints: [Option<Taint>; MAX_EXPRESSION_STACK_USIZE],
    pending: Taint,
}
impl VmStack {
    pub(super) fn new() -> Self {
        Self {
            values: ArrayVec::new(),
            taints: [None; MAX_EXPRESSION_STACK_USIZE],
            pending: Taint::Clean,
        }
    }
    pub(super) fn len(&self) -> usize {
        self.values.len()
    }
    pub(super) fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    pub(super) fn reset_operand_taint(&mut self) {
        self.pending = Taint::Clean;
    }
    pub(super) fn join_operand_taint(&mut self, taint: Taint) {
        self.pending = join_taint(self.pending, taint);
    }
    pub(super) fn push_tainted(&mut self, value: SlotValue, taint: Taint) -> ExprResult<()> {
        let target = self
            .taints
            .get_mut(self.values.len())
            .ok_or(ExprError::StackOverflow {
                max: MAX_EXPRESSION_STACK,
            })?;
        super::environment::push_value(&mut self.values, value)?;
        *target = Some(taint);
        Ok(())
    }
    pub(super) fn pop_tainted(&mut self) -> ExprResult<(SlotValue, Taint)> {
        let value = super::environment::pop_value(&mut self.values)?;
        let taint = self
            .taints
            .get_mut(self.values.len())
            .and_then(Option::take)
            .ok_or(ExprError::StackUnderflow)?;
        Ok((value, taint))
    }
    pub(super) fn push(&mut self, value: SlotValue) -> ExprResult<()> {
        let result = self.push_tainted(value, self.pending);
        self.pending = Taint::Clean;
        result
    }
    fn pop_value(&mut self) -> ExprResult<SlotValue> {
        let (value, taint) = self.pop_tainted()?;
        self.join_operand_taint(taint);
        Ok(value)
    }
}
pub(super) fn push_value(stack: &mut VmStack, value: SlotValue) -> ExprResult<()> {
    stack.push(value)
}
pub(super) fn pop_value(stack: &mut VmStack) -> ExprResult<SlotValue> {
    stack.pop_value()
}
pub(super) fn pop_pair(stack: &mut VmStack) -> ExprResult<(SlotValue, SlotValue)> {
    let right = pop_value(stack)?;
    let left = pop_value(stack)?;
    Ok((left, right))
}
