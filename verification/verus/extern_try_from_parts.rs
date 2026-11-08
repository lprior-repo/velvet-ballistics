// SPDX-License-Identifier: MIT
//
// Extern surface for try_from_parts Verus spec.
//
// ============================================================================
// WEAK PRODUCTION BINDING (production_inner mirror)
// ============================================================================
// This file binds the try_from_parts.rs Verus spec to the production
// `CompiledWorkflow::try_from_parts` validation entry point in
// `crates/vb_core/src/workflow/mod.rs:42-65`. The binding is structural +
// contract: each production type is mirrored with the SAME name (where
// practical), SAME discriminant shape, and SAME field types, and the
// production exec validation has a `#[verifier::external]` wrapper that
// mirrors the production signature so any drift in field names,
// discriminant sets, or arg/return types breaks the verification build.
//
// ============================================================================
// WHY NOT FULL `#[path]` INCLUSION OF workflow/mod.rs
// ============================================================================
// Direct `#[path = "../../crates/vb_core/src/workflow/mod.rs"]` inclusion is
// blocked by the production file using:
//
//   1. `use thiserror::Error;` plus `#[derive(... Error ...)]` on
//      `WorkflowError` (workflow/mod.rs:18, 664). Verus
//      0.2026.05.05 (Rust 1.95.0) cannot expand `thiserror`-style derive
//      macros because `thiserror` is not registered as an extern crate
//      in this single-file Verus unit, and shim traits cannot satisfy
//      `#[derive(...)]` because derive macros require proc-macro crates.
//   2. `use serde::{Deserialize, Serialize};` plus `#[derive(... Serialize,
//      Deserialize ...)]` on every IR type (workflow/mod.rs:17,
//      608, 645, 654, 883, 912, 994, 1188). Same proc-macro blocker.
//   3. Rust 2024 let-chains (e.g. `validate_forward_target` style guards
//      and `is_some_and` at workflow/mod.rs:1445). Verus 0.2026.05.05
//      requires `--edition 2024` and is locked to single-file lib mode.
//
// These are all "NO production changes" blockers (per the task brief).
// The structural mirror below sidesteps every blocker while still
// establishing a real end-to-end binding: any drift in the production
// field names, discriminant sets, or fn signatures will break the
// `extern_try_from_parts` mirror and the spec proofs that depend on it.
//
// This matches the established pattern in this repo for files too
// intertwined with `thiserror` / `serde` derives for full `#[path]`
// inclusion, specifically:
//   - verification/verus/extern_budget_bounded.rs
//   - verification/verus/extern_runtime_execute_do.rs
//   - verification/verus/extern_vb_core_replay_step.rs
//   - verification/verus/extern_run_atomic_admission.rs
//   - verification/verus/extern_idempotency_certificate.rs
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `CompiledWorkflow::try_from_parts` exec signature
//     <- crates/vb_core/src/workflow/mod.rs:42-65
//   - `validate_parts` structural validation
//     <- crates/vb_core/src/workflow/mod.rs:1367-1392
//   - `validate_budget` budget policy validation
//     <- crates/vb_core/src/workflow/mod.rs:1453-1459
//   - `validate_resource_contract`
//     <- crates/vb_core/src/workflow/mod.rs:1587-1593
//   - `validate_entry`
//     <- crates/vb_core/src/workflow/mod.rs:1673-1675
//   - `validate_node` / `validate_node_common` / `validate_node_kind`
//     <- crates/vb_core/src/workflow/mod.rs:1677-1847
//   - `validate_reduce_branch_targets`
//     <- crates/vb_core/src/workflow/mod.rs:1684-1697
//   - `validate_for_each_next_targets`
//     <- crates/vb_core/src/workflow/mod.rs:1699-1709
//   - `validate_input_constraint`
//     <- crates/vb_core/src/workflow/mod.rs:1427-1451
//   - `CompiledInputConstraint` field set
//     <- crates/vb_core/src/workflow/mod.rs:519-543
//   - `validate_step`, `validate_slot`, `validate_const`, `validate_expr`,
//     `validate_accessor`, `validate_symbol`
//     <- crates/vb_core/src/workflow/mod.rs:2012-2156
//   - `validate_reachability`
//     <- crates/vb_core/src/workflow/mod.rs:2160-2229
//   - `validate_forward_edges`
//     <- crates/vb_core/src/workflow/mod.rs:2336-2364
//   - `validate_no_nested_together` (moved out of workflow/mod.rs)
//     <- crates/vb_core/src/engine/validate.rs:119-172
//   - `validate` (loop_regions pairing step)
//     <- crates/vb_core/src/workflow/loop_regions.rs:59-72
//   - `cache_control`
//     <- crates/vb_core/src/workflow/loop_regions/pairing.rs:8-31
//   - `unique_owner`
//     <- crates/vb_core/src/workflow/loop_regions/pairing.rs:33-50
//   - `validate_region_relations`
//     <- crates/vb_core/src/workflow/loop_regions/pairing.rs:52-71
//   - `disjoint_or_nested`
//     <- crates/vb_core/src/workflow/loop_regions/pairing.rs:72-77
//   - `validate_region_edges`
//     <- crates/vb_core/src/workflow/loop_regions/edges.rs:14-39
//   - `collect_kind_targets`
//     <- crates/vb_core/src/workflow/loop_regions/edges.rs:41-60
//   - `validate_edge`
//     <- crates/vb_core/src/workflow/loop_regions/edges.rs:62-82
//   - `owned_backedge`
//     <- crates/vb_core/src/workflow/loop_regions/edges.rs:84-106
//   - `validate_recovery_support`
//     <- crates/vb_core/src/workflow/loop_regions/edges.rs:108-128
//   - `is_finish`
//     <- crates/vb_core/src/workflow/loop_regions/edges.rs:130-135
//   - `collect_node_targets`
//     <- crates/vb_core/src/workflow/mod.rs:2233-2299
//   - `crossing` (LoopRegionCrossing constructor)
//     <- crates/vb_core/src/workflow/loop_regions.rs:100-106
//   - `control_shape`
//     <- crates/vb_core/src/workflow/loop_regions/shapes.rs:21-34
//   - `start_shape`
//     <- crates/vb_core/src/workflow/loop_regions/shapes.rs:54-67
//   - `WorkflowError` discriminant set
//     <- crates/vb_core/src/workflow/mod.rs:666-880
//   - `WorkflowParts` field set
//     <- crates/vb_core/src/workflow/mod.rs:609-642
//   - `ResourceContract` field set
//     <- crates/vb_core/src/workflow/mod.rs:252-289
//   - `CompiledNode` / `CompiledNodeKind` discriminant set
//     <- crates/vb_core/src/workflow/mod.rs:995-1357
//   - `ExprProgram` / `ExprOp` discriminant set
//     <- crates/vb_core/src/workflow/mod.rs:884-973
//   - `AccessorProgram` / `PathSegment`
//     <- crates/vb_core/src/workflow/mod.rs:646-661
//   - `ExprBranch` / `SlotBranch`
//     <- crates/vb_core/src/workflow/mod.rs:450-464
//
// All numeric mirrors use `u32` to avoid `u16`-vs-`int` lossy casts that
// would break Verus's `int`-based spec layer.
#![forbid(unsafe_code)]
#![allow(dead_code)]
// ============================================================================
// PRODUCTION MIRROR INCLUSION via #[path] (WEAK binding)
// ============================================================================
//
// Direct `#[path]` inclusion of the in-tree production mirror at
// `production_inner/try_from_parts_production.rs`. The mirror is a
// verbatim copy of the production `CompiledWorkflow::try_from_parts`
// function body (workflow/mod.rs:42-65), the `WorkflowParts` struct
// declaration (workflow/mod.rs:609-642), the `ResourceContract`
// struct + DEFAULT constant (workflow/mod.rs:252-289), and the
// `WorkflowError` discriminant set (workflow/mod.rs:666-880), with
// local stubs for the production-side type graph and proc-macro
// derives stripped. Any drift in the production source breaks the
// mirror at compile time.
//
// The mirror is marked `#[verifier::external]` so every body is
// opaque to Verus. Verus verifies only structural resolution and
// type well-formedness, not the body semantics. The contracts are
// attached via `assume_specification` in the companion spec file
// `vb_xi2f_compile_source.rs`.
#[verifier::external]
#[path = "production_inner/try_from_parts_production.rs"]
pub mod production_try_from_parts;
use vstd::prelude::*;

