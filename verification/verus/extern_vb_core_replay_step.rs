// SPDX-License-Identifier: MIT
//
// Extern surface for vb_core_replay_step_spec Verus spec.
// Models the production replay primitives in `vb_core::replay::*` as pure
// decision fns so Verus can reason about their determinism and stack-safety
// preconditions.
//
// Production bindings (BINDING LEDGER):
//   - `replay_step` / `replay_step_with_collect` decision fn at
//     crates/vb_core/src/replay/step.rs:118-192. The pure projection
//     models the action kind (Continue / Finished / Suspended / Err)
//     by node-kind discriminant.
//   - `replay_choose_slot` at crates/vb_core/src/replay/choose/mod.rs:12-58.
//     The first branch whose condition slot is `Bool(true)` wins; if all
//     are false (or non-bool) the `otherwise` target is used; if there is
//     no otherwise and no match the fn returns Internal.
//   - `replay_choose_expr` at crates/vb_core/src/replay/choose/mod.rs:61-104.
//     Identical structure to `replay_choose_slot` but the condition is
//     evaluated through `eval_expr_for_replay` against a `ValueStore`.
//   - `eval_replay_op` at crates/vb_core/src/replay/ops.rs:13-44. Each
//     `ExprOp` discriminant drives a stack delta (push, pop, pop2+push).
//     The pure projection summarizes the delta as `SpecOpStackDelta`.
//   - `pop_pair` at crates/vb_core/src/replay/ops.rs:244-248. Pops two
//     `SlotValue`s from the top of `ReplayExprStack`. Precondition:
//     `stack.len >= 2 && stack.len <= stack.capacity`.
//   - `pop_i64_pair` at crates/vb_core/src/replay/ops.rs:250-254. Same
//     precondition as `pop_pair` PLUS both popped values must be
//     `SlotValue::I64`; otherwise the fn returns
//     `ExpressionEvalFailed`.
//
// The replay module is the bedrock of journal-driven determinism
// (every reconstruction of slot state goes through `replay_step`), yet
// the audit found zero fuzz coverage. The projections below capture
// every decision branch the production fns exercise on the relevant
// inputs and reduce them to scalars that Verus can reason about.
//
// All enum discriminants below mirror the production enum ordering;
// the binding ledger captures each.
#![forbid(unsafe_code)]
#![allow(dead_code)]

// ---------------------------------------------------------------------------
// Mirror types
// ---------------------------------------------------------------------------

/// Mirror of the closed `ReplayAction` set at
/// `crates/vb_core/src/replay/step.rs:50-57`. The projection collapses
/// the `ReplayError` variants into typed Err variants so the spec fn
/// has a single decision-shaped return.
#[derive(Clone, Copy)]
pub enum SpecReplayAction {
    /// `ReplayAction::Continue(next)` — caller should advance pc to `next`.
    Continue(u32),
    /// `ReplayAction::Finished` — the run finished.
    Finished,
    /// `ReplayAction::Suspended { step, kind }` — non-deterministic node.
    Suspended { step: u32, kind: u8 },
    /// `ReplayError::StepNotFound { step }`.
    ErrStepNotFound,
    /// `ReplayError::SlotNotAvailable { slot }`.
    ErrSlotNotAvailable,
    /// `ReplayError::ExpressionEvalFailed { step }`.
    ErrExpressionEvalFailed,
    /// `ReplayError::Internal { reason }` — anything else.
    ErrInternal,
}

/// Mirror of the closed `CompiledNodeKind` discriminant set. Only the
/// kinds that `replay_step` actually handles are enumerated; the
/// "unsupported" fallthrough is `ErrInternal`. Discriminant order
/// matches the production `CompiledNodeKind` at
/// `crates/vb_core/src/workflow/node.rs`.
#[derive(Clone, Copy)]
pub enum SpecNodeKind {
    Nop,
    SetConst,
    Copy,
    EvalExpr,
    BuildObject,
    BuildList,
    Finish,
    Jump,
    Do,
    Ask,
    WaitUntil,
    WaitEvent,
    ChooseSlot,
    Choose,
    CollectStart,
    CollectPage,
    CollectNext,
    CollectFinish,
    Unsupported,
}

/// Mirror of the closed `SuspensionKind` discriminant set at
/// `crates/vb_core/src/replay/step.rs:18-27`.
pub enum SpecSuspensionKind {
    /// `SuspensionKind::ActionPending` (discriminant 0).
    ActionPending,
    /// `SuspensionKind::AskPending` (discriminant 1).
    AskPending,
    /// `SuspensionKind::WaitUntil` (discriminant 2).
    WaitUntil,
    /// `SuspensionKind::WaitEvent` (discriminant 3).
    WaitEvent,
}

