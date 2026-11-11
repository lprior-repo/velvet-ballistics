// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for budget.rs (budget_bounded scope)
// ============================================================================
//
// This file is the WEAK (production_inner/) production mirror for the
// `budget_bounded.rs` Verus spec. It is a hand-written structural mirror
// of the production surface in `crates/vb_core/src/budget.rs` and its
// `budget/` submodules (`usage.rs`, `loop_body.rs`) with the
// following substitutions relative to direct `#[path]` inclusion of the
// production source:
//
//   1. Production `use thiserror::Error;` and `#[derive(... serde::Serialize,
//      serde::Deserialize ...)]` at budget/validate_error.rs:5 / aggregate.rs:10 are NOT re-included.
//      The mirror uses a `#[derive(Clone, Copy, ...)]` subset that
//      Verus 0.2026.05.05 can parse without proc-macro support. Field
//      NAMES and TYPES match production byte-for-byte, so any drift
//      in the budget.rs field names or discriminant shapes breaks
//      this mirror.
//
//   2. Production `#[error("...")]` thiserror attributes on BudgetError
//      variants are stripped; the discriminant set is preserved verbatim.
//      The error Display strings live only in production; the mirror
//      only carries the discriminant shape.
//
//   3. Production method bodies of `WholeWorkflowBudget::compute`,
//      `BoundednessPolicy::validate`, `validate_aggregate_budget`,
//      `validate_step_ceilings`, `add_dim`, `sub_dim`,
//      `aggregate_resource_usage_*` are all replaced by no-op
//      `loop {}` placeholders marked `#[verifier::external]`. Verus
//      skips body verification; the spec contracts attached via
//      `assume_specification` in the companion spec file
//      `budget_bounded.rs` state the production behavior the spec
//      proofs discharge.
//
// DRIFT POLICY: This file MUST be regenerated from
// `crates/vb_core/src/budget.rs` and its `budget/` submodules
// (`usage.rs`, `usage_arith.rs`, `loop_body.rs`, `aggregate.rs`, `aggregate_validate.rs`, `validate.rs`, `validate_error.rs`, `compute.rs`, `small_linear.rs`) whenever production changes. The
// mirror is annotated at the top of every section with the
// originating production line range so regeneration is mechanical.
//
// This file is included by the companion extern file
// (`verification/verus/extern_budget_bounded.rs`) via `#[path]` so the
// type declarations are nameable in spec mode. Each production
// method body is marked `#[verifier::external]` so the body is opaque
// while the signature participates in `assume_specification` binding
// in the companion spec file `budget_bounded.rs`.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `WholeWorkflowBudget`                  <- crates/vb_core/src/budget/compute.rs:17-93
// Production `crates/vb_core/src/budget/compute.rs:17-93`
//   - `WholeWorkflowBudget::compute`         <- crates/vb_core/src/budget/compute.rs:100-106
// Production `crates/vb_core/src/budget/compute.rs:100-106`
//   - `BoundednessPolicy`                    <- crates/vb_core/src/budget/validate.rs:9-56
// Production `crates/vb_core/src/budget/validate.rs:9-56`
//   - `BoundednessPolicy::validate`          <- crates/vb_core/src/budget/validate.rs:88-147
// Production `crates/vb_core/src/budget/validate.rs:88-147`
//   - `BudgetError`                          <- crates/vb_core/src/budget/validate_error.rs:10-57
//   - `AggregateResourceBudget`              <- crates/vb_core/src/budget/aggregate.rs:11-49
// Production `crates/vb_core/src/budget/aggregate.rs:11-49`
//   - `AggregateResourceBudget::from_workflow`  <- crates/vb_core/src/budget/aggregate_validate.rs:12-23
//   - `AggregateResourceBudget::from_whole_workflow_budget`
//                                            <- crates/vb_core/src/budget/aggregate_validate.rs:25-59
// Production `crates/vb_core/src/budget/aggregate_validate.rs:25-59`
//   - `AggregateResourceCapacity`            <- crates/vb_core/src/budget/aggregate.rs:53-94
// Production `crates/vb_core/src/budget/aggregate.rs:53-94`
//   - `AggregateResourceUsage`               <- crates/vb_core/src/budget/aggregate.rs:98-132
// Production `crates/vb_core/src/budget/aggregate.rs:98-132`
//   - `AggregateResourceUsage::try_add_budget`
//                                            <- crates/vb_core/src/budget/usage_arith.rs:9-123
// Production `crates/vb_core/src/budget/usage_arith.rs:9-123`
//   - `AggregateResourceUsage::try_subtract_budget`
//                                            <- crates/vb_core/src/budget/usage_arith.rs:125-239
// Production `crates/vb_core/src/budget/usage_arith.rs:125-239`
//   - `AggregateResourceUsage::fits_within`   <- crates/vb_core/src/budget/usage.rs:11-140
// Production `crates/vb_core/src/budget/usage.rs:11-140`
//   - `AggregateResourceUsage::check_policy` <- crates/vb_core/src/budget/usage.rs:145-236
// Production `crates/vb_core/src/budget/usage.rs:145-236`
//   - `validate_aggregate_budget`            <- crates/vb_core/src/budget/aggregate_validate.rs:109-243
// Production `crates/vb_core/src/budget/aggregate_validate.rs:109-243`
//   - `validate_step_ceilings`               <- crates/vb_core/src/budget.rs:24-59
//   - `add_dim`, `sub_dim`                   <- crates/vb_core/src/budget.rs:61-79
// Production `crates/vb_core/src/budget.rs:61-79`
//                                            (pure checked_add / checked_sub;
//                                             these are the pure decision fns
//                                             we exercise via `assume_specification`)
//   - `count_total_steps`                    <- crates/vb_core/src/budget/loop_body.rs:11-17
//                                            (pure DFS step counter; spec mirror is
//                                             `spec_count_total_steps_result` below)
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in this file are NOT verified by
// Verus. Each exec fn below is `#[verifier::external]` so Verus skips
// body verification, and the contracts attached via `assume_specification`
// in the companion spec file (`budget_bounded.rs`) state the production
// behavior the spec proofs discharge. Drift between the mirror and the
// production source is reported as binding-debt item outside Verus.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ============================================================================
// ID types — mirrors of `crates/vb_core/src/ids/mod.rs`
// ============================================================================
//
// The production `ids` module is a `macro_rules!`-generated family of newtype
// structs (RunId(u64), StepIdx(u16), SlotIdx(u16), ...). The mirror below
// replicates every type referenced by `budget.rs`. Each type exposes the
// same constructor / accessor surface the production code uses so a
// signature drift breaks this mirror.

