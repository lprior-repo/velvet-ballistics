use crate::limits::MAX_EXPRESSION_STACK_USIZE;
use crate::value::{Taint, join_taint};

pub(crate) struct OperandTaints {
    entries: [Option<Taint>; MAX_EXPRESSION_STACK_USIZE],
    pending: Taint,
}
impl OperandTaints {
    pub(crate) const fn new() -> Self {
        Self {
            entries: [None; MAX_EXPRESSION_STACK_USIZE],
            pending: Taint::Clean,
        }
    }
    pub(crate) fn reset(&mut self) {
        self.pending = Taint::Clean;
    }
    pub(crate) fn join(&mut self, taint: Taint) {
        self.pending = join_taint(self.pending, taint);
    }
    pub(crate) const fn pending(&self) -> Taint {
        self.pending
    }
    pub(crate) fn put(&mut self, index: usize, taint: Taint) -> Option<()> {
        *self.entries.get_mut(index)? = Some(taint);
        Some(())
    }
    pub(crate) fn peek(&self, index: usize) -> Option<Taint> {
        self.entries.get(index).copied().flatten()
    }
    pub(crate) fn take(&mut self, index: usize) -> Option<Taint> {
        self.entries.get_mut(index)?.take()
    }
}

#[cfg(kani)]
mod kani_harnesses;