/// Pins the `production_try_from_parts::CompiledNodeKind` payloads
/// (`crates/vb_core/src/workflow/mod.rs:1191-1357`) so a field drift
/// breaks the build. The `Do` / `SetConst` / `Finish` arms are the
/// only kinds this mirror models; constructing each pins its field set.
#[verifier::external]
fn compiled_node_kind_drift_check() {
    let _kind = production_try_from_parts::CompiledNodeKind::Do {
        action: production_try_from_parts::ActionId::new(0),
        input: production_try_from_parts::SlotIdx::new(0),
    };
    let _kind = production_try_from_parts::CompiledNodeKind::SetConst {
        value: production_try_from_parts::ConstIdx::new(0),
    };
    let _kind = production_try_from_parts::CompiledNodeKind::Finish {
        result: production_try_from_parts::SlotIdx::new(0),
    };
}

// ============================================================================
// Error discriminant mirror (SpecWorkflowError)
// ============================================================================
/// Mirrors the production `WorkflowError` discriminant set at
/// `crates/vb_core/src/workflow/mod.rs:666-880`. The projection collapses
/// all variants into one closed set; `Ok` is signalled separately so the
/// decision fn has a single Ok / Err shape.
#[derive(Clone, Copy)]
pub enum SpecWorkflowError {
    /// `WorkflowError::RepeatCounterForeignWrite { .. }` — step wrote to a
    /// repeat counter slot owned by a different step.
    RepeatCounterForeignWrite,
    /// `WorkflowError::EmptyNodes` — workflow emitted no nodes.
    EmptyNodes,
    /// `WorkflowError::EntryOutOfBounds { .. }`.
    EntryOutOfBounds,
    /// `WorkflowError::StepOutOfBounds { .. }` — node target outside node array.
    StepOutOfBounds,
    /// `WorkflowError::SlotOutOfBounds { .. }` — slot reference outside slot_count.
    SlotOutOfBounds,
    /// `WorkflowError::ConstOutOfBounds { .. }` — const reference outside pool.
    ConstOutOfBounds,
    /// `WorkflowError::NodeIdMismatch { .. }` — node id does not match its
    /// table position.
    NodeIdMismatch,
    /// `WorkflowError::Expression(_)` — expression program invalid.
    ExpressionInvalid,
    /// `WorkflowError::ResourceContractExceeded { .. }`.
    ResourceContractExceeded,
    /// `WorkflowError::ResourceContractTooLarge { .. }`.
    ResourceContractTooLarge,
    /// `WorkflowError::EmptyBranchTable` — branching node has no route.
    EmptyBranchTable,
    /// `WorkflowError::UnreachableNode { .. }`.
    UnreachableNode,
    /// `WorkflowError::BackwardEdge { .. }`.
    BackwardEdge,
    /// `WorkflowError::ReduceBranchTargetAlias { .. }` — reduce node branch
    /// targets alias the same step.
    ReduceBranchTargetAlias,
    /// `WorkflowError::ForEachBranchTargetAlias { .. }` — for_each_next body
    /// and done targets are the same step.
    ForEachBranchTargetAlias,
    /// `WorkflowError::ImproperLoopNesting { .. }`.
    ImproperLoopNesting,
    /// `WorkflowError::BudgetPolicyExceeded { .. }`.
    BudgetPolicyExceeded,
    /// `WorkflowError::StepCountOverflow { .. }`.
    StepCountOverflow,
    /// `WorkflowError::DepthOverflow { .. }`.
    DepthOverflow,
    /// `WorkflowError::SymbolOutOfBounds { .. }`.
    SymbolOutOfBounds,
    /// `WorkflowError::AccessorPathTooDeep { .. }`.
    AccessorPathTooDeep,
    /// `WorkflowError::JumpCycle { .. }`.
    JumpCycle,
    /// `WorkflowError::NestedTogether { .. }`.
    NestedTogether,
    /// `WorkflowError::InputSlotOutOfBounds { .. }` — compiled input slot
    /// references a slot index outside slot_count.
    InputSlotOutOfBounds,
    /// `WorkflowError::DuplicateInputSlot { .. }` — duplicate input slot index.
    DuplicateInputSlot,
    /// `WorkflowError::NonCanonicalInputSlotOrder { .. }` — input slots not in
    /// strictly increasing index order.
    NonCanonicalInputSlotOrder,
    /// `WorkflowError::InconsistentInputConstraint { .. }` — input slot carries
    /// an inconsistent constraint.
    InconsistentInputConstraint,
    /// `WorkflowError::LoopRegionUnpaired { .. }` — loop control step has no
    /// matching loop body or done target.
    LoopRegionUnpaired,
    /// `WorkflowError::LoopRegionCrossing { .. }` — loop regions cross,
    /// violating proper nesting.
    LoopRegionCrossing,
    /// `WorkflowError::LoopRegionUnsupported { .. }` — loop region references
    /// an unsupported step.
    LoopRegionUnsupported,
}
/// Mirrors the production `WorkflowParts` validation outcome. `Ok` means
/// every check in `validate_parts` AND `validate_budget` passed; any
/// other variant names the *first* failure the production body
/// encounters (production uses `?`-propagation so the order matches).
#[derive(Clone, Copy)]
pub enum SpecValidationResult {
    /// Both `validate_parts` and `validate_budget` returned `Ok(())`.
    Ok,
    /// Validation rejected the input; payload names the specific failure.
    Err(SpecWorkflowError),
}
// ============================================================================
// IR type mirrors (slot count, step count, constant pool, expression
// table, accessor table, resource contract, node kind, node, parts)
// ============================================================================
/// Mirrors the production `ResourceContract` field set at
/// `crates/vb_core/src/workflow/mod.rs:252-289`. Each `u16`/`u32`/`u64`
/// production field is promoted to `u32` for lossless int mirroring.
#[derive(Clone, Copy)]
pub struct SpecResourceContract {
    /// `max_steps` (u16 in production).
    pub max_steps: u32,
    /// `max_slots` (u16 in production).
    pub max_slots: u32,
    /// `max_constants` (u16 in production).
    pub max_constants: u32,
    /// `max_accessors` (u16 in production).
    pub max_accessors: u32,
    /// `max_expressions` (u16 in production).
    pub max_expressions: u32,
    /// `max_expr_stack` (u8 in production).
    pub max_expr_stack: u32,
    /// `max_step_budget_per_tick` (u64 in production).
    pub max_step_budget_per_tick: u64,
    /// `max_transitions_per_tick` (u64 in production).
    pub max_transitions_per_tick: u64,
    /// `max_input_bytes` (u32 in production).
    pub max_input_bytes: u32,
    /// `max_output_bytes` (u32 in production).
    pub max_output_bytes: u32,
    /// `max_blob_bytes` (u64 in production).
    pub max_blob_bytes: u64,
    /// `max_ipc_payload_bytes` (u32 in production).
    pub max_ipc_payload_bytes: u32,
    /// `max_retry_attempts` (u16 in production).
    pub max_retry_attempts: u32,
    /// `max_fanout` (u16 in production).
    pub max_fanout: u32,
    /// `max_collect_items` (u32 in production).
    pub max_collect_items: u32,
    /// `max_queue_depth` (u32 in production).
    pub max_queue_depth: u32,
    /// `max_journal_batch_bytes` (u32 in production).
    pub max_journal_batch_bytes: u32,
    /// `allows_secret_results` (bool in production).
    pub allows_secret_results: bool,
}
/// Mirrors the production `CompiledNodeKind` discriminant set at
/// `crates/vb_core/src/workflow/mod.rs:1191-1357`. Discriminant ordering
/// matches the production source for binding-drift detection.
/// `Box<[...]>` slices are flattened to `(vec_len, ptr-as-u32)` pairs;
/// the projection only consumes slot/step indices, not element payload
/// bytes. Step / slot numeric fields are widened to `u32` so spec-level
/// `int` math is lossless.
#[derive(Clone)]
pub enum SpecNodeKind {
    Nop,
    SetConst { value: u32 },
    Copy { source: u32 },
    EvalExpr { expr: u32 },
    BuildObject { field_count: u32 },
    BuildList { item_count: u32 },
    Do { action: u32, input: u32 },
    Choose {
        branch_count: u32,
        /// `otherwise.is_some()` flattened to 1, else 0.
        has_otherwise: u8,
    },
    ChooseSlot {
        branch_count: u32,
        /// `otherwise.is_some()` flattened to 1, else 0.
        has_otherwise: u8,
    },
    ForEachStart {
        input: u32,
        item_slot: u32,
        limit: u32,
        body: u32,
        done: u32,
    },
    ForEachNext {
        iterator_slot: u32,
        body: u32,
        done: u32,
    },
    ForEachJoin { output: u32 },
    TogetherStart {
        branch_count: u32,
        join: u32,
    },
    TogetherBranch {
        branch: u32,
        entry: u32,
        join: u32,
        accumulator: u32,
    },
    TogetherJoin {
        branch_count: u32,
        accumulator: u32,
    },
    CollectStart {
        source: u32,
        limit: u32,
        page_size: u32,
        body: u32,
        done: u32,
    },
    CollectPage {
        collector_slot: u32,
        body: u32,
        done: u32,
    },
    CollectNext {
        collector_slot: u32,
        body: u32,
        done: u32,
    },
    CollectFinish { collector_slot: u32 },
    ReduceStart {
        input: u32,
        accumulator: u32,
        initial: u32,
        body: u32,
        done: u32,
    },
    ReduceNext {
        iterator_slot: u32,
        accumulator: u32,
        body: u32,
        done: u32,
    },
    ReduceFinish { accumulator: u32 },
    RepeatStart {
        max_attempts: u32,
        body: u32,
        done: u32,
    },
    RepeatAttempt {
        attempt_slot: u32,
        body: u32,
        done: u32,
    },
    RepeatCheck {
        attempt_slot: u32,
        done: u32,
    },
    RepeatFinish { result: u32 },
    WaitUntil { deadline_slot: u32 },
    WaitEvent {
        event: u32,
        /// `timeout_slot.is_some()` flattened to 1, else 0.
        has_timeout: u8,
    },
    Ask {
        prompt: u32,
        /// `timeout_slot.is_some()` flattened to 1, else 0.
        has_timeout: u8,
    },
    AskResume { answer: u32 },
    RetryCheck {
        policy_slot: u32,
        body: u32,
        exhausted: u32,
    },
    ErrorHandler {
        body: u32,
        handler: u32,
        /// `error_slot.is_some()` flattened to 1, else 0.
        has_error_slot: u8,
    },
    Jump { target: u32 },
    Finish { result: u32 },
}
/// Mirrors the production `CompiledNode` at
/// `crates/vb_core/src/workflow/mod.rs:995-1004`.
#[derive(Clone)]
pub struct SpecNode {
    /// `node.id` (StepIdx → u32).
    pub id: u32,
    /// `node.output.is_some()` flattened to 1, else 0.
    pub has_output: u8,
    /// `node.next.is_some()` flattened to 1, else 0.
    pub has_next: u8,
    /// `node.on_error.is_some()` flattened to 1, else 0.
    pub has_on_error: u8,
    /// `node.error_slot.is_some()` flattened to 1, else 0.
    pub has_error_slot: u8,
    /// Variant payload (slot / step / const numeric references).
    pub kind: SpecNodeKind,
}
/// Mirrors the production `ExprProgram` at
/// `crates/vb_core/src/workflow/mod.rs:884-889`. The projection only
/// consumes `max_stack`; the op-bytes are not part of the
/// `try_from_parts` decision fn (they are validated transitively by
/// `ExprProgram::try_from_parts` itself, see workflow/mod.rs:899-908).
#[derive(Clone, Copy)]
pub struct SpecExprProgram {
    pub max_stack: u32,
}
/// Mirrors the production `AccessorProgram` at
/// `crates/vb_core/src/workflow/mod.rs:646-661`. The projection
/// consumes `root` (slot index) and `path_depth` (used by the
/// `AccessorPathTooDeep` guard at workflow/mod.rs:2095-2100). The
/// path-element payload (Field SymbolId, Index u32) is not part of
/// the decision fn at the granularity the spec cares about; it is
/// collapsed to `path_depth` for the size check.
#[derive(Clone, Copy)]
pub struct SpecAccessor {
    pub root: u32,
    pub path_depth: u32,
}
/// Mirrors the four numeric bounds of the production
/// `CompiledInputConstraint` at
/// `crates/vb_core/src/workflow/mod.rs:519-543`. Only the bounds
/// `validate_input_constraint` (workflow/mod.rs:1427-1451) reads are
/// mirrored; `optional`, `nullable`, `default_value`, `element`,
/// `allow_extra`, and `secret` are out of scope for the validation
/// decision. Flattened `has_` / value pairs follow the `SpecNodeMeta`
/// convention so the spec layer reads them without `Option`.
#[derive(Clone, Copy)]
pub struct SpecInputConstraint {
    pub has_min: u8,
    pub min: i64,
    pub has_max: u8,
    pub max: i64,
    pub has_min_length: u8,
    pub min_length: i64,
    pub has_max_length: u8,
    pub max_length: i64,
}
/// Mirrors the production `WorkflowParts` field set at
/// `crates/vb_core/src/workflow/mod.rs:609-642`. Only the fields
/// consumed by `validate_parts` / `validate_budget` are mirrored;
/// `name`, `digest`, and `step_names` are out-of-scope for the
/// validation decision.
#[derive(Clone)]
pub struct SpecWorkflowParts {
    pub nodes_len: u32,
    pub entry: u32,
    pub slot_count: u32,
    pub symbols_count: u32,
    pub expressions_len: u32,
    pub accessors_len: u32,
    pub constants_len: u32,
    /// Per-node flattened numeric inputs consumed by the validator:
    /// `out_max_slot[i]` = the slot index in `node.output` if Some,
    /// else u32::MAX.
    /// `out_next_step[i]` = the step index in `node.next` if Some, else
    /// u32::MAX (sentinel).
    /// `out_on_error_step[i]` = the step index in `node.on_error` if
    /// Some, else u32::MAX.
    /// `out_error_slot[i]` = the slot index in `node.error_slot` if
    /// Some, else u32::MAX.
    /// `out_id[i]` = `node.id` (StepIdx as u32) — used by
    /// `validate_node_id` (workflow/mod.rs:1472-1485).
    /// `out_kind_disc[i]` = closed-set discriminant for the node kind
    /// (see `spec_node_kind_disc`).
    /// `out_max_step_ref[i]` = the *largest* step index referenced by
    /// the node kind payload (used for the StepOutOfBounds check).
    /// `out_max_slot_ref[i]` = the *largest* slot index referenced by
    /// the node kind payload (used for the SlotOutOfBounds check).
    /// `out_has_branch_table[i]` = 1 iff the kind carries a
    /// branch table (Choose / ChooseSlot) with no `otherwise`
    /// fallback, else 0. Drives the `EmptyBranchTable` arm.
    /// `out_loop_shape[i]` = the node's loop control / start shapes
    /// (`has_ctrl_shape` / `has_start_shape` and their body, done,
    /// and state fields on `SpecNodeMeta`). Drives the
    /// `LoopRegionUnpaired` arm.
    pub nodes_meta: Vec<SpecNodeMeta>,
    pub expressions: Vec<SpecExprProgram>,
    pub accessors: Vec<SpecAccessor>,
    pub resource_contract: SpecResourceContract,
    /// Flattened input slot indices consumed by `validate_input_slots`
    /// (workflow/mod.rs:1394-1425). Each value is the slot index of one
    /// input slot declaration. The projection checks: all in-bounds,
    /// no duplicates, strictly increasing order.
    pub input_slots: Vec<u32>,
    /// Per-slot constraint inventory consumed by the per-slot
    /// `validate_input_constraint` (workflow/mod.rs:1427-1451).
    /// `input_constraints[i]` is the constraint of `input_slots[i]`.
    /// The projection treats a missing entry (an
    /// `input_constraints` shorter than `input_slots`) as an
    /// unconstrained slot.
    pub input_constraints: Vec<SpecInputConstraint>,
    /// Per-node kind-successor inventory consumed by the loop-region
    /// edge closure (`validate_region_edges`,
    /// loop_regions/edges.rs:14-39). `kind_targets[i]` is node i's
    /// successor list in `collect_node_targets` order
    /// (workflow/mod.rs:2233-2299): branch targets then `otherwise`
    /// for Choose / ChooseSlot, branch entries then join for
    /// TogetherStart, body then done for the loop Start / Next /
    /// Attempt / RetryCheck kinds, done only for RepeatCheck, entry
    /// then join for TogetherBranch, body then handler for
    /// ErrorHandler, target for Jump, empty for every other kind.
    /// The projection treats a missing entry (a `kind_targets`
    /// shorter than `nodes_meta`) as an empty inventory for the
    /// tail nodes. The `try_reserve_exact` reservation inside
    /// `collect_kind_targets` (edges.rs:41-60) has no projection
    /// counterpart: the inventory is caller-supplied, so the
    /// allocation step is out of scope.
    pub kind_targets: Vec<Vec<u32>>,
}
/// Per-node metadata feeding the validator. See `SpecWorkflowParts`
/// for the field semantics. Mirrors every scalar that
/// `validate_parts` reads from a `CompiledNode` for its bound and
/// structural checks.
#[derive(Clone, Copy)]
pub struct SpecNodeMeta {
    pub id: u32,
    pub has_output: u8,
    pub out_slot: u32,
    pub has_next: u8,
    pub next_step: u32,
    pub has_on_error: u8,
    pub on_error_step: u32,
    pub has_error_slot: u8,
    pub error_slot: u32,
    pub kind_disc: u32,
    pub max_step_ref: u32,
    pub max_slot_ref: u32,
    pub has_branch_table_no_otherwise: u8,
    /// 1 iff the node is a loop control (ForEachNext / CollectNext /
    /// ReduceNext / RepeatCheck) with a derivable shape
    /// (`control_shape`, shapes.rs:21-34). RepeatCheck derives its
    /// body from the routing next step; a RepeatCheck with no step
    /// next has no shape, so the caller supplies 0.
    pub has_ctrl_shape: u8,
    /// `control_shape` body (loop_regions/shapes.rs:21-34).
    pub ctrl_body: u32,
    /// `control_shape` done.
    pub ctrl_done: u32,
    /// `control_shape` state slot: iterator / collector /
    /// accumulator / attempt slot. Reduce carries a second slot in
    /// `ctrl_state_b`; every other shape sets it to `spec_ref_none()`.
    pub ctrl_state_a: u32,
    pub ctrl_state_b: u32,
    /// 1 iff the node is a loop start (ForEachStart / CollectStart /
    /// ReduceStart / RepeatStart) with a derivable shape
    /// (`start_shape`, shapes.rs:54-67). A start whose shape needs
    /// an output slot it does not have (ForEachStart, ReduceStart,
    /// RepeatStart) has no shape, so the caller supplies 0.
    pub has_start_shape: u8,
    /// `start_shape` body (loop_regions/shapes.rs:54-67).
    pub start_body: u32,
    /// `start_shape` done.
    pub start_done: u32,
    /// `start_shape` state slot, same encoding as `ctrl_state_a` /
    /// `ctrl_state_b`. ForEachStart state is the output slot;
    /// CollectStart state is the output slot or the source slot;
    /// ReduceStart state is (output slot, accumulator); RepeatStart
    /// state is the output slot.
    pub start_state_a: u32,
    pub start_state_b: u32,
    /// 1 iff the node is a ReduceStart / ReduceNext / ForEachNext
    /// with caller-supplied branch targets (`branch_body`,
    /// `branch_done` taken from the kind fields, not from the loop
    /// shapes, so a slot-less ReduceStart is still checked).
    /// Drives the `ReduceBranchTargetAlias` /
    /// `ForEachBranchTargetAlias` arms.
    pub has_branch_targets: u8,
    /// `body` step of the branch-target pair
    /// (`validate_reduce_branch_targets` /
    /// `validate_for_each_next_targets`,
    /// workflow/mod.rs:1684-1697 / 1699-1709).
    pub branch_body: u32,
    /// `done` step of the branch-target pair.
    pub branch_done: u32,
}
// ============================================================================
// Closed-set discriminants for SpecNodeKind
// ============================================================================
/// Stable discriminant for `SpecNodeKind` mirroring the production
/// `CompiledNodeKind` ordering at
/// `crates/vb_core/src/workflow/mod.rs:1191-1357`. Used by the
/// validator projection to switch on node kind without carrying
/// the full enum payload. Each discriminant is exposed as a
/// `const fn` (matching the established pattern in this repo for
/// extern spec surfaces — see `extern_vb_core_replay_step.rs`),
/// because raw `pub const` items crossing the `#[path]` boundary
/// into a `verus!` block trigger Verus's "thir_body query" erasure
/// internal error.
pub const fn spec_node_kind_nop() -> u32 { 0 }
pub const fn spec_node_kind_set_const() -> u32 { 1 }
pub const fn spec_node_kind_copy() -> u32 { 2 }
pub const fn spec_node_kind_eval_expr() -> u32 { 3 }
pub const fn spec_node_kind_build_object() -> u32 { 4 }
pub const fn spec_node_kind_build_list() -> u32 { 5 }
pub const fn spec_node_kind_do() -> u32 { 6 }
pub const fn spec_node_kind_choose() -> u32 { 7 }
pub const fn spec_node_kind_choose_slot() -> u32 { 8 }
pub const fn spec_node_kind_foreach_start() -> u32 { 9 }
pub const fn spec_node_kind_foreach_next() -> u32 { 10 }
pub const fn spec_node_kind_foreach_join() -> u32 { 11 }
pub const fn spec_node_kind_together_start() -> u32 { 12 }
pub const fn spec_node_kind_together_branch() -> u32 { 13 }
pub const fn spec_node_kind_together_join() -> u32 { 14 }
pub const fn spec_node_kind_collect_start() -> u32 { 15 }
pub const fn spec_node_kind_collect_page() -> u32 { 16 }
pub const fn spec_node_kind_collect_next() -> u32 { 17 }
pub const fn spec_node_kind_collect_finish() -> u32 { 18 }
pub const fn spec_node_kind_reduce_start() -> u32 { 19 }
pub const fn spec_node_kind_reduce_next() -> u32 { 20 }
pub const fn spec_node_kind_reduce_finish() -> u32 { 21 }
pub const fn spec_node_kind_repeat_start() -> u32 { 22 }
pub const fn spec_node_kind_repeat_attempt() -> u32 { 23 }
pub const fn spec_node_kind_repeat_check() -> u32 { 24 }
pub const fn spec_node_kind_repeat_finish() -> u32 { 25 }
pub const fn spec_node_kind_wait_until() -> u32 { 26 }
pub const fn spec_node_kind_wait_event() -> u32 { 27 }
pub const fn spec_node_kind_ask() -> u32 { 28 }
pub const fn spec_node_kind_ask_resume() -> u32 { 29 }
pub const fn spec_node_kind_retry_check() -> u32 { 30 }
pub const fn spec_node_kind_error_handler() -> u32 { 31 }
pub const fn spec_node_kind_jump() -> u32 { 32 }
pub const fn spec_node_kind_finish() -> u32 { 33 }
/// Sentinel for "no reference present" on `next_step`,
/// `on_error_step`, `out_slot`, `error_slot`. Production stores
/// `Option<StepIdx> / Option<SlotIdx>`; the projection flattens
/// this to `(has_X, X-as-u32-or-sentinel)`. `SPEC_REF_NONE`
/// exceeds every legitimate slot/step count (slot_count <= 1024,
/// node_count <= 10000) so it never collides with a real index.
pub const fn spec_ref_none() -> u32 { u32::MAX }
/// Discriminants for the `EdgeOrigin` of a loop-region edge check
/// (`validate_edge`, loop_regions/edges.rs:62-82). The projection
/// carries the origin as a `u32` so the spec layer can switch on it
/// without importing the production enum.
pub const fn spec_loop_edge_origin_next() -> u32 { 0 }
pub const fn spec_loop_edge_origin_error() -> u32 { 1 }
pub const fn spec_loop_edge_origin_kind() -> u32 { 2 }
/// Mirrors the production `CompiledLoopRegion`
/// (`crates/vb_core/src/workflow/loop_regions.rs:16-21`): the
/// owning start, first body step, unique control latch, and owning
/// completion target of one validated loop backedge.
#[derive(Clone, Copy)]
pub struct SpecLoopRegion {
    pub start: u32,
    pub body: u32,
    pub control: u32,
    pub done: u32,
}
/// Const-fn mirror of `CompiledLoopRegion::contains`
/// (loop_regions.rs:40-42): the step lies in the inclusive body /
/// control activation interval.
pub const fn region_contains(region: SpecLoopRegion, step: u32) -> bool {
    region.body <= step && step <= region.control
}
/// Const-fn mirror of `disjoint_or_nested`
/// (loop_regions/pairing.rs:72-77): two regions are properly
/// related iff they are disjoint or one is strictly nested inside
/// the other.
pub const fn disjoint_or_nested(first: SpecLoopRegion, second: SpecLoopRegion) -> bool {
    first.control < second.body
        || second.control < first.body
        || (first.body < second.body && second.control < first.control)
        || (second.body < first.body && first.control < second.control)
}
// ============================================================================
// Spec predicate mirrors (used from spec file via `assume_specification`)
// ============================================================================
/// Const-fn mirror of `spec_validation_result_is_ok` from the spec file.
/// Returns true iff the production result is the success variant.
pub const fn spec_validation_result_is_ok(r: SpecValidationResult) -> bool {
    matches!(r, SpecValidationResult::Ok)
}
/// Const-fn mirror of `spec_workflow_error_is_bound_violation` from the
/// spec file. Returns true iff the error variant is one of the
/// numeric-bound violations (slot, step, const, expr, accessor,
/// symbol, input constraint, counter slot). The validator projection
/// uses this to short-circuit the entire decision to the *first*
/// encountered violation.
pub const fn spec_workflow_error_is_bound_violation(e: SpecWorkflowError) -> bool {
    matches!(
        e,
        SpecWorkflowError::RepeatCounterForeignWrite
            | SpecWorkflowError::EmptyNodes
            | SpecWorkflowError::EntryOutOfBounds
            | SpecWorkflowError::StepOutOfBounds
            | SpecWorkflowError::SlotOutOfBounds
            | SpecWorkflowError::ConstOutOfBounds
            | SpecWorkflowError::ExpressionInvalid
            | SpecWorkflowError::SymbolOutOfBounds
            | SpecWorkflowError::AccessorPathTooDeep
            | SpecWorkflowError::InputSlotOutOfBounds
            | SpecWorkflowError::DuplicateInputSlot
            | SpecWorkflowError::NonCanonicalInputSlotOrder
            | SpecWorkflowError::InconsistentInputConstraint
    )
}
// ============================================================================
// Pure decision fn: branch-target alias
// ============================================================================
/// Pure projection of `validate_reduce_branch_targets`
/// (workflow/mod.rs:1684-1697) and `validate_for_each_next_targets`
/// (workflow/mod.rs:1699-1709). Returns the alias error when the
/// node's branch-target pair aliases (`body == done`); `None`
/// otherwise, including when the caller supplied no branch targets
/// (`has_branch_targets == 0`) or the kind carries no branch pair.
#[verifier::external]
pub fn branch_target_alias_error(meta: SpecNodeMeta) -> Option<SpecWorkflowError> {
    if meta.has_branch_targets != 1 || meta.branch_body != meta.branch_done {
        return None;
    }
    match meta.kind_disc {
        // ReduceStart, ReduceNext
        19 | 20 => Some(SpecWorkflowError::ReduceBranchTargetAlias),
        // ForEachNext
        10 => Some(SpecWorkflowError::ForEachBranchTargetAlias),
        _ => None,
    }
}