/// Mirror of the stack-delta effect of `eval_replay_op`. The projection
/// reduces the per-op effect to a coarse delta + a possible error
/// (e.g. unsupported ops, pop-from-empty-stack).
#[derive(Clone, Copy)]
pub enum SpecOpStackDelta {
    /// op pushes one value: net +1.
    Push,
    /// op pops one value (e.g. Not): net -1.
    Pop,
    /// op pops two values then pushes one (e.g. Eq): net -1.
    Pop2Push,
    /// op pops two values then pushes one but additionally requires
    /// the popped values to be I64 (e.g. Add, Sub, Mul, Div, Gt, Gte,
    /// Lt, Lte). Net -1.
    Pop2I64Push,
    /// op is unsupported by replay; eval_replay_op returns
    /// `Internal { reason: "unsupported expression op for replay" }`.
    Unsupported,
}

// ---------------------------------------------------------------------------
// Spec predicates (math layer) — const-fn mirrors for documentation
// ---------------------------------------------------------------------------
//
// The predicates below are `const fn` rather than `spec fn` because
// they are referenced from exec-mode test code in the spec file. The
// spec mirrors of these predicates (referenced from `assume_specification`
// ensures clauses and from `proof fn`s) live in the companion spec
// file `vb_core_replay_step_spec.rs`.
//
// The two are required to agree; the binding ledger records this.

/// Const-fn mirror of `spec_node_kind_valid` (the spec fn lives in
/// the spec file). Returns true iff `kind` is one of the documented
/// `SpecNodeKind` variants.
pub const fn spec_node_kind_valid(kind: SpecNodeKind) -> bool {
    matches!(
        kind,
        SpecNodeKind::Nop
            | SpecNodeKind::SetConst
            | SpecNodeKind::Copy
            | SpecNodeKind::EvalExpr
            | SpecNodeKind::BuildObject
            | SpecNodeKind::BuildList
            | SpecNodeKind::Finish
            | SpecNodeKind::Jump
            | SpecNodeKind::Do
            | SpecNodeKind::Ask
            | SpecNodeKind::WaitUntil
            | SpecNodeKind::WaitEvent
            | SpecNodeKind::ChooseSlot
            | SpecNodeKind::Choose
            | SpecNodeKind::CollectStart
            | SpecNodeKind::CollectPage
            | SpecNodeKind::CollectNext
            | SpecNodeKind::CollectFinish
            | SpecNodeKind::Unsupported
    )
}

/// Const-fn mirror of `spec_replay_action_valid` (the spec fn lives
/// in the spec file). Returns true iff `action` is one of the
/// documented `SpecReplayAction` variants.
pub const fn spec_replay_action_valid(action: SpecReplayAction) -> bool {
    matches!(
        action,
        SpecReplayAction::Continue(_)
            | SpecReplayAction::Finished
            | SpecReplayAction::Suspended { .. }
            | SpecReplayAction::ErrStepNotFound
            | SpecReplayAction::ErrSlotNotAvailable
            | SpecReplayAction::ErrExpressionEvalFailed
            | SpecReplayAction::ErrInternal
    )
}

/// Const-fn mirror of `spec_op_stack_delta_valid` (the spec fn lives
/// in the spec file).
pub const fn spec_op_stack_delta_valid(delta: SpecOpStackDelta) -> bool {
    matches!(
        delta,
        SpecOpStackDelta::Push
            | SpecOpStackDelta::Pop
            | SpecOpStackDelta::Pop2Push
            | SpecOpStackDelta::Pop2I64Push
            | SpecOpStackDelta::Unsupported
    )
}

/// Const-fn mirror of `spec_pop_pair_precondition` (the spec fn lives
/// in the spec file). The production fn at
/// `crates/vb_core/src/replay/ops.rs:244-248` calls `stack.pop()?`
/// twice; each `pop` decrements `self.len` and only succeeds when
/// `self.len > 0`. The combined precondition is therefore
/// `stack_len >= 2`. Capacity is also part of the well-formed
/// invariant: `0 <= stack_len <= capacity`.
pub const fn spec_pop_pair_precondition(stack_len: u8, capacity: u8) -> bool {
    stack_len >= 2 && stack_len <= capacity
}

/// Const-fn mirror of `spec_pop_i64_pair_precondition` (the spec fn
/// lives in the spec file). Same stack-safety precondition as
/// `pop_pair` plus the additional constraint that the two popped
/// values must both be `SlotValue::I64`.
pub const fn spec_pop_i64_pair_precondition(
    stack_len: u8,
    capacity: u8,
    top_two_are_i64: bool,
) -> bool {
    stack_len >= 2 && stack_len <= capacity && top_two_are_i64
}

