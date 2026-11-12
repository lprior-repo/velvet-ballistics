use super::*;
use crate::mod_compile_errors::CompileError;
use saphyr::Yaml;
use vb_core::{CompiledNode, ConstIdx, ConstValue, SlotIdx, StepIdx, WorkflowError};

// DEAD_CODE: confirmed unused via grep
#[derive(Debug, Default)]
#[allow(dead_code)]
pub(super) struct WorkflowBuilder {
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    max_slot: Option<usize>,
}

impl WorkflowBuilder {
    // DEAD_CODE: confirmed unused via grep
    #[allow(dead_code)]
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn push_constant(&mut self, value: ConstValue) -> Result<ConstIdx, CompileError> {
        let index = u16::try_from(self.constants.len()).map_err(|_| {
            CompileError::Workflow(WorkflowError::ConstOutOfBounds {
                constant: ConstIdx::new(u16::MAX),
            })
        })?;
        self.constants.push(value);
        Ok(ConstIdx::new(index))
    }

    pub(super) fn record_slot(&mut self, slot: SlotIdx) {
        let value = slot.as_usize();
        self.max_slot = Some(match self.max_slot {
            Some(current) => current.max(value),
            None => value,
        });
    }

    pub(super) fn slot_count(&self) -> Result<u16, CompileError> {
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

#[allow(dead_code)]
pub(super) fn compile_step(
    step: &Yaml<'_>,
    index: usize,
    last_step: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    source_ir_starts: &[StepIdx],
    builder: &mut WorkflowBuilder,
) -> Result<Vec<CompiledNode>, CompileError> {
    let StepSpec { primitive, body } = step_spec(step, index)?;
    let node = match primitive {
        StepPrimitive::Run | StepPrimitive::Do => compile_run(
            body,
            index,
            last_step,
            id,
            next,
            primitive.as_str(),
            builder,
        ),
        StepPrimitive::Set | StepPrimitive::Save => compile_save(
            body,
            index,
            last_step,
            id,
            next,
            primitive.as_str(),
            builder,
        ),
        StepPrimitive::Choose => {
            compile_choose(body, index, last_step, id, source_ir_starts, builder)
        }
        StepPrimitive::ForEach => return compile_for_each(body, index, last_step, id, builder),
        StepPrimitive::Parallel => {
            return compile_parallel(body, index, last_step, id, source_ir_starts, builder);
        }
        StepPrimitive::Collect => {
            // Legacy 3-node collect emission is retired: it structurally
            // self-loops (CollectPage.body is its own id) and can never
            // reach CollectFinish. The bounded CollectNext lowering is the
            // live path.
            return Err(CompileError::UnsupportedStepPrimitive {
                step: index,
                primitive: "collect",
            });
        }
        StepPrimitive::Aggregate => {
            return compile_aggregate(body, index, last_step, id, next, builder);
        }
        StepPrimitive::Repeat => {
            return compile_repeat(body, index, last_step, id, next, builder);
        }
        StepPrimitive::Wait => compile_wait(body, index, last_step, id, next, builder),
        StepPrimitive::Ask => return compile_ask(body, index, last_step, id, next, builder),
        StepPrimitive::Finish => return compile_finish(body, index, last_step, id, builder),
    }?;
    Ok(vec![node])
}