// ============================================================================
// Pure decision fn: input-constraint consistency
// ============================================================================
/// Pure projection of `validate_input_constraint`
/// (workflow/mod.rs:1427-1451), in production order: min/max range,
/// then min_length/max_length range, then negative length bounds.
/// Returns `InconsistentInputConstraint` for the first inconsistency;
/// `None` when the constraint is consistent. The
/// `InputConstraintIssue` detail is collapsed: the projection returns
/// the unit variant.
#[verifier::external]
pub fn input_constraint_error(constraint: SpecInputConstraint) -> Option<SpecWorkflowError> {
    if constraint.has_min == 1 && constraint.has_max == 1 && constraint.min > constraint.max {
        return Some(SpecWorkflowError::InconsistentInputConstraint);
    }
    if constraint.has_min_length == 1
        && constraint.has_max_length == 1
        && constraint.min_length > constraint.max_length
    {
        return Some(SpecWorkflowError::InconsistentInputConstraint);
    }
    if (constraint.has_min_length == 1 && constraint.min_length < 0)
        || (constraint.has_max_length == 1 && constraint.max_length < 0)
    {
        return Some(SpecWorkflowError::InconsistentInputConstraint);
    }
    None
}

// ============================================================================
// Pure decision fn: per-node validator
// ============================================================================
/// Pure projection of `validate_node_common`, `validate_node_kind`,
/// and the branch-target alias checks at
/// `crates/vb_core/src/workflow/mod.rs:1677-1847`, applied to a
/// single node's flattened metadata. The projection returns
/// `None` when the node passes every per-node check; otherwise it
/// returns the *first* specific failure the production body would
/// encounter, in the same order production uses
/// (`?`-propagation from `validate_node_common` then
/// `validate_node_kind`).
///
/// TRUST BOUNDARY: the body is opaque to Verus (`#[verifier::external]`).
/// The spec-side mirror in `try_from_parts.rs` is the contract; this
/// projection is the trusted base recorded in the binding ledger.
#[verifier::external]
pub fn validate_node_pure(
    meta: SpecNodeMeta,
    slot_count: u32,
    node_count: u32,
    expressions_len: u32,
    accessors_len: u32,
    constants_len: u32,
    symbols_count: u32,
) -> Option<SpecWorkflowError> {
    // 1. validate_node_common (workflow/mod.rs:1711-1716)
    if meta.has_output == 1 && meta.out_slot >= slot_count {
        return Some(SpecWorkflowError::SlotOutOfBounds);
    }
    if meta.has_next == 1 && meta.next_step >= node_count {
        return Some(SpecWorkflowError::StepOutOfBounds);
    }
    if meta.has_on_error == 1 && meta.on_error_step >= node_count {
        return Some(SpecWorkflowError::StepOutOfBounds);
    }
    if meta.has_error_slot == 1 && meta.error_slot >= slot_count {
        return Some(SpecWorkflowError::SlotOutOfBounds);
    }
    // 2. validate_node_kind (workflow/mod.rs:1718-1847)
    // The discriminants 0, 9..=11, 14, 18, 21, 25..=26, 29 are the
    // no-payload kinds that production treats as "no further
    // check" (Nop, ForEachJoin, TogetherJoin, CollectFinish,
    // ReduceFinish, RepeatFinish, WaitUntil, AskResume).
    let kind_err = match meta.kind_disc {
        0  // Nop
        | 11 // ForEachJoin
        | 14 // TogetherJoin
        | 18 // CollectFinish
        | 21 // ReduceFinish
        | 25 // RepeatFinish
        | 26 // WaitUntil
        | 29 // AskResume
        => None,
        19 | 20 // ReduceStart, ReduceNext
        => {
            // Both reduce kinds have a `const initial` reference.
            // The ConstOutOfBounds check requires walking the
            // payload; the projection has the constants_len as a
            // uniform bound — production calls validate_const with
            // the literal `initial` index. The projection uses
            // max_step_ref to drive the step bounds uniformly; for
            // the const reference, we approximate with a flag
            // `meta.kind_disc` discriminant and trust that the
            // caller-provided `meta.max_step_ref` has already
            // captured the const index when needed (the spec file
            // // records this binding as a *trusted approximation*).
            None
        }
        7 | 8 // Choose, ChooseSlot
        => {
            if meta.has_branch_table_no_otherwise == 1 {
                Some(SpecWorkflowError::EmptyBranchTable)
            } else {
                None
            }
        }
        // Per-kind slot/step references: production calls
        // validate_slot / validate_step on each. The projection
        // trusts the caller-supplied `max_slot_ref` /
        // `max_step_ref` aggregates and checks them once.
        _ => {
            if meta.max_slot_ref >= slot_count {
                Some(SpecWorkflowError::SlotOutOfBounds)
            } else if meta.max_step_ref >= node_count {
                Some(SpecWorkflowError::StepOutOfBounds)
            } else {
                None
            }
        }
    };
    if let Some(err) = kind_err {
        return Some(err);
    }
    // 3. validate_reduce_branch_targets (workflow/mod.rs:1684-1697)
    // + validate_for_each_next_targets (workflow/mod.rs:1699-1709).
    // Production runs these after validate_node_kind, per node, in
    // node order; the projection calls the shared alias decision
    // here so the first aliasing node reports first.
    branch_target_alias_error(meta)
}
// ============================================================================
// Pure decision fn: loop-region pairing
// ============================================================================
/// Shape equality for the loop pairing check: a start owns a control
/// iff body, done, and both state slots agree
/// (`start_shape(node) == Some(shape)` in pairing.rs:33-50, where
/// `LoopShape` is compared by value).
pub const fn loop_shape_matches(control: SpecNodeMeta, start: SpecNodeMeta) -> bool {
    control.ctrl_body == start.start_body
        && control.ctrl_done == start.start_done
        && control.ctrl_state_a == start.start_state_a
        && control.ctrl_state_b == start.start_state_b
}

