//! Compiled workflow node types.

use crate::ids::{ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId};
use serde::{Deserialize, Serialize};

/// One compiled state-machine node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledNode {
    /// Step index of this node.
    pub id: StepIdx,
    /// Optional output slot written by this node.
    pub output: Option<SlotIdx>,
    /// Optional fallthrough step.
    pub next: Option<StepIdx>,
    /// Node behavior.
    pub kind: CompiledNodeKind,
}

/// Hot-path node variants. All references are numeric.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum CompiledNodeKind {
    /// No-op transition to `next`.
    Nop,
    /// Write a constant-pool value into the output slot.
    SetConst {
        /// Constant-pool index.
        value: ConstIdx,
    },
    /// Copy one slot into the output slot.
    Copy {
        /// Source slot.
        source: SlotIdx,
    },
    /// Evaluate expression bytecode into `output`.
    EvalExpr { expr: ExprIdx },
    /// Build an object handle from numeric field and slot references.
    BuildObject {
        fields: Box<[(SymbolId, SlotIdx)]>,
    },
    /// Build a list handle from numeric slot references.
    BuildList { items: Box<[SlotIdx]> },
    /// Schedule an external action and suspend.
    Do { action: ActionId, input: SlotIdx },
    /// Branch using a pre-materialized boolean condition slot.
    Choose {
        /// Ordered expression branches.
        branches: Box<[super::compiled_workflow::ExprBranch]>,
        /// Target when no branch condition is true.
        otherwise: Option<StepIdx>,
    },
    /// Branch using pre-materialized boolean condition slots.
    ChooseSlot {
        /// Ordered slot branches.
        branches: Box<[super::compiled_workflow::SlotBranch]>,
        /// Target when no branch condition is true.
        otherwise: Option<StepIdx>,
    },
    /// Start a bounded for-each loop.
    ForEachStart {
        input: SlotIdx,
        item_slot: SlotIdx,
        limit: u32,
        body: StepIdx,
        done: StepIdx,
    },
    /// Advance a bounded for-each loop.
    ForEachNext {
        iterator_slot: SlotIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Join a for-each loop output.
    ForEachJoin { output: SlotIdx },
    /// Start bounded parallel branches.
    TogetherStart {
        branches: Box<[StepIdx]>,
        join: StepIdx,
    },
    /// Execute one together branch.
    TogetherBranch {
        branch: u16,
        entry: StepIdx,
        join: StepIdx,
        /// Slot holding the accumulator list (shared with TogetherStart).
        accumulator: SlotIdx,
    },
    /// Join together branches.
    TogetherJoin {
        branch_count: u16,
        /// Slot holding the accumulator list (shared with TogetherStart).
        accumulator: SlotIdx,
    },
    /// Start bounded collection.
    CollectStart {
        source: SlotIdx,
        limit: u32,
        page_size: u32,
        body: StepIdx,
        done: StepIdx,
    },
    /// Process one collection page.
    CollectPage {
        collector_slot: SlotIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Advance collection.
    CollectNext {
        collector_slot: SlotIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Finish collection.
    CollectFinish { collector_slot: SlotIdx },
    /// Start bounded reduction.
    ReduceStart {
        input: SlotIdx,
        accumulator: SlotIdx,
        initial: ConstIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Advance reduction.
    ReduceNext {
        iterator_slot: SlotIdx,
        accumulator: SlotIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Finish reduction.
    ReduceFinish { accumulator: SlotIdx },
    /// Start bounded repeat.
    RepeatStart {
        max_attempts: u16,
        body: StepIdx,
        done: StepIdx,
    },
    /// Execute repeat attempt.
    RepeatAttempt {
        attempt_slot: SlotIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Check repeat state.
    RepeatCheck {
        attempt_slot: SlotIdx,
        done: StepIdx,
    },
    /// Finish repeat.
    RepeatFinish { result: SlotIdx },
    /// Wait until a deadline slot.
    WaitUntil { deadline_slot: SlotIdx },
    /// Wait for an event slot.
    WaitEvent {
        event: SlotIdx,
        timeout_slot: Option<SlotIdx>,
    },
    /// Ask for external input.
    Ask {
        prompt: SlotIdx,
        timeout_slot: Option<SlotIdx>,
    },
    /// Resume an ask.
    AskResume { answer: SlotIdx },
    /// Check retry policy.
    RetryCheck {
        policy_slot: SlotIdx,
        body: StepIdx,
        exhausted: StepIdx,
    },
    /// Run error handler.
    ErrorHandler {
        /// Body step to execute.
        body: StepIdx,
        /// Handler step to route to on body failure.
        handler: StepIdx,
        /// Optional slot to write failed step index for handler consumption.
        error_slot: Option<SlotIdx>,
    },
    /// Jump to a numeric target.
    Jump { target: StepIdx },
    /// Finish the run with the selected result slot.
    Finish {
        /// Result slot.
        result: SlotIdx,
    },
}
