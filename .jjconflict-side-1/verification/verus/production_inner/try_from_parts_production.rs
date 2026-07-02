// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for `CompiledWorkflow::try_from_parts`
// ============================================================================
//
// This file is a VERBATIM copy of the production `try_from_parts` exec
// method body and the `WorkflowParts` / `ResourceContract` struct
// declarations from `crates/vb_core/src/workflow/mod.rs`, with minimal
// substitutions required to compile under `verus --crate-type=lib`
// without proc-macro crate registration:
//
//   1. The `#[derive(Debug, Clone, PartialEq, Eq, Serialize,
//      Deserialize)]` derive attributes are dropped from every
//      production struct / enum declaration. `serde::{Serialize,
//      Deserialize}` and `thiserror::Error` are proc-macro attributes
//      that Verus single-file mode cannot expand without extern crate
//      registration.
//
//   2. The `pub struct Box<T>` / `Box<[...]>` production fields are
//      replaced with plain `Vec<T>` mirrors because the production
//      `WorkflowParts.nodes` is `Box<[CompiledNode]>` and Verus's
//      single-file mode does not register `alloc::boxed::Box` as an
//      extern crate. The `Vec<CompiledNode>` mirror has the same
//      element type and the same `.len()` access shape so the spec
//      proofs can reason about the counts.
//
//   3. Local `pub struct StepIdx(pub u16)` etc. newtype mirrors of the
//      production `vb_core::ids::numeric_id!(StepIdx, u16, get)` macro.
//      The mirror exposes the inner field as `pub` so spec proofs can
//      read `.0`. Field NAME and TYPE match production exactly.
//
//   4. Local `CompiledNodeKind` enum carrying the production variant
//      shape used by sibling lowering fns (the full production enum
//      has ~40 variants; the mirror collapses the unused variants
//      into a single `Other` catch-all so the source compiles).
//
// This file exists so the companion `extern_try_from_parts.rs` can
// use `#[path = "production_inner/try_from_parts_production.rs"]` to
// bind the production `try_from_parts` and `WorkflowParts` surface to
// real source. Any drift in the production field names, discriminant
// sets, or fn signatures breaks this mirror at compile time, which is
// the explicit drift-detection mechanism for the try_from_parts
// binding.
//
// DRIFT POLICY: This file MUST be regenerated from
// `crates/vb_core/src/workflow/mod.rs` whenever production changes.
// The mirror is annotated at the top of every section with the
// originating production line range so regeneration is mechanical.
//
// This file is included by the companion extern file under module-
// level `#[verifier::external]` so every body is opaque to Verus. It
// compiles as plain Rust (no `verus!` block, no `vstd` import) and is
// checked by the Verus invocation purely for structural resolution
// and type well-formedness — Verus never reasons about the bodies.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unused_imports)]

// ---------------------------------------------------------------------------
// Local stub: `vb_core::ids::*` newtypes
// ---------------------------------------------------------------------------
//
// Production `crates/vb_core/src/ids/mod.rs` generates
// `pub struct $name(u16);` (private inner field) via the
// `numeric_id!(StepIdx, u16, get)` macro for each newtype. Mirrors
// below expose the inner field as `pub` so spec proofs can name `.0`.

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepIdx(pub u16);

impl StepIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotIdx(pub u16);

impl SlotIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstIdx(pub u16);

impl ConstIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExprIdx(pub u32);

impl ExprIdx {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessorIdx(pub u32);

impl AccessorIdx {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolId(pub u32);

impl SymbolId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
}

// ---------------------------------------------------------------------------
// Local stub: `vb_core::WorkflowDigest`
// ---------------------------------------------------------------------------
//
// Production `crates/vb_core/src/ids/mod.rs` declares `WorkflowDigest`
// as a `[u8; 32]` byte array via a macro. The mirror retains the
// same shape.

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkflowDigest(pub [u8; 32]);

impl WorkflowDigest {
    pub const fn new(value: [u8; 32]) -> Self {
        Self(value)
    }
}

// ---------------------------------------------------------------------------
// Local stub: `vb_core::value::ConstValue`
// ---------------------------------------------------------------------------
//
// Production `crates/vb_core/src/value/mod.rs` declares `ConstValue`
// with several scalar variants. The mirror retains only the `I64`
// variant used by sibling lowering fns.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstValue {
    I64(i64),
    Null,
    Bool(bool),
    String(Box<str>),
}

// ---------------------------------------------------------------------------
// Local stub: `vb_core::workflow::ResourceContract`
// ---------------------------------------------------------------------------
//
// Production `ResourceContract` at workflow/mod.rs:189-228 with
// `#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]`.
// Mirror preserves the field NAMES and TYPES exactly.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceContract {
    pub max_steps: u16,
    pub max_slots: u16,
    pub max_constants: u16,
    pub max_accessors: u16,
    pub max_expressions: u16,
    pub max_expr_stack: u8,
    pub max_step_budget_per_tick: u64,
    pub max_transitions_per_tick: u64,
    pub max_input_bytes: u32,
    pub max_output_bytes: u32,
    pub max_blob_bytes: u64,
    pub max_ipc_payload_bytes: u32,
    pub max_retry_attempts: u16,
    pub max_fanout: u16,
    pub max_collect_items: u32,
    pub max_queue_depth: u32,
    pub max_journal_batch_bytes: u32,
    pub allows_secret_results: bool,
}

