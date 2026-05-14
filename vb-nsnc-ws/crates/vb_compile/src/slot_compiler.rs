#![forbid(unsafe_code)]
//! Slot compiler state for building node arrays during step lowering.
//!
//! Tracks slot allocation, constant pool, expression programs, and accessor
//! programs during step lowering.

mod compile_error;

pub use compile_error::{CompileError, CompileErrors};

use vb_core::{
    AccessorProgram, ActionId, CompiledNode, ConstIdx, ConstValue, ExprIdx, ExprProgram,
    ResourceContract, SlotIdx, StepIdx, WorkflowDigest, WorkflowParts,
};

use crate::SourceMark;

/// Mutable slot compiler state for building node arrays.
///
/// Tracks slot allocation, constant pool, expression programs, and accessor
/// programs during step lowering.
#[derive(Debug, Default)]
pub struct SlotCompiler {
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    expressions: Vec<ExprProgram>,
    accessors: Vec<AccessorProgram>,
    max_slot: Option<usize>,
}

impl SlotCompiler {
    /// Creates a new empty slot compiler.
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

    /// Pushes an expression program and returns its index.
    pub fn push_expression(&mut self, program: ExprProgram) -> Result<ExprIdx, CompileError> {
        let index = u16::try_from(self.expressions.len()).map_err(|_| {
            CompileError::ExpressionLoweringUnsupported {
                feature: "expression table overflow",
            }
        })?;
        self.expressions.push(program);
        Ok(ExprIdx::new(index))
    }

    /// Pushes an accessor program and returns its index.
    pub fn push_accessor(
        &mut self,
        program: AccessorProgram,
    ) -> Result<vb_core::AccessorIdx, CompileError> {
        let index = u16::try_from(self.accessors.len()).map_err(|_| {
            CompileError::ExpressionLoweringUnsupported {
                feature: "accessor table overflow",
            }
        })?;
        self.accessors.push(program);
        Ok(vb_core::AccessorIdx::new(index))
    }

    /// Records a slot reference for slot count tracking.
    pub fn record_slot(&mut self, slot: SlotIdx) {
        let value = slot.as_usize();
        self.max_slot = Some(match self.max_slot {
            Some(current) => current.max(value),
            None => value,
        });
    }

    /// Pushes a compiled node into the node array.
    pub fn push_node(&mut self, node: CompiledNode) {
        self.nodes.push(node);
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

    /// Builds the final workflow parts from accumulated state.
    pub fn build_parts(
        self,
        name: &str,
        digest: WorkflowDigest,
    ) -> Result<WorkflowParts, CompileError> {
        Ok(WorkflowParts {
            name: Box::from(name),
            digest,
            slot_count: self.slot_count()?,
            symbols_count: 0,
            nodes: self.nodes.into_boxed_slice(),
            expressions: self.expressions.into_boxed_slice(),
            accessors: self.accessors.into_boxed_slice(),
            constants: self.constants.into_boxed_slice(),
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::workflow::ExprOp;

    fn ensure(condition: bool, message: &'static str) -> Result<(), String> {
        if condition {
            Ok(())
        } else {
            Err(message.to_owned())
        }
    }

    #[test]
    fn new_compiler_has_zero_slot_count() -> Result<(), String> {
        let compiler = SlotCompiler::new();
        ensure(compiler.slot_count()? == 0, "new compiler should report zero slots")
    }

    #[test]
    fn record_slot_tracks_max() -> Result<(), String> {
        let mut compiler = SlotCompiler::new();
        compiler.record_slot(SlotIdx::new(5));
        ensure(compiler.slot_count()? == 6, "should be max_slot + 1")?;
        compiler.record_slot(SlotIdx::new(2));
        ensure(compiler.slot_count()? == 6, "lower slot should not change max")
    }

    #[test]
    fn push_constant_returns_sequential_indices() -> Result<(), String> {
        let mut compiler = SlotCompiler::new();
        let a = compiler.push_constant(ConstValue::I64(1))?;
        let b = compiler.push_constant(ConstValue::Bool(false))?;
        let c = compiler.push_constant(ConstValue::Null)?;
        ensure(a.as_u16() == 0, "first index should be 0")?;
        ensure(b.as_u16() == 1, "second index should be 1")?;
        ensure(c.as_u16() == 2, "third index should be 2")
    }

    #[test]
    fn push_constant_overflow_rejected() -> Result<(), String> {
        let mut compiler = SlotCompiler::new();
        let count = usize::from(u16::MAX);
        for i in 0..count {
            let val = i64::try_from(i).map_err(|e| e.to_string())?;
            compiler.push_constant(ConstValue::I64(val))?;
        }
        match compiler.push_constant(ConstValue::I64(0)) {
            Err(CompileError::Workflow(_)) => Ok(()),
            other => Err(format!("expected Workflow error, got {other:?}")),
        }
    }

    #[test]
    fn push_expression_returns_sequential_indices() -> Result<(), String> {
        let mut compiler = SlotCompiler::new();
        let program_a = ExprProgram::try_from_ops(
            vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice(),
        )
        .map_err(|e| format!("expr program: {e:?}"))?;
        let program_b = ExprProgram::try_from_ops(
            vec![ExprOp::LoadConst(ConstIdx::new(1))].into_boxed_slice(),
        )
        .map_err(|e| format!("expr program: {e:?}"))?;
        let a = compiler.push_expression(program_a)?;
        let b = compiler.push_expression(program_b)?;
        ensure(a.as_u16() == 0, "first expression index should be 0")?;
        ensure(b.as_u16() == 1, "second expression index should be 1")
    }

    #[test]
    fn push_accessor_returns_sequential_indices() -> Result<(), String> {
        let mut compiler = SlotCompiler::new();
        let acc_a = AccessorProgram {
            root: SlotIdx::new(0),
            path: Box::new([]),
        };
        let acc_b = AccessorProgram {
            root: SlotIdx::new(1),
            path: Box::new([]),
        };
        let a = compiler.push_accessor(acc_a)?;
        let b = compiler.push_accessor(acc_b)?;
        ensure(a.as_u16() == 0, "first accessor index should be 0")?;
        ensure(b.as_u16() == 1, "second accessor index should be 1")
    }

    #[test]
    fn push_node_accumulates_nodes() -> Result<(), String> {
        let mut compiler = SlotCompiler::new();
        let node_a = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        };
        let node_b = CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(2)),
            error_slot: None,
            on_error: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        };
        compiler.push_node(node_a);
        compiler.push_node(node_b);
        ensure(compiler.nodes.len() == 2, "should have 2 nodes")
    }

    #[test]
    fn build_parts_preserves_constants_and_slots() -> Result<(), String> {
        let mut compiler = SlotCompiler::new();
        compiler.push_constant(ConstValue::I64(42))?;
        compiler.push_constant(ConstValue::Bool(true))?;
        compiler.record_slot(SlotIdx::new(3));
        compiler.record_slot(SlotIdx::new(7));
        let digest = WorkflowDigest::from_bytes([0u8; 32]);
        let parts = compiler.build_parts("test", digest)?;
        ensure(parts.constants.len() == 2, "should have 2 constants")?;
        ensure(parts.slot_count == 8, "slot_count should be max + 1")?;
        ensure(parts.name.as_ref() == "test", "name should be preserved")
    }

    #[test]
    fn build_parts_empty_compiler() -> Result<(), String> {
        let compiler = SlotCompiler::new();
        let digest = WorkflowDigest::from_bytes([0u8; 32]);
        let parts = compiler.build_parts("empty", digest)?;
        ensure(parts.slot_count == 0, "empty compiler should have 0 slots")?;
        ensure(parts.constants.is_empty(), "should have no constants")?;
        ensure(parts.nodes.is_empty(), "should have no nodes")
    }
}