/// Mirror of `RunId` (u64 newtype) at `crates/vb_core/src/ids/mod.rs:80`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RunId(pub u64);

impl RunId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Mirror of `StepIdx` (u16 newtype) at `crates/vb_core/src/ids/mod.rs:70`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct StepIdx(pub u16);

impl StepIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

/// Mirror of `SlotIdx` (u16 newtype) at `crates/vb_core/src/ids/mod.rs:71`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SlotIdx(pub u16);

impl SlotIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

// ============================================================================
// CompiledNodeKind — restricted mirror used by the spec contract surface
// ============================================================================
//
// The production `CompiledNodeKind` enum at
// `crates/vb_core/src/workflow/mod.rs:1553-1719` has 34 variants. The
// mirror below is the spec-side discriminant set the spec proofs reason
// about. We do NOT mirror every variant because the budget-bounded spec
// only needs the discriminant shape to refer to node-kind arms in the
// budget traversal decision fns (ForEachStart / CollectStart / etc.).
// This mirror is intentionally restricted; the production enum is
// referenced only by name in the spec contracts.

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub enum CompiledNodeKind {
    Nop,
    Do {
        action: u16,
        input: SlotIdx,
    },
    ForEachStart {
        input: SlotIdx,
        item_slot: SlotIdx,
        limit: u32,
        body: StepIdx,
        done: StepIdx,
    },
    CollectStart {
        source: SlotIdx,
        limit: u32,
        page_size: u32,
        body: StepIdx,
        done: StepIdx,
    },
    RepeatStart {
        max_attempts: u16,
        body: StepIdx,
        done: StepIdx,
    },
    Jump {
        target: StepIdx,
    },
    Choose {
        otherwise: Option<StepIdx>,
    },
    ChooseSlot {
        otherwise: Option<StepIdx>,
    },
    Finish,
    WaitUntil,
    WaitEvent,
    Ask,
    Other,
}