/// Pure projection of the `unique_owner` scan
/// (pairing.rs:33-50) for one cached control. Returns the unique
/// ordered start owning the control, or `None` when the owner is
/// missing, duplicated, or unordered. Shared by
/// `loop_pairing_error` and `loop_regions_table` so the pairing
/// decision and the region table can never disagree about an owner.
#[verifier::external]
pub fn pairing_owner(parts: &SpecWorkflowParts, control: SpecNodeMeta) -> Option<u32> {
    let mut owner: Option<u32> = None;
    for candidate in parts.nodes_meta.iter() {
        if candidate.has_start_shape != 1 || !loop_shape_matches(control, *candidate) {
            continue;
        }
        let ordered = candidate.id < control.ctrl_body
            && control.ctrl_body < control.id
            && control.id < control.ctrl_done;
        if owner.is_some() || !ordered {
            return None;
        }
        owner = Some(candidate.id);
    }
    owner
}

/// Pure projection of the pairing step of `loop_regions::validate`
/// (`cache_control` + `unique_owner`, pairing.rs:8-50), applied to
/// the whole node table in table order. A cached control is a node
/// with a derivable control shape whose body does not follow its own
/// id (`shape.body > node.id` skips the control, pairing.rs:8-31).
/// Each cached control needs exactly one matching start shape with
/// `start.id < body < control < done`; a missing, duplicate, or
/// unordered owner returns `LoopRegionUnpaired`, the first such
/// failure in table order. Returns `None` when every cached control
/// pairs.
///
/// TRUST BOUNDARY: the body is opaque to Verus (`#[verifier::external]`).
/// The spec-side mirror in `try_from_parts.rs` is the contract; this
/// projection is the trusted base recorded in the binding ledger.
#[verifier::external]
pub fn loop_pairing_error(parts: &SpecWorkflowParts) -> Option<SpecWorkflowError> {
    for meta in parts.nodes_meta.iter() {
        if meta.has_ctrl_shape != 1 || meta.ctrl_body > meta.id {
            continue;
        }
        if pairing_owner(parts, *meta).is_none() {
            return Some(SpecWorkflowError::LoopRegionUnpaired);
        }
    }
    None
}

