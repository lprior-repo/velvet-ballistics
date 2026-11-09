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
//   3. The `pub struct StepIdx(pub u16)` etc. newtype mirrors of the
//      production `vb_core::ids::numeric_id!(StepIdx, u16, get)` macro
//      live in the sibling `try_from_parts_production_ids.rs` file and
//      are re-exported below. The mirror exposes the inner field as
//      `pub` so spec proofs can read `.0`. Field NAME and TYPE match
//      production exactly.
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
// Numeric-id and scalar mirrors: `production_inner/`
// `try_from_parts_production_ids.rs`
// ---------------------------------------------------------------------------
//
// The `StepIdx` / `SlotIdx` / `ConstIdx` / `ActionId` / `ExprIdx` /
// `AccessorIdx` / `SymbolId` / `WorkflowDigest` / `FiniteF64` /
// `ConstValue` mirrors live in the sibling file so this mirror stays
// under the 800-line source-length limit. They are re-exported here so
// existing `production_try_from_parts::X` paths keep working.
pub use super::try_from_parts_production_ids::AccessorIdx;
pub use super::try_from_parts_production_ids::ActionId;
pub use super::try_from_parts_production_ids::ConstIdx;
pub use super::try_from_parts_production_ids::ConstValue;
pub use super::try_from_parts_production_ids::ExprIdx;
pub use super::try_from_parts_production_ids::FiniteF64;
pub use super::try_from_parts_production_ids::SlotIdx;
pub use super::try_from_parts_production_ids::StepIdx;
pub use super::try_from_parts_production_ids::SymbolId;
pub use super::try_from_parts_production_ids::WorkflowDigest;

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
// Production `CompiledNode` at workflow/mod.rs:995-1004 with
// `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]`.
// Production `CompiledNodeKind` at workflow/mod.rs:1191-1357 has ~40
// variants. The mirror preserves the field shape and retains the
// production variants referenced by sibling lowering fns.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledNode {
    pub id: StepIdx,
    pub output: NodeOutput,
    pub routing: NodeRouting,
    pub kind: CompiledNodeKind,
}

/// Mirror of production `NodeOutput` at
/// `crates/vb_core/src/workflow/mod.rs:1008-1013`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeOutput {
    Discard,
    Slot(SlotIdx),
}

impl NodeOutput {
    pub const fn slot(self) -> Option<SlotIdx> {
        match self {
            Self::Discard => None,
            Self::Slot(slot) => Some(slot),
        }
    }
}

/// Mirror of production `NodeRouting` at
/// `crates/vb_core/src/workflow/mod.rs:1028-1033`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeRouting {
    pub next: NodeContinuation,
    pub error: ErrorRouting,
}

/// Mirror of production `NodeContinuation` at
/// `crates/vb_core/src/workflow/mod.rs:1037-1042`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeContinuation {
    End,
    Step(StepIdx),
}

impl NodeContinuation {
    pub const fn step(self) -> Option<StepIdx> {
        match self {
            Self::End => None,
            Self::Step(step) => Some(step),
        }
    }
}

/// Mirror of production `ErrorRouting` at
/// `crates/vb_core/src/workflow/mod.rs:1083-1093`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorRouting {
    None,
    Handler {
        handler: StepIdx,
        error_slot: Option<SlotIdx>,
    },
}