// ============================================================================
// ResourceContract — mirror of `crates/vb_core/src/workflow/mod.rs:258-295`
// ============================================================================

/// Mirror of `ResourceContract`. Field names and types match production
/// line-by-line so any drift in budget.rs use sites breaks the mirror.
#[derive(Clone, Copy, PartialEq, Eq)]
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

// ============================================================================
// WholeWorkflowBudget — production mirror
// ============================================================================
//
// Mirror of `crates/vb_core/src/budget/compute.rs:17-93`. Every field name, type,
// and visibility matches production exactly. Drift in any field breaks
// the mirror.

/// Mirror of production `WholeWorkflowBudget` at
/// `crates/vb_core/src/budget/compute.rs:17-93`. Field-by-field copy of the
/// production struct. The exec fn `WholeWorkflowBudget::compute` below
/// is `#[verifier::external]` and accepts the production signature.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WholeWorkflowBudget {
    pub max_total_steps: u64,
    pub max_total_slots: u64,
    pub max_fanout: u16,
    pub max_nesting_depth: u16,
    pub max_steps_executable: u32,
    pub max_action_tickets: u32,
    pub max_parallel_in_flight: u16,
    pub max_retries_per_action: u16,
    pub max_gather_pages: u32,
    pub max_gather_items: u32,
    pub max_for_each_iterations: u32,
    pub max_together_branches: u16,
    pub max_repeat_attempts: u16,
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

impl WholeWorkflowBudget {
    /// Production wrapper for `WholeWorkflowBudget::compute` at
    /// `crates/vb_core/src/budget/compute.rs:100-106`. Body skipped by Verus;
    /// contract attached via `assume_specification` in the spec file.
    #[verifier::external]
    pub fn compute(
        _nodes: &[super::workflow::CompiledNode],
        _entry: StepIdx,
        _contract: &ResourceContract,
    ) -> Result<Self, super::workflow::WorkflowError> {
        // Placeholder body — production logic lives in budget.rs and
        // is not re-verified here.
        loop {}
    }
}

// ============================================================================
// BoundednessPolicy — production mirror
// ============================================================================
//
// Mirror of `crates/vb_core/src/budget/validate.rs:9-56`.

/// Mirror of production `BoundednessPolicy` at
/// `crates/vb_core/src/budget/validate.rs:9-56`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BoundednessPolicy {
    pub max_total_steps: u64,
    pub max_total_slots: u64,
    pub max_fanout: u16,
    pub max_nesting_depth: u16,
    pub absolute_max_action_tickets: u32,
    pub absolute_max_parallel: u16,
    pub absolute_max_run_time_seconds: u64,
    pub absolute_max_result_bytes: u32,
    pub absolute_max_steps_executable: u32,
    pub absolute_max_timer_entries: u32,
    pub absolute_max_trace_events: u64,
    pub absolute_max_journal_batch_bytes: u32,
    pub absolute_max_queue_depth: u32,
    pub absolute_max_ipc_payload_bytes: u32,
    pub absolute_max_blob_bytes: u64,
    pub absolute_max_input_bytes: u32,
    pub absolute_max_open_gates: u32,
    pub absolute_max_signal_subscriptions: u32,
    pub absolute_max_incident_emissions: u64,
    pub absolute_max_transitions: u64,
    pub absolute_max_journal_bytes_total: u64,
    pub absolute_max_value_store_entries: u64,
    pub absolute_max_cost_units: u64,
}