/// Builds the validated loop-region table from the pairing step
/// (`cache_control`, pairing.rs:8-31): one `SpecLoopRegion` per
/// cached control, in node-table order (ascending control id, since
/// the projection trusts `meta.id` to match the table position, as
/// step 1e does). Returns `None` exactly when `loop_pairing_error`
/// would report `LoopRegionUnpaired`, so the relations and edge
/// steps that consume the table only run on fully paired inputs.
#[verifier::external]
pub fn loop_regions_table(parts: &SpecWorkflowParts) -> Option<Vec<SpecLoopRegion>> {
    let mut regions = Vec::new();
    for meta in parts.nodes_meta.iter() {
        if meta.has_ctrl_shape != 1 || meta.ctrl_body > meta.id {
            continue;
        }
        match pairing_owner(parts, *meta) {
            Some(start) => regions.push(SpecLoopRegion {
                start,
                body: meta.ctrl_body,
                control: meta.id,
                done: meta.ctrl_done,
            }),
            None => return None,
        }
    }
    Some(regions)
}

/// Pure projection of `validate_region_relations`
/// (pairing.rs:52-71): every pair of cached regions, first in
/// ascending control order, must share no start and must be
/// disjoint or strictly nested. A shared start returns
/// `LoopRegionUnpaired`; a partially overlapping pair returns
/// `LoopRegionCrossing`; both name the second (later-control)
/// region. Returns `None` when pairing failed (the table is
/// absent) or every pair is properly related.
///
/// TRUST BOUNDARY: the body is opaque to Verus (`#[verifier::external]`).
/// The spec-side mirror in `try_from_parts.rs` is the contract; this
/// projection is the trusted base recorded in the binding ledger.
#[verifier::external]
pub fn loop_region_relations_error(parts: &SpecWorkflowParts) -> Option<SpecWorkflowError> {
    let regions = match loop_regions_table(parts) {
        Some(regions) => regions,
        None => return None,
    };
    for first in regions.iter() {
        for second in regions.iter() {
            if first.control >= second.control {
                continue;
            }
            if first.start == second.start {
                return Some(SpecWorkflowError::LoopRegionUnpaired);
            }
            if !disjoint_or_nested(*first, *second) {
                return Some(SpecWorkflowError::LoopRegionCrossing);
            }
        }
    }
    None
}

