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
pub mod budget;
pub mod capability;
pub mod contract_encoding;
pub mod diagnostic;
pub mod engine;
pub mod error;
pub mod errors;
pub mod frame;
pub mod ids;
pub mod limits;
pub mod policy;
pub mod replay;
pub mod shard;
pub mod span;
pub mod value;
pub mod value_store;
pub mod workflow;

#[cfg(kani)]
pub mod kani_expr_bound;

#[cfg(kani)]
pub mod kani_capability_harnesses;

#[cfg(kani)]
pub mod kani_idempotency_gates;

#[cfg(kani)]
pub mod kani_taint_propagation;

#[cfg(kani)]
pub mod kani_step_budget_zero;

#[cfg(kani)]
pub mod kani_step_budget_one;

#[cfg(kani)]
pub mod kani_step_budget;

#[cfg(kani)]
pub mod kani_step_budget_try_take_arbitrary;

#[cfg(kani)]
pub mod kani_budget_arithmetic_refinement;

#[cfg(kani)]
pub mod kani_index_access;

#[cfg(kani)]
pub mod kani_resource_budget_bounded;

#[cfg(kani)]
pub mod kani_workflow_arbitrary;

#[cfg(kani)]
pub mod kani_workflow_budget_harnesses;

#[cfg(kani)]
pub mod kani_step_harnesses;

#[cfg(kani)]
pub mod kani_step_state_transition;

#[cfg(kani)]
pub mod kani_taint;

#[cfg(kani)]
pub mod kani_vbjpq733_proofs;

#[cfg(all(kani, feature = "kani-diagnostic-codes"))]
pub mod kani;

pub use action::{
    ActionContract, ActionError, ActionFailure, ActionFailureCode, ActionFailureReport,
    ActionInput, ActionJournalEvent, ActionOutcome, ActionOutput, ActionOutputReady, ActionResult,
    ActionResumeRejection, ActionResumeReport, ActionTicket, Idempotency, IdempotencyViolation,
    RetrySafety, SideEffect, issue_action_ticket, propagate_action_taint, validate_action_dispatch,
    validate_action_outcome, validate_idempotency_key_ingredients, verify_idempotency,
};
pub use budget::{
    AggregateBudgetError, AggregateReservation, AggregateResourceBudget, AggregateResourceCapacity,
    AggregateResourceUsage, BoundednessPolicy, BudgetError, WholeWorkflowBudget,
    validate_aggregate_budget,
};
pub use capability::{Capability, CapabilitySet};
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
    SymbolId, WorkflowDigest, WorkflowId,
};
pub use policy::RuntimePolicy;
pub use span::{Located, SourceMap, Span, SpanError, Spanned};
pub use value::{ConstValue, FiniteF64, SlotValue, Taint, join_taint};
pub use value_store::{ObjectField, ValueStore};
pub use workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprBranch, ExprOp,
    ExprProgram, PathSegment, ResourceContract, SlotBranch, WorkflowError, WorkflowParts,
    check_expr_stack_bound,
};