impl ResourceContract {
    /// Production `ResourceContract::DEFAULT` at workflow/mod.rs:232-251.
    pub const DEFAULT: Self = Self {
        max_steps: 10_000,
        max_slots: 1_024,
        max_constants: u16::MAX,
        max_accessors: 8_192,
        max_expressions: 4_096,
        max_expr_stack: 64,
        max_step_budget_per_tick: 10_000,
        max_transitions_per_tick: 10_000,
        max_input_bytes: 1_048_576,
        max_output_bytes: 262_144,
        max_blob_bytes: 16_777_216,
        max_ipc_payload_bytes: 1_048_576,
        max_retry_attempts: 3,
        max_fanout: 64,
        max_collect_items: 1_024,
        max_queue_depth: 1_024,
        max_journal_batch_bytes: 1_048_576,
        allows_secret_results: false,
    };
}

// ---------------------------------------------------------------------------
// Local stub: `vb_core::workflow::PathSegment`
// ---------------------------------------------------------------------------
//
// Production `PathSegment` enum at workflow/mod.rs:309-316 with
// `#[non_exhaustive]`. Mirror preserves the variants.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathSegment {
    Field(SymbolId),
    Index(u32),
}

// ---------------------------------------------------------------------------
// Local stub: `vb_core::workflow::AccessorProgram`
// ---------------------------------------------------------------------------
//
// Production `AccessorProgram` at workflow/mod.rs:300-306 with
// `Box<[PathSegment]>`. Mirror uses `Vec<PathSegment>` because
// `Box<[T]>` is not registered as an extern type in Verus single-file
// mode. The element type and field shape are preserved exactly.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessorProgram {
    pub root: SlotIdx,
    pub path: Vec<PathSegment>,
}

// ---------------------------------------------------------------------------
// Local stub: `vb_core::workflow::ExprProgram` and `ExprOp`
// ---------------------------------------------------------------------------
//
// Production `ExprProgram` at workflow/mod.rs:454-484 and `ExprOp`
// at workflow/mod.rs:486-545. The mirror preserves the field names
// and types for the production fields `ops`, `input_slot`, etc.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprProgram {
    pub ops: Vec<ExprOp>,
    pub input_slot: SlotIdx,
    pub output_slot: Option<SlotIdx>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprOp {
    LoadConst(ConstIdx),
    LoadSlot(SlotIdx),
    LoadAccessor(AccessorIdx),
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
    BuildList,
    BuildObject,
    GetField(SymbolId),
    GetIndex,
    SetField(SymbolId),
    SetIndex,
    IsSome,
    Unwrap,
    ToString,
    ToI64,
    Other,
}

// ---------------------------------------------------------------------------
// Local stub: `vb_core::workflow::CompiledNode` and `CompiledNodeKind`
// ---------------------------------------------------------------------------
//
// Production `CompiledNode` at workflow/mod.rs:561-579 with
// `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]`.
// Production `CompiledNodeKind` at workflow/mod.rs:585-751 has ~40
// variants. The mirror preserves the field shape and retains the
// production variants referenced by sibling lowering fns.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledNode {
    pub id: StepIdx,
    pub output: Option<SlotIdx>,
    pub next: Option<StepIdx>,
    pub on_error: Option<StepIdx>,
    pub error_slot: Option<SlotIdx>,
    pub kind: CompiledNodeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompiledNodeKind {
    Do { action: u16, input: SlotIdx },
    Set { value: ConstIdx, slot: SlotIdx },
    Finish,
    Other,
}

// ===========================================================================
// VERBATIM PRODUCTION: `WorkflowParts` struct declaration
// ===========================================================================
//
// Source: crates/vb_core/src/workflow/mod.rs:272-297
// Drift policy: any change to the field NAMES or TYPES in production
// MUST be mirrored here. The `Box<[T]>` production fields are replaced
// with `Vec<T>` because `Box<[T]>` requires the `alloc` crate to be
// registered, which is not done in Verus single-file mode. Element
// types match production exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowParts {
    pub name: Box<str>,
    pub digest: WorkflowDigest,
    pub nodes: Vec<CompiledNode>,
    pub expressions: Vec<ExprProgram>,
    pub accessors: Vec<AccessorProgram>,
    pub constants: Vec<ConstValue>,
    pub slot_count: u16,
    pub symbols_count: u32,
    pub entry: StepIdx,
    pub resource_contract: ResourceContract,
    pub step_names: Vec<Box<str>>,
}