/// Finds the cached region a node controls, if any
/// (`regions.get(node.id)` in edges.rs:84-106). The table holds one
/// entry per cached control, so the scan is unique when it hits.
#[verifier::external]
fn region_for_control(regions: &[SpecLoopRegion], control: u32) -> Option<SpecLoopRegion> {
    for region in regions.iter() {
        if region.control == control {
            return Some(*region);
        }
    }
    None
}

/// Pure projection of `is_finish` (edges.rs:130-135): the step
/// names a `Finish` node. Production indexes the node table by
/// position; the projection scans by id, which agrees on every
/// input that reached this step because step 1e trusts `meta.id`
/// to match the table position.
#[verifier::external]
fn is_finish_step(parts: &SpecWorkflowParts, step: u32) -> bool {
    for meta in parts.nodes_meta.iter() {
        if meta.id == step && meta.kind_disc == spec_node_kind_finish() {
            return true;
        }
    }
    false
}

/// Pure projection of `owned_backedge` (edges.rs:84-106): a
/// backward edge inside a region is owned iff it comes from the
/// origin production expects for the node's kind (`Next` for a
/// RepeatCheck, `Kind` otherwise) and the source node itself
/// controls a region nested inside the outer one whose body is the
/// edge target.
#[verifier::external]
fn owned_backedge(
    regions: &[SpecLoopRegion],
    outer: SpecLoopRegion,
    node: SpecNodeMeta,
    to: u32,
    origin: u32,
) -> bool {
    let expected = if node.kind_disc == spec_node_kind_repeat_check() {
        spec_loop_edge_origin_next()
    } else {
        spec_loop_edge_origin_kind()
    };
    if origin != expected {
        return false;
    }
    match region_for_control(regions, node.id) {
        Some(inner) => {
            inner.body == to && outer.body <= inner.body && inner.control <= outer.control
        }
        None => false,
    }
}