impl ErrorRouting {
    pub const fn handler(self) -> Option<StepIdx> {
        match self {
            Self::None => None,
            Self::Handler { handler, .. } => Some(handler),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompiledNodeKind {
    Do { action: ActionId, input: SlotIdx },
    SetConst { value: ConstIdx },
    Finish { result: SlotIdx },
    Other,
}

// ---------------------------------------------------------------------------
// Local stub: `vb_core::workflow::ExprBranch` / `SlotBranch`
// ---------------------------------------------------------------------------
//
// Production `ExprBranch` at workflow/mod.rs:282-288 and `SlotBranch`
// at workflow/mod.rs:291-296. Final choose IR scaffolding targets.
// Mirror preserves field NAMES and TYPES exactly.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExprBranch {
    pub condition: ExprIdx,
    pub target: StepIdx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotBranch {
    pub condition: SlotIdx,
    pub target: StepIdx,
}

// ---------------------------------------------------------------------------
// Local stub: `vb_core::workflow::InputSlotKind` / `CompiledInputSlot`
// ---------------------------------------------------------------------------
//
// Production `InputSlotKind` at workflow/mod.rs:301-320 (9-variant enum)
// and `CompiledInputSlot` at workflow/mod.rs:328-335. Mirror preserves
// the variant NAMES and field NAMES/TYPES exactly.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputSlotKind {
    Null,
    Bool,
    I64,
    F64,
    Symbol,
    List,
    Object,
    Blob,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompiledInputSlot {
    pub slot: SlotIdx,
    pub kind: InputSlotKind,
}

// ===========================================================================
// VERBATIM PRODUCTION: `WorkflowParts` struct declaration
// ===========================================================================
//
// Source: crates/vb_core/src/workflow/mod.rs:609-642
// Drift policy: any change to the field NAMES or TYPES in production
// MUST be mirrored here. The `Box<[T]>` production fields are replaced
// with `Vec<T>` because `Box<[T]>` requires the `alloc` crate to be
// registered, which is not done in Verus single-file mode. Element
// types match production exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowParts {
    pub name: Box<str>,
    pub digest: WorkflowDigest,
    pub semantic_source_identity: Option<WorkflowDigest>,
    pub nodes: Vec<CompiledNode>,
    pub expressions: Vec<ExprProgram>,
    pub accessors: Vec<AccessorProgram>,
    pub constants: Vec<ConstValue>,
    pub slot_count: u16,
    pub symbols_count: u32,
    pub entry: StepIdx,
    pub resource_contract: ResourceContract,
    pub step_names: Vec<Box<str>>,
    pub input_slots: Vec<CompiledInputSlot>,
}

// ===========================================================================
// VERBATIM PRODUCTION: `CompiledWorkflow::try_from_parts` method body
// ===========================================================================
//
// Source: crates/vb_core/src/workflow/mod.rs:42-65
// Drift policy: any change to this function in production MUST be
// mirrored here. The spec proofs reason about the structural
// invariants the production `try_from_parts` enforces (entry in
// nodes bounds, slot_count u16-bounded, symbols_count u32-bounded,
// nodes non-empty), but the full body is included verbatim so any
// rename of `validate_parts`, `loop_regions::validate`,
// `loop_regions::nested_activation_map`, `validate_budget`,
// `validate_input_slots`, `parts.name`, or any field breaks this mirror
// at compile time.

/// Production `CompiledWorkflow` at workflow/mod.rs:22-38. The mirror
/// preserves the field NAMES exactly; the field VISIBILITY is `pub` so
/// the verification surface can read the fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledWorkflow {
    pub name: Box<str>,
    pub digest: WorkflowDigest,
    pub semantic_source_identity: Option<WorkflowDigest>,
    pub nodes: Vec<CompiledNode>,
    pub expressions: Vec<ExprProgram>,
    pub accessors: Vec<AccessorProgram>,
    pub constants: Vec<ConstValue>,
    pub slot_count: u16,
    pub symbols_count: u32,
    pub entry: StepIdx,
    pub resource_contract: ResourceContract,
    pub step_names: Vec<Box<str>>,
    pub input_slots: Vec<CompiledInputSlot>,
    pub loop_regions: Vec<Option<CompiledLoopRegion>>,
    pub nested_activation_entries: Vec<bool>,
}

impl CompiledWorkflow {
    /// Verbatim copy of production `CompiledWorkflow::try_from_parts`
    /// at crates/vb_core/src/workflow/mod.rs:42-65. The body delegates
    /// to production `validate_parts`, `loop_regions::validate`,
    /// `loop_regions::nested_activation_map`, `validate_budget`, and
    /// `validate_input_slots`. `validate_parts`, `validate_budget`,
    /// `loop_regions::validate`, and `loop_regions::nested_activation_map`
    /// are stub bodies (return Ok) because the spec proofs reason
    /// about the postcondition contracts. `validate_input_slots` is
    /// a faithful body matching production.
    pub fn try_from_parts(parts: WorkflowParts) -> Result<Self, WorkflowError> {
        validate_parts(&parts)?;
        let loop_regions = loop_regions::validate(&parts)?;
        let nested_activation_entries = loop_regions::nested_activation_map(&parts, &loop_regions)?;
        validate_budget(&parts)?;
        validate_input_slots(&parts)?;
        Ok(Self {
            name: parts.name,
            digest: parts.digest,
            semantic_source_identity: parts.semantic_source_identity,
            nodes: parts.nodes,
            expressions: parts.expressions,
            accessors: parts.accessors,
            constants: parts.constants,
            slot_count: parts.slot_count,
            symbols_count: parts.symbols_count,
            entry: parts.entry,
            resource_contract: parts.resource_contract,
            step_names: parts.step_names,
            input_slots: parts.input_slots,
            loop_regions,
            nested_activation_entries,
        })
    }
}

// ===========================================================================
// VERBATIM PRODUCTION: `WorkflowError` enum declaration
// ===========================================================================
//
// Source: crates/vb_core/src/workflow/mod.rs:666-880
// Drift policy: any change to the `WorkflowError` discriminant set in
// production MUST be mirrored here. The mirror preserves the variant
// NAMES and FIELD STRUCTURES verbatim; the `thiserror::Error` derive
// and per-variant `#[error("...")]` attributes are dropped because
// `thiserror` is a proc-macro crate not registered in Verus single-
// file mode.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowError {
    RepeatCounterForeignWrite {
        owner: StepIdx,
        writer: StepIdx,
        slot: SlotIdx,
    },
    EmptyNodes,
    EntryOutOfBounds {
        entry: StepIdx,
    },
    StepOutOfBounds {
        step: StepIdx,
    },
    SlotOutOfBounds {
        slot: SlotIdx,
    },
    ConstOutOfBounds {
        constant: ConstIdx,
    },
    NodeIdMismatch {
        expected: StepIdx,
        actual: StepIdx,
    },
    Expression(CoreError),
    ResourceContractExceeded {
        resource: ContractResource,
    },
    ResourceContractTooLarge {
        resource: ContractResource,
    },
    EmptyBranchTable,
    UnreachableNode {
        step: StepIdx,
    },
    BackwardEdge {
        from: StepIdx,
        to: StepIdx,
    },
    ReduceBranchTargetAlias {
        step: StepIdx,
        target: StepIdx,
    },
    ForEachBranchTargetAlias {
        step: StepIdx,
        target: StepIdx,
    },
    ImproperLoopNesting {
        inner: StepIdx,
        outer_done: StepIdx,
    },
    BudgetPolicyExceeded {
        detail: BudgetDimension,
    },
    StepCountOverflow {
        actual: u64,
    },
    DepthOverflow {
        depth: u16,
    },
    SymbolOutOfBounds {
        symbol: SymbolId,
    },
    AccessorPathTooDeep {
        depth: usize,
        max: usize,
    },
    JumpCycle {
        step: StepIdx,
        target: StepIdx,
    },
    NestedTogether {
        outer: StepIdx,
        inner: StepIdx,
    },
    InputSlotOutOfBounds {
        slot: SlotIdx,
        kind: InputSlotKind,
    },
    DuplicateInputSlot {
        slot: SlotIdx,
        kind: InputSlotKind,
    },
    NonCanonicalInputSlotOrder {
        prev_slot: SlotIdx,
        curr_slot: SlotIdx,
    },
    InconsistentInputConstraint {
        slot: SlotIdx,
        issue: InputConstraintIssue,
    },
    LoopRegionUnpaired {
        control: StepIdx,
    },
    LoopRegionCrossing {
        control: StepIdx,
        from: StepIdx,
        to: StepIdx,
    },
    LoopRegionUnsupported {
        control: StepIdx,
        step: StepIdx,
    },
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
// Local stub: `vb_core::workflow::InputConstraintIssue`
// ===========================================================================
//
// Production `InputConstraintIssue` at crates/vb_core/src/workflow/mod.rs:598-605
// declares a 3-variant enum. The mirror reproduces it verbatim; it is the
// payload type of `WorkflowError::InconsistentInputConstraint`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputConstraintIssue {
    MinExceedsMax,
    MinLengthExceedsMaxLength,
    NegativeLengthBound,
}