impl BoundednessPolicy {
    /// Production constant `BoundednessPolicy::DEFAULT` at
    /// `crates/vb_core/src/budget/validate.rs:60-84`. Verus treats this as
    /// an external constant; the spec mirrors it via
    /// `spec_boundedness_policy_default` in the companion spec file.
    #[allow(non_upper_case_globals)]
    pub const DEFAULT: Self = Self {
        max_total_steps: 1_000_000,
        max_total_slots: 65_535,
        max_fanout: 64,
        max_nesting_depth: 8,
        absolute_max_action_tickets: 100_000,
        absolute_max_parallel: 256,
        absolute_max_run_time_seconds: 2_592_000,
        absolute_max_result_bytes: 262_144,
        absolute_max_steps_executable: 1_000_000,
        absolute_max_timer_entries: 1_000_000,
        absolute_max_trace_events: 1_000_000,
        absolute_max_journal_batch_bytes: 1_048_576,
        absolute_max_queue_depth: 1_024,
        absolute_max_ipc_payload_bytes: 1_048_576,
        absolute_max_blob_bytes: 16_777_216,
        absolute_max_input_bytes: 1_048_576,
        absolute_max_open_gates: 1_000_000,
        absolute_max_signal_subscriptions: 1_000_000,
        absolute_max_incident_emissions: 1_000_000,
        absolute_max_transitions: 10_000_000,
        absolute_max_journal_bytes_total: 1_048_576_000_000,
        absolute_max_value_store_entries: 2_000_000,
        absolute_max_cost_units: 100_000,
    };

    /// Production wrapper for `BoundednessPolicy::validate` at
    /// `crates/vb_core/src/budget/validate.rs:88-147`.
    #[verifier::external]
    pub fn validate(&self, _budget: &WholeWorkflowBudget) -> Result<(), BudgetError> {
        loop {}
    }
}

// ============================================================================
// BudgetError — production mirror (discriminant set)
// ============================================================================
//
// Mirror of `crates/vb_core/src/budget/validate_error.rs:10-57`. The full production
// enum has 23 variants, all mirrored here.

/// Mirror of production `BudgetError` discriminant set at
/// `crates/vb_core/src/budget/validate_error.rs:10-57`. All 23 production variants
/// are mirrored.
#[derive(Clone, Copy)]
pub enum BudgetError {
    TotalStepsExceeded { actual: u64, limit: u64 },
    TotalSlotsExceeded { actual: u64, limit: u64 },
    FanoutExceeded { actual: u16, limit: u16 },
    NestingDepthExceeded { actual: u16, limit: u16 },
    ActionTicketsExceeded { actual: u32, limit: u32 },
    ParallelExceeded { actual: u16, limit: u16 },
    RunTimeExceeded { actual: u64, limit: u64 },
    ResultBytesExceeded { actual: u32, limit: u32 },
    StepsExecutableExceeded { actual: u32, limit: u32 },
    TimerEntriesExceeded { actual: u32, limit: u32 },
    TraceEventsExceeded { actual: u64, limit: u64 },
    JournalBatchBytesExceeded { actual: u32, limit: u32 },
    QueueDepthExceeded { actual: u32, limit: u32 },
    IpcPayloadBytesExceeded { actual: u32, limit: u32 },
    BlobBytesExceeded { actual: u64, limit: u64 },
    InputBytesExceeded { actual: u32, limit: u32 },
    OpenGatesExceeded { actual: u32, limit: u32 },
    SignalSubscriptionsExceeded { actual: u32, limit: u32 },
    IncidentEmissionsExceeded { actual: u64, limit: u64 },
    TransitionsExceeded { actual: u64, limit: u64 },
    JournalBytesTotalExceeded { actual: u64, limit: u64 },
    ValueStoreEntriesExceeded { actual: u64, limit: u64 },
    CostUnitsExceeded { actual: u64, limit: u64 },
}

