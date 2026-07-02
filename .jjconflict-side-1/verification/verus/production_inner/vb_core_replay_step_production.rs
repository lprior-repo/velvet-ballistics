// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for vb_core_replay_step_spec Verus spec
// ============================================================================
//
// This file is the WEAK (production_inner/) production mirror for the
// `vb_core_replay_step_spec.rs` Verus spec. It contains pure
// projections of the production replay primitives in
// `crates/vb_core/src/replay/*` so Verus can reason about their
// determinism and stack-safety preconditions.
//
// The substitutions relative to direct `#[path]` inclusion of the
// production source are documented in the companion extern file
// (`verification/verus/extern_vb_core_replay_step.rs`) header. In
// summary, the production replay module transitively depends on
// `vb_core::frame::RunFrame`, `vb_core::value::ValueStore`, and
// `vb_core::workflow::CompiledWorkflow`, which contain heap
// allocations, indices, and runtime internals that Verus does not
// model end-to-end. The mirror captures every decision branch the
// production fns take on the relevant inputs and reduces them to
// scalars (mirror enums + integer discriminants) that Verus can
// reason about.
//
// DRIFT POLICY: This file MUST be regenerated from the production
// sources whenever production changes. The mirror is annotated at the
// top of every section with the originating production line range so
// regeneration is mechanical.
//
// This file is included by the companion extern file
// (`verification/verus/extern_vb_core_replay_step.rs`) via `#[path]`.
// Each production method body is marked `#[verifier::external]` so the
// body is opaque to Verus while the signature participates in the
// `assume_specification` binding in the companion spec file
// `vb_core_replay_step_spec.rs`.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `SpecReplayAction` (7-variant enum)              <- crates/vb_core/src/replay/step.rs:50-57
//   - `SpecNodeKind` (18-variant enum)                <- crates/vb_core/src/workflow/node.rs
//   - `SpecSuspensionKind` (4-variant enum)           <- crates/vb_core/src/replay/step.rs:18-27
//   - `SpecOpStackDelta` (5-variant enum)             <- projection of crates/vb_core/src/replay/ops.rs:13-44
//   - `replay_step_pure_decision`                     <- crates/vb_core/src/replay/step.rs:128-192
//   - `replay_choose_slot_pure_decision`              <- crates/vb_core/src/replay/choose/mod.rs:12-58
//   - `replay_choose_expr_pure_decision`              <- crates/vb_core/src/replay/choose/mod.rs:61-104
//   - `eval_replay_op_stack_delta`                    <- crates/vb_core/src/replay/ops.rs:13-44
//   - `pop_pair_pure`                                 <- crates/vb_core/src/replay/ops.rs:244-248
//   - `pop_i64_pair_pure`                             <- crates/vb_core/src/replay/ops.rs:250-254
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in this mirror are NOT verified by
// Verus. Each exec fn is `#[verifier::external]` so Verus skips body
// verification. The contracts attached via `assume_specification` in
// the companion spec file (`vb_core_replay_step_spec.rs`) state the
// production behavior the spec proofs discharge. Drift between the
// mirror and the production source is reported as binding-debt
// tracked outside Verus.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ============================================================================
// Mirror types
// ============================================================================

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
#[derive(Clone, Copy)]
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

// ============================================================================
// Spec predicates (math layer) — const-fn mirrors for documentation
// ============================================================================
//
// The predicates below are `const fn` rather than `spec fn` because
// they are referenced from exec-mode test code in the spec file. The
// spec mirrors of these predicates (referenced from `assume_specification`
// ensures clauses and from `proof fn`s) live in the companion spec
// file `vb_core_replay_step_spec.rs`.

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

// ============================================================================
// Pure projection: replay_step decision
// ============================================================================

/// Pure decision fn mirroring the action-kind decision of
/// `replay_step_with_collect` at
/// `crates/vb_core/src/replay/step.rs:128-192`.
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
        SpecNodeKind::Do => SpecReplayAction::Suspended { step: current_pc, kind: 0 },
        SpecNodeKind::Ask => SpecReplayAction::Suspended { step: current_pc, kind: 1 },
        SpecNodeKind::WaitUntil => SpecReplayAction::Suspended { step: current_pc, kind: 2 },
        SpecNodeKind::WaitEvent => SpecReplayAction::Suspended { step: current_pc, kind: 3 },
        SpecNodeKind::ChooseSlot => SpecReplayAction::Continue(next_step),
        SpecNodeKind::Choose => SpecReplayAction::Continue(next_step),
        SpecNodeKind::CollectStart => SpecReplayAction::Continue(next_step),
        SpecNodeKind::CollectPage => SpecReplayAction::Continue(next_step),
        SpecNodeKind::CollectNext => SpecReplayAction::Continue(next_step),
        SpecNodeKind::CollectFinish => SpecReplayAction::Continue(next_step),
        SpecNodeKind::Unsupported => SpecReplayAction::ErrInternal,
    }
}

// ============================================================================
// Pure projection: replay_choose_slot / replay_choose_expr
// ============================================================================

/// Pure decision fn mirroring `replay_choose_slot` at
/// `crates/vb_core/src/replay/choose/mod.rs:12-58`.
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
/// `crates/vb_core/src/replay/choose/mod.rs:61-104`.
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

// ============================================================================
// Pure projection: eval_replay_op stack delta
// ============================================================================

/// Pure projection of `eval_replay_op` at
/// `crates/vb_core/src/replay/ops.rs:13-44`.
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

// ============================================================================
// Pure projection: pop_pair / pop_i64_pair preconditions
// ============================================================================

/// Pure projection of `pop_pair` at
/// `crates/vb_core/src/replay/ops.rs:244-248`.
#[verifier::external]
pub fn pop_pair_pure(stack_len: u8, capacity: u8) -> bool {
    spec_pop_pair_precondition(stack_len, capacity)
}

/// Pure projection of `pop_i64_pair` at
/// `crates/vb_core/src/replay/ops.rs:250-254`.
#[verifier::external]
pub fn pop_i64_pair_pure(stack_len: u8, capacity: u8, top_two_are_i64: bool) -> bool {
    spec_pop_i64_pair_precondition(stack_len, capacity, top_two_are_i64)
}