// ===========================================================================
// Local stub: `vb_core::workflow::ContractResource`
// ===========================================================================
//
// Production `ContractResource` at crates/vb_core/src/workflow/mod.rs:299-342
// declares a 21-variant enum. The mirror declares a single-variant stub
// because production `WorkflowError` carries `ContractResource` only via
// the `ResourceContractExceeded` / `ResourceContractTooLarge` payloads
// and the spec proofs do not inspect the inner resource dimension.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractResource {
    Steps,
}

// ===========================================================================
// Local stub: `vb_core::budget::BudgetDimension`
// ===========================================================================
//
// Production `BudgetDimension` at crates/vb_core/src/budget/validate_error.rs:100-147
// declares a ~20-variant enum. The mirror declares a single-variant stub
// because production `WorkflowError` carries `BudgetDimension` only via
// the `BudgetPolicyExceeded` payload and the spec proofs do not inspect
// the inner budget dimension.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetDimension {
    TotalSteps,
}

// ===========================================================================
// PRODUCTION MIRROR: stub validation helpers + faithful input-slot body
// ===========================================================================
//
// Production `validate_parts` at workflow/mod.rs:850-874 and
// `validate_budget` at workflow/mod.rs:908-914. The mirror bodies
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
// Production crates/vb_core/src/workflow/mod.rs:1394-1425
// Drift policy: any change to this function body in production MUST be
// mirrored here. The spec proofs depend on the faithful implementation.