// ============================================================================
// Aggregate resource types — production mirror
// ============================================================================
//
// Mirror of `crates/vb_core/src/budget/aggregate.rs:11-282`. These types are the
// runtime-admission surface that the boundedness policy validates
// against.

/// Mirror of production `AggregateResourceBudget` at
/// `crates/vb_core/src/budget/aggregate.rs:11-49`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AggregateResourceBudget {
    pub max_steps_executable: u32,
    pub max_action_tickets: u32,
    pub max_parallel_in_flight: u16,
    pub max_retries_per_action: u16,
    pub max_gather_pages: u32,
    pub max_gather_items: u32,
    pub max_for_each_iterations: u32,
    pub max_together_branches: u16,
    pub max_repeat_attempts: u16,
    pub max_run_time_seconds: u64,
    pub max_result_bytes: u32,
    pub max_total_slots_written: u32,
    pub max_timer_entries: u32,
    pub max_trace_events: u64,
    pub max_queue_depth: u32,
    pub max_journal_batch_bytes: u32,
    pub max_ipc_payload_bytes: u32,
    pub max_blob_bytes: u64,
    pub max_input_bytes: u32,
    pub max_step_budget_per_tick: u64,
    pub max_transitions_per_tick: u64,
    pub max_open_gates: u32,
    pub max_signal_subscriptions: u32,
    pub max_incident_emissions: u64,
    pub max_transitions: u64,
    pub max_journal_bytes_total: u64,
    pub max_value_store_entries: u64,
    pub max_cost_units: u64,
}

/// Mirror of production `AggregateResourceCapacity` at
/// `crates/vb_core/src/budget/aggregate.rs:53-94`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AggregateResourceCapacity {
    pub max_steps_executable: u64,
    pub max_action_tickets: u64,
    pub max_parallel_in_flight: u32,
    pub max_gather_pages: u64,
    pub max_gather_items: u64,
    pub max_result_bytes: u64,
    pub max_total_slots_written: u64,
    pub max_timer_entries: u64,
    pub max_trace_events: u64,
    pub max_active_runs: u64,
    pub max_queue_depth: u64,
    pub max_journal_batch_bytes: u64,
    pub max_ipc_payload_bytes: u64,
    pub max_blob_bytes: u64,
    pub max_input_bytes: u64,
    pub max_step_budget_per_tick: u64,
    pub max_transitions_per_tick: u64,
    pub max_open_gates: u64,
    pub max_signal_subscriptions: u64,
    pub max_incident_emissions: u64,
    pub max_transitions: u64,
    pub max_journal_bytes_total: u64,
    pub max_value_store_entries: u64,
    pub max_cost_units: u64,
    pub tenant_quota_cost_units: u64,
    pub global_quota_cost_units: u64,
}

/// Mirror of production `AggregateResourceUsage` at
/// `crates/vb_core/src/budget/aggregate.rs:98-132`.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct AggregateResourceUsage {
    pub max_steps_executable: u64,
    pub max_action_tickets: u64,
    pub max_parallel_in_flight: u64,
    pub max_gather_pages: u64,
    pub max_gather_items: u64,
    pub max_result_bytes: u64,
    pub max_total_slots_written: u64,
    pub max_timer_entries: u64,
    pub max_trace_events: u64,
    pub max_active_runs: u64,
    pub max_queue_depth: u64,
    pub max_journal_batch_bytes: u64,
    pub max_ipc_payload_bytes: u64,
    pub max_blob_bytes: u64,
    pub max_input_bytes: u64,
    pub max_step_budget_per_tick: u64,
    pub max_transitions_per_tick: u64,
    pub max_open_gates: u64,
    pub max_signal_subscriptions: u64,
    pub max_incident_emissions: u64,
    pub max_transitions: u64,
    pub max_journal_bytes_total: u64,
    pub max_value_store_entries: u64,
    pub max_cost_units: u64,
}

