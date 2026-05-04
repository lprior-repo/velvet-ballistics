#![forbid(unsafe_code)]
//! Workflow builder for step compilation.

use vb_core::{CompiledNode, ConstIdx, ConstValue, SlotIdx};

use super::slot_compiler::CompileError;

/// Workflow builder state for step compilation.
#[derive(Debug, Default)]
pub struct WorkflowBuilder {
    pub(crate) nodes: Vec<CompiledNode>,
    pub(crate) constants: Vec<ConstValue>,
    pub(crate) max_slot: Option<usize>,
}

impl WorkflowBuilder {
    /// Creates a new empty workflow builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes a constant value and returns its index.
    pub fn push_constant(&mut self, value: ConstValue) -> Result<ConstIdx, CompileError> {
        let index = u16::try_from(self.constants.len()).map_err(|_| {
            CompileError::Workflow(vb_core::WorkflowError::ConstOutOfBounds {
                constant: ConstIdx::new(u16::MAX),
            })
        })?;
        self.constants.push(value);
        Ok(ConstIdx::new(index))
    }

    /// Records a slot reference for slot count tracking.
    pub fn record_slot(&mut self, slot: SlotIdx) {
        let value = slot.as_usize();
        self.max_slot = Some(match self.max_slot {
            Some(current) => current.max(value),
            None => value,
        });
    }

    /// Returns the current slot count.
    pub fn slot_count(&self) -> Result<u16, CompileError> {
        match self.max_slot {
            Some(value) => {
                let count = value
                    .checked_add(1)
                    .ok_or(CompileError::SlotIndexOutOfRange { value: i64::MAX })?;
                u16::try_from(count).map_err(|_| CompileError::SlotIndexOutOfRange {
                    value: i64::from(u16::MAX),
                })
            }
            None => Ok(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ensure(condition: bool, message: &'static str) -> Result<(), String> {
        if condition {
            Ok(())
        } else {
            Err(message.to_owned())
        }
    }

    #[test]
    fn new_builder_has_zero_slot_count() -> Result<(), String> {
        let builder = WorkflowBuilder::new();
        ensure(builder.slot_count()? == 0, "new builder should report zero slots")
    }

    #[test]
    fn record_slot_tracks_max_slot() -> Result<(), String> {
        let mut builder = WorkflowBuilder::new();
        builder.record_slot(SlotIdx::new(3));
        ensure(builder.slot_count()? == 4, "slot_count should be max_slot + 1")?;
        builder.record_slot(SlotIdx::new(7));
        ensure(builder.slot_count()? == 8, "slot_count should track max")?;
        builder.record_slot(SlotIdx::new(1));
        ensure(builder.slot_count()? == 8, "recording lower slot should not decrease count")
    }

    #[test]
    fn push_constant_returns_sequential_indices() -> Result<(), String> {
        let mut builder = WorkflowBuilder::new();
        let a = builder.push_constant(ConstValue::I64(10))?;
        let b = builder.push_constant(ConstValue::Bool(true))?;
        let c = builder.push_constant(ConstValue::Null)?;
        ensure(a.as_u16() == 0, "first constant should be index 0")?;
        ensure(b.as_u16() == 1, "second constant should be index 1")?;
        ensure(c.as_u16() == 2, "third constant should be index 2")
    }

    #[test]
    fn push_constant_overflow_rejected() -> Result<(), String> {
        let mut builder = WorkflowBuilder::new();
        let count = usize::from(u16::MAX);
        for i in 0..count {
            let val = i64::try_from(i).map_err(|e| e.to_string())?;
            builder.push_constant(ConstValue::I64(val))?;
        }
        match builder.push_constant(ConstValue::I64(0)) {
            Err(CompileError::Workflow(_)) => Ok(()),
            other => Err(format!("expected Workflow error, got {other:?}")),
        }
    }

    #[test]
    fn default_builder_is_empty() -> Result<(), String> {
        let builder = WorkflowBuilder::default();
        ensure(builder.constants.is_empty(), "constants should be empty")?;
        ensure(builder.nodes.is_empty(), "nodes should be empty")?;
        ensure(builder.max_slot.is_none(), "max_slot should be None")
    }
}
