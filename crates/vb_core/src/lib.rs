#![forbid(unsafe_code)]
// Pedantic allows: these lints are documentation-only or would require pervasive
// changes with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]

//! Hot in-memory execution core for Velvet Ballastics.
//!
//! This crate owns the compiled workflow IR, numeric identifiers, runtime slot
//! model, and synchronous state-machine loop. It intentionally has no async,
//! no storage, no HTTP, and no YAML dependencies.

pub mod action;
pub mod action_pack;
pub mod artifact;
pub mod budget;
#[cfg(test)]
mod budget_vb_8mdp_7_prop_tests;
pub mod capability;
pub mod capability_grant;
pub mod contract_encoding;
pub mod diagnostic;
pub mod engine;
pub mod error;
pub mod errors;
pub mod frame;
pub mod ids;
pub mod input_validation;
pub mod limits;
pub mod manual_gate;
pub mod policy;
pub mod replay;
pub mod root_bundle;
pub mod shard;
pub mod span;
pub mod time;
pub mod value;
pub mod value_store;
pub mod workflow;

#[cfg(feature = "verus-kernels")]
pub mod proof_kernels;

#[cfg(kani)]
#[path = "verification/kani/kani_expr_bound.rs"]
pub mod kani_expr_bound;

#[cfg(kani)]
#[path = "verification/kani/kani_capability_harnesses.rs"]
pub mod kani_capability_harnesses;

#[cfg(kani)]
#[path = "verification/kani/kani_idempotency_gates.rs"]
pub mod kani_idempotency_gates;

#[cfg(kani)]
#[path = "verification/kani/kani_taint_propagation.rs"]
pub mod kani_taint_propagation;

#[cfg(kani)]
#[path = "verification/kani/kani_step_budget_zero.rs"]
pub mod kani_step_budget_zero;

#[cfg(kani)]
#[path = "verification/kani/kani_step_budget_one.rs"]
pub mod kani_step_budget_one;

#[cfg(kani)]
#[path = "verification/kani/kani_step_budget.rs"]
pub mod kani_step_budget;

#[cfg(kani)]
#[path = "verification/kani/kani_step_budget_try_take_arbitrary.rs"]
pub mod kani_step_budget_try_take_arbitrary;

#[cfg(kani)]
#[path = "verification/kani/kani_budget_arithmetic_refinement.rs"]
pub mod kani_budget_arithmetic_refinement;

#[cfg(kani)]
#[path = "verification/kani/kani_index_access.rs"]
pub mod kani_index_access;

#[cfg(kani)]
#[path = "verification/kani/kani_resource_budget_bounded.rs"]
pub mod kani_resource_budget_bounded;

#[cfg(kani)]
#[path = "verification/kani/kani_workflow_arbitrary.rs"]
pub mod kani_workflow_arbitrary;

#[cfg(kani)]
#[path = "verification/kani/kani_workflow_budget_harnesses.rs"]
pub mod kani_workflow_budget_harnesses;

#[cfg(kani)]
#[path = "verification/kani/kani_step_harnesses.rs"]
pub mod kani_step_harnesses;

#[cfg(kani)]
#[path = "verification/kani/kani_step_state_transition.rs"]
pub mod kani_step_state_transition;

#[cfg(kani)]
#[path = "verification/kani/kani_taint.rs"]
pub mod kani_taint;

#[cfg(kani)]
#[path = "verification/kani/kani_vbjpq733_proofs.rs"]
pub mod kani_vbjpq733_proofs;

#[cfg(all(kani, feature = "kani-diagnostic-codes"))]
#[path = "verification/kani/diagnostic/mod.rs"]
pub mod kani;

// Verification harnesses (gated internally via cfg).
mod verification;

pub use action::{
    ActionContract, ActionError, ActionFailure, ActionFailureCode, ActionFailureReport,
    ActionInput, ActionJournalEvent, ActionOutcome, ActionOutput, ActionOutputReady, ActionResult,
    ActionResumeRejection, ActionResumeReport, ActionTicket, Idempotency, IdempotencyViolation,
    NewActionTicket, RetrySafety, SideEffect, issue_action_ticket, propagate_action_taint,
    validate_action_dispatch, validate_action_outcome, validate_idempotency_key_ingredients,
    verify_idempotency,
};
pub use budget::{
    AggregateBudgetError, AggregateReservation, AggregateResourceBudget, AggregateResourceCapacity,
    AggregateResourceUsage, BoundednessPolicy, BudgetError, BudgetExhaustionOutcome,
    RemainingBudget, RemainingBudgetError, ShardUsageSnapshot, WholeWorkflowBudget,
    check_tenant_quota, validate_aggregate_budget,
};
pub use capability::{Capability, CapabilitySet};
pub use capability_grant::{
    GrantAdmissionMode, GrantDispatchContext, GrantError, GrantId, GrantSignature, GrantTimeMs,
    MAX_REVOKED_IDS, RevocationEpoch, ScopedDispatch, ScopedGrant, ScopedGrantParams,
};
pub use diagnostic::{
    CODE_REGISTRY, CodeCategory, CodeEntry, Diagnostic, DiagnosticCode, DiagnosticCodeParseError,
    HasSymbolicCode, Severity, SymbolicCode, SymbolicCodeParseError,
};
pub use engine::{
    EngineSignal, ErrorHandlerOutcome, ErrorSlotData, StepBudget, build_list, build_object,
    drive_deterministic, eval_accessor, eval_expr, journal_action_suspended, new_run_frame,
    resume_action_completion, resume_action_failure, route_error_handler, run_until_blocked,
    step_once, validate_compiled_workflow, validate_no_nested_together, validate_node_bounds,
    validate_resource_contract, validate_transition_target,
};
pub use errors::{CoreError, CoreResult, EngineError};
pub use frame::{RunFrame, StepState};
pub use ids::{
    AccessorIdx, ActionId, BlobId, BranchCount, BranchIdx, ConstIdx, EventSeq, ExprIdx,
    FanoutLimit, ListId, MaxAttempts, ObjectId, RetryCount, RunId, SeqNo, SlotIdx, StepIdx,
    SymbolId, TenantId, WorkflowDigest, WorkflowId,
};
pub use policy::RuntimePolicy;
pub use span::{Located, SourceMap, Span, SpanError, Spanned};
pub use value::{ConstValue, FiniteF64, SlotValue, Taint, join_taint};
pub use value_store::{ObjectField, ValueStore};
pub use workflow::{
    AccessorProgram, CompiledInputConstraint, CompiledInputSlot, CompiledNode, CompiledNodeKind,
    CompiledWorkflow, ExprBranch, ExprOp, ExprProgram, InputConstraintIssue, InputSlotKind,
    PathSegment, ResourceContract, SlotBranch, WorkflowError, WorkflowParts,
    check_expr_stack_bound, node, node_with_handler,
};