// ============================================================================
// AggregateBudgetError — production mirror (restricted variants)
// ============================================================================
//
// Mirror of `crates/vb_core/src/budget/aggregate.rs:208-277`.

/// Mirror of production `AggregateBudgetError` at
/// `crates/vb_core/src/budget/aggregate.rs:208-277`. All 9 production variants
/// are mirrored.
#[derive(Clone, Copy)]
pub enum AggregateBudgetError {
    PolicyExceeded {
        resource: &'static str,
        actual: u64,
        limit: u64,
    },
    CapacityExceeded {
        resource: &'static str,
        requested: u64,
        available: u64,
    },
    Overflow {
        resource: &'static str,
    },
    Underflow {
        resource: &'static str,
    },
    StepCeilingExceeded {
        requested: u64,
        limit: u64,
    },
    PerTickCeilingExceeded {
        requested: u64,
        limit: u64,
    },
    ReservationNotFound {
        run: RunId,
    },
    WorkflowBudget,
    InvalidCapacity {
        resource: &'static str,
    },
}

// ============================================================================
// Extern fns — `#[verifier::external]` wrappers mirroring production
// signatures
// ============================================================================
//
// These exec fns re-export the production decision logic. Verus skips
// body verification (the bodies are no-op `loop {}`); the actual
// contracts are attached via `assume_specification` in the companion
// spec file (`budget_bounded.rs`).

/// Production wrapper for `WholeWorkflowBudget::compute` at
/// `crates/vb_core/src/budget/compute.rs:100-106`. Body skipped by Verus.
#[verifier::external]
pub fn whole_workflow_budget_compute(
    nodes: &[super::workflow::CompiledNode],
    entry: StepIdx,
    contract: &ResourceContract,
) -> Result<WholeWorkflowBudget, super::workflow::WorkflowError> {
    WholeWorkflowBudget::compute(nodes, entry, contract)
}

/// Production wrapper for `BoundednessPolicy::validate` at
/// `crates/vb_core/src/budget/validate.rs:88-147`. Body skipped by Verus.
#[verifier::external]
pub fn boundedness_policy_validate(
    policy: &BoundednessPolicy,
    budget: &WholeWorkflowBudget,
) -> Result<(), BudgetError> {
    policy.validate(budget)
}

/// Production wrapper for `validate_aggregate_budget` at
/// `crates/vb_core/src/budget/aggregate_validate.rs:109-243`. Body skipped by Verus.
#[verifier::external]
pub fn validate_aggregate_budget(
    budget: &AggregateResourceBudget,
    policy: &BoundednessPolicy,
) -> Result<(), AggregateBudgetError> {
    // Production signature is `pub fn validate_aggregate_budget(...)`
    // and returns `Result<(), AggregateBudgetError>`.
    let _ = (budget, policy);
    loop {}
}

