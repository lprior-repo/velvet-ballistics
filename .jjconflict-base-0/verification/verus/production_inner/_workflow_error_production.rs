// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for WorkflowError
// ============================================================================
//
// This file is a VERBATIM copy of the production `WorkflowError` enum
// declaration from `crates/vb_core/src/workflow/mod.rs:321-452` with
// three minimal substitutions required to compile under
// `verus --crate-type=lib` without proc-macro crate registration:
//
//   1. The `#[derive(Debug, Clone, Error, PartialEq, Eq)]` derive on
//      `WorkflowError` (production workflow/mod.rs:319) and the
//      per-variant `#[error("...")]` attribute strings
//      (production workflow/mod.rs:323, 326, 332, 338, 344, 350, 358,
//      361, 367, 373, 376, 382, 388, 394, 398, 404, 410, 416, 422,
//      430-432, 445) are removed because `thiserror::Error` is a
//      proc-macro attribute crate that Verus single-file mode cannot
//      expand without `--extern thiserror=...` registration.
//
//   2. The `#[non_exhaustive]` attribute on `WorkflowError` (production
//      workflow/mod.rs:320) is removed because it is a built-in
//      attribute that is unrelated to the discriminant set and is not
//      preserved in the structural mirror.
//
//   3. The newtype types `StepIdx`, `SlotIdx`, `ConstIdx`, `SymbolId`
//      (declared in production via the `numeric_id!($name, $inner, get)`
//      macro at crates/vb_core/src/ids/mod.rs:9-40) and the enum
//      `CoreError` (declared at crates/vb_core/src/errors.rs:167 with
//      the `#[derive(... Error ...)]` derive) are inlined as plain
//      `pub struct $name(pub $inner)` newtypes and a placeholder
//      `CoreError` enum. Field names and types match production exactly
//      so the discriminant projection used by `From<WorkflowError> for
//      CompileError` at crates/vb_compile/src/mod_compile_errors/kind.rs
//      :54 (`CompileError::Workflow(workflow_error)`) resolves identically.
//
// The 20-variant discriminant set is preserved EXACTLY:
//   1. EmptyNodes
//   2. EntryOutOfBounds { entry: StepIdx }
//   3. StepOutOfBounds { step: StepIdx }
//   4. SlotOutOfBounds { slot: SlotIdx }
//   5. ConstOutOfBounds { constant: ConstIdx }
//   6. NodeIdMismatch { expected: StepIdx, actual: StepIdx }
//   7. Expression(CoreError)
//   8. ResourceContractExceeded { resource: &'static str }
//   9. ResourceContractTooLarge { resource: &'static str }
//   10. EmptyBranchTable
//   11. UnreachableNode { step: StepIdx }
//   12. BackwardEdge { from: StepIdx, to: StepIdx }
//   13. ImproperLoopNesting { inner: StepIdx, outer_done: StepIdx }
//   14. BudgetPolicyExceeded { detail: &'static str }
//   15. StepCountOverflow { actual: u64 }
//   16. DepthOverflow { depth: u16 }
//   17. SymbolOutOfBounds { symbol: SymbolId }
//   18. AccessorPathTooDeep { depth: usize, max: usize }
//   19. JumpCycle { step: StepIdx, target: StepIdx }
//   20. NestedTogether { outer: StepIdx, inner: StepIdx }
//
// DRIFT POLICY: This file MUST be regenerated from
// `crates/vb_core/src/workflow/mod.rs:321-452` whenever production
// adds, removes, or renames a `WorkflowError` variant. The mirror is
// annotated at the top of every section with the originating
// production line range so regeneration is mechanical. Drift between
// this mirror and the production source breaks the
// `extern_vb_xi2f_error_mapping` Verus build, which is the explicit
// drift-detection mechanism the user requires.
//
// This file is included by the companion extern file under
// `#[verifier::external]` so every body is opaque to Verus. It
// compiles as plain Rust (no `verus!` block, no `vstd` import) and
// is checked by the Verus invocation purely for structural
// resolution and type well-formedness — Verus never reasons about
// the bodies.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ---------------------------------------------------------------------------
// Local stubs for the production newtype ids used by WorkflowError
// ---------------------------------------------------------------------------
//
// Production declares these via
// `crates/vb_core/src/ids/mod.rs:9-40`:
//
//   macro_rules! numeric_id {
//       ($name:ident, $inner:ty, $accessor:ident) => {
//           pub struct $name($inner);
//           impl $name { ... pub const fn new(...) ... pub const fn get(...) }
//       };
//   }
//   numeric_id!(StepIdx, u16, get);
//   numeric_id!(SlotIdx, u16, get);
//   numeric_id!(ConstIdx, u16, get);
//   numeric_id!(SymbolId, u32, get);
//
// We inline them here as plain `pub struct $name(pub $inner)` so the
// field names (`entry`, `step`, `slot`, `constant`, `expected`,
// `actual`, `from`, `to`, `inner`, `outer_done`, `symbol`, `target`,
// `outer`) all resolve. The inner type matches production exactly so
// the `From<WorkflowError> for CompileError` projection at
// kind.rs:54 carries the same field shapes.

/// Production `StepIdx` newtype at `crates/vb_core/src/ids/mod.rs:55`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepIdx(pub u16);

/// Production `SlotIdx` newtype at `crates/vb_core/src/ids/mod.rs:56`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotIdx(pub u16);

/// Production `ConstIdx` newtype at `crates/vb_core/src/ids/mod.rs:60`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstIdx(pub u16);

/// Production `SymbolId` newtype at `crates/vb_core/src/ids/mod.rs:61`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolId(pub u32);

