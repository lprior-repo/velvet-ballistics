#![forbid(unsafe_code)]
#![cfg_attr(kani, allow(dead_code))]

use crate::errors::CoreError;
use crate::ids::{ConstIdx, SlotIdx, StepIdx, SymbolId};
use crate::workflow::WorkflowError;
use thiserror::Error;

use super::traversal::BudgetTraversalError;

/// Budget validation failures.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BudgetError {
    #[error("total steps exceeded: {actual} > {limit}")]
    TotalStepsExceeded { actual: u64, limit: u64 },
    #[error("total slots exceeded: {actual} > {limit}")]
    TotalSlotsExceeded { actual: u64, limit: u64 },
    #[error("fanout exceeded: {actual} > {limit}")]
    FanoutExceeded { actual: u16, limit: u16 },
    #[error("nesting depth exceeded: {actual} > {limit}")]
    NestingDepthExceeded { actual: u16, limit: u16 },
    #[error("parallel exceeded: {actual} > {limit}")]
    ParallelExceeded { actual: u16, limit: u16 },
    #[error("action tickets exceeded: {actual} > {limit}")]
    ActionTicketsExceeded { actual: u32, limit: u32 },
    #[error("run time exceeded: {actual} > {limit}")]
    RunTimeExceeded { actual: u64, limit: u64 },
    #[error("result bytes exceeded: {actual} > {limit}")]
    ResultBytesExceeded { actual: u32, limit: u32 },
    #[error("steps executable exceeded: {actual} > {limit}")]
    StepsExecutableExceeded { actual: u32, limit: u32 },
    #[error("timer entries exceeded: {actual} > {limit}")]
    TimerEntriesExceeded { actual: u32, limit: u32 },
    #[error("trace events exceeded: {actual} > {limit}")]
    TraceEventsExceeded { actual: u64, limit: u64 },
    #[error("journal batch bytes exceeded: {actual} > {limit}")]
    JournalBatchBytesExceeded { actual: u32, limit: u32 },
    #[error("queue depth exceeded: {actual} > {limit}")]
    QueueDepthExceeded { actual: u32, limit: u32 },
    #[error("ipc payload bytes exceeded: {actual} > {limit}")]
    IpcPayloadBytesExceeded { actual: u32, limit: u32 },
    #[error("blob bytes exceeded: {actual} > {limit}")]
    BlobBytesExceeded { actual: u64, limit: u64 },
    #[error("input bytes exceeded: {actual} > {limit}")]
    InputBytesExceeded { actual: u32, limit: u32 },
    #[error("workflow entry step {entry:?} is outside the node array")]
    WorkflowEntryOutOfBounds { entry: StepIdx },
    #[error("workflow target step {step:?} is outside the node array")]
    WorkflowStepOutOfBounds { step: StepIdx },
    #[error("workflow slot {slot:?} is outside slot_count")]
    WorkflowSlotOutOfBounds { slot: SlotIdx },
    #[error("workflow constant {constant:?} is outside the constant pool")]
    WorkflowConstOutOfBounds { constant: ConstIdx },
    #[error("workflow node id mismatch: expected {expected:?}, found {actual:?}")]
    WorkflowNodeIdMismatch { expected: StepIdx, actual: StepIdx },
    #[error("workflow expression error: {source}")]
    WorkflowExpression { source: CoreError },
    #[error("workflow resource contract exceeded: {resource}")]
    ResourceContractExceeded { resource: &'static str },
    #[error("workflow resource contract too large: {resource}")]
    ResourceContractTooLarge { resource: &'static str },
    #[error("workflow branch table must contain a branch or otherwise target")]
    EmptyBranchTable,
    #[error("workflow node {step:?} is unreachable")]
    UnreachableNode { step: StepIdx },
    #[error("workflow backward edge from {from:?} to {to:?}")]
    BackwardEdge { from: StepIdx, to: StepIdx },
    #[error("workflow inner loop at {inner:?} exceeds outer loop done at {outer_done:?}")]
    ImproperLoopNesting { inner: StepIdx, outer_done: StepIdx },
    #[error("workflow budget policy exceeded: {detail}")]
    BudgetPolicyExceeded { detail: &'static str },
    #[error("workflow step count overflow: {actual}")]
    StepCountOverflow { actual: u64 },
    #[error("workflow symbol {symbol:?} exceeds symbols_count")]
    WorkflowSymbolOutOfBounds { symbol: SymbolId },
    #[error("workflow accessor path depth {depth} exceeds maximum {max}")]
    AccessorPathTooDeep { depth: usize, max: usize },
    #[error("workflow jump cycle from {step:?} to {target:?}")]
    JumpCycle { step: StepIdx, target: StepIdx },
    #[error("invalid compiled workflow: {reason}")]
    InvalidCompiledWorkflow { reason: &'static str },
}

impl From<WorkflowError> for BudgetError {
    fn from(error: WorkflowError) -> Self {
        match error {
            WorkflowError::EmptyNodes => Self::InvalidCompiledWorkflow {
                reason: "compiled workflow must contain at least one node",
            },
            WorkflowError::EntryOutOfBounds { entry } => Self::WorkflowEntryOutOfBounds { entry },
            WorkflowError::StepOutOfBounds { step } => Self::WorkflowStepOutOfBounds { step },
            WorkflowError::SlotOutOfBounds { slot } => Self::WorkflowSlotOutOfBounds { slot },
            WorkflowError::ConstOutOfBounds { constant } => {
                Self::WorkflowConstOutOfBounds { constant }
            }
            WorkflowError::NodeIdMismatch { expected, actual } => {
                Self::WorkflowNodeIdMismatch { expected, actual }
            }
            WorkflowError::Expression(source) => budget_error_from_core_expression(source),
            WorkflowError::ResourceContractExceeded { resource } => {
                Self::ResourceContractExceeded { resource }
            }
            WorkflowError::ResourceContractTooLarge { resource } => {
                Self::ResourceContractTooLarge { resource }
            }
            WorkflowError::EmptyBranchTable => Self::EmptyBranchTable,
            WorkflowError::UnreachableNode { step } => Self::UnreachableNode { step },
            WorkflowError::BackwardEdge { from, to } => Self::BackwardEdge { from, to },
            WorkflowError::ImproperLoopNesting { inner, outer_done } => {
                Self::ImproperLoopNesting { inner, outer_done }
            }
            WorkflowError::BudgetPolicyExceeded { detail } => Self::BudgetPolicyExceeded { detail },
            WorkflowError::StepCountOverflow { actual } => Self::StepCountOverflow { actual },
            WorkflowError::SymbolOutOfBounds { symbol } => {
                Self::WorkflowSymbolOutOfBounds { symbol }
            }
            WorkflowError::AccessorPathTooDeep { depth, max } => {
                Self::AccessorPathTooDeep { depth, max }
            }
            WorkflowError::JumpCycle { step, target } => Self::JumpCycle { step, target },
        }
    }
}

impl From<BudgetTraversalError> for BudgetError {
    fn from(error: BudgetTraversalError) -> Self {
        match error {
            BudgetTraversalError::EntryOutOfBounds { entry } => {
                Self::WorkflowEntryOutOfBounds { entry }
            }
            BudgetTraversalError::StepOutOfBounds { step } => {
                Self::WorkflowStepOutOfBounds { step }
            }
            BudgetTraversalError::StepCountOverflow { actual } => {
                Self::StepCountOverflow { actual }
            }
            BudgetTraversalError::JumpCycle { step, target } => Self::JumpCycle { step, target },
            BudgetTraversalError::InvalidCompiledWorkflow { reason } => {
                Self::InvalidCompiledWorkflow { reason }
            }
        }
    }
}

fn budget_error_from_core_expression(source: CoreError) -> BudgetError {
    match source {
        CoreError::InvalidCompiledWorkflow { reason } => {
            BudgetError::InvalidCompiledWorkflow { reason }
        }
        other => BudgetError::WorkflowExpression { source: other },
    }
}