// ---------------------------------------------------------------------------
// Pure projection: replay_step decision
// ---------------------------------------------------------------------------

/// Pure decision fn mirroring the action-kind decision of
/// `replay_step_with_collect` at
/// `crates/vb_core/src/replay/step.rs:128-192`.
///
/// The production fn dispatches on `CompiledNodeKind`. This projection
/// reduces every per-branch outcome to one of the seven
/// `SpecReplayAction` variants. The relevant input scalars are:
///
/// - `kind`: which node-kind arm the dispatch enters. The projection
///   does not model internal sub-decisions (whether `node.next` is
///   set, whether `node.output` is set, whether the constant index is
///   in bounds, whether the slot exists) because Verus cannot
///   faithfully model those branches without an end-to-end model of
///   `RunFrame` and `CompiledWorkflow`. The spec bounds the
///   *high-level* decision; deeper preconditions are encoded by the
///   `Ok`/`Err` outcome itself (ErrStepNotFound, ErrSlotNotAvailable,
///   ErrInternal).
///
/// - `current_pc`: the step index of the node being replayed. Used
///   only for the `Suspended` payload so the journal can locate the
///   blocking node.
///
/// - `next_step`: the next pc the production fn would set. Only
///   meaningful for `Continue`. For `Finished`, `Suspended`, and the
///   typed errors the value is ignored.
///
/// TRUST BOUNDARY: the body of this pure decision is opaque to Verus
/// (`#[verifier::external]`). The contract is attached via
/// `assume_specification` in the companion spec file.
#[verifier::external]
pub fn replay_step_pure_decision(
    kind: SpecNodeKind,
    current_pc: u32,
    next_step: u32,
) -> SpecReplayAction {
    match kind {
        SpecNodeKind::Nop => SpecReplayAction::Continue(next_step),
        SpecNodeKind::SetConst => SpecReplayAction::Continue(next_step),
        SpecNodeKind::Copy => SpecReplayAction::Continue(next_step),
        SpecNodeKind::EvalExpr => SpecReplayAction::Continue(next_step),
        SpecNodeKind::BuildObject => SpecReplayAction::Continue(next_step),
        SpecNodeKind::BuildList => SpecReplayAction::Continue(next_step),
        SpecNodeKind::Finish => SpecReplayAction::Finished,
        SpecNodeKind::Jump => SpecReplayAction::Continue(next_step),
        SpecNodeKind::Do => SpecReplayAction::Suspended {
            step: current_pc,
            kind: 0,
        },
        SpecNodeKind::Ask => SpecReplayAction::Suspended {
            step: current_pc,
            kind: 1,
        },
        SpecNodeKind::WaitUntil => SpecReplayAction::Suspended {
            step: current_pc,
            kind: 2,
        },
        SpecNodeKind::WaitEvent => SpecReplayAction::Suspended {
            step: current_pc,
            kind: 3,
        },
        SpecNodeKind::ChooseSlot => SpecReplayAction::Continue(next_step),
        SpecNodeKind::Choose => SpecReplayAction::Continue(next_step),
        SpecNodeKind::CollectStart => SpecReplayAction::Continue(next_step),
        SpecNodeKind::CollectPage => SpecReplayAction::Continue(next_step),
        SpecNodeKind::CollectNext => SpecReplayAction::Continue(next_step),
        SpecNodeKind::CollectFinish => SpecReplayAction::Continue(next_step),
        SpecNodeKind::Unsupported => SpecReplayAction::ErrInternal,
    }
}

// ---------------------------------------------------------------------------
// Pure projection: replay_choose_slot
// ---------------------------------------------------------------------------

/// Pure decision fn mirroring `replay_choose_slot` at
/// `crates/vb_core/src/replay/choose/mod.rs:12-58`.
///
/// The production fn iterates the branch list in order, returning on
/// the first branch whose condition slot holds `Bool(true)`. If no
/// branch matches and `otherwise` is `None`, it returns
/// `Internal { reason: "choose_slot no branch matched and no
/// otherwise" }`; otherwise it returns `Continue(otherwise)`.
///
/// The projection reduces this to:
///
/// - `first_matching_branch_index`: the index of the first branch
///   whose condition is `Bool(true)`. If no branch matches, set to
///   `branch_count` (out-of-range sentinel).
/// - `has_otherwise`: 1 iff `otherwise.is_some()`.
/// - `otherwise_target`: the `otherwise` step index (ignored if
///   `has_otherwise == 0`).
/// - `branch_target_at_match`: the target of the matching branch.
#[verifier::external]
pub fn replay_choose_slot_pure_decision(
    first_matching_branch_index: u32,
    branch_count: u32,
    has_otherwise: u8,
    otherwise_target: u32,
    branch_target_at_match: u32,
) -> SpecReplayAction {
    if first_matching_branch_index < branch_count {
        SpecReplayAction::Continue(branch_target_at_match)
    } else if has_otherwise == 1 {
        SpecReplayAction::Continue(otherwise_target)
    } else {
        SpecReplayAction::ErrInternal
    }
}

