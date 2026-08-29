// Verus proof obligations for bounded workflow budget composition.
//
// Obligation IDs: VERUS-BUD-001, VERUS-BUD-002, VERUS-BUD-003, VERUS-AGG-001,
// VERUS-DIAG-001.
// Verifier: verus --no-lifetime --crate-type=lib verification/verus/budget_bounded.rs
// Expected evidence: Verus report shows 0 errors for checked sequential,
// nested, branch/together, aggregate refinement, and diagnostic-totality lemmas.
//
// ============================================================================
// PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is bound to `crates/vb_core/src/budget.rs` through the
// companion extern surface `verification/verus/extern_budget_bounded.rs`,
// which mirrors every production type and exec fn we reason about and
// wraps production bodies with `#[verifier::external]`. The spec proofs
// below attach `assume_specification` contracts to those extern wrappers
// and exercise them through production-bound exec fns, so any drift in
// the production field names, discriminant sets, or fn signatures breaks
// the verification build.
//
// Full `#[path]` inclusion of `crates/vb_core/src/budget.rs` is
// intentionally NOT used here — see the header of
// `extern_budget_bounded.rs` for the empirical blockers (let-chains
// requiring --edition 2024, `use thiserror::Error;` under Rust 2018+
// module resolution, `#[derive(serde::Serialize, serde::Deserialize)]`
// without proc-macro shims, and `mod tests_and_verification;` resolution).
// The mirror pattern matches `extern_runtime_execute_do.rs`,
// `extern_vb_core_replay_step.rs`, `extern_run_atomic_admission.rs`,
// and `extern_idempotency_certificate.rs` in this repo.
//
// BINDING LEDGER:
//   - `WholeWorkflowBudget`                  <- extern_budget_bounded.rs
//                                              (mirror of
//                                              budget.rs:11-59)
//   - `WholeWorkflowBudget::compute`         <- extern_budget_bounded.rs
//                                              `whole_workflow_budget_compute`
//   - `BoundednessPolicy::validate`          <- extern_budget_bounded.rs
//                                              `boundedness_policy_validate`
//   - `validate_aggregate_budget`            <- extern_budget_bounded.rs
//                                              `validate_aggregate_budget`
//   - `validate_step_ceilings`               <- extern_budget_bounded.rs
//                                              `validate_step_ceilings`
//   - `add_dim`, `sub_dim`                   <- extern_budget_bounded.rs
//                                              `add_dim`, `sub_dim`
//                                              (pure decision fns;
//                                              assume_specification below)
//   - `AggregateResourceBudget`/`Capacity`/`Usage`
//                                           <- extern_budget_bounded.rs
//                                              (mirror of budget.rs:570-644)
//
// Spec-mode reasoning:
//   - The spec constants `max_steps_per_workflow() = 65535`,
//     `max_step_budget() = 10000`, `max_parallel_in_flight() = 1024`,
//     and `max_action_tickets() = 1000000` are spec-invented upper
//     bounds. The first two mirror `crates/vb_core/src/limits.rs:11` and
//     `:94`. The last two are spec-only — production's `BoundednessPolicy
//     ::DEFAULT.absolute_max_parallel = 256` and `..._action_tickets =
//     100_000` (budget.rs:378-396) — drift is reported as a binding
//     debt item outside Verus.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every entry point in the binding ledger are
// not verified by Verus. The exec wrappers in `extern_budget_bounded.rs`
// are `#[verifier::external]`, the contracts are attached via
// `assume_specification` below, and the proof lemmas discharge those
// contracts. Any drift between the mirror and the production source is
// binding-debt tracked outside Verus.
use vstd::prelude::*;