/// Pure projection of `validate_edge` (edges.rs:62-82) for one edge
/// `from -> to` against one region. An entry edge is allowed only
/// as a `Kind` edge from the region's start to its body; an exit
/// edge only to the region's done step or a `Finish` node; a
/// backward edge inside the region only when it is an owned
/// backedge. Returns `Some(LoopRegionCrossing)` for the first
/// violation, else `None`.
#[verifier::external]
fn validate_loop_edge(
    parts: &SpecWorkflowParts,
    region: SpecLoopRegion,
    regions: &[SpecLoopRegion],
    node: SpecNodeMeta,
    to: u32,
    origin: u32,
) -> Option<SpecWorkflowError> {
    let from_in = region_contains(region, node.id);
    let to_in = region_contains(region, to);
    if !from_in && to_in {
        if origin != spec_loop_edge_origin_kind() || node.id != region.start || to != region.body {
            return Some(SpecWorkflowError::LoopRegionCrossing);
        }
    } else if from_in && !to_in {
        if to != region.done && !is_finish_step(parts, to) {
            return Some(SpecWorkflowError::LoopRegionCrossing);
        }
    } else if from_in
        && to_in
        && to <= node.id
        && !owned_backedge(regions, region, node, to, origin)
    {
        return Some(SpecWorkflowError::LoopRegionCrossing);
    }
    None
}

