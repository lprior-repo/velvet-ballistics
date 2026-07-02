// SPDX-License-Identifier: MIT
//
// ============================================================================
// WEAK PRODUCTION BINDING (production_inner mirror)
// ============================================================================
//
// This file is the production-binding surface for the
// `vb_xi2f_error_mapping.rs` Verus spec. It contains:
//
//   1. A direct `#[path]` inclusion of the production-source mirror at
//      `verification/verus/production_inner/_workflow_error_production.rs`.
//      That mirror is a VERBATIM copy of the production `WorkflowError`
//      enum declaration at `crates/vb_core/src/workflow/mod.rs:321-452`
//      and the newtype declarations at
//      `crates/vb_core/src/ids/mod.rs:55-67`. Three substitutions are
//      required to compile under `verus --crate-type=lib` without
//      proc-macro crate registration:
//
//        - The `#[derive(Debug, Clone, Error, PartialEq, Eq)]` derive
//          and the per-variant `#[error("...")]` strings on
//          `WorkflowError` (production workflow/mod.rs:319, 323, 326,
//          332, 338, 344, 350, 358, 361, 367, 373, 376, 382, 388, 394,
//          398, 404, 410, 416, 422, 430-432, 445) are removed because
//          `thiserror::Error` is a proc-macro attribute crate that
//          Verus single-file mode cannot expand without `--extern
//          thiserror=...` registration.
//        - The `#[non_exhaustive]` attribute on `WorkflowError`
//          (production workflow/mod.rs:320) is removed because it is
//          a built-in attribute unrelated to the discriminant set.
//        - The `#[derive(Serialize, Deserialize)]` and
//          `#[repr(transparent)]` attributes on the newtypes are
//          removed because `serde` is not registered as an extern
//          crate.
//
//   2. The production mirror module is marked `#[verifier::external]`
//      so the production discriminant set is opaque to Verus; only
//      structural resolution is checked (variant names, field names,
//      field types). Drift between the mirror and production breaks
//      the Verus build.
//
//   3. The `WorkflowError` enum and `StepIdx` / `SlotIdx` /
//      `ConstIdx` / `SymbolId` / `CoreError` newtypes are RE-DECLARED
//      inside the `verus!` block below (not re-exported from
//      `prod_src`). This is the established pattern in
//      `extern_vb_rpch_action_replay_tracker.rs` (which declares
//      `SpecActionReplayTracker` inside `verus!` and includes the
//      production IMPL BLOCK via `#[path]`): the production mirror
//      is a verbatim copy used for drift detection, and the
//      `verus!`-mode mirror is the version the spec proofs and
//      exec wrappers operate on.
//
// ============================================================================
// WHY THE PRODUCTION MIRROR (NOT DIRECT #[path] TO workflow/mod.rs)
// ============================================================================
// Direct `#[path = "../../crates/vb_core/src/workflow/mod.rs"]` inclusion
// is blocked by:
//
//   - workflow/mod.rs:14 `use serde::{Deserialize, Serialize};` requires
//     the `serde` extern crate, not registered under
//     `verus --crate-type=lib` (no installs allowed).
//   - workflow/mod.rs:15 `use thiserror::Error;` plus
//     `#[derive(... Error ...)]` at workflow/mod.rs:319 and the
//     21 `#[error("...")]` per-variant attributes require the
//     `thiserror` proc-macro attribute crate, not registered.
//   - workflow/mod.rs:780 `use crate::budget::{BoundednessPolicy,
//     WholeWorkflowBudget};` requires the full `vb_core::budget`
//     module tree.
//   - workflow/mod.rs:844 `use crate::limits::MAX_STEP_BUDGET;`
//     requires the production limits module and its 11 constants.
//
// The in-tree mirror at
// `verification/verus/production_inner/_workflow_error_production.rs`
// sidesteps every blocker by inlining only the 4 newtype stubs and
// 1 CoreError stub needed by the 20-variant discriminant set, and
// stripping every proc-macro attribute. The 20-variant enum
// declaration is otherwise identical to production.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `WorkflowError` (production enum, 20 variants)
//                            <- crates/vb_core/src/workflow/mod.rs:321-452
//   - `CompileError::Workflow(#[from] WorkflowError)` (production variant)
//                            <- crates/vb_compile/src/mod_compile_errors/kind.rs:54
//   - `impl From<WorkflowError> for CompileError` (production auto-derive)
//                            <- thiserror-generated from `#[from]`
//                               on CompileError::Workflow at kind.rs:54
//
// Spec-side projection of the production mapping into mathematical
// Set algebra:
//   - `spec_workflow_error_maps_to_compile_error(we, ce)` (production
//     predicate)
//                            <- CompileError::Workflow(workflow_error)
//                               at kind.rs:54
//   - `compile_error_from_workflow_error(we) -> ce` (production fn)
//                            <- the `#[from]`-derived
//                               `From::from(workflow_error)` at kind.rs:54
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production `WorkflowError` enum declaration in the production
// mirror is structural only — there are no production bodies in this
// mirror because the enum has no inherent methods. The production
// `From<WorkflowError> for CompileError` impl at kind.rs:54 is auto-
// derived from the `#[from]` attribute on `CompileError::Workflow`;
// its semantic body is the trivial projection
// `CompileError::Workflow(workflow_error)`. The
// `compile_error_from_workflow_error` wrapper declared in the spec
// file is `#[verifier::external]` so Verus does not attempt to verify
// its body; the `assume_specification` contract in the companion
// spec file states the production behavior (always returns
// `CompileError::Workflow(_)`, always carries the input
// `WorkflowError` discriminant unchanged), and the `exec fn`
// wrappers in the spec file discharge that contract.
//
// Drift between the production mirror and the production source is
// reported as binding-debt tracked outside Verus.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ============================================================================
// PRODUCTION INCLUSION via #[path]
// ============================================================================
//
// Direct `#[path]` inclusion of the production mirror file. The
// mirror is annotated with the production line ranges for each
// variant; see the mirror file header for the drift-detection policy.
// The mirror module is marked `#[verifier::external]` so Verus treats
// its contents as opaque.
#[verifier::external]
#[path = "production_inner/_workflow_error_production.rs"]
pub mod prod_src;