/// Production wrapper for `validate_step_ceilings` at
/// `crates/vb_core/src/budget.rs:24-59`. Body is a faithful copy
/// of the production body; the local hard-limit constants are part
/// of that body. Body skipped by Verus.
#[verifier::external]
pub fn validate_step_ceilings(
    budget: &AggregateResourceBudget,
) -> Result<(), AggregateBudgetError> {
    const HARD_MAX_STEP_BUDGET_PER_TICK: u64 = 1_000_000;
    const HARD_MAX_TRANSITIONS_PER_TICK: u64 = 1_000_000;

    if budget.max_step_budget_per_tick == 0 {
        return Err(AggregateBudgetError::StepCeilingExceeded {
            requested: 0,
            limit: HARD_MAX_STEP_BUDGET_PER_TICK,
        });
    }
    if budget.max_step_budget_per_tick > HARD_MAX_STEP_BUDGET_PER_TICK {
        return Err(AggregateBudgetError::StepCeilingExceeded {
            requested: budget.max_step_budget_per_tick,
            limit: HARD_MAX_STEP_BUDGET_PER_TICK,
        });
    }

    if budget.max_transitions_per_tick == 0 {
        return Err(AggregateBudgetError::PerTickCeilingExceeded {
            requested: 0,
            limit: HARD_MAX_TRANSITIONS_PER_TICK,
        });
    }
    if budget.max_transitions_per_tick > HARD_MAX_TRANSITIONS_PER_TICK {
        return Err(AggregateBudgetError::PerTickCeilingExceeded {
            requested: budget.max_transitions_per_tick,
            limit: HARD_MAX_TRANSITIONS_PER_TICK,
        });
    }

    Ok(())
}

/// Production wrapper for `add_dim` at
/// `crates/vb_core/src/budget.rs:61-69`. Pure checked_add — this
/// is a pure decision fn and the spec attaches an `assume_specification`
/// contract in `budget_bounded.rs`.
#[verifier::external]
pub fn add_dim(
    current: u64,
    requested: u64,
    resource: &'static str,
) -> Result<u64, AggregateBudgetError> {
    current
        .checked_add(requested)
        .ok_or(AggregateBudgetError::Overflow { resource })
}

/// Production wrapper for `sub_dim` at
/// `crates/vb_core/src/budget.rs:71-79`. Pure checked_sub — pure
/// decision fn with `assume_specification` contract in the spec.
#[verifier::external]
pub fn sub_dim(
    current: u64,
    requested: u64,
    resource: &'static str,
) -> Result<u64, AggregateBudgetError> {
    current
        .checked_sub(requested)
        .ok_or(AggregateBudgetError::Underflow { resource })
}

/// Production wrapper for `AggregateResourceUsage::try_add_budget` at
/// `crates/vb_core/src/budget/usage_arith.rs:9-123`.
#[verifier::external]
pub fn aggregate_resource_usage_try_add_budget(
    usage: &AggregateResourceUsage,
    budget: &AggregateResourceBudget,
) -> Result<AggregateResourceUsage, AggregateBudgetError> {
    let _ = (usage, budget);
    loop {}
}

/// Production wrapper for `AggregateResourceUsage::try_subtract_budget`
/// at `crates/vb_core/src/budget/usage_arith.rs:125-239`.
#[verifier::external]
pub fn aggregate_resource_usage_try_subtract_budget(
    usage: &AggregateResourceUsage,
    budget: &AggregateResourceBudget,
) -> Result<AggregateResourceUsage, AggregateBudgetError> {
    let _ = (usage, budget);
    loop {}
}

/// Production wrapper for `AggregateResourceUsage::fits_within` at
/// `crates/vb_core/src/budget/usage.rs:11-140`.
#[verifier::external]
pub fn aggregate_resource_usage_fits_within(
    usage: &AggregateResourceUsage,
    capacity: &AggregateResourceCapacity,
) -> Result<(), AggregateBudgetError> {
    let _ = (usage, capacity);
    loop {}
}

/// Production wrapper for `AggregateResourceUsage::check_policy` at
/// `crates/vb_core/src/budget/usage.rs:145-236`.
#[verifier::external]
pub fn aggregate_resource_usage_check_policy(
    usage: &AggregateResourceUsage,
    policy: &BoundednessPolicy,
) -> Result<(), AggregateBudgetError> {
    let _ = (usage, policy);
    loop {}
}