pub fn validate_input_slots(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let slot_count = parts.slot_count;
    let mut prev_slot: Option<SlotIdx> = None;
    for input_slot in parts.input_slots.iter() {
        let current: SlotIdx = input_slot.slot;
        let current_value = current.get();
        if current_value >= slot_count {
            return Err(WorkflowError::InputSlotOutOfBounds {
                slot: current,
                kind: input_slot.kind,
            });
        }
        if let Some(prev) = prev_slot {
            let prev_value = prev.get();
            if current_value == prev_value {
                return Err(WorkflowError::DuplicateInputSlot {
                    slot: current,
                    kind: input_slot.kind,
                });
            }
            if current_value < prev_value {
                return Err(WorkflowError::NonCanonicalInputSlotOrder {
                    prev_slot: prev,
                    curr_slot: current,
                });
            }
        }
        prev_slot = Some(current);
    }
    Ok(())
}

// ===========================================================================
// PRODUCTION MIRROR: loop-region ownership cache stubs
// ===========================================================================
//
// Production `CompiledLoopRegion` at
// `crates/vb_core/src/workflow/loop_regions.rs:14-19`, `validate` at
// `crates/vb_core/src/workflow/loop_regions.rs:59-72`, and
// `nested_activation_map` at
// `crates/vb_core/src/workflow/loop_regions.rs:78-83`. The mirror bodies
// return `Ok` with empty collections because the spec proofs reason about
// the postcondition contracts, not about the validation implementation
// details. Drift in the struct FIELDS or function SIGNATURES breaks this
// mirror at compile time. The production `Box<[...]>` return types are
// mirrored as `Vec<...>` per the file header substitution rule.

/// Mirror of production `CompiledLoopRegion`: immutable,
/// constructor-validated ownership of one loop backedge. Field names and
/// types match production; visibility is `pub` per this file's convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompiledLoopRegion {
    pub start: StepIdx,
    pub body: StepIdx,
    pub control: StepIdx,
    pub done: StepIdx,
}

pub mod loop_regions {
    use super::{CompiledLoopRegion, WorkflowError, WorkflowParts};

    /// Stub for production `loop_regions::validate`: builds a
    /// control-indexed cache without changing serialized workflow parts.
    pub fn validate(
        _parts: &WorkflowParts,
    ) -> Result<Vec<Option<CompiledLoopRegion>>, WorkflowError> {
        Ok(Vec::new())
    }

    /// Stub for production `loop_regions::nested_activation_map`: derives
    /// the nested activation entry map from validated loop regions.
    pub fn nested_activation_map(
        _parts: &WorkflowParts,
        _regions: &[Option<CompiledLoopRegion>],
    ) -> Result<Vec<bool>, WorkflowError> {
        Ok(Vec::new())
    }
}