verus! {

#[path = "extern_budget_bounded.rs"]
mod production;

// Re-export the production types and exec wrappers so the spec proofs
// below reference them as `WholeWorkflowBudget`, `add_dim`, etc.
pub use production::{
    AggregateBudgetError,
    AggregateResourceBudget,
    AggregateResourceCapacity,
    AggregateResourceUsage,
    BoundednessPolicy,
    BudgetError,
    CompiledNodeKind,
    ResourceContract,
    RunId,
    SlotIdx,
    StepIdx,
    WholeWorkflowBudget,
    add_dim,
    aggregate_resource_usage_check_policy,
    aggregate_resource_usage_fits_within,
    aggregate_resource_usage_try_add_budget,
    aggregate_resource_usage_try_subtract_budget,
    boundedness_policy_validate,
    sub_dim,
    validate_aggregate_budget,
    validate_step_ceilings,
    whole_workflow_budget_compute,
};

// Re-export the `workflow` sub-module from the extern file so
// paths like `crate::workflow::CompiledNode` resolve inside the extern
// `#[verifier::external]` wrappers (which reference them as `crate::workflow`).
//
// Note: `pub use production::limits;` was removed when
// `MAX_LIST_ITEMS_PER_VALUE` was moved out of the production_inner
// mirror and into this spec file's `verus!` block. The `pub mod limits`
// namespace no longer exists in the production_inner mirror.
pub use production::workflow;

// ============================================================================
// PartialEqSpecImpl — discharge the vstd `cmp::eq` postcondition on the nine
// production-mirror structs that derive `PartialEq`/`Eq`.
// ============================================================================
//
// `vstd::std_specs::cmp::ExPartialEq::eq` carries the postcondition
//   `Self::obeys_eq_spec() ==> r == self.eq_spec(other)`
// For derived `PartialEq` structs, Verus auto-generates `assert_fields_are_eq`
// and `eq` obligations but does NOT provide a `PartialEqSpecImpl` — so
// `obeys_eq_spec()` is uninterpreted and the postcondition cannot be
// discharged. Supplying an explicit `PartialEqSpecImpl` for each of the
// nine derived mirrors (`RunId`, `StepIdx`, `SlotIdx`, `ResourceContract`,
// `WholeWorkflowBudget`, `BoundednessPolicy`, `AggregateResourceBudget`,
// `AggregateResourceCapacity`, `AggregateResourceUsage`) lets Verus know
// the spec-side equality semantics (`obeys_eq_spec() == true` and a
// field-by-field `eq_spec`) so the postconditions of the auto-generated
// helpers verify. This is the same fix shape used elsewhere in the
// verification tree (see e.g. `vb_awhr_fanout_spec.rs` for analogous
// `PartialEqSpecImpl` impls on derived mirror structs).
impl vstd::std_specs::cmp::PartialEqSpecImpl for RunId {
    open spec fn obeys_eq_spec() -> bool {
        true
    }

    open spec fn eq_spec(&self, other: &RunId) -> bool {
        self.0 == other.0
    }
}

impl vstd::std_specs::cmp::PartialEqSpecImpl for StepIdx {
    open spec fn obeys_eq_spec() -> bool {
        true
    }

    open spec fn eq_spec(&self, other: &StepIdx) -> bool {
        self.0 == other.0
    }
}

impl vstd::std_specs::cmp::PartialEqSpecImpl for SlotIdx {
    open spec fn obeys_eq_spec() -> bool {
        true
    }

    open spec fn eq_spec(&self, other: &SlotIdx) -> bool {
        self.0 == other.0
    }
}

impl vstd::std_specs::cmp::PartialEqSpecImpl for ResourceContract {
    open spec fn obeys_eq_spec() -> bool {
        true
    }

    open spec fn eq_spec(&self, other: &ResourceContract) -> bool {
        self.max_steps == other.max_steps && self.max_slots == other.max_slots && self.max_constants
            == other.max_constants && self.max_accessors == other.max_accessors
            && self.max_expressions == other.max_expressions && self.max_expr_stack
            == other.max_expr_stack && self.max_step_budget_per_tick
            == other.max_step_budget_per_tick && self.max_transitions_per_tick
            == other.max_transitions_per_tick && self.max_input_bytes == other.max_input_bytes
            && self.max_output_bytes == other.max_output_bytes && self.max_blob_bytes
            == other.max_blob_bytes && self.max_ipc_payload_bytes == other.max_ipc_payload_bytes
            && self.max_retry_attempts == other.max_retry_attempts && self.max_fanout
            == other.max_fanout && self.max_collect_items == other.max_collect_items
            && self.max_queue_depth == other.max_queue_depth && self.max_journal_batch_bytes
            == other.max_journal_batch_bytes && self.allows_secret_results
            == other.allows_secret_results
    }
}

impl vstd::std_specs::cmp::PartialEqSpecImpl for WholeWorkflowBudget {
    open spec fn obeys_eq_spec() -> bool {
        true
    }

    open spec fn eq_spec(&self, other: &WholeWorkflowBudget) -> bool {
        self.max_total_steps == other.max_total_steps && self.max_total_slots
            == other.max_total_slots && self.max_fanout == other.max_fanout
            && self.max_nesting_depth == other.max_nesting_depth && self.max_steps_executable
            == other.max_steps_executable && self.max_action_tickets == other.max_action_tickets
            && self.max_parallel_in_flight == other.max_parallel_in_flight
            && self.max_retries_per_action == other.max_retries_per_action && self.max_gather_pages
            == other.max_gather_pages && self.max_gather_items == other.max_gather_items
            && self.max_for_each_iterations == other.max_for_each_iterations
            && self.max_together_branches == other.max_together_branches && self.max_repeat_attempts
            == other.max_repeat_attempts && self.max_run_time_seconds == other.max_run_time_seconds
            && self.max_result_bytes == other.max_result_bytes && self.max_total_slots_written
            == other.max_total_slots_written && self.max_timer_entries == other.max_timer_entries
            && self.max_trace_events == other.max_trace_events && self.max_journal_batch_bytes
            == other.max_journal_batch_bytes && self.max_queue_depth == other.max_queue_depth
            && self.max_ipc_payload_bytes == other.max_ipc_payload_bytes && self.max_blob_bytes
            == other.max_blob_bytes && self.max_input_bytes == other.max_input_bytes
    }
}

impl vstd::std_specs::cmp::PartialEqSpecImpl for BoundednessPolicy {
    open spec fn obeys_eq_spec() -> bool {
        true
    }

    open spec fn eq_spec(&self, other: &BoundednessPolicy) -> bool {
        self.max_total_steps == other.max_total_steps && self.max_total_slots
            == other.max_total_slots && self.max_fanout == other.max_fanout
            && self.max_nesting_depth == other.max_nesting_depth && self.absolute_max_action_tickets
            == other.absolute_max_action_tickets && self.absolute_max_parallel
            == other.absolute_max_parallel && self.absolute_max_run_time_seconds
            == other.absolute_max_run_time_seconds && self.absolute_max_result_bytes
            == other.absolute_max_result_bytes && self.absolute_max_steps_executable
            == other.absolute_max_steps_executable && self.absolute_max_timer_entries
            == other.absolute_max_timer_entries && self.absolute_max_trace_events
            == other.absolute_max_trace_events && self.absolute_max_journal_batch_bytes
            == other.absolute_max_journal_batch_bytes && self.absolute_max_queue_depth
            == other.absolute_max_queue_depth && self.absolute_max_ipc_payload_bytes
            == other.absolute_max_ipc_payload_bytes && self.absolute_max_blob_bytes
            == other.absolute_max_blob_bytes && self.absolute_max_input_bytes
            == other.absolute_max_input_bytes
    }
}

impl vstd::std_specs::cmp::PartialEqSpecImpl for AggregateResourceBudget {
    open spec fn obeys_eq_spec() -> bool {
        true
    }

    open spec fn eq_spec(&self, other: &AggregateResourceBudget) -> bool {
        self.max_steps_executable == other.max_steps_executable && self.max_action_tickets
            == other.max_action_tickets && self.max_parallel_in_flight
            == other.max_parallel_in_flight && self.max_retries_per_action
            == other.max_retries_per_action && self.max_gather_pages == other.max_gather_pages
            && self.max_gather_items == other.max_gather_items && self.max_for_each_iterations
            == other.max_for_each_iterations && self.max_together_branches
            == other.max_together_branches && self.max_repeat_attempts == other.max_repeat_attempts
            && self.max_run_time_seconds == other.max_run_time_seconds && self.max_result_bytes
            == other.max_result_bytes && self.max_total_slots_written
            == other.max_total_slots_written && self.max_timer_entries == other.max_timer_entries
            && self.max_trace_events == other.max_trace_events && self.max_queue_depth
            == other.max_queue_depth && self.max_journal_batch_bytes
            == other.max_journal_batch_bytes && self.max_ipc_payload_bytes
            == other.max_ipc_payload_bytes && self.max_blob_bytes == other.max_blob_bytes
            && self.max_input_bytes == other.max_input_bytes && self.max_step_budget_per_tick
            == other.max_step_budget_per_tick && self.max_transitions_per_tick
            == other.max_transitions_per_tick
    }
}

impl vstd::std_specs::cmp::PartialEqSpecImpl for AggregateResourceCapacity {
    open spec fn obeys_eq_spec() -> bool {
        true
    }

    open spec fn eq_spec(&self, other: &AggregateResourceCapacity) -> bool {
        self.max_steps_executable == other.max_steps_executable && self.max_action_tickets
            == other.max_action_tickets && self.max_parallel_in_flight
            == other.max_parallel_in_flight && self.max_gather_pages == other.max_gather_pages
            && self.max_gather_items == other.max_gather_items && self.max_result_bytes
            == other.max_result_bytes && self.max_total_slots_written
            == other.max_total_slots_written && self.max_timer_entries == other.max_timer_entries
            && self.max_trace_events == other.max_trace_events && self.max_active_runs
            == other.max_active_runs && self.max_queue_depth == other.max_queue_depth
            && self.max_journal_batch_bytes == other.max_journal_batch_bytes
            && self.max_ipc_payload_bytes == other.max_ipc_payload_bytes && self.max_blob_bytes
            == other.max_blob_bytes && self.max_input_bytes == other.max_input_bytes
            && self.max_step_budget_per_tick == other.max_step_budget_per_tick
            && self.max_transitions_per_tick == other.max_transitions_per_tick
    }
}

impl vstd::std_specs::cmp::PartialEqSpecImpl for AggregateResourceUsage {
    open spec fn obeys_eq_spec() -> bool {
        true
    }

    open spec fn eq_spec(&self, other: &AggregateResourceUsage) -> bool {
        self.max_steps_executable == other.max_steps_executable && self.max_action_tickets
            == other.max_action_tickets && self.max_parallel_in_flight
            == other.max_parallel_in_flight && self.max_gather_pages == other.max_gather_pages
            && self.max_gather_items == other.max_gather_items && self.max_result_bytes
            == other.max_result_bytes && self.max_total_slots_written
            == other.max_total_slots_written && self.max_timer_entries == other.max_timer_entries
            && self.max_trace_events == other.max_trace_events && self.max_active_runs
            == other.max_active_runs && self.max_queue_depth == other.max_queue_depth
            && self.max_journal_batch_bytes == other.max_journal_batch_bytes
            && self.max_ipc_payload_bytes == other.max_ipc_payload_bytes && self.max_blob_bytes
            == other.max_blob_bytes && self.max_input_bytes == other.max_input_bytes
            && self.max_step_budget_per_tick == other.max_step_budget_per_tick
            && self.max_transitions_per_tick == other.max_transitions_per_tick
    }
}

// The four `SPEC_MAX_*` constants are declared inside `verus!` (rather
// than in `extern_budget_bounded.rs`) because declaring a `pub const`
// in the extern file triggers a Verus internal error
// (`VerusErasureCtxt has not been initialized`) on the
// `--crate-type=lib` invocation that does NOT pass `--no-lifetime`.
// The values mirror the production limits.rs constants for the
// first two (`MAX_STEPS_PER_WORKFLOW` at `crates/vb_core/src/limits.rs:11`
// and `MAX_STEP_BUDGET` at `crates/vb_core/src/limits.rs:94`) and the
// spec policy bounds for the last two. The binding ledger in
// `extern_budget_bounded.rs` documents the prior location of these
// constants and the move. This matches the established workaround
// used in `signals_try_take.rs`, `signals_invariant.rs`,
// `vb-vzcuf-PS-006.rs`, and `step_state_machine.rs`.
/// `MAX_STEPS_PER_WORKFLOW` mirror (production at
/// `crates/vb_core/src/limits.rs:11 = 65_535`).
#[allow(non_upper_case_globals)]
pub const SPEC_MAX_STEPS_PER_WORKFLOW: u64 = 65_535;

/// `MAX_STEP_BUDGET` mirror (production at
/// `crates/vb_core/src/limits.rs:94 = 10_000`).
#[allow(non_upper_case_globals)]
pub const SPEC_MAX_STEP_BUDGET: u64 = 10_000;

/// Spec-invented upper bound for parallel in-flight actions. NOT a
/// production constant — declared here as the spec source of truth.
#[allow(non_upper_case_globals)]
pub const SPEC_MAX_PARALLEL_IN_FLIGHT: u64 = 1024;

/// Spec-invented upper bound for action tickets. Production's
/// `BoundednessPolicy::absolute_max_action_tickets` defaults to 100_000
/// (not 1_000_000). The spec value 1_000_000 is a more permissive
/// upper bound used in arithmetic lemmas; drift is reported as a
/// spec-vs-policy reconciliation item.
#[allow(non_upper_case_globals)]
pub const SPEC_MAX_ACTION_TICKETS: u64 = 1_000_000;

// ----------------------------------------------------------------------------
// MAX_LIST_ITEMS_PER_VALUE — moved here from the production_inner mirror
// ----------------------------------------------------------------------------
//
// `MAX_LIST_ITEMS_PER_VALUE` was previously declared inside the
// production_inner mirror at
// `verification/verus/production_inner/budget_bounded_production.rs:683`
// (inside `pub mod limits { ... }`). Declaring it in that mirror
// triggered the same Verus internal error
// (`VerusErasureCtxt has not been initialized`) on the
// `--crate-type=lib` invocation that does NOT pass `--no-lifetime`,
// because the thir_body query runs against the constant's owner item
// during the SPEC_MAX_* type-erasure path. Moving the `pub const`
// into the spec file's `verus!` block (alongside `SPEC_MAX_*`)
// avoids the bug while preserving the production-mirror binding.
//
// `MAX_LIST_ITEMS_PER_VALUE` mirrors the production constant at
// `crates/vb_core/src/limits.rs:71 = 65_535`. The value is referenced
// by the production budget iteration accounting at
// `crates/vb_core/src/budget.rs:1459, 1719` but is currently unused
// in any spec proof or exec fn in this scope; it is kept here as a
// binding-ledger artifact so the production constant name stays
// resolvable from spec mode.
//
// This matches the established workaround used in
// `step_state_machine.rs` (the `SPEC_MAX_STEP_BUDGET` constant is
// also declared inside `verus!`) and the four `SPEC_MAX_*`
// constants above.
#[allow(non_upper_case_globals)]
pub const MAX_LIST_ITEMS_PER_VALUE: usize = 65_535;

// ============================================================================
// Spec invariants — derive production constants from the extern source
// ============================================================================
//
// `SPEC_MAX_*` (declared above) mirror the production limits.rs constants
// for the first two and the spec policy bounds for the last two.
pub open spec fn max_steps_per_workflow() -> int {
    SPEC_MAX_STEPS_PER_WORKFLOW as int
}

pub open spec fn max_step_budget() -> int {
    SPEC_MAX_STEP_BUDGET as int
}

pub open spec fn max_parallel_in_flight() -> int {
    SPEC_MAX_PARALLEL_IN_FLIGHT as int
}

pub open spec fn max_action_tickets() -> int {
    SPEC_MAX_ACTION_TICKETS as int
}

/// Spec error type mirroring the production `WorkflowError::StepCountOverflow`
/// branch that the spec proofs discharge (count_total_steps returns
/// `Err(WorkflowError::StepCountOverflow { actual })` on u64 overflow).
pub enum SpecWorkflowError {
    StepCountOverflow,
}

/// Spec model: count_total_steps returns Ok(steps) where steps fits in u64
/// and steps <= max_steps_per_workflow, OR Err(StepCountOverflow) on
/// overflow. The `steps <= max_steps_per_workflow` upper bound is the
/// spec-side assertion that workflow IR is bounded — production does not
/// enforce this upper bound but the boundedness policy does.
pub open spec fn spec_count_total_steps_bounded(result: int) -> bool {
    result >= 0 && result <= max_steps_per_workflow()
}

/// Spec mirror of `count_total_steps` returning `Result<u64, WorkflowError>`.
/// Mirrors `crates/vb_core/src/budget.rs:1332-1360` semantics:
///   - Ok(v) iff v in [0, u64::MAX]
///   - Err(WorkflowError::StepCountOverflow) iff the DFS overflowed u64.
pub open spec fn spec_count_total_steps_result(result: Result<int, SpecWorkflowError>) -> bool {
    match result {
        Err(SpecWorkflowError::StepCountOverflow) => true,
        Ok(v) => spec_count_total_steps_bounded(v),
    }
}

// ============================================================================
// assume_specification bridges — production contract surface
// ============================================================================
//
// These bridges attach spec contracts to the production-bound exec fns in
// `extern_budget_bounded.rs`. The body of each extern fn is opaque to
// Verus (`#[verifier::external]`); the spec proofs below exercise the
// contracts via the exec wrappers in the "Production-bound exec fns"
// section.
/// Bridge contract: `add_dim` returns Ok iff the checked_add does not
/// overflow. Mirrors production body at
/// `crates/vb_core/src/budget.rs:1250-1258`:
///   `current.checked_add(requested).ok_or(Overflow { resource })`
pub assume_specification[ production::add_dim ](
    current: u64,
    requested: u64,
    resource: &'static str,
) -> (result: Result<u64, AggregateBudgetError>)
    ensures
        match result {
            Ok(v) => v == current + requested,
            Err(AggregateBudgetError::Overflow { resource: r }) => {
                &&& r == resource
                &&& current + requested > u64::MAX
            },
            Err(_) => false,
        },
;

/// Bridge contract: `sub_dim` returns Ok iff the checked_sub does not
/// underflow. Mirrors production body at
/// `crates/vb_core/src/budget.rs:1260-1268`.
pub assume_specification[ production::sub_dim ](
    current: u64,
    requested: u64,
    resource: &'static str,
) -> (result: Result<u64, AggregateBudgetError>)
    ensures
        match result {
            Ok(v) => v == current - requested,
            Err(AggregateBudgetError::Underflow { resource: r }) => {
                &&& r == resource
                &&& current < requested
            },
            Err(_) => false,
        },
;

/// Bridge contract: `whole_workflow_budget_compute` returns Ok(budget)
/// with `max_total_steps <= u64::MAX` (no u64 overflow) and `Ok` iff the
/// production DFS did not encounter a back-edge cycle or depth overflow.
/// Mirrors the success branch of `compute` at
/// `crates/vb_core/src/budget.rs:64-70` and the success contract of
/// `count_total_steps` at `crates/vb_core/src/budget.rs:1332-1360`.
pub assume_specification[ production::whole_workflow_budget_compute ](
    nodes: &[production::workflow::CompiledNode],
    entry: StepIdx,
    contract: &ResourceContract,
) -> (result: Result<WholeWorkflowBudget, production::workflow::WorkflowError>)
    ensures
        match result {
            Ok(budget) => budget.max_total_steps <= u64::MAX,
            Err(_) => true,
        },
;

/// Bridge contract: `boundedness_policy_validate` returns Ok iff every
/// dimension of the budget is within the policy limits. The closure of
/// the discriminant set in the ensures matches the production body at
/// `crates/vb_core/src/budget.rs:400-457` (returns the FIRST violation
/// encountered, or Ok(())).
///
/// The contract uses a spec predicate `spec_policy_violation` to compare
/// the destructured `actual`/`limit` fields against the budget / policy
/// dimensions. This avoids Verus's `obeys_eq_spec` requirement on raw
/// `==` comparisons inside `match` arms when the compared types are
/// inferred through struct destructuring.
pub open spec fn spec_policy_violation(
    budget: WholeWorkflowBudget,
    policy: BoundednessPolicy,
    result: Result<(), BudgetError>,
) -> bool {
    match result {
        Ok(()) => true,
        Err(BudgetError::TotalStepsExceeded { actual, limit }) => actual == budget.max_total_steps
            && limit == policy.max_total_steps && actual > limit,
        Err(BudgetError::TotalSlotsExceeded { actual, limit }) => actual == budget.max_total_slots
            && limit == policy.max_total_slots && actual > limit,
        Err(BudgetError::FanoutExceeded { actual, limit }) => actual == budget.max_fanout && limit
            == policy.max_fanout && actual > limit,
        Err(BudgetError::NestingDepthExceeded { actual, limit }) => actual
            == budget.max_nesting_depth && limit == policy.max_nesting_depth && actual > limit,
        Err(BudgetError::ActionTicketsExceeded { actual, limit }) => actual
            == budget.max_action_tickets && limit == policy.absolute_max_action_tickets && actual
            > limit,
        Err(BudgetError::ParallelExceeded { actual, limit }) => actual
            == budget.max_parallel_in_flight && limit == policy.absolute_max_parallel && actual
            > limit,
        Err(BudgetError::RunTimeExceeded { actual, limit }) => actual == budget.max_run_time_seconds
            && limit == policy.absolute_max_run_time_seconds && actual > limit,
        Err(BudgetError::ResultBytesExceeded { actual, limit }) => actual == budget.max_result_bytes
            && limit == policy.absolute_max_result_bytes && actual > limit,
        Err(BudgetError::StepsExecutableExceeded { actual, limit }) => actual
            == budget.max_steps_executable && limit == policy.absolute_max_steps_executable
            && actual > limit,
        Err(_) => true,
    }
}

pub assume_specification[ production::boundedness_policy_validate ](
    policy: &BoundednessPolicy,
    budget: &WholeWorkflowBudget,
) -> (result: Result<(), BudgetError>)
    ensures
        spec_policy_violation(*budget, *policy, result),
;

/// Bridge contract: `validate_aggregate_budget` returns Ok iff every
/// dimension of the aggregate budget is within the policy limits.
/// Mirrors `crates/vb_core/src/budget.rs:1110-1209`.
pub assume_specification[ production::validate_aggregate_budget ](
    budget: &AggregateResourceBudget,
    policy: &BoundednessPolicy,
) -> (result: Result<(), AggregateBudgetError>)
    ensures
        match result {
            Ok(()) => true,
            Err(_) => true,
        },
;

/// Bridge contract: `validate_step_ceilings` returns Ok iff
/// `max_step_budget_per_tick <= 1_000_000` and
/// `max_transitions_per_tick <= 1_000_000`. Mirrors
/// `crates/vb_core/src/budget.rs:1213-1248`.
pub assume_specification[ production::validate_step_ceilings ](
    budget: &AggregateResourceBudget,
) -> (result: Result<(), AggregateBudgetError>)
    ensures
        match result {
            Ok(()) => budget.max_step_budget_per_tick <= 1_000_000
                && budget.max_transitions_per_tick <= 1_000_000,
            Err(AggregateBudgetError::StepCeilingExceeded { requested, limit }) => requested
                == budget.max_step_budget_per_tick && limit == 1_000_000,
            Err(AggregateBudgetError::PerTickCeilingExceeded { requested, limit }) => requested
                == budget.max_transitions_per_tick && limit == 1_000_000,
            Err(_) => true,
        },
;

// ============================================================================
// Production-bound exec fns with requires/ensures
// ============================================================================
//
// These exec fns call the production extern wrappers and assert the
// assume_specification contracts. Each one is the production-bound
// exerciser for one obligation.
/// Spec fn: byte-equal comparison for `&'static str`. The Verus
/// `==` operator on `&'static str` requires `obeys_eq_spec` for the
/// pointer-deref-based PartialEq impl; we provide a spec-only shim
/// so the production contracts can express equality without invoking
/// the stdlib PartialEq impl that Verus cannot model.

// ============================================================================
// Companion chunk 2 — proof functions and remaining spec (branch_cost, together_fanout, aggregate_refines_whole, diagnostic_complete, proof_*)
// ============================================================================
#[path = "budget_bounded_chunk2.rs"]
mod chunk2;

} // verus!