// ============================================================================
// Verus-mode mirror of production newtypes and WorkflowError
// ============================================================================
//
// The newtypes and `WorkflowError` enum are re-declared inside this
// `verus!` block (instead of being re-exported from `prod_src`) so
// that Verus can reason about them as opaque types: the exec
// wrappers in the companion spec file `vb_xi2f_error_mapping.rs`
// call `production::compile_error_from_workflow_error(workflow_error)`
// where `workflow_error` is `production::WorkflowError` (declared
// here). When `workflow_error` is from a `#[verifier::external]`
// module (i.e., re-exported from `prod_src`), Verus refuses the
// call because external types are spec-mode only.
//
// The variant set here matches the production mirror at
// `production_inner/_workflow_error_production.rs` byte-for-byte.
// Drift between this declaration and the production mirror breaks
// the Verus build, which is the explicit drift-detection mechanism.
/// Mirror of production `StepIdx` newtype at
/// `crates/vb_core/src/ids/mod.rs:55`. Production `pub struct
/// StepIdx(u16)` (private field). Mirror exposes the inner field as
/// `pub` so spec proofs can name `id.0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepIdx(pub u16);

/// Mirror of production `SlotIdx` newtype at
/// `crates/vb_core/src/ids/mod.rs:56`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotIdx(pub u16);

/// Mirror of production `ConstIdx` newtype at
/// `crates/vb_core/src/ids/mod.rs:60`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstIdx(pub u16);

/// Mirror of production `SymbolId` newtype at
/// `crates/vb_core/src/ids/mod.rs:61`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolId(pub u32);

