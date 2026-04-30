//! Synchronous in-memory state-machine loop.

use crate::errors::EngineError;
use crate::ids::{RunId, SlotIdx, StepIdx};
use crate::value::{SlotValue, Taint};
use crate::workflow::{CompiledNodeKind, CompiledWorkflow};

/// Bounded number of steps a caller may execute in one engine slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct StepBudget(u32);

impl StepBudget {
    /// Largest bounded execution slice representable by the runtime.
    pub const MAX: Self = Self(u32::MAX);

    /// Creates a non-zero budget.
    pub const fn new(value: u32) -> Result<Self, EngineError> {
        if value == 0 {
            Err(EngineError::EmptyStepBudget)
        } else {
            Ok(Self(value))
        }
    }

    const fn get(self) -> u32 {
        self.0
    }
}

/// Outcome of one or more engine transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineSignal {
    /// The run made progress and can continue immediately.
    Continue,
    /// The run finished with a result value.
    Finished(SlotValue),
    /// The caller's execution slice ended before completion.
    BudgetExhausted,
}

/// Runtime state for one workflow run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunFrame {
    id: RunId,
    current: StepIdx,
    slots: Box<[SlotValue]>,
    taint: Box<[Taint]>,
    steps_executed: u64,
}

impl RunFrame {
    /// Creates a new run frame with all slots initialized to null and clean.
    #[must_use]
    pub fn new(id: RunId, workflow: &CompiledWorkflow) -> Self {
        let slot_count = usize::from(workflow.slot_count());
        Self {
            id,
            current: workflow.entry(),
            slots: vec![SlotValue::Null; slot_count].into_boxed_slice(),
            taint: vec![Taint::Clean; slot_count].into_boxed_slice(),
            steps_executed: 0,
        }
    }

    /// Run identifier.
    #[must_use]
    pub const fn id(&self) -> RunId {
        self.id
    }

    /// Current program counter.
    #[must_use]
    pub const fn current_step(&self) -> StepIdx {
        self.current
    }

    /// Number of deterministic steps executed by this frame.
    #[must_use]
    pub const fn steps_executed(&self) -> u64 {
        self.steps_executed
    }

    /// Reads a slot by numeric index.
    pub fn slot(&self, slot: SlotIdx) -> Result<&SlotValue, EngineError> {
        self.slots
            .get(slot.as_usize())
            .ok_or(EngineError::SlotOutOfBounds { slot })
    }

    /// Reads a slot taint marker by numeric index.
    pub fn taint(&self, slot: SlotIdx) -> Result<Taint, EngineError> {
        self.taint
            .get(slot.as_usize())
            .copied()
            .ok_or(EngineError::SlotOutOfBounds { slot })
    }

    #[inline]
    fn write_slot(
        &mut self,
        slot: SlotIdx,
        value: SlotValue,
        taint: Taint,
    ) -> Result<(), EngineError> {
        let index = slot.as_usize();
        let slot_entry = self
            .slots
            .get_mut(index)
            .ok_or(EngineError::SlotOutOfBounds { slot })?;
        let taint_entry = self
            .taint
            .get_mut(index)
            .ok_or(EngineError::SlotOutOfBounds { slot })?;
        *slot_entry = value;
        *taint_entry = taint;
        Ok(())
    }

    #[inline]
    fn jump_to(&mut self, next: StepIdx) -> Result<(), EngineError> {
        self.current = next;
        self.steps_executed = self
            .steps_executed
            .checked_add(1)
            .ok_or(EngineError::StepCounterOverflow)?;
        Ok(())
    }
}

/// Executes one compiled node.
pub fn step_once(plan: &CompiledWorkflow, run: &mut RunFrame) -> Result<EngineSignal, EngineError> {
    let pc = run.current_step();
    let node = plan
        .node(pc)
        .ok_or(EngineError::InvalidProgramCounter { step: pc })?;
    execute_node(plan, run, node.kind)
}

#[inline]
fn execute_node(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    kind: CompiledNodeKind,
) -> Result<EngineSignal, EngineError> {
    match kind {
        CompiledNodeKind::SetConst {
            output,
            value,
            next,
        } => set_const(plan, run, output, value, next),
        CompiledNodeKind::Copy {
            output,
            source,
            next,
        } => copy_slot(run, output, source, next),
        CompiledNodeKind::ChooseSlot {
            condition,
            on_true,
            on_false,
        } => choose_branch(run, condition, on_true, on_false),
        CompiledNodeKind::Finish { result } => finish_run(run, result),
        CompiledNodeKind::FinishConst { value } => finish_const(plan, value),
    }
}

/// Executes deterministic nodes until finish or budget exhaustion.
pub fn run_until_blocked(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    budget: StepBudget,
) -> Result<EngineSignal, EngineError> {
    let mut remaining = budget.get();
    while remaining > 0 {
        let signal = step_once(plan, run)?;
        if !matches!(signal, EngineSignal::Continue) {
            return Ok(signal);
        }
        remaining = remaining.saturating_sub(1);
    }
    Ok(EngineSignal::BudgetExhausted)
}

#[inline]
fn set_const(
    plan: &CompiledWorkflow,
    run: &mut RunFrame,
    output: SlotIdx,
    value: crate::ids::ConstIdx,
    next: StepIdx,
) -> Result<EngineSignal, EngineError> {
    let constant = plan
        .constant(value)
        .copied()
        .ok_or(EngineError::ConstOutOfBounds { constant: value })?;
    let slot_value = constant.to_slot_value()?;
    run.write_slot(output, slot_value, Taint::Clean)?;
    run.jump_to(next)?;
    Ok(EngineSignal::Continue)
}