/// Pure decision fn mirroring `replay_choose_expr` at
/// `crates/vb_core/src/replay/choose/mod.rs:61-104`. The structure is
/// identical to `replay_choose_slot` except the condition is evaluated
/// through `eval_expr_for_replay` instead of a direct slot read; the
/// projection therefore shares the same shape. `eval_failed` is 1 iff
/// the underlying `eval_expr_for_replay` returned an error.
#[verifier::external]
pub fn replay_choose_expr_pure_decision(
    first_matching_branch_index: u32,
    branch_count: u32,
    has_otherwise: u8,
    otherwise_target: u32,
    branch_target_at_match: u32,
    eval_failed: u8,
) -> SpecReplayAction {
    if eval_failed == 1 {
        SpecReplayAction::ErrExpressionEvalFailed
    } else if first_matching_branch_index < branch_count {
        SpecReplayAction::Continue(branch_target_at_match)
    } else if has_otherwise == 1 {
        SpecReplayAction::Continue(otherwise_target)
    } else {
        SpecReplayAction::ErrInternal
    }
}

// ---------------------------------------------------------------------------
// Pure projection: eval_replay_op stack delta
// ---------------------------------------------------------------------------

/// Pure projection of `eval_replay_op` at
/// `crates/vb_core/src/replay/ops.rs:13-44`. The production fn
/// dispatches on `ExprOp`; the projection reduces every arm to the
/// stack-delta shape. Verus bounds the precondition for `Pop` /
/// `Pop2Push` / `Pop2I64Push` via the stack-len check on the caller.
///
/// `op_disc` is the `ExprOp` discriminant (mirroring the production
/// `ExprOp` ordering at `crates/vb_core/src/workflow/expr.rs`):
///   0  LoadSlot     -> Push
///   1  LoadConst    -> Push
///   2  LoadAccessor -> Push
///   3  Eq           -> Pop2Push
///   4  NotEq        -> Pop2Push
///   5  And          -> Pop2Push (additionally expects bool)
///   6  Or           -> Pop2Push (additionally expects bool)
///   7  Not          -> Pop
///   8  Add          -> Pop2I64Push
///   9  Sub          -> Pop2I64Push
///  10  Mul          -> Pop2I64Push
///  11  Div          -> Pop2I64Push
///  12  Gt           -> Pop2I64Push
///  13  Gte          -> Pop2I64Push
///  14  Lt           -> Pop2I64Push
///  15  Lte          -> Pop2I64Push
///  16+ anything else -> Unsupported (production returns Internal)
#[verifier::external]
pub fn eval_replay_op_stack_delta(op_disc: u8) -> SpecOpStackDelta {
    match op_disc {
        0 | 1 | 2 => SpecOpStackDelta::Push,
        3 | 4 | 5 | 6 => SpecOpStackDelta::Pop2Push,
        7 => SpecOpStackDelta::Pop,
        8..=15 => SpecOpStackDelta::Pop2I64Push,
        _ => SpecOpStackDelta::Unsupported,
    }
}

// ---------------------------------------------------------------------------
// Pure projection: pop_pair / pop_i64_pair preconditions
// ---------------------------------------------------------------------------

/// Pure projection of `pop_pair` at
/// `crates/vb_core/src/replay/ops.rs:244-248`. Returns true iff the
/// production fn would succeed; this is the formal witness for the
/// stack-bound contract that the audit found missing in fuzz coverage.
#[verifier::external]
pub fn pop_pair_pure(stack_len: u8, capacity: u8) -> bool {
    spec_pop_pair_precondition(stack_len, capacity)
}

/// Pure projection of `pop_i64_pair` at
/// `crates/vb_core/src/replay/ops.rs:250-254`. Returns true iff the
/// production fn would succeed; the additional `top_two_are_i64` flag
/// encodes the I64 type check that the production body performs via
/// `expect_i64_replay`.
#[verifier::external]
pub fn pop_i64_pair_pure(stack_len: u8, capacity: u8, top_two_are_i64: bool) -> bool {
    spec_pop_i64_pair_precondition(stack_len, capacity, top_two_are_i64)
}