// ---------------------------------------------------------------------------
// Local stub for the production `CoreError` enum used by WorkflowError
// ---------------------------------------------------------------------------
//
// Production `CoreError` is declared at `crates/vb_core/src/errors.rs:167`
// with `#[derive(Debug, Error, Clone, PartialEq, Eq)]` and a 24-variant
// discriminant set. The `WorkflowError::Expression(#[from] CoreError)`
// variant at workflow/mod.rs:359 can carry any CoreError variant.
// For the purposes of this binding, the `Expression` discriminant is
// opaque: the spec only reasons about whether the variant is `Expression`
// (vs the other 19), not about the inner `CoreError` payload. The stub
// below declares a placeholder variant sufficient for the structural
// binding. Spec proofs do not destructure the inner CoreError.

/// Production `CoreError` placeholder at `crates/vb_core/src/errors.rs:167`.
/// Payload is opaque to this binding — see comment above.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// Opaque placeholder for the 24-variant production enum.
    Other,
}

// ---------------------------------------------------------------------------
// Verbatim copy of the production `WorkflowError` enum declaration
// ---------------------------------------------------------------------------
//
// Production source: `crates/vb_core/src/workflow/mod.rs:321-452`.
//
// Substitutions (documented at the top of this file):
//   - `#[derive(Debug, Clone, Error, PartialEq, Eq)]` removed.
//   - `#[non_exhaustive]` removed.
//   - All `#[error("...")]` per-variant attribute strings removed.
//   - `#[from]` on `Expression(CoreError)` removed (does not change the
//     enum shape, only suppresses the `From<CoreError> for WorkflowError`
//     auto-derive which is not used by this binding).

/// Production `WorkflowError` enum at
/// `crates/vb_core/src/workflow/mod.rs:321-452`. The 20-variant
/// discriminant set is preserved EXACTLY (see the file header for
/// the variant list and production line ranges).
///
/// Standard Rust trait derives (`Debug`, `Clone`, `PartialEq`, `Eq`)
/// are kept here so the production mirror remains a faithful clone
/// of production behavior at the trait level. The `thiserror::Error`
/// derive (production workflow/mod.rs:319) and the per-variant
/// `#[error("...")]` attributes (production workflow/mod.rs:323, 326,
/// 332, 338, 344, 350, 358, 361, 367, 373, 376, 382, 388, 394, 398,
/// 404, 410, 416, 422, 430-432, 445) are the ONLY attributes removed
/// (see file header for rationale).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowError {
    /// Production: workflow/mod.rs:323-324.
    EmptyNodes,
    /// Production: workflow/mod.rs:327-330.
    EntryOutOfBounds {
        /// Invalid entry step.
        entry: StepIdx,
    },
    /// Production: workflow/mod.rs:333-336.
    StepOutOfBounds {
        /// Invalid target step.
        step: StepIdx,
    },
    /// Production: workflow/mod.rs:339-342.
    SlotOutOfBounds {
        /// Invalid slot.
        slot: SlotIdx,
    },
    /// Production: workflow/mod.rs:345-348.
    ConstOutOfBounds {
        /// Invalid constant.
        constant: ConstIdx,
    },
    /// Production: workflow/mod.rs:351-356.
    NodeIdMismatch {
        /// Expected node id for this table position.
        expected: StepIdx,
        /// Actual node id emitted by the compiler.
        actual: StepIdx,
    },
    /// Production: workflow/mod.rs:359.
    Expression(CoreError),
    /// Production: workflow/mod.rs:362-365.
    ResourceContractExceeded {
        /// Resource name.
        resource: &'static str,
    },
    /// Production: workflow/mod.rs:368-371.
    ResourceContractTooLarge {
        /// Resource name.
        resource: &'static str,
    },
    /// Production: workflow/mod.rs:374.
    EmptyBranchTable,
    /// Production: workflow/mod.rs:377-380.
    UnreachableNode {
        /// Unreachable step index.
        step: StepIdx,
    },
    /// Production: workflow/mod.rs:383-388.
    BackwardEdge {
        /// Source step of the backward edge.
        from: StepIdx,
        /// Target step of the backward edge.
        to: StepIdx,
    },
    /// Production: workflow/mod.rs:391-396.
    ImproperLoopNesting {
        /// Inner loop start step.
        inner: StepIdx,
        /// Outer loop done step.
        outer_done: StepIdx,
    },
    /// Production: workflow/mod.rs:399-402.
    BudgetPolicyExceeded {
        /// Human-readable detail describing which dimension failed.
        detail: &'static str,
    },
    /// Production: workflow/mod.rs:405-408.
    StepCountOverflow {
        /// The overflowing value.
        actual: u64,
    },
    /// Production: workflow/mod.rs:411-414.
    DepthOverflow {
        /// The actual pre-overflow depth value.
        depth: u16,
    },
    /// Production: workflow/mod.rs:417-420.
    SymbolOutOfBounds {
        /// Invalid symbol identifier.
        symbol: SymbolId,
    },
    /// Production: workflow/mod.rs:423-428.
    AccessorPathTooDeep {
        /// Actual path depth.
        depth: usize,
        /// Maximum allowed path depth.
        max: usize,
    },
    /// Production: workflow/mod.rs:433-438.
    JumpCycle {
        /// Step issuing the jump.
        step: StepIdx,
        /// Jump target creating the cycle.
        target: StepIdx,
    },
    /// Production: workflow/mod.rs:446-451.
    NestedTogether {
        /// The outer `TogetherStart` step that owns the branch body.
        outer: StepIdx,
        /// The inner `TogetherStart` step reachable from the outer branch.
        inner: StepIdx,
    },
}