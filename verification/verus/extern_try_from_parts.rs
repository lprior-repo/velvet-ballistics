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
//   - `validate_budget_result` budget error collapse
//     <- crates/vb_core/src/workflow/mod.rs:1461-1470
//   - `WholeWorkflowBudget` field set
//     <- crates/vb_core/src/budget/compute.rs:17-93
//   - `WholeWorkflowBudget::compute`
//     <- crates/vb_core/src/budget/compute.rs:100-106
//   - `BoundednessPolicy` field set
//     <- crates/vb_core/src/budget/validate.rs:9-56
//   - `BoundednessPolicy::DEFAULT`
//     <- crates/vb_core/src/budget/validate.rs:60-84
//   - `BoundednessPolicy::validate`
//     <- crates/vb_core/src/budget/validate.rs:88-147
//   - `validate_budget_dimensions`
//     <- crates/vb_core/src/budget/validate.rs:149-196
//   - `validate_extended_budget`
//     <- crates/vb_core/src/budget/validate.rs:198-215
//   - `validate_payload_budget`
//     <- crates/vb_core/src/budget/validate.rs:217-252
//   - `BudgetError`
//     <- crates/vb_core/src/budget/validate_error.rs:10-57
//   - `BudgetError::dimension`
//     <- crates/vb_core/src/budget/validate_error.rs:65-91
//   - `BudgetDimension`
//     <- crates/vb_core/src/budget/validate_error.rs:100-147
//   - `validate_resource_contract`
//     <- crates/vb_core/src/workflow/mod.rs:1587-1593
//   - `validate_entry`
//     <- crates/vb_core/src/workflow/mod.rs:1673-1675
//   - `validate_node` / `validate_node_common` / `validate_node_kind`
//     <- crates/vb_core/src/workflow/mod.rs:1677-1847
//   - `validate_slots`
//     <- crates/vb_core/src/workflow/mod.rs:1853-1858
//   - `validate_build_list`
//     <- crates/vb_core/src/workflow/mod.rs:1860-1867
//   - `validate_build_object`
//     <- crates/vb_core/src/workflow/mod.rs:1869-1882
//   - `validate_for_each_start`
//     <- crates/vb_core/src/workflow/mod.rs:1884-1894
//   - `validate_slot_and_steps`
//     <- crates/vb_core/src/workflow/mod.rs:1896-1904
//   - `validate_two_steps`
//     <- crates/vb_core/src/workflow/mod.rs:1906-1913
//   - `validate_together`
//     <- crates/vb_core/src/workflow/mod.rs:1915-1925
//   - `validate_nonzero_u16`
//     <- crates/vb_core/src/workflow/mod.rs:1927-1933
//   - `validate_reduce_start`
//     <- crates/vb_core/src/workflow/mod.rs:1935-1947
//   - `validate_reduce_next`
//     <- crates/vb_core/src/workflow/mod.rs:1949-1959
//   - `validate_repeat_start`
//     <- crates/vb_core/src/workflow/mod.rs:1961-1969
//   - `validate_slot_choose`
//     <- crates/vb_core/src/workflow/mod.rs:1971-1982
//   - `validate_expr_choose`
//     <- crates/vb_core/src/workflow/mod.rs:1984-1995
//   - `validate_branch_route`
//     <- crates/vb_core/src/workflow/mod.rs:1997-2006
//   - `MAX_LIST_ITEMS_PER_VALUE` / `MAX_OBJECT_FIELDS_PER_VALUE`
//     <- crates/vb_core/src/limits.rs:71-75
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
//   - `validate_accessor_paths`
//     <- crates/vb_core/src/workflow/mod.rs:2089-2119
//   - `validate_constants_symbols`
//     <- crates/vb_core/src/workflow/mod.rs:2122-2132
//   - `validate_build_object_symbols`
//     <- crates/vb_core/src/workflow/mod.rs:2135-2147
//   - `MAX_PATH_DEPTH`
//     <- crates/vb_core/src/limits.rs:59
//   - `validate_reachability`
//     <- crates/vb_core/src/workflow/mod.rs:2160-2229
//   - `validate_forward_edges`
//     <- crates/vb_core/src/workflow/mod.rs:2336-2364
//   - `validate_kind_edges`
//     <- crates/vb_core/src/workflow/mod.rs:2367-2426
//   - `validate_choose_slot_edges`
//     <- crates/vb_core/src/workflow/mod.rs:2428-2441
//   - `validate_choose_expr_edges`
//     <- crates/vb_core/src/workflow/mod.rs:2443-2456
//   - `validate_loop_done_only`
//     <- crates/vb_core/src/workflow/mod.rs:2458-2460
//   - `repeat_check_validate` (`nearest_active_owner`)
//     <- crates/vb_core/src/workflow/mod.rs:2465-2582
//   - `validate_loop_start_edges`
//     <- crates/vb_core/src/workflow/mod.rs:2588-2596
//   - `validate_together_start_edges`
//     <- crates/vb_core/src/workflow/mod.rs:2598-2608
//   - `validate_together_branch_edges`
//     <- crates/vb_core/src/workflow/mod.rs:2610-2618
//   - `validate_forward_target`
//     <- crates/vb_core/src/workflow/mod.rs:2621-2630
//   - `push_loop_span`
//     <- crates/vb_core/src/workflow/mod.rs:2633-2680
//   - `validate_no_nested_together` (moved out of workflow/mod.rs)
//     <- crates/vb_core/src/engine/validate.rs:119-172
//   - `walk_contains_together_start`
//     <- crates/vb_core/src/engine/validate.rs:177-205
//   - `find_inner_together_start`
//     <- crates/vb_core/src/engine/validate.rs:209-237
//   - `validate` (repeat_counter_ownership)
//     <- crates/vb_core/src/workflow/repeat_counter_ownership.rs:6-12
//   - `identify_counter_owners`
//     <- crates/vb_core/src/workflow/repeat_counter_ownership.rs:14-35
//   - `validate_slot_ownership`
//     <- crates/vb_core/src/workflow/repeat_counter_ownership.rs:37-72
//   - `is_paired_check`
//     <- crates/vb_core/src/workflow/repeat_counter_ownership.rs:74-109
//   - `writes_slot`
//     <- crates/vb_core/src/workflow/repeat_counter_ownership.rs:111-137
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
/// Mirror of the production `ExprOp` stack-effect classes
/// (`crates/vb_core/src/workflow/mod.rs:914-973`, classified by
/// `expr_stack_effect` at 2728-2741): `Push` covers `LoadSlot` /
/// `LoadConst` (effect 0,1); `LoadAccessor` covers `LoadAccessor`
/// (effect 0,1) and carries the accessor index for the
/// `validate_accessor` bound check (workflow/mod.rs:2076-2086);
/// `Unary` covers `Not`, `Exists`, `Length`, `Empty`, `Sum`,
/// `Count`, `Unique` (effect 1,1); `AppendIf` covers `AppendIf`
/// (effect 3,1); `Binary` covers every remaining op (`Eq`,
/// `NotEq`, `Gt`, `Gte`, `Lt`, `Lte`, `And`, `Or`, `Add`, `Sub`,
/// `Mul`, `Div`, `Contains`, `StartsWith`, `EndsWith`, `Has`,
/// `Merge`, `Append`) (effect 2,1).
#[derive(Clone, Copy)]
pub enum SpecExprOp {
    Push,
    LoadAccessor(u32),
    Unary,
    AppendIf,
    Binary,
}
/// Mirrors the production `ExprProgram` at
/// `crates/vb_core/src/workflow/mod.rs:884-889`. The projection
/// consumes `max_stack` and the `ops` inventory: `expr_error`
/// models the full per-op `validate_expressions` decision
/// (`ExprProgram::try_from_parts`, workflow/mod.rs:899-908).
#[derive(Clone)]
pub struct SpecExprProgram {
    pub max_stack: u32,
    pub ops: Vec<SpecExprOp>,
}
/// Mirrors the production `PathSegment` at
/// `crates/vb_core/src/workflow/mod.rs:656-661`: `Field` carries the
/// interned symbol id, `Index` the list index. The `u32::MAX`
/// index value is reserved by `validate_accessor_paths`
/// (workflow/mod.rs:2089-2119) and is modelled by
/// `accessor_path_error`.
#[derive(Clone, Copy)]
pub enum SpecPathSegment {
    Field(u32),
    Index(u32),
}
/// Mirrors the production `AccessorProgram` at
/// `crates/vb_core/src/workflow/mod.rs:646-661`. The projection
/// consumes `root` (slot index) and the full `path` segment
/// inventory: `accessor_path_error` models the per-segment
/// `validate_accessor_paths` decision (depth cap from
/// `MAX_PATH_DEPTH`, `Field` symbol bounds, reserved
/// `Index(u32::MAX)`).
#[derive(Clone)]
pub struct SpecAccessor {
    pub root: u32,
    pub path: Vec<SpecPathSegment>,
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
/// One ordered validation operation of production
/// `validate_node_kind` (`crates/vb_core/src/workflow/mod.rs:1718-1847`)
/// for a single node. The per-node `kind_refs` inventory lists these
/// in the exact order production checks them, so the projection
/// returns the same first failure production's `?`-propagation
/// would. `Slot` / `Step` / `Const` / `Expr` check one payload
/// reference against `slot_count` / `node_count` / `constants_len` /
/// `expressions_len`; `Count(len, cap)` is the list/object field-count
/// cap (`validate_build_list` / `validate_build_object`, cap 65_535
/// from `crates/vb_core/src/limits.rs:71-75`); `NonZero(value)` is the
/// branch-count / retry-attempts nonzero check
/// (`validate_nonzero_u16`). Optional payload references (WaitEvent /
/// Ask timeout slot, Choose / ChooseSlot `otherwise`) appear only
/// when present; no-payload kinds carry an empty inventory.
#[derive(Clone, Copy)]
pub enum SpecKindRef {
    Slot(u32),
    Step(u32),
    Const(u32),
    Expr(u32),
    Count(u32, u32),
    NonZero(u32),
}
/// Mirrors the production `WholeWorkflowBudget` field set at
/// `crates/vb_core/src/budget/compute.rs:17-93`: the computed budget
/// `WholeWorkflowBudget::compute` derives by walking the IR
/// (budget/compute.rs:100-106). Production `u64` fields stay `u64`;
/// production `u16` fields are promoted to `u32` per this
/// file's numeric-mirror convention. The seven fields
/// `BoundednessPolicy::DEFAULT.validate` never reads
/// (`max_retries_per_action`, `max_gather_pages`,
/// `max_gather_items`, `max_for_each_iterations`,
/// `max_together_branches`, `max_repeat_attempts`,
/// `max_total_slots_written`) are carried so the inventory is
/// a faithful mirror of the computed budget; the decision fn
/// below checks exactly the 23 validated dimensions, in
/// production `validate` order. The compute walk itself stays
/// caller-supplied: `budget_bounded.rs` is the exhaustive
/// coverage of `compute`'s boundedness.
#[derive(Clone, Copy)]
pub struct SpecComputedBudget {
    pub max_total_steps: u64,
    pub max_total_slots: u64,
    pub max_fanout: u32,
    pub max_nesting_depth: u32,
    pub max_steps_executable: u32,
    pub max_action_tickets: u32,
    pub max_parallel_in_flight: u32,
    pub max_retries_per_action: u32,
    pub max_gather_pages: u32,
    pub max_gather_items: u32,
    pub max_for_each_iterations: u32,
    pub max_together_branches: u32,
    pub max_repeat_attempts: u32,
    pub max_run_time_seconds: u64,
    pub max_result_bytes: u32,
    pub max_total_slots_written: u32,
    pub max_timer_entries: u32,
    pub max_trace_events: u64,
    pub max_journal_batch_bytes: u32,
    pub max_queue_depth: u32,
    pub max_ipc_payload_bytes: u32,
    pub max_blob_bytes: u64,
    pub max_input_bytes: u32,
    pub max_open_gates: u32,
    pub max_signal_subscriptions: u32,
    pub max_incident_emissions: u64,
    pub max_transitions: u64,
    pub max_journal_bytes_total: u64,
    pub max_value_store_entries: u64,
    pub max_cost_units: u64,
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
    /// Per-node kind-specific written-slot inventory consumed by
    /// the repeat-counter ownership check (`writes_slot`,
    /// repeat_counter_ownership.rs:111-137). `written_slots[i]`
    /// is node i's kind-specific written slots in `writes_slot`
    /// match order: ForEachStart item_slot, ForEachNext
    /// iterator_slot, TogetherBranch / TogetherJoin accumulator,
    /// CollectStart source, CollectNext collector_slot, ReduceStart
    /// input then accumulator, ReduceNext iterator_slot, RepeatCheck
    /// attempt_slot, RetryCheck policy_slot, ErrorHandler error_slot,
    /// AskResume answer. The output slot and the routing error slot
    /// are NOT in this inventory: the projection reads them from
    /// the node's meta (`has_output` / `out_slot`,
    /// `has_error_slot` / `error_slot`), matching production's
    /// output-first, error-second check order. The projection
    /// treats a missing entry (a `written_slots` shorter than
    /// `nodes_meta`) as no kind-specific writes for the tail nodes.
    /// The `CollectStart` source arm is the caller's
    /// responsibility: the inventory includes the source slot only
    /// when the node's output is none, matching production's
    /// `node.output.slot().is_none() && *source == slot` guard.
    pub written_slots: Vec<Vec<u32>>,
    /// Per-node ordered kind-reference inventory consumed by
    /// `validate_node_kind` (workflow/mod.rs:1718-1847).
    /// `kind_refs[i]` is node i's `SpecKindRef` list in the exact
    /// order production checks the kind payload: SetConst
    /// Const(value); Copy Slot(source); EvalExpr Expr(expr);
    /// BuildObject Count(fields, 65535) then one Slot per field,
    /// in field order; BuildList Count(items, 65535) then one Slot
    /// per item, in item order; Do Slot(input); ChooseSlot
    /// Slot(condition) then Step(target) per branch, in branch
    /// order, then Step(otherwise) when present; Choose
    /// Expr(condition) then Step(target) per branch, in branch
    /// order, then Step(otherwise) when present; ForEachStart
    /// Slot(input), Slot(item_slot), Step(body), Step(done);
    /// ForEachNext Slot(iterator_slot), Step(body), Step(done);
    /// ForEachJoin Slot(output); TogetherStart one Step per branch,
    /// in branch order, then Step(join); TogetherBranch Step(entry),
    /// Step(join), Slot(accumulator); TogetherJoin
    /// NonZero(branch_count), Slot(accumulator); CollectStart
    /// Slot(source), Step(body), Step(done); CollectPage /
    /// CollectNext Slot(collector_slot), Step(body), Step(done);
    /// CollectFinish Slot(collector_slot); ReduceStart Slot(input),
    /// Slot(accumulator), Const(initial), Step(body), Step(done);
    /// ReduceNext Slot(iterator_slot), Slot(accumulator), Step(body),
    /// Step(done); ReduceFinish Slot(accumulator); RepeatStart
    /// NonZero(max_attempts), Step(body), Step(done); RepeatAttempt
    /// Slot(attempt_slot), Step(body), Step(done); RepeatCheck
    /// Slot(attempt_slot), Step(done); RepeatFinish Slot(result);
    /// WaitUntil Slot(deadline_slot); WaitEvent Slot(event) then
    /// Slot(timeout_slot) when present; Ask Slot(prompt) then
    /// Slot(timeout_slot) when present; AskResume Slot(answer);
    /// RetryCheck Slot(policy_slot), Step(body), Step(exhausted);
    /// ErrorHandler Step(body), Step(handler); Jump Step(target);
    /// Finish Slot(result); Nop empty. The Choose / ChooseSlot
    /// EmptyBranchTable check (`validate_branch_route`,
    /// workflow/mod.rs:1997-2006) runs before this inventory, driven
    /// by the node's `has_branch_table_no_otherwise` flag. The
    /// projection treats a missing entry (a `kind_refs` shorter than
    /// `nodes_meta`) as an empty inventory for the tail nodes.
    pub kind_refs: Vec<Vec<SpecKindRef>>,
    /// Symbol inventory of the constant pool consumed by
    /// `validate_constants_symbols` (workflow/mod.rs:2122-2132).
    /// `constants_symbols` lists, in pool order, the `SymbolId`
    /// of every `ConstValue::Symbol` constant
    /// (value.rs:169); non-`Symbol` constants contribute no
    /// entry, so the inventory is symbol-only by construction
    /// and no sentinel is needed. The projection returns
    /// `SymbolOutOfBounds` for the first entry at or beyond
    /// `symbols_count`, matching production's `validate_symbol`
    /// (workflow/mod.rs:2150-2156).
    pub constants_symbols: Vec<u32>,
    /// Per-node BuildObject field-symbol inventory consumed by
    /// `validate_build_object_symbols` (workflow/mod.rs:2135-2147).
    /// `build_object_field_symbols[i]` is node i's BuildObject
    /// field symbols in field order; nodes of any other kind
    /// carry an empty entry. The projection treats a missing
    /// entry (a `build_object_field_symbols` shorter than
    /// `nodes_meta`) as no BuildObject fields for the tail nodes,
    /// and returns `SymbolOutOfBounds` for the first field
    /// symbol at or beyond `symbols_count`.
    pub build_object_field_symbols: Vec<Vec<u32>>,
    /// Caller-supplied computed budget consumed by step 4
    /// (`validate_budget`, workflow/mod.rs:1453-1459). Production
    /// derives it with `WholeWorkflowBudget::compute`
    /// (budget/compute.rs:100-106) and validates it against
    /// `BoundednessPolicy::DEFAULT` (budget/validate.rs:60-84); the
    /// projection checks the supplied value with `budget_error`
    /// instead, so the compute walk stays with the caller.
    pub computed_budget: SpecComputedBudget,
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
/// exceeds every legitimate slot/step count (slot_count <= 65535,
/// node_count <= 65535) so it never collides with a real index.
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
/// (loop_regions.rs:46-48): the step lies in the inclusive body /
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
/// Pure projection of one ordered `SpecKindRef` inventory:
/// `validate_node_kind` (`crates/vb_core/src/workflow/mod.rs:1718-1847`)
/// checks the kind payload references in inventory order and returns
/// the first failure production's `?`-propagation would return:
/// `SlotOutOfBounds` (`validate_slot`, crates/vb_core/src/workflow/mod.rs:2030-2036),
/// `StepOutOfBounds` (`validate_step`, crates/vb_core/src/workflow/mod.rs:2022-2028),
/// `ConstOutOfBounds` (`validate_const`, crates/vb_core/src/workflow/mod.rs:2038-2044),
/// `ExpressionInvalid` (`validate_expr`, crates/vb_core/src/workflow/mod.rs:2012-2020, which
/// returns `WorkflowError::Expression(CoreError::ExprOutOfBounds)`),
/// `ResourceContractExceeded` for a count over its cap
/// (`validate_build_list` / `validate_build_object`,
/// crates/vb_core/src/workflow/mod.rs:1860-1867 / 1869-1882) or a zero nonzero-checked value
/// (`validate_nonzero_u16`, crates/vb_core/src/workflow/mod.rs:1927-1933). Returns `None` when
/// every inventory entry is in bounds. The `EmptyBranchTable` check
/// (`validate_branch_route`, crates/vb_core/src/workflow/mod.rs:1997-2006) runs before this
/// inventory in production and is handled by the caller.
#[verifier::external]
pub fn kind_ref_error(
    refs: &Vec<SpecKindRef>,
    slot_count: u32,
    node_count: u32,
    constants_len: u32,
    expressions_len: u32,
) -> Option<SpecWorkflowError> {
    for r in refs {
        match r {
            SpecKindRef::Slot(value) => {
                if *value >= slot_count {
                    return Some(SpecWorkflowError::SlotOutOfBounds);
                }
            }
            SpecKindRef::Step(value) => {
                if *value >= node_count {
                    return Some(SpecWorkflowError::StepOutOfBounds);
                }
            }
            SpecKindRef::Const(value) => {
                if *value >= constants_len {
                    return Some(SpecWorkflowError::ConstOutOfBounds);
                }
            }
            SpecKindRef::Expr(value) => {
                if *value >= expressions_len {
                    return Some(SpecWorkflowError::ExpressionInvalid);
                }
            }
            SpecKindRef::Count(len, cap) => {
                if *len > *cap {
                    return Some(SpecWorkflowError::ResourceContractExceeded);
                }
            }
            SpecKindRef::NonZero(value) => {
                if *value == 0 {
                    return Some(SpecWorkflowError::ResourceContractExceeded);
                }
            }
        }
    }
    None
}

/// Pure projection of `BoundednessPolicy::DEFAULT.validate` applied
/// to a caller-supplied computed budget
/// (`crates/vb_core/src/budget/validate.rs:88-147`, via
/// `validate_extended_budget` at budget/validate.rs:198-215,
/// `validate_payload_budget` at budget/validate.rs:217-252, and
/// `validate_budget_dimensions` at budget/validate.rs:149-196). Checks the 23
/// validated dimensions in production first-violation order and
/// returns `BudgetPolicyExceeded` for the first dimension over
/// its `DEFAULT` cap (budget/validate.rs:60-84), or `None` when every
/// dimension is within caps. Production collapses the typed
/// `BudgetError` to `WorkflowError::BudgetPolicyExceeded`
/// (`validate_budget_result`, workflow/mod.rs:1461-1470), so the
/// projection carries no dimension payload: the first-violation
/// order is modelled by the check order itself. Caps, in
/// production order: total steps 1_000_000, total slots 65_535,
/// fanout 64, nesting depth 8, action tickets 100_000, parallel
/// in flight 256, run time 2_592_000, result bytes 262_144,
/// steps executable 1_000_000, timer entries 1_000_000, trace
/// events 1_000_000, journal batch bytes 1_048_576, queue depth
/// 1_024, IPC payload bytes 1_048_576, blob bytes 16_777_216,
/// input bytes 1_048_576, open gates 1_000_000, signal
/// subscriptions 1_000_000, incident emissions 1_000_000,
/// transitions 10_000_000, journal bytes total
/// 1_048_576_000_000, value-store entries 2_000_000, cost units
/// 100_000.
///
/// TRUST BOUNDARY: the body is opaque to Verus (`#[verifier::external]`).
/// The spec-side mirror in `try_from_parts.rs` is the contract; this
/// projection is the trusted base recorded in the binding ledger.
#[verifier::external]
pub fn budget_error(budget: SpecComputedBudget) -> Option<SpecWorkflowError> {
    if budget.max_total_steps > 1_000_000 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_total_slots > 65_535 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_fanout > 64 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_nesting_depth > 8 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_action_tickets > 100_000 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_parallel_in_flight > 256 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_run_time_seconds > 2_592_000 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_result_bytes > 262_144 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_steps_executable > 1_000_000 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_timer_entries > 1_000_000 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_trace_events > 1_000_000 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_journal_batch_bytes > 1_048_576 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_queue_depth > 1_024 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_ipc_payload_bytes > 1_048_576 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_blob_bytes > 16_777_216 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_input_bytes > 1_048_576 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_open_gates > 1_000_000 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_signal_subscriptions > 1_000_000 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_incident_emissions > 1_000_000 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_transitions > 10_000_000 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_journal_bytes_total > 1_048_576_000_000 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_value_store_entries > 2_000_000 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    if budget.max_cost_units > 100_000 {
        return Some(SpecWorkflowError::BudgetPolicyExceeded);
    }
    None
}

/// Pure projection of `validate_node_id`
/// (workflow/mod.rs:1472-1485): returns `Some(NodeIdMismatch)`
/// when the node's id differs from its table position, or `None`
/// when they match. Production's `u16::try_from(index)` arm is
/// unreachable on inputs that reach step 1e: step 1a caps
/// `max_steps` at 65535 and rejects a longer table first, so
/// every table position is below 65536 and the conversion cannot
/// fail; the projection compares the ids directly and models no
/// overflow error.
///
/// TRUST BOUNDARY: the body is opaque to Verus (`#[verifier::external]`).
/// The spec-side mirror in `try_from_parts.rs` is the contract; this
/// projection is the trusted base recorded in the binding ledger.
#[verifier::external]
pub fn node_id_error(meta: SpecNodeMeta, index: usize) -> Option<SpecWorkflowError> {
    if meta.id as usize == index {
        None
    } else {
        Some(SpecWorkflowError::NodeIdMismatch)
    }
}

/// Stack effect of one `SpecExprOp`, mirroring production
/// `expr_stack_effect` (workflow/mod.rs:2728-2741): `(pop, push)`.
fn expr_stack_effect(op: SpecExprOp) -> (u32, u32) {
    match op {
        SpecExprOp::Push => (0, 1),
        SpecExprOp::LoadAccessor(_) => (0, 1),
        SpecExprOp::Unary => (1, 1),
        SpecExprOp::AppendIf => (3, 1),
        SpecExprOp::Binary => (2, 1),
    }
}

/// Pure projection of `validate_expressions`
/// (workflow/mod.rs:2046-2055) for one expression program:
/// `ExprProgram::try_from_parts` (workflow/mod.rs:899-908) runs
/// `check_expr_stack_bound` (workflow/mod.rs:976-987), which runs
/// `validate_expr_op_count` (workflow/mod.rs:2682-2690), then
/// simulates the stack in op order with
/// `apply_expr_stack_effect` (workflow/mod.rs:2692-2702) and the
/// per-op `validate_expr_stack_capacity` check
/// (workflow/mod.rs:2704-2710) against the claimed `max_stack`,
/// then `validate_expr_final_depth` (workflow/mod.rs:2712-2720),
/// then the computed-required == claimed-max_stack comparison,
/// and finally `validate_expression_accessors`
/// (workflow/mod.rs:2057-2067) with `validate_accessor`
/// (workflow/mod.rs:2076-2086). Every failure is a `CoreError`
/// collapsed by `WorkflowError::Expression(#[from] CoreError)`
/// (workflow/mod.rs:714) to `SpecWorkflowError::ExpressionInvalid`,
/// so the projection returns `Some(ExpressionInvalid)` for the
/// first failure in that order, or `None` when the program
/// validates. `LoadSlot` / `LoadConst` indices are not
/// bounds-checked by `validate_expressions` (only their stack
/// effect matters). Depth and required are `u32`: the per-op
/// capacity check keeps depth at or below the claimed `max_stack`,
/// and step 1a already enforces
/// `max_stack <= contract.max_expr_stack <= 64` (crates/vb_core/src/limits.rs:43),
/// so the `u8` checked-add overflow arm of
/// `apply_expr_stack_effect` is unreachable on inputs that reach
/// step 1c and no overflow is modelled here.
///
/// TRUST BOUNDARY: the body is opaque to Verus (`#[verifier::external]`).
/// The spec-side mirror in `try_from_parts.rs` is the contract; this
/// projection is the trusted base recorded in the binding ledger.
#[verifier::external]
pub fn expr_error(program: &SpecExprProgram, accessors_len: u32) -> Option<SpecWorkflowError> {
    if program.ops.len() > 256 {
        return Some(SpecWorkflowError::ExpressionInvalid);
    }
    let mut depth: u32 = 0;
    let mut required: u32 = 0;
    for op in &program.ops {
        let (pop, push) = expr_stack_effect(*op);
        if depth < pop {
            return Some(SpecWorkflowError::ExpressionInvalid);
        }
        depth = depth - pop + push;
        if depth > required {
            required = depth;
        }
        if required > program.max_stack || required > 64 {
            return Some(SpecWorkflowError::ExpressionInvalid);
        }
    }
    if depth == 0 || depth > 1 {
        return Some(SpecWorkflowError::ExpressionInvalid);
    }
    if required != program.max_stack {
        return Some(SpecWorkflowError::ExpressionInvalid);
    }
    for op in &program.ops {
        if let SpecExprOp::LoadAccessor(index) = op {
            if *index >= accessors_len {
                return Some(SpecWorkflowError::ExpressionInvalid);
            }
        }
    }
    None
}

/// Pure projection of `validate_accessor_paths`
/// (workflow/mod.rs:2089-2119) for one accessor program, in
/// production order: the path length against `MAX_PATH_DEPTH`
/// (crates/vb_core/src/limits.rs:59), then each segment in
/// order — `Field(symbol)` via `validate_symbol`
/// (workflow/mod.rs:2150-2156, `symbol.get() < symbols_count`
/// else `SymbolOutOfBounds`), `Index(u32::MAX)` via the reserved
/// value arm (`ExpressionInvalid`). Returns the first failure,
/// or `None` when the path validates.
///
/// TRUST BOUNDARY: the body is opaque to Verus (`#[verifier::external]`).
/// The spec-side mirror in `try_from_parts.rs` is the contract; this
/// projection is the trusted base recorded in the binding ledger.
#[verifier::external]
pub fn accessor_path_error(
    accessor: &SpecAccessor,
    symbols_count: u32,
) -> Option<SpecWorkflowError> {
    if accessor.path.len() > 16 {
        return Some(SpecWorkflowError::AccessorPathTooDeep);
    }
    for segment in &accessor.path {
        match *segment {
            SpecPathSegment::Field(symbol) => {
                if symbol >= symbols_count {
                    return Some(SpecWorkflowError::SymbolOutOfBounds);
                }
            }
            SpecPathSegment::Index(index) => {
                if index == u32::MAX {
                    return Some(SpecWorkflowError::ExpressionInvalid);
                }
            }
        }
    }
    None
}

/// Pure projection of `validate_constants_symbols`
/// (workflow/mod.rs:2122-2132): each `ConstValue::Symbol`
/// constant in pool order via `validate_symbol`
/// (workflow/mod.rs:2150-2156, `symbol.get() < symbols_count`
/// else `SymbolOutOfBounds`). Reads the caller-supplied
/// symbol-only `constants_symbols` inventory, so non-`Symbol`
/// constants never reach the check. Returns the first failure,
/// or `None` when every pooled symbol validates.
///
/// TRUST BOUNDARY: the body is opaque to Verus (`#[verifier::external]`).
/// The spec-side mirror in `try_from_parts.rs` is the contract; this
/// projection is the trusted base recorded in the binding ledger.
#[verifier::external]
pub fn constants_symbols_error(parts: &SpecWorkflowParts) -> Option<SpecWorkflowError> {
    for symbol in &parts.constants_symbols {
        if *symbol >= parts.symbols_count {
            return Some(SpecWorkflowError::SymbolOutOfBounds);
        }
    }
    None
}

/// Pure projection of `validate_build_object_symbols`
/// (workflow/mod.rs:2135-2147): each `BuildObject` field symbol,
/// node by node in node order and field by field in field order,
/// via `validate_symbol` (workflow/mod.rs:2150-2156,
/// `symbol.get() < symbols_count` else `SymbolOutOfBounds`).
/// Reads the caller-supplied `build_object_field_symbols`
/// inventory; a missing entry means the node carries no
/// BuildObject fields. Returns the first failure, or `None`
/// when every field symbol validates.
///
/// TRUST BOUNDARY: the body is opaque to Verus (`#[verifier::external]`).
/// The spec-side mirror in `try_from_parts.rs` is the contract; this
/// projection is the trusted base recorded in the binding ledger.
#[verifier::external]
pub fn build_object_symbols_error(parts: &SpecWorkflowParts) -> Option<SpecWorkflowError> {
    for fields in &parts.build_object_field_symbols {
        for symbol in fields {
            if *symbol >= parts.symbols_count {
                return Some(SpecWorkflowError::SymbolOutOfBounds);
            }
        }
    }
    None
}

/// Pure projection of `validate_node_common`, `validate_node_kind`,
/// and the branch-target alias checks at
/// `crates/vb_core/src/workflow/mod.rs:1677-1847`, applied to a
/// single node's flattened metadata. The projection returns
/// `None` when the node passes every per-node check; otherwise it
/// returns the *first* specific failure the production body would
/// encounter, in the same order production uses
/// (`?`-propagation from `validate_node_common` then
/// `validate_node_kind`). The kind-payload checks read the
/// caller-supplied ordered `kind_refs` inventory instead of the
/// former `max_step_ref` / `max_slot_ref` aggregates, so the
/// per-kind check order (slot-before-step for the slot-first kinds,
/// step-before-slot for TogetherBranch / ErrorHandler, const third
/// for ReduceStart, expr interleaved per Choose branch) is modelled
/// rather than trusted.
///
/// TRUST BOUNDARY: the body is opaque to Verus (`#[verifier::external]`).
/// The spec-side mirror in `try_from_parts.rs` is the contract; this
/// projection is the trusted base recorded in the binding ledger.
#[verifier::external]
pub fn validate_node_pure(
    meta: SpecNodeMeta,
    kind_refs: &Vec<SpecKindRef>,
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
    // 2. validate_node_kind (workflow/mod.rs:1718-1847). Choose /
    // ChooseSlot run validate_branch_route first (EmptyBranchTable
    // when zero branches and no otherwise), then the ordered
    // kind-ref inventory; every other kind runs the inventory
    // directly. No-payload kinds carry an empty inventory.
    if (meta.kind_disc == 7 || meta.kind_disc == 8)
        && meta.has_branch_table_no_otherwise == 1
    {
        return Some(SpecWorkflowError::EmptyBranchTable);
    }
    if let Some(err) = kind_ref_error(
        kind_refs,
        slot_count,
        node_count,
        constants_len,
        expressions_len,
    ) {
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
/// (`start_shape(node) == Some(shape)` in crates/vb_core/src/workflow/loop_regions/pairing.rs:33-50, where
/// `LoopShape` is compared by value).
pub const fn loop_shape_matches(control: SpecNodeMeta, start: SpecNodeMeta) -> bool {
    control.ctrl_body == start.start_body
        && control.ctrl_done == start.start_done
        && control.ctrl_state_a == start.start_state_a
        && control.ctrl_state_b == start.start_state_b
}

/// Pure projection of the `unique_owner` scan
/// (crates/vb_core/src/workflow/loop_regions/pairing.rs:33-50) for one cached control. Returns the unique
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
/// (`cache_control` + `unique_owner`, crates/vb_core/src/workflow/loop_regions/pairing.rs:8-50), applied to
/// the whole node table in table order. A cached control is a node
/// with a derivable control shape whose body does not follow its own
/// id (`shape.body > node.id` skips the control, crates/vb_core/src/workflow/loop_regions/pairing.rs:8-31).
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
/// (`cache_control`, crates/vb_core/src/workflow/loop_regions/pairing.rs:8-31): one `SpecLoopRegion` per
/// cached control, in node-table order (ascending control id, since
/// step 1e validates `meta.id` against the table position before
/// this step runs). Returns `None` exactly when `loop_pairing_error`
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
/// (crates/vb_core/src/workflow/loop_regions/pairing.rs:52-71): every pair of cached regions, first in
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
/// input that reached this step because step 1e validates `meta.id`
/// against the table position first.
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
// Pure decision fn: repeat-counter ownership
// ============================================================================
/// Pure projection of `is_paired_check`
/// (repeat_counter_ownership.rs:74-109): the node is the paired
/// `RepeatCheck` for the counter slot `slot` owned by `owner` iff
/// it is a `RepeatCheck` whose attempt slot is the counter slot,
/// whose control shape exists (routing next is a step), whose done
/// matches the owner's start done, whose id lies strictly between
/// the owner's body and done, and whose routing next step is the
/// owner's body. The owner lookup scans by id, which agrees with
/// production's `parts.nodes.get(owner.as_usize())` position index
/// on every input that reached this step because step 1e validates
/// `meta.id` against the table position first. An owner carries a start
/// shape by construction (`start_state` needs only the output
/// slot, which owners have by definition), so a missing start
/// shape means the node is not paired.
#[verifier::external]
fn is_paired_repeat_check(
    parts: &SpecWorkflowParts,
    node: SpecNodeMeta,
    slot: u32,
    owner: u32,
) -> bool {
    if node.kind_disc != spec_node_kind_repeat_check() {
        return false;
    }
    if node.has_ctrl_shape != 1 || node.ctrl_state_a != slot {
        return false;
    }
    for candidate in parts.nodes_meta.iter() {
        if candidate.id != owner {
            continue;
        }
        if candidate.kind_disc != spec_node_kind_repeat_start()
            || candidate.has_start_shape != 1
        {
            return false;
        }
        return node.ctrl_done == candidate.start_done
            && candidate.start_body < node.id
            && node.id < candidate.start_done
            && node.ctrl_body == candidate.start_body;
    }
    false
}

/// Pure projection of `writes_slot`
/// (repeat_counter_ownership.rs:111-137) for the node at table
/// index `index`: the output slot, the routing error slot, or any
/// slot in the caller-supplied `written_slots` inventory for the
/// node. A missing inventory entry means the node writes no
/// kind-specific slot.
#[verifier::external]
fn node_writes_slot(parts: &SpecWorkflowParts, index: usize, slot: u32) -> bool {
    let meta = match parts.nodes_meta.get(index) {
        Some(meta) => *meta,
        None => return false,
    };
    if meta.has_output == 1 && meta.out_slot == slot {
        return true;
    }
    if meta.has_error_slot == 1 && meta.error_slot == slot {
        return true;
    }
    match parts.written_slots.get(index) {
        Some(slots) => slots.contains(&slot),
        None => false,
    }
}

/// Pure projection of `repeat_counter_ownership::validate`
/// (repeat_counter_ownership.rs:6-12), applied to the whole node
/// table in table order. Phase 1 (`identify_counter_owners`,
/// repeat_counter_ownership.rs:14-35) claims one counter slot per
/// `RepeatStart` with an output slot; a second owner claiming the
/// same slot returns `RepeatCounterForeignWrite`. Phase 2
/// (`validate_slot_ownership`, repeat_counter_ownership.rs:37-72)
/// checks every claim in claim order: the owner node itself and
/// its paired `RepeatCheck` may only touch the slot through their
/// routing error slot; every other node must not write the slot
/// (`writes_slot`, repeat_counter_ownership.rs:111-137). Returns
/// the first failure, else `None`.
///
/// TRUST BOUNDARY: the body is opaque to Verus (`#[verifier::external]`).
/// The spec-side mirror in `try_from_parts.rs` is the contract; this
/// projection is the trusted base recorded in the binding ledger.
#[verifier::external]
pub fn repeat_counter_ownership_error(parts: &SpecWorkflowParts) -> Option<SpecWorkflowError> {
    let mut owners: Vec<(u32, u32)> = Vec::new();
    for meta in parts.nodes_meta.iter() {
        if meta.kind_disc != spec_node_kind_repeat_start() || meta.has_output != 1 {
            continue;
        }
        for (slot, _) in owners.iter() {
            if *slot == meta.out_slot {
                return Some(SpecWorkflowError::RepeatCounterForeignWrite);
            }
        }
        owners.push((meta.out_slot, meta.id));
    }
    for (slot, owner) in owners.iter() {
        for (index, meta) in parts.nodes_meta.iter().enumerate() {
            if meta.id == *owner {
                if meta.has_error_slot == 1 && meta.error_slot == *slot {
                    return Some(SpecWorkflowError::RepeatCounterForeignWrite);
                }
                continue;
            }
            if is_paired_repeat_check(parts, *meta, *slot, *owner) {
                if meta.has_error_slot == 1 && meta.error_slot == *slot {
                    return Some(SpecWorkflowError::RepeatCounterForeignWrite);
                }
                continue;
            }
            if node_writes_slot(parts, index, *slot) {
                return Some(SpecWorkflowError::RepeatCounterForeignWrite);
            }
        }
    }
    None
}
// ============================================================================
// Pure decision fns: validate_forward_edges
// ============================================================================
/// Pure projection of `validate_forward_target`
/// (workflow/mod.rs:2621-2630): a target step is strictly forward
/// from the current node iff it exceeds the node's id. The
/// projection compares against `meta.id`, which step 1e validates
/// against the table position; production compares
/// against the enumeration position.
#[verifier::external]
fn forward_target_error(target: u32, current: u32) -> Option<SpecWorkflowError> {
    if target > current {
        None
    } else {
        Some(SpecWorkflowError::BackwardEdge)
    }
}

/// Pure projection of the loop-span end `push_loop_span` reads
/// (workflow/mod.rs:2633-2680): the `done` step of a loop start
/// kind, or the `join` step of a `TogetherStart`, from the
/// caller-supplied `kind_targets` inventory (`body` then `done`
/// for loop starts, branch entries then `join` for
/// `TogetherStart`). Returns `None` for non-start kinds and when
/// the inventory has no entry for the node.
#[verifier::external]
fn loop_span_end(parts: &SpecWorkflowParts, index: usize) -> Option<u32> {
    let meta = match parts.nodes_meta.get(index) {
        Some(meta) => meta,
        None => return None,
    };
    let targets = match parts.kind_targets.get(index) {
        Some(targets) => targets,
        None => return None,
    };
    if meta.kind_disc == spec_node_kind_foreach_start()
        || meta.kind_disc == spec_node_kind_collect_start()
        || meta.kind_disc == spec_node_kind_reduce_start()
        || meta.kind_disc == spec_node_kind_repeat_start()
    {
        targets.get(1).copied()
    } else if meta.kind_disc == spec_node_kind_together_start() {
        targets.last().copied()
    } else {
        None
    }
}

/// Pure projection of `repeat_check_validate::validate`
/// (workflow/mod.rs:2465-2582) for a `RepeatCheck` node at
/// `check_index` whose routing `next` step `back_target` points
/// backward: the nearest active enclosing structured start
/// (`nearest_active_owner`, workflow/mod.rs:2470-2488) must be a
/// `RepeatStart` whose body is the back target, whose done
/// matches the check's done, and whose output slot is the check's
/// attempt slot; the owner must precede the check, its body must
/// precede the check, and the check must precede its done. A
/// slot-less owner has no output slot, so it can never satisfy
/// the attempt-slot match. Returns `BackwardEdge` for the first
/// failing condition, else `None`. Positions are `meta.id`
/// values, which step 1e validates against the table position.
#[verifier::external]
fn repeat_check_owner_error(
    parts: &SpecWorkflowParts,
    check_index: usize,
    back_target: u32,
) -> Option<SpecWorkflowError> {
    let check_meta = match parts.nodes_meta.get(check_index) {
        Some(meta) => *meta,
        None => return Some(SpecWorkflowError::BackwardEdge),
    };
    let mut owner_index: Option<usize> = None;
    let mut j = check_index;
    while j > 0 {
        j -= 1;
        let candidate = match parts.nodes_meta.get(j) {
            Some(meta) => *meta,
            None => continue,
        };
        let is_start = candidate.kind_disc == spec_node_kind_foreach_start()
            || candidate.kind_disc == spec_node_kind_collect_start()
            || candidate.kind_disc == spec_node_kind_reduce_start()
            || candidate.kind_disc == spec_node_kind_repeat_start()
            || candidate.kind_disc == spec_node_kind_together_start();
        if !is_start {
            continue;
        }
        match loop_span_end(parts, j) {
            Some(end) if end > check_meta.id => {
                owner_index = Some(j);
                break;
            }
            _ => continue,
        }
    }
    let owner_index = match owner_index {
        Some(owner_index) => owner_index,
        None => return Some(SpecWorkflowError::BackwardEdge),
    };
    let owner_meta = match parts.nodes_meta.get(owner_index) {
        Some(meta) => *meta,
        None => return Some(SpecWorkflowError::BackwardEdge),
    };
    if owner_meta.kind_disc != spec_node_kind_repeat_start() {
        return Some(SpecWorkflowError::BackwardEdge);
    }
    if owner_meta.has_start_shape != 1 {
        return Some(SpecWorkflowError::BackwardEdge);
    }
    if check_meta.has_ctrl_shape != 1 {
        return Some(SpecWorkflowError::BackwardEdge);
    }
    let owner_targets = match parts.kind_targets.get(owner_index) {
        Some(targets) => targets,
        None => return Some(SpecWorkflowError::BackwardEdge),
    };
    let owner_body = match owner_targets.get(0) {
        Some(body) => *body,
        None => return Some(SpecWorkflowError::BackwardEdge),
    };
    let owner_done = match owner_targets.get(1) {
        Some(done) => *done,
        None => return Some(SpecWorkflowError::BackwardEdge),
    };
    if owner_meta.start_state_a != check_meta.ctrl_state_a {
        return Some(SpecWorkflowError::BackwardEdge);
    }
    if owner_body != back_target {
        return Some(SpecWorkflowError::BackwardEdge);
    }
    if owner_done != check_meta.ctrl_done {
        return Some(SpecWorkflowError::BackwardEdge);
    }
    if owner_meta.id >= check_meta.id {
        return Some(SpecWorkflowError::BackwardEdge);
    }
    if owner_body >= check_meta.id {
        return Some(SpecWorkflowError::BackwardEdge);
    }
    if check_meta.id >= owner_done {
        return Some(SpecWorkflowError::BackwardEdge);
    }
    None
}

/// Pure projection of one `kind_targets` entry of
/// `validate_kind_edges` (workflow/mod.rs:2367-2426): the target
/// at `pos` must point strictly forward from the node. A missing
/// inventory entry, or a missing position, means the caller
/// supplied no target there, so the check passes.
#[verifier::external]
fn kind_target_forward_error(
    parts: &SpecWorkflowParts,
    index: usize,
    current: u32,
    pos: usize,
) -> Option<SpecWorkflowError> {
    let targets = match parts.kind_targets.get(index) {
        Some(targets) => targets,
        None => return None,
    };
    match targets.get(pos) {
        Some(target) => forward_target_error(*target, current),
        None => None,
    }
}

/// Pure projection of the forward check over every
/// `kind_targets` entry of one node (`validate_choose_slot_edges`
/// / `validate_choose_expr_edges` /
/// `validate_together_start_edges`, workflow/mod.rs:2428-2441,
/// 2443-2457, 2598-2609): branch targets in inventory order,
/// then the fallback / join.
#[verifier::external]
fn kind_targets_forward_error(
    parts: &SpecWorkflowParts,
    index: usize,
    current: u32,
) -> Option<SpecWorkflowError> {
    let targets = match parts.kind_targets.get(index) {
        Some(targets) => targets,
        None => return None,
    };
    for target in targets.iter() {
        if let Some(err) = forward_target_error(*target, current) {
            return Some(err);
        }
    }
    None
}

/// Pure projection of `validate_kind_edges`
/// (workflow/mod.rs:2367-2426) for one node: which
/// `kind_targets` positions production checks depends on the
/// kind. Loop `*Next` / `*Attempt` / `RetryCheck` / `RepeatCheck`
/// / `ErrorHandler` kinds check only their forward target
/// (`done` / `exhausted` / `handler`); the back-edge slots
/// (`body`) are not forward edges. `*Start` kinds check `body`
/// then `done`; `TogetherBranch` checks `entry` then `join`;
/// `Jump` checks its target; `Choose` / `ChooseSlot` /
/// `TogetherStart` check every target in inventory order; every
/// other kind has no kind edges.
#[verifier::external]
fn kind_edges_error(
    parts: &SpecWorkflowParts,
    index: usize,
    meta: SpecNodeMeta,
) -> Option<SpecWorkflowError> {
    let current = meta.id;
    if meta.kind_disc == spec_node_kind_jump() {
        kind_target_forward_error(parts, index, current, 0)
    } else if meta.kind_disc == spec_node_kind_choose()
        || meta.kind_disc == spec_node_kind_choose_slot()
        || meta.kind_disc == spec_node_kind_together_start()
    {
        kind_targets_forward_error(parts, index, current)
    } else if meta.kind_disc == spec_node_kind_foreach_start()
        || meta.kind_disc == spec_node_kind_collect_start()
        || meta.kind_disc == spec_node_kind_reduce_start()
        || meta.kind_disc == spec_node_kind_repeat_start()
        || meta.kind_disc == spec_node_kind_together_branch()
    {
        match kind_target_forward_error(parts, index, current, 0) {
            Some(err) => Some(err),
            None => kind_target_forward_error(parts, index, current, 1),
        }
    } else if meta.kind_disc == spec_node_kind_foreach_next()
        || meta.kind_disc == spec_node_kind_collect_page()
        || meta.kind_disc == spec_node_kind_collect_next()
        || meta.kind_disc == spec_node_kind_reduce_next()
        || meta.kind_disc == spec_node_kind_repeat_attempt()
        || meta.kind_disc == spec_node_kind_retry_check()
        || meta.kind_disc == spec_node_kind_error_handler()
    {
        kind_target_forward_error(parts, index, current, 1)
    } else if meta.kind_disc == spec_node_kind_repeat_check() {
        kind_target_forward_error(parts, index, current, 0)
    } else {
        None
    }
}

/// Pure projection of `validate_forward_edges`
/// (workflow/mod.rs:2336-2364): every node in table order must
/// have strictly forward routing and kind edges, except a
/// `RepeatCheck` routing back-edge validated against its owning
/// `RepeatStart` (`repeat_check_validate`), and loop spans must
/// be properly nested (`push_loop_span`, Check D:
/// `ImproperLoopNesting`). Returns the first failure in
/// production order (routing `next`, routing error handler, kind
/// edges, span nesting), else `None`. Positions are `meta.id`
/// values, which step 1e validates against the table position;
/// the span stack mirrors `push_loop_span` exactly
/// (retire finished spans before the nesting comparison, so
/// sequential loops are not rejected).
///
/// TRUST BOUNDARY: the body is opaque to Verus
/// (`#[verifier::external]`). The spec-side mirror in
/// `try_from_parts.rs` is the contract; this projection is the
/// trusted base recorded in the binding ledger.
#[verifier::external]
pub fn forward_edges_error(parts: &SpecWorkflowParts) -> Option<SpecWorkflowError> {
    let mut spans: Vec<(u32, u32)> = Vec::new();
    for (index, meta) in parts.nodes_meta.iter().enumerate() {
        let current = meta.id;
        if meta.has_next == 1 {
            if meta.kind_disc == spec_node_kind_repeat_check() && meta.next_step <= current {
                if let Some(err) = repeat_check_owner_error(parts, index, meta.next_step) {
                    return Some(err);
                }
            } else if let Some(err) = forward_target_error(meta.next_step, current) {
                return Some(err);
            }
        }
        if meta.has_on_error == 1 {
            if let Some(err) = forward_target_error(meta.on_error_step, current) {
                return Some(err);
            }
        }
        if let Some(err) = kind_edges_error(parts, index, *meta) {
            return Some(err);
        }
        if let Some(end) = loop_span_end(parts, index) {
            loop {
                let done = match spans.last() {
                    Some((_, done)) => *done,
                    None => break,
                };
                if done <= current {
                    spans.pop();
                } else {
                    break;
                }
            }
            if let Some((_, outer_done)) = spans.last() {
                if end > *outer_done {
                    return Some(SpecWorkflowError::ImproperLoopNesting);
                }
            }
            spans.push((current, end));
        }
    }
    None
}

/// Pure projection of `validate_reachability`
/// (workflow/mod.rs:2160-2229): breadth-first walk from the entry
/// over each node's routing `next` step, routing error handler,
/// and kind targets (`collect_node_targets`,
/// workflow/mod.rs:2233-2299). Returns the first unreachable node
/// in table order, else `None`. An entry at or past the node
/// count returns `None`, matching production (entry bounds are
/// already enforced by step 1b); an out-of-range target is
/// skipped, matching production (bounds are enforced by the
/// per-node checks). Positions are table indices; step 1e validates
/// the caller-supplied `meta.id` values against them first.
///
/// TRUST BOUNDARY: the body is opaque to Verus
/// (`#[verifier::external]`). The spec-side mirror in
/// `try_from_parts.rs` is the contract; this projection is the
/// trusted base recorded in the binding ledger.
#[verifier::external]
pub fn reachability_error(parts: &SpecWorkflowParts) -> Option<SpecWorkflowError> {
    let node_count = parts.nodes_len;
    if parts.nodes_meta.is_empty() {
        return None;
    }
    let entry = parts.entry;
    if entry >= node_count {
        return None;
    }
    let mut visited: Vec<bool> = vec![false; node_count as usize];
    let mut queue: Vec<u32> = Vec::new();
    let Some(entry_flag) = visited.get_mut(entry as usize) else {
        return None;
    };
    *entry_flag = true;
    queue.push(entry);
    let mut head = 0usize;
    while head < queue.len() {
        let current = match queue.get(head) {
            Some(&v) => v,
            None => break,
        };
        head = match head.checked_add(1) {
            Some(v) => v,
            None => break,
        };
        let meta = match parts.nodes_meta.get(current as usize) {
            Some(meta) => meta,
            None => break,
        };
        let mut targets: Vec<u32> = Vec::new();
        if meta.has_next == 1 {
            targets.push(meta.next_step);
        }
        if meta.has_on_error == 1 {
            targets.push(meta.on_error_step);
        }
        if let Some(kind_targets) = parts.kind_targets.get(current as usize) {
            for target in kind_targets {
                targets.push(*target);
            }
        }
        for target in targets {
            if target < node_count {
                let Some(flag) = visited.get_mut(target as usize) else {
                    continue;
                };
                if !*flag {
                    *flag = true;
                    queue.push(target);
                }
            }
        }
    }
    for was_visited in visited.iter() {
        if !was_visited {
            return Some(SpecWorkflowError::UnreachableNode);
        }
    }
    None
}

/// Pure projection of `walk_contains_together_start`
/// (engine/validate.rs:177-205): follows the routing `next`
/// chain from `start_idx`, stopping at `stop_idx` (exclusive),
/// and reports whether any node on the chain is a
/// `TogetherStart` distinct from `outer`. Membership is read
/// from the caller-supplied `together_starts` id inventory,
/// matching production's `BTreeSet<StepIdx>` built from the same
/// nodes. The stack holds at most one step at a time (each node
/// pushes at most its single `next`), so the walk is the linear
/// chain production walks.
///
/// TRUST BOUNDARY: the body is opaque to Verus
/// (`#[verifier::external]`). The spec-side mirror in
/// `try_from_parts.rs` is the contract; this projection is the
/// trusted base recorded in the binding ledger.
#[verifier::external]
fn walk_contains_together_start(
    parts: &SpecWorkflowParts,
    together_starts: &[u32],
    start_idx: u32,
    stop_idx: u32,
    outer: u32,
) -> bool {
    let mut visited: Vec<bool> = vec![false; parts.nodes_len as usize];
    let mut stack: Vec<u32> = vec![start_idx];
    while let Some(idx) = stack.pop() {
        if idx >= parts.nodes_len || idx == stop_idx {
            continue;
        }
        let Some(flag) = visited.get_mut(idx as usize) else {
            continue;
        };
        if *flag {
            continue;
        }
        *flag = true;
        let meta = match parts.nodes_meta.get(idx as usize) {
            Some(meta) => meta,
            None => continue,
        };
        if meta.id != outer && together_starts.contains(&meta.id) {
            return true;
        }
        if meta.has_next == 1 {
            stack.push(meta.next_step);
        }
    }
    false
}

/// Pure projection of `validate_no_nested_together`
/// (engine/validate.rs:119-172): a `TogetherStart` whose branch
/// body contains another `TogetherStart` is rejected. Branch
/// entries and the join read the caller-supplied
/// `kind_targets` inventory (branch entries then join for
/// `TogetherStart`); a branch whose entry or join is out of
/// range is skipped, matching production. With fewer than two
/// `TogetherStart` nodes nesting is impossible, matching
/// production's early return. Returns the first failure in
/// table order, else `None`. The error variant carries no
/// payload at this projection granularity; production's
/// `NestedTogether { outer, inner }` first inner is the one
/// `walk_contains_together_start` finds, so the decision
/// agrees.
///
/// TRUST BOUNDARY: the body is opaque to Verus
/// (`#[verifier::external]`). The spec-side mirror in
/// `try_from_parts.rs` is the contract; this projection is the
/// trusted base recorded in the binding ledger.
#[verifier::external]
pub fn nested_together_error(parts: &SpecWorkflowParts) -> Option<SpecWorkflowError> {
    let node_count = parts.nodes_len;
    let mut together_starts: Vec<u32> = Vec::new();
    for meta in &parts.nodes_meta {
        if meta.kind_disc == spec_node_kind_together_start() {
            together_starts.push(meta.id);
        }
    }
    if together_starts.len() < 2 {
        return None;
    }
    for (index, meta) in parts.nodes_meta.iter().enumerate() {
        if meta.kind_disc != spec_node_kind_together_start() {
            continue;
        }
        let outer = meta.id;
        let targets = match parts.kind_targets.get(index) {
            Some(targets) => targets,
            None => continue,
        };
        if targets.is_empty() {
            continue;
        }
        let join = match targets.last() {
            Some(join) => *join,
            None => continue,
        };
        for branch_entry in &targets[..targets.len() - 1] {
            let entry_idx = *branch_entry;
            if entry_idx >= node_count {
                continue;
            }
            if join >= node_count {
                continue;
            }
            if walk_contains_together_start(parts, &together_starts, entry_idx, join, outer) {
                return Some(SpecWorkflowError::NestedTogether);
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
///   - `repeat_counter_ownership::validate`
///     (repeat_counter_ownership.rs:6-12),
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
    // The TooLarge arms live in `check_resource_contract_limits`
    // (workflow/mod.rs:1496-1585): steps (1499-1503,
    // MAX_STEPS_PER_WORKFLOW 65535), slots (1504-1508,
    // MAX_SLOTS_PER_WORKFLOW 65535), constants (1509-1513,
    // MAX_CONSTANTS 65535), accessors (1514-1518,
    // MAX_ACCESSORS 8192), expressions (1519-1523, MAX_EXPRESSIONS
    // 4096), expr_stack, step_budget (1529-1533, MAX_STEP_BUDGET
    // 10000), transitions (1534-1538, MAX_STEP_BUDGET 10000),
    // input (1539-1543, MAX_INPUT_BYTES 16777216),
    // output (1544-1548, MAX_OUTPUT_BYTES 16777216), blob
    // (1549-1553, MAX_BLOB_BYTES 67108864), ipc (1554-1558,
    // MAX_IPC_PAYLOAD_BYTES 16777216), retry (1559-1563,
    // MAX_RETRY_ATTEMPTS 10), fanout (1564-1568, MAX_FANOUT 256),
    // collect (1569-1573, MAX_COLLECT_ITEMS 1048576), queue
    // (1574-1578, MAX_QUEUE_DEPTH 1048576), journal (1579-1583,
    // MAX_JOURNAL_BATCH_BYTES 16777216). The transitions
    // over-cap arm returns ResourceContractTooLarge, matching
    // production; the zero-transitions check below returns
    // ResourceContractExceeded after the expr-stack loop, matching
    // `validate_transitions_nonzero` (workflow/mod.rs:1600-1607).
    // The caller-supplied `slot_count`,
    // `accessors_len`, and `expressions_len` are trusted to be the
    // true inventory lengths. Each is checked against its contract
    // cap in production `validate_resource_usage` order; a
    // violation collapses to ResourceContractExceeded or
    // ResourceContractTooLarge, and the projection returns the
    // first one that fires.
    let contract = parts.resource_contract;
    if contract.max_steps > 65535 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if parts.nodes_len > contract.max_steps {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractExceeded);
    }
    if contract.max_slots > 65535 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if contract.max_constants > 65535 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if contract.max_accessors > 8192 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if contract.max_expressions > 4096 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if parts.slot_count > contract.max_slots {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractExceeded);
    }
    if parts.constants_len > contract.max_constants {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractExceeded);
    }
    if parts.accessors_len > contract.max_accessors {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractExceeded);
    }
    if parts.expressions_len > contract.max_expressions {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractExceeded);
    }
    if contract.max_expr_stack > 64 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if contract.max_step_budget_per_tick > 10000 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if contract.max_transitions_per_tick > 10000 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if contract.max_input_bytes > 16777216 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if contract.max_output_bytes > 16777216 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if contract.max_blob_bytes > 67108864 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if contract.max_ipc_payload_bytes > 16777216 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if contract.max_retry_attempts > 10 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if contract.max_fanout > 256 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if contract.max_collect_items > 1048576 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if contract.max_queue_depth > 1048576 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    if contract.max_journal_batch_bytes > 16777216 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractTooLarge);
    }
    for expression in &parts.expressions {
        if expression.max_stack > contract.max_expr_stack {
            return SpecValidationResult::Err(SpecWorkflowError::ResourceContractExceeded);
        }
    }
    if contract.max_transitions_per_tick == 0 {
        return SpecValidationResult::Err(SpecWorkflowError::ResourceContractExceeded);
    }
    // 1b. entry bounds (workflow/mod.rs:1673-1675, 1372).
    if parts.entry >= node_count {
        return SpecValidationResult::Err(SpecWorkflowError::EntryOutOfBounds);
    }
    // 1c. validate_expressions (workflow/mod.rs:2046-2055).
    // Per expression, in order: `ExprProgram::try_from_parts`
    // (workflow/mod.rs:899-908) then
    // `validate_expression_accessors` (workflow/mod.rs:2057-2067);
    // `expr_error` models the full per-op detail (op count at
    // workflow/mod.rs:2682-2690, stack simulation at 976-987 with
    // effects at 2728-2741, final depth at 2712-2720, max_stack
    // mismatch at 899-908, accessor bounds at 2076-2086).
    for expression in &parts.expressions {
        if let Some(err) = expr_error(expression, parts.accessors_len) {
            return SpecValidationResult::Err(err);
        }
    }
    // 1d. validate_accessors (workflow/mod.rs:2069-2074) — root
    // slot bounds only. Path depth and per-segment checks live in
    // the validate_accessor_paths step below (1f).
    for accessor in &parts.accessors {
        if accessor.root >= parts.slot_count {
            return SpecValidationResult::Err(SpecWorkflowError::SlotOutOfBounds);
        }
    }
    // 1e. per-node loop: validate_node_id + validate_node.
    // The kind-ref inventory is per node; a missing entry (a
    // `kind_refs` shorter than `nodes_meta`) means the node carries
    // no kind-payload references.
    let empty_refs: Vec<SpecKindRef> = Vec::new();
    for (index, meta) in parts.nodes_meta.iter().enumerate() {
        // validate_node_id (workflow/mod.rs:1472-1485), before
        // validate_node for this node (workflow/mod.rs:1375-1378).
        if let Some(err) = node_id_error(*meta, index) {
            return SpecValidationResult::Err(err);
        }
        let kind_refs = match parts.kind_refs.get(index) {
            Some(refs) => refs,
            None => &empty_refs,
        };
        let per_node_err = validate_node_pure(
            *meta,
            kind_refs,
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
    // 1f. validate_accessor_paths (workflow/mod.rs:2089-2119) —
    // per accessor, in order: path depth against MAX_PATH_DEPTH
    // (crates/vb_core/src/limits.rs:59), then per-segment Field symbol bounds and
    // reserved Index(u32::MAX). Runs after the per-node loop, in
    // production position.
    for accessor in &parts.accessors {
        if let Some(err) = accessor_path_error(accessor, parts.symbols_count) {
            return SpecValidationResult::Err(err);
        }
    }
    // 1g. validate_constants_symbols + validate_build_object_symbols
    // (workflow/mod.rs:2122-2147) — per-symbol detail modelled from
    // the caller-supplied `constants_symbols` and
    // `build_object_field_symbols` inventories, in production order:
    // the constant pool first, then the BuildObject fields.
    if let Some(err) = constants_symbols_error(parts) {
        return SpecValidationResult::Err(err);
    }
    if let Some(err) = build_object_symbols_error(parts) {
        return SpecValidationResult::Err(err);
    }
    // 1h. validate_reachability (workflow/mod.rs:2160-2229) —
    // every node must be reachable from the entry over routing
    // `next`, routing error handlers, and kind targets.
    if let Some(err) = reachability_error(parts) {
        return SpecValidationResult::Err(err);
    }
    // validate_forward_edges (workflow/mod.rs:2336-2364) —
    // routing and kind edges must point strictly forward (except
    // a RepeatCheck back-edge validated against its owning
    // RepeatStart), and loop spans must nest properly.
    if let Some(err) = forward_edges_error(parts) {
        return SpecValidationResult::Err(err);
    }
    // 1i. repeat_counter_ownership::validate
    // (repeat_counter_ownership.rs:6-12). Production runs this
    // inside validate_parts after validate_forward_edges and
    // before validate_no_nested_together: one counter slot per
    // RepeatStart with an output slot, and no other node may write
    // a claimed slot (the owner and its paired RepeatCheck are
    // exempt except for their routing error slot). The written
    // slots read the caller-supplied `written_slots` inventory;
    // a missing entry means the node writes no kind-specific slot.
    if let Some(err) = repeat_counter_ownership_error(parts) {
        return SpecValidationResult::Err(err);
    }
    // validate_no_nested_together (engine/validate.rs:119-172) —
    // no `TogetherStart` may be reachable from another
    // `TogetherStart`'s branch body.
    if let Some(err) = nested_together_error(parts) {
        return SpecValidationResult::Err(err);
    }
    // ---- Step 2: loop-region validation ----
    // Production calls loop_regions::validate (loop_regions.rs:59-72)
    // next, in this order: pairing (`cache_control` +
    // `unique_owner`, crates/vb_core/src/workflow/loop_regions/pairing.rs:8-50), region relations
    // (`validate_region_relations`, crates/vb_core/src/workflow/loop_regions/pairing.rs:52-71), then edge
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
    // Production calls WholeWorkflowBudget::compute (budget/compute.rs:100-106)
    // then BoundednessPolicy::DEFAULT.validate (budget/validate.rs:88-147);
    // validate_budget_result (workflow/mod.rs:1461-1470) collapses
    // any BudgetError to BudgetPolicyExceeded. The projection checks
    // the caller-supplied `computed_budget` inventory with
    // `budget_error`, which mirrors the 23 validated dimensions in
    // production first-violation order. The compute walk itself stays
    // caller-supplied: its boundedness is exhaustively covered by
    // `budget_bounded.rs`, not re-modelled here.
    if let Some(err) = budget_error(parts.computed_budget) {
        return SpecValidationResult::Err(err);
    }
    // ---- Step 5: input slot validation ----
    // Production calls validate_input_slots(&parts)? (crates/vb_core/src/workflow/mod.rs:47,
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