// ============================================================================
// Companion namespace `crate::workflow` shim
// ============================================================================
//
// This provides the namespace for the budget.rs use sites
// (`use crate::workflow::CompiledNode` etc.). It is a marker type —
// production bodies are not re-verified inside Verus.
//
// `pub mod limits { ... }` was previously declared here to mirror the
// production constant `MAX_LIST_ITEMS_PER_VALUE = 65_535` at
// `crates/vb_core/src/limits.rs:71`. It has been removed because
// declaring a `pub const` in this mirror triggers a Verus internal
// error (`VerusErasureCtxt has not been initialized`) on the
// `--crate-type=lib` invocation that does NOT pass `--no-lifetime`.
// The constant now lives in the spec file's `verus!` block at
// `verification/verus/budget_bounded.rs` alongside the four
// `SPEC_MAX_*` constants. See the binding-ledger comment there for
// the production source line and the drift detection rationale.

pub mod workflow {
    use super::{SlotIdx, StepIdx};

    /// Mirror of production `CompiledNode` at
    /// `crates/vb_core/src/workflow/mod.rs:1357-1366`. Every field name,
    /// type, and visibility matches production exactly. Drift in any
    /// field breaks the mirror.
    #[derive(Clone, Copy)]
    pub struct CompiledNode {
        pub id: StepIdx,
        pub output: NodeOutput,
        pub routing: NodeRouting,
        pub kind: super::CompiledNodeKind,
    }

    /// Mirror of production `NodeOutput` at
    /// `crates/vb_core/src/workflow/mod.rs:1008-1013`.
    #[derive(Clone, Copy)]
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
    #[derive(Clone, Copy)]
    pub struct NodeRouting {
        pub next: NodeContinuation,
        pub error: ErrorRouting,
    }

    /// Mirror of production `NodeContinuation` at
    /// `crates/vb_core/src/workflow/mod.rs:1037-1042`.
    #[derive(Clone, Copy)]
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
    #[derive(Clone, Copy)]
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

    /// Marker mirror of production `WorkflowError` discriminant set at
    /// `crates/vb_core/src/workflow/mod.rs:1028-1242`. Restricted to the
    /// variants referenced by budget.rs.
    #[derive(Clone, Copy)]
    pub enum WorkflowError {
        EntryOutOfBounds { entry: StepIdx },
        StepOutOfBounds { step: StepIdx },
        StepCountOverflow { actual: u64 },
        DepthOverflow { depth: u16 },
        JumpCycle { step: StepIdx, target: StepIdx },
        EmptyNodes,
        SlotOutOfBounds,
        ConstOutOfBounds,
        NodeIdMismatch { expected: StepIdx, actual: StepIdx },
        Expression,
        ResourceContractExceeded,
        ResourceContractTooLarge,
        EmptyBranchTable,
        UnreachableNode { step: StepIdx },
        BackwardEdge { from: StepIdx, to: StepIdx },
        ImproperLoopNesting { inner: StepIdx, outer_done: StepIdx },
        BudgetPolicyExceeded { detail: &'static str },
        Other,
    }
}

// ============================================================================
// Spec-side mirror of the budget limit constants referenced by spec
// ============================================================================
//
// The four `SPEC_MAX_*` constants (SPEC_MAX_STEPS_PER_WORKFLOW,
// SPEC_MAX_STEP_BUDGET, SPEC_MAX_PARALLEL_IN_FLIGHT,
// SPEC_MAX_ACTION_TICKETS) AND the `MAX_LIST_ITEMS_PER_VALUE`
// constant are declared inside the spec file's `verus!` block, NOT
// here. Declaring a `pub const` in this extern file triggers a Verus
// internal error (`VerusErasureCtxt has not been initialized`) on
// the `--crate-type=lib` invocation that does NOT pass `--no-lifetime`;
// the spec file mirrors each constant with the same value and a
// binding-ledger comment that cites the production source line. This
// matches the established workaround used in
// `extern_signals_try_take.rs`, `extern_signals_invariant.rs`,
// `extern_vb_vzcuf_PS_006.rs`, and `extern_step_state_machine.rs`.