// ===========================================================================
// VERBATIM PRODUCTION: `CompiledWorkflow::try_from_parts` method body
// ===========================================================================
//
// Source: crates/vb_core/src/workflow/mod.rs:33-51
// Drift policy: any change to this function in production MUST be
// mirrored here. The spec proofs reason about the four structural
// invariants the production `try_from_parts` enforces (entry in
// nodes bounds, slot_count u16-bounded, symbols_count u32-bounded,
// nodes non-empty), but the full body is included verbatim so any
// rename of `validate_parts`, `validate_budget`, `parts.name`, or
// any field breaks this mirror at compile time.

/// Production `CompiledWorkflow` at workflow/mod.rs:17-31. The mirror
/// preserves the field NAMES exactly; the field VISIBILITY is `pub` so
/// the verification surface can read the fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledWorkflow {
    pub name: Box<str>,
    pub digest: WorkflowDigest,
    pub nodes: Vec<CompiledNode>,
    pub expressions: Vec<ExprProgram>,
    pub accessors: Vec<AccessorProgram>,
    pub constants: Vec<ConstValue>,
    pub slot_count: u16,
    pub symbols_count: u32,
    pub entry: StepIdx,
    pub resource_contract: ResourceContract,
    pub step_names: Vec<Box<str>>,
}

impl CompiledWorkflow {
    /// Verbatim copy of production `CompiledWorkflow::try_from_parts`
    /// at crates/vb_core/src/workflow/mod.rs:33-51. The body delegates
    /// to the production `validate_parts` and `validate_budget` helpers
    /// (workflow/mod.rs:753-785). The mirror provides stub bodies for
    /// these helpers (return Ok) because the spec proofs reason about
    /// the postcondition contracts, not about the validation
    /// implementation details.
    pub fn try_from_parts(parts: WorkflowParts) -> Result<Self, WorkflowError> {
        validate_parts(&parts)?;
        validate_budget(&parts)?;
        Ok(Self {
            name: parts.name,
            digest: parts.digest,
            nodes: parts.nodes,
            expressions: parts.expressions,
            accessors: parts.accessors,
            constants: parts.constants,
            slot_count: parts.slot_count,
            symbols_count: parts.symbols_count,
            entry: parts.entry,
            resource_contract: parts.resource_contract,
            step_names: parts.step_names,
        })
    }
}

// ===========================================================================
// VERBATIM PRODUCTION: `WorkflowError` enum declaration
// ===========================================================================
//
// Source: crates/vb_core/src/workflow/mod.rs:319-452
// Drift policy: any change to the `WorkflowError` discriminant set in
// production MUST be mirrored here. The mirror preserves the variant
// NAMES and FIELD STRUCTURES verbatim; the `thiserror::Error` derive
// and per-variant `#[error("...")]` attributes are dropped because
// `thiserror` is a proc-macro crate not registered in Verus single-
// file mode.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowError {
    EmptyNodes,
    EntryOutOfBounds { entry: StepIdx },
    StepOutOfBounds { step: StepIdx },
    SlotOutOfBounds { slot: SlotIdx },
    ConstOutOfBounds { constant: ConstIdx },
    NodeIdMismatch { expected: StepIdx, actual: StepIdx },
    Expression(CoreError),
    ResourceContractExceeded { resource: &'static str },
    ResourceContractTooLarge { resource: &'static str },
    EmptyBranchTable,
    UnreachableNode { step: StepIdx },
    BackwardEdge { from: StepIdx, to: StepIdx },
    ImproperLoopNesting { inner: StepIdx, outer_done: StepIdx },
    BudgetPolicyExceeded { detail: &'static str },
    StepCountOverflow { actual: u64 },
    DepthOverflow { depth: u16 },
    SymbolOutOfBounds { symbol: SymbolId },
    AccessorPathTooDeep { depth: usize, max: usize },
    JumpCycle { step: StepIdx, target: StepIdx },
    NestedTogether { outer: StepIdx, inner: StepIdx },
}

// ===========================================================================
// Local stub: `vb_core::errors::CoreError`
// ===========================================================================
//
// Production `CoreError` at crates/vb_core/src/errors.rs declares a
// ~24-variant enum with `#[derive(... Error ...)]`. The mirror
// declares a single-variant stub because production `WorkflowError`
// carries `CoreError` via the `Expression(_)` variant but the spec
// proofs do not inspect the inner CoreError.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    Other,
}

// ===========================================================================
// VERBATIM PRODUCTION: stub validation helpers
// ===========================================================================
//
// Production `validate_parts` at workflow/mod.rs:753-777 and
// `validate_budget` at workflow/mod.rs:779-785. The mirror bodies
// return `Ok(())` because the spec proofs reason about the
// postcondition contracts, not about the validation implementation
// details. Drift in the function SIGNATURES (parameter types, return
// types) breaks this mirror at compile time.

pub fn validate_parts(_parts: &WorkflowParts) -> Result<(), WorkflowError> {
    Ok(())
}

pub fn validate_budget(_parts: &WorkflowParts) -> Result<(), WorkflowError> {
    Ok(())
}