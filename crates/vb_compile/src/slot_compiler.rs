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