#[inline]
fn copy_slot(
    run: &mut RunFrame,
    output: SlotIdx,
    source: SlotIdx,
    next: StepIdx,
) -> Result<EngineSignal, EngineError> {
    let value = *run.slot(source)?;
    let taint = run.taint(source)?;
    run.write_slot(output, value, taint)?;
    run.jump_to(next)?;
    Ok(EngineSignal::Continue)
}

#[inline]
fn choose_branch(
    run: &mut RunFrame,
    condition: SlotIdx,
    on_true: StepIdx,
    on_false: StepIdx,
) -> Result<EngineSignal, EngineError> {
    let next = match run.slot(condition)? {
        SlotValue::Bool(true) => on_true,
        SlotValue::Bool(false) => on_false,
        _ => return Err(EngineError::NonBoolCondition { slot: condition }),
    };
    run.jump_to(next)?;
    Ok(EngineSignal::Continue)
}

#[inline]
fn finish_run(run: &RunFrame, result: SlotIdx) -> Result<EngineSignal, EngineError> {
    Ok(EngineSignal::Finished(*run.slot(result)?))
}

#[inline]
fn finish_const(
    plan: &CompiledWorkflow,
    value: crate::ids::ConstIdx,
) -> Result<EngineSignal, EngineError> {
    let constant = plan
        .constant(value)
        .copied()
        .ok_or(EngineError::ConstOutOfBounds { constant: value })?;
    let slot_value = constant.to_slot_value()?;
    Ok(EngineSignal::Finished(slot_value))
}

#[cfg(test)]
mod tests {
    use super::{EngineSignal, RunFrame, StepBudget, run_until_blocked};
    use crate::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
    use crate::value::{ConstValue, SlotValue};
    use crate::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts};

    #[test]
    fn set_chain_finishes_with_slot_value() {
        let workflow = tiny_workflow(ConstValue::I64(42));
        assert!(workflow.is_ok());
        let Ok(workflow) = workflow else {
            return;
        };
        let mut run = RunFrame::new(RunId::new(7), &workflow);

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX);

        assert_eq!(result, Ok(EngineSignal::Finished(SlotValue::I64(42))));
        assert_eq!(run.steps_executed(), 1);
    }

    #[test]
    fn set_chain_finishes_with_object_slot_value() {
        let workflow = tiny_workflow(ConstValue::Bool(true));
        assert!(workflow.is_ok());
        let Ok(workflow) = workflow else {
            return;
        };
        let mut run = RunFrame::new(RunId::new(8), &workflow);

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX);

        assert_eq!(result, Ok(EngineSignal::Finished(SlotValue::Bool(true))));
    }

    #[test]
    fn const_finish_returns_constant_pool_value() -> Result<(), String> {
        let workflow = const_finish_workflow(ConstValue::Bool(true), ConstIdx::new(0))
            .map_err(|error| error.to_string())?;
        let mut run = RunFrame::new(RunId::new(9), &workflow);

        let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX)
            .map_err(|error| error.to_string())?;

        if result == EngineSignal::Finished(SlotValue::Bool(true)) {
            Ok(())
        } else {
            Err(format!("unexpected const finish result: {result:?}"))
        }
    }

    #[test]
    fn const_finish_rejects_missing_constant() -> Result<(), String> {
        let result = const_finish_workflow(ConstValue::Null, ConstIdx::new(1));

        match result {
            Err(crate::WorkflowError::ConstOutOfBounds { constant })
                if constant == ConstIdx::new(1) =>
            {
                Ok(())
            }
            other => Err(format!("unexpected const validation result: {other:?}")),
        }
    }

    #[test]
    fn zero_budget_is_rejected() {
        let budget = StepBudget::new(0);

        assert!(budget.is_err());
    }

    fn tiny_workflow(value: ConstValue) -> Result<CompiledWorkflow, crate::WorkflowError> {
        CompiledWorkflow::try_from_parts(tiny_workflow_parts(value))
    }

    fn tiny_workflow_parts(value: ConstValue) -> WorkflowParts {
        WorkflowParts {
            name: Box::<str>::from("tiny"),
            digest: WorkflowDigest::from_bytes([1; 32]),
            nodes: tiny_workflow_nodes(),
            expressions: Box::new([]),
            constants: vec![value].into_boxed_slice(),
            slot_count: 1,
            entry: StepIdx::new(0),
        }
    }

    fn tiny_workflow_nodes() -> Box<[CompiledNode]> {
        vec![tiny_set_const_node(), tiny_finish_node()].into_boxed_slice()
    }

    fn tiny_set_const_node() -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(0),
            kind: CompiledNodeKind::SetConst {
                output: SlotIdx::new(0),
                value: ConstIdx::new(0),
                next: StepIdx::new(1),
            },
        }
    }

    fn tiny_finish_node() -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(1),
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }
    }

    fn const_finish_workflow(
        value: ConstValue,
        constant: ConstIdx,
    ) -> Result<CompiledWorkflow, crate::WorkflowError> {
        let digest = WorkflowDigest::from_bytes([2; 32]);
        let nodes = vec![CompiledNode {
            id: StepIdx::new(0),
            kind: CompiledNodeKind::FinishConst { value: constant },
        }];
        let parts = WorkflowParts {
            name: Box::<str>::from("const_finish"),
            digest,
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            constants: vec![value].into_boxed_slice(),
            slot_count: 0,
            entry: StepIdx::new(0),
        };
        CompiledWorkflow::try_from_parts(parts)
    }
}