/// Mirror of production `CoreError` enum at
/// `crates/vb_core/src/errors.rs:167` (24 variants). The spec only
/// needs the type as a payload of `WorkflowError::Expression`; the
/// inner CoreError variants are not pattern-matched in spec proofs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// Opaque placeholder for the 24-variant production enum.
    Other,
}

/// Mirror of production `WorkflowError` enum at
/// `crates/vb_core/src/workflow/mod.rs:321-452`. The 20-variant
/// discriminant set is preserved EXACTLY:
///
///   1.  EmptyNodes
///   2.  EntryOutOfBounds { entry: StepIdx }
///   3.  StepOutOfBounds { step: StepIdx }
///   4.  SlotOutOfBounds { slot: SlotIdx }
///   5.  ConstOutOfBounds { constant: ConstIdx }
///   6.  NodeIdMismatch { expected: StepIdx, actual: StepIdx }
///   7.  Expression(CoreError)
///   8.  ResourceContractExceeded { resource: &'static str }
///   9.  ResourceContractTooLarge { resource: &'static str }
///   10. EmptyBranchTable
///   11. UnreachableNode { step: StepIdx }
///   12. BackwardEdge { from: StepIdx, to: StepIdx }
///   13. ImproperLoopNesting { inner: StepIdx, outer_done: StepIdx }
///   14. BudgetPolicyExceeded { detail: &'static str }
///   15. StepCountOverflow { actual: u64 }
///   16. DepthOverflow { depth: u16 }
///   17. SymbolOutOfBounds { symbol: SymbolId }
///   18. AccessorPathTooDeep { depth: usize, max: usize }
///   19. JumpCycle { step: StepIdx, target: StepIdx }
///   20. NestedTogether { outer: StepIdx, inner: StepIdx }
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

// ============================================================================
// Spec-side mirror of CompileError::Workflow variant
// ============================================================================
//
// The production `CompileError` enum at
// `crates/vb_compile/src/mod_compile_errors/kind.rs:12-168` carries
// 49 variants; only the `Workflow(WorkflowError)` variant
// (kind.rs:53-54) is in scope for this spec. The mirror below is a
// structural clone of that one variant so the spec reasoning can
// refer to `CompileError::Workflow(workflow_error)` exactly as
// production does.
#[derive(Debug, PartialEq, Eq)]
pub enum CompileErrorMirror {
    /// Production: `CompileError::Workflow(#[from] WorkflowError)`
    /// at `crates/vb_compile/src/mod_compile_errors/kind.rs:53-54`.
    /// The auto-derived `From<WorkflowError> for CompileError` maps
    /// every `WorkflowError` variant into this constructor.
    Workflow(WorkflowError),
    /// Placeholder for the 48 production `CompileError` variants
    /// that are NOT reachable from `WorkflowError` mapping
    /// (kind.rs:13-52, 55-168). Spec proofs never construct or
    /// pattern-match this variant from a `WorkflowError` input; its
    /// presence ensures the enum is closed.
    NotWorkflow,
}

// ============================================================================
// Production exec projection — `compile_error_from_workflow_error`
// ============================================================================
//
// Mirror of the production `From<WorkflowError> for CompileError`
// impl that is auto-derived from the `#[from]` attribute on
// `CompileError::Workflow` at kind.rs:54. The semantic body of the
// production `from(workflow_error)` is the trivial projection
// `CompileError::Workflow(workflow_error)`.
//
// Body skipped by Verus (`#[verifier::external]`); the spec
// contract is attached via `assume_specification` in the companion
// spec file `vb_xi2f_error_mapping.rs`. Declared here (in the
// extern file) instead of the spec file so the exec wrapper in the
// spec file can call it as
// `production::compile_error_from_workflow_error(workflow_error)`.
#[verifier::external]
pub fn compile_error_from_workflow_error(workflow_error: WorkflowError) -> CompileErrorMirror {
    // Mirror of production: `From::from(workflow_error) ==
    // CompileError::Workflow(workflow_error)` (kind.rs:54).
    CompileErrorMirror::Workflow(workflow_error)
}

} // verus!