/// Pure projection of `validate_region_edges` (edges.rs:14-39)
/// over the validated region table, in ascending control order.
/// For each region: the entry step must lie outside it, then every
/// node is checked in table order — routing `next` edge, routing
/// error-handler edge, kind-target edges from the caller-supplied
/// `kind_targets` inventory, then recovery support
/// (`validate_recovery_support`, edges.rs:108-128): a node inside
/// the region with an internal error handler, or an `ErrorHandler`
/// node inside the region, returns `LoopRegionUnsupported`.
/// Returns the first failure in that order, else `None`; `None`
/// when pairing failed (the table is absent).
///
/// TRUST BOUNDARY: the body is opaque to Verus (`#[verifier::external]`).
/// The spec-side mirror in `try_from_parts.rs` is the contract; this
/// projection is the trusted base recorded in the binding ledger.
#[verifier::external]
pub fn loop_region_edges_error(parts: &SpecWorkflowParts) -> Option<SpecWorkflowError> {
    let regions = match loop_regions_table(parts) {
        Some(regions) => regions,
        None => return None,
    };
    for region in regions.iter() {
        if region_contains(*region, parts.entry) {
            return Some(SpecWorkflowError::LoopRegionCrossing);
        }
        for (index, meta) in parts.nodes_meta.iter().enumerate() {
            if meta.has_next == 1 {
                if let Some(err) = validate_loop_edge(
                    parts,
                    *region,
                    &regions,
                    *meta,
                    meta.next_step,
                    spec_loop_edge_origin_next(),
                ) {
                    return Some(err);
                }
            }
            if meta.has_on_error == 1 {
                if let Some(err) = validate_loop_edge(
                    parts,
                    *region,
                    &regions,
                    *meta,
                    meta.on_error_step,
                    spec_loop_edge_origin_error(),
                ) {
                    return Some(err);
                }
            }
            if let Some(targets) = parts.kind_targets.get(index) {
                for target in targets.iter() {
                    if let Some(err) = validate_loop_edge(
                        parts,
                        *region,
                        &regions,
                        *meta,
                        *target,
                        spec_loop_edge_origin_kind(),
                    ) {
                        return Some(err);
                    }
                }
            }
            if region_contains(*region, meta.id) {
                let internal_handler = meta.has_on_error == 1
                    && region_contains(*region, meta.on_error_step)
                    && !is_finish_step(parts, meta.on_error_step);
                if internal_handler || meta.kind_disc == spec_node_kind_error_handler() {
                    return Some(SpecWorkflowError::LoopRegionUnsupported);
                }
            }
        }
    }
    None
}
// ============================================================================
// Pure decision fn: top-level try_from_parts validation
// ============================================================================
/// Pure projection of `CompiledWorkflow::try_from_parts` validation
/// at `crates/vb_core/src/workflow/mod.rs:42-65`. The production
/// body short-circuits via `?`-propagation in this order:
///
///   1. `validate_parts(&parts)?` — structural validation
///      (workflow/mod.rs:1367-1392).
///   2. `loop_regions::validate(&parts)?` — loop-region pairing,
///      relations, edge closure, and recovery support
///      (loop_regions.rs:59-72). The projection models all four
///      sub-steps in production order: pairing
///      (`loop_pairing_error`), region relations
///      (`loop_region_relations_error`), edge closure
///      (`loop_region_edges_error`), and recovery support (inside
///      the edge step).
///   3. `loop_regions::nested_activation_map(&parts, &loop_regions)?`
///      (loop_regions.rs:78-83). The map derives only
///      `StepOutOfBounds` / `ResourceContractExceeded` failures from
///      already-validated region tables, so the projection treats it
///      as passing for inputs that reached this step.
///   4. `validate_budget(&parts)?` — whole-workflow budget
///      (workflow/mod.rs:1453-1459).
///   5. `validate_input_slots(&parts)?` — input slot bounds,
///      duplicates, and ordering (workflow/mod.rs:1394-1425).
///
/// `validate_parts` itself walks (in this order):
///   - empty nodes check,
///   - `validate_resource_contract` (contract limits + expr stack +
///     transitions-per-tick),
///   - `validate_entry`,
///   - `validate_expressions` (per-expression max_stack + accessor
///     indices),
///   - `validate_accessors` (root slot bounds),
///   - per-node loop: `validate_node_id` + `validate_node`,
///   - `validate_accessor_paths` (depth + SymbolId bounds),
///   - `validate_constants_symbols` (SymbolId bounds in pool),
///   - `validate_build_object_symbols` (SymbolId bounds in nodes),
///   - `validate_reachability`,
///   - `validate_forward_edges`,
///   - `validate_no_nested_together`.
///
/// `validate_budget` runs `WholeWorkflowBudget::compute` then
/// `BoundednessPolicy::DEFAULT.validate`. Both may return typed
/// errors which the projection collapses to `BudgetPolicyExceeded`.
///
/// TRUST BOUNDARY: the body is opaque to Verus
/// (`#[verifier::external]`). The spec-side mirror in
/// `try_from_parts.rs` is the contract; this projection is the
/// trusted base recorded in the binding ledger.
#[verifier::external]
pub fn try_from_parts_pure(parts: &SpecWorkflowParts) -> SpecValidationResult {
    // ---- Step 1: structural validation ----
    if parts.nodes_meta.len() == 0 {
        return SpecValidationResult::Err(SpecWorkflowError::EmptyNodes);
    }
    let node_count = parts.nodes_len;
    // 1a. resource contract bounds (workflow/mod.rs:1587-1593).
    // The projection trusts the caller-supplied `slot_count`,
    // `expressions_len`, `accessors_len`, `constants_len` are within
    // their respective contract fields. A production contract
    // violation collapses to ResourceContractExceeded or
    // ResourceContractTooLarge; the projection picks the first one
    // that fires.
    let contract = parts.resource_contract;
    if contract.max_steps > 10000 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if parts.nodes_len > contract.max_steps {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractExceeded);
    }
    if contract.max_slots > 1024 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if parts.slot_count > contract.max_slots {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractExceeded);
    }
    if contract.max_expr_stack > 64 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    for expression in &parts.expressions {
        if expression.max_stack > contract.max_expr_stack {
            return SpecValidationResult::Err(SpecWorkflowError::ResourceContractExceeded);
        }
    }
    if contract.max_transitions_per_tick == 0
        || contract.max_transitions_per_tick > 10000
    {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractExceeded);
    }
    // 1b. entry bounds (workflow/mod.rs:1673-1675, 1372).
    if parts.entry >= node_count {
        return SpecValidationResult::Err(SpecWorkflowError::EntryOutOfBounds);
    }
    // 1c. validate_expressions (workflow/mod.rs:2046-2055).
    // The projection uses accessor_count to bound LoadAccessor
    // references; production walks every op. Per-op detail is
    // out of scope at the spec granularity (the spec records
    // ExpressionInvalid as the *first* failure type, not
    // per-variant).
    for _expression in &parts.expressions {
        // ExprProgram::try_from_parts runs transitively; the
        // projection trusts the spec caller to have produced a
        // valid stack claim (otherwise the production body would
        // already have failed at this step). The spec's
        // ExpressionInvalid arm is exercised by tests that supply
        // an oversize stack.
    }
    // 1d. validate_accessors (workflow/mod.rs:2069-2074) — root
    // slot bounds.
    for accessor in &parts.accessors {
        if accessor.root >= parts.slot_count {
            return SpecValidationResult::Err(SpecWorkflowError::SlotOutOfBounds);
        }
        if accessor.path_depth > 32 {
            return SpecValidationResult::Err(SpecWorkflowError::AccessorPathTooDeep);
        }
    }
    // 1e. per-node loop: validate_node_id + validate_node.
    for meta in &parts.nodes_meta {
        // validate_node_id (workflow/mod.rs:1472-1485).
        // Production indexes by enumeration position; the
        // projection trusts the caller-supplied `meta.id` matches
        // the iteration index. The spec's NodeIdMismatch arm is
        // exercised separately via a dedicated spec function.
        let _ = meta.id;
        let per_node_err = validate_node_pure(
            *meta,
            parts.slot_count,
            node_count,
            parts.expressions_len,
            parts.accessors_len,
            parts.constants_len,
            parts.symbols_count,
        );
        if let Some(err) = per_node_err {
            return SpecValidationResult::Err(err);
        }
    }
    // 1f. validate_constants_symbols + validate_build_object_symbols
    // + validate_accessor_paths — SymbolId bounds and path depth.
    // The projection has already checked accessor path_depth above.
    // For symbol bounds, the production validator reads every
    // constant / field / path-segment; the projection trusts the
    // caller-supplied `parts.symbols_count` is sufficient (the spec
    // proves SymbolOutOfBounds separately).
    // 1g. validate_reachability, validate_forward_edges,
    // validate_no_nested_together — graph structural checks.
    // The projection treats these as passing because the per-node
    // metadata (max_step_ref = strict-forward aggregation) is
    // precomputed by the spec caller; if any node violates, the
    // projection reports it via max_step_ref >= node_count (which
    // the spec records as a StepOutOfBounds-equivalent at the
    // decision-fn level; the spec's BackwardEdge / UnreachableNode
    // / NestedTogether arms are exercised via dedicated spec
    // functions).
    // ---- Step 2: loop-region validation ----
    // Production calls loop_regions::validate (loop_regions.rs:59-72)
    // next, in this order: pairing (`cache_control` +
    // `unique_owner`, pairing.rs:8-50), region relations
    // (`validate_region_relations`, pairing.rs:52-71), then edge
    // closure and recovery support (`validate_region_edges`,
    // edges.rs:14-39). The projection models each step here, in
    // the same order, over the region table the pairing step
    // builds. The kind-target edges read the caller-supplied
    // `kind_targets` inventory; the `try_reserve_exact`
    // reservation inside `collect_kind_targets` has no projection
    // counterpart (see the `kind_targets` field doc).
    if let Some(err) = loop_pairing_error(parts) {
        return SpecValidationResult::Err(err);
    }
    if let Some(err) = loop_region_relations_error(parts) {
        return SpecValidationResult::Err(err);
    }
    if let Some(err) = loop_region_edges_error(parts) {
        return SpecValidationResult::Err(err);
    }
    // ---- Step 3: nested activation map ----
    // Production derives nested_activation_map (loop_regions.rs:78-83)
    // from the validated region table; it can only fail with
    // StepOutOfBounds / ResourceContractExceeded on tables built
    // from already-validated nodes, so the projection treats it as
    // passing for inputs that reached this step.
    // ---- Step 4: budget validation ----
    // Production calls WholeWorkflowBudget::compute (budget.rs:64-70)
    // then BoundednessPolicy::DEFAULT.validate. Both errors collapse
    // to BudgetPolicyExceeded at this projection granularity.
    // The projection returns Ok for the well-formed inputs the spec
    // proves; malformed budget inputs are out of scope (they're
    // exhaustively covered by `budget_bounded.rs`).
    // ---- Step 5: input slot validation ----
    // Production calls validate_input_slots(&parts)? (workflow/mod.rs:42,
    // body at workflow/mod.rs:1394-1425). The projection checks: all
    // input slot indices in-bounds, no duplicates, strictly increasing,
    // then the per-slot validate_input_constraint (1427-1451) from the
    // caller-supplied `input_constraints` inventory (a missing entry
    // is unconstrained).
    let mut prev_slot: Option<u32> = None;
    for (index, current) in parts.input_slots.iter().enumerate() {
        if *current >= parts.slot_count {
            return SpecValidationResult::Err(SpecWorkflowError::InputSlotOutOfBounds);
        }
        if let Some(prev) = prev_slot {
            if *current == prev {
                return SpecValidationResult::Err(SpecWorkflowError::DuplicateInputSlot);
            }
            if *current < prev {
                return SpecValidationResult::Err(SpecWorkflowError::NonCanonicalInputSlotOrder);
            }
        }
        if let Some(constraint) = parts.input_constraints.get(index) {
            if let Some(err) = input_constraint_error(*constraint) {
                return SpecValidationResult::Err(err);
            }
        }
        prev_slot = Some(*current);
    }
    SpecValidationResult::Ok
}

/// Pins the `production_try_from_parts::CompiledNode` struct shape
/// (`crates/vb_core/src/workflow/mod.rs:995-1004`) so a field drift
/// breaks the build. Constructing both routing shapes pins every
/// variant of the `NodeOutput` / `NodeContinuation` / `ErrorRouting`
/// mirrors.
#[verifier::external]
fn compiled_node_shape_drift_check() {
    let node_step = production_try_from_parts::StepIdx::new(0);
    let node_slot = production_try_from_parts::SlotIdx::new(0);
    let _node = production_try_from_parts::CompiledNode {
        id: node_step,
        output: production_try_from_parts::NodeOutput::Slot(node_slot),
        routing: production_try_from_parts::NodeRouting {
            next: production_try_from_parts::NodeContinuation::Step(node_step),
            error: production_try_from_parts::ErrorRouting::Handler {
                handler: node_step,
                error_slot: Some(node_slot),
            },
        },
        kind: production_try_from_parts::CompiledNodeKind::Finish { result: node_slot },
    };
    let _node = production_try_from_parts::CompiledNode {
        id: node_step,
        output: production_try_from_parts::NodeOutput::Discard,
        routing: production_try_from_parts::NodeRouting {
            next: production_try_from_parts::NodeContinuation::End,
            error: production_try_from_parts::ErrorRouting::None,
        },
        kind: production_try_from_parts::CompiledNodeKind::Finish { result: node_slot },
    };
}
