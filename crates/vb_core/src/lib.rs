#![forbid(unsafe_code)]
// Pedantic allows: these lints are documentation-only or would require pervasive
// changes with no functional impact on correctness or safety.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]

//! Hot in-memory execution core for velvet-ballistics.
//!
//! This crate owns the compiled workflow IR, numeric identifiers, runtime slot
//! model, and synchronous state-machine loop. It intentionally has no async,
//! no storage, no HTTP, and no YAML dependencies.

pub mod action;
pub mod budget;
pub mod capability;
pub mod check;
pub mod contract_encoding;
pub mod diagnostic;
pub mod engine;
pub mod error;
pub mod errors;
pub mod frame;
pub mod git;
pub mod ids;
pub mod limits;
pub mod policy;
pub mod replay;
pub mod shard;
pub mod span;
pub mod value;
pub mod value_store;
pub mod workflow;

// HVR-PO-CORE-001/HVR-PO-CORE-003/HVR-PO-CORE-004: keep legacy Kani groups out of the vb-god2f feature lane.
#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub mod kani_expr_bound;

#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub mod kani_capability_harnesses;

#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub mod kani_idempotency_gates;

#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub mod kani_taint_propagation;

#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub mod kani_step_budget_zero;

#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub mod kani_step_budget_one;

#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub mod kani_step_budget;

#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub mod kani_step_budget_try_take_arbitrary;

#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub mod kani_budget_arithmetic_refinement;

#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub mod kani_index_access;

#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub mod kani_resource_budget_bounded;

#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub mod kani_workflow_arbitrary;

#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub mod kani_workflow_budget_harnesses;

#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub mod kani_step_harnesses;

#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub mod kani_step_state_transition;

#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub mod kani_taint;

#[cfg(all(kani, not(feature = "kani-vb-god2f-proof-kernels")))]
pub mod kani_vbjpq733_proofs;

#[cfg(all(kani, feature = "kani-diagnostic-codes"))]
pub mod kani;

#[cfg(all(kani, feature = "kani-resource-contract-boundaries"))]
pub mod kani_validate_resource_contract_boundaries;

#[cfg(all(kani, feature = "kani-vb-god2f-proof-kernels"))]
pub mod kani_vb_god2f_resource_replacement;

#[cfg(all(kani, feature = "kani-vb-god2f-proof-kernels"))]
pub mod kani_vb_god2f_step_state_replacement;

#[cfg(all(kani, feature = "kani-vb-god2f-proof-kernels"))]
pub mod kani_vb_god2f_taint_replacement;

pub mod verification;

pub use action::{
    ActionContract, ActionError, ActionFailure, ActionFailureCode, ActionInput, ActionJournalEvent,
    ActionOutcome, ActionOutput, ActionOutputReady, ActionResult, ActionTicket, Idempotency,
    IdempotencyViolation, MockMarker, RetrySafety, SideEffect, issue_action_ticket,
    propagate_action_taint, validate_action_dispatch, validate_action_outcome,
    validate_idempotency_key_ingredients, verify_idempotency,
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
    step_once, validate_compiled_workflow, validate_node_bounds, validate_resource_contract,
    validate_transition_target,
};
pub use errors::{CoreError, CoreResult, EngineError};
pub use frame::{RunFrame, StepState};
pub use ids::{
    AccessorIdx, ActionId, BlobId, BranchCount, BranchIdx, ConstIdx, EventSeq, ExprIdx,
    FanoutLimit, ListId, MaxAttempts, ObjectId, RetryCount, RunId, SeqNo, SlotIdx, StepIdx,
    SymbolId, WorkflowDigest, WorkflowId,
};
pub use policy::{
    ContractViolation, ProfileName, ProfileValidationError, RuntimeLimitsConfig,
    RuntimeLimitsProfile, RuntimePolicy,
};
pub use span::{Located, SourceMap, Span, Spanned};
pub use value::{ConstValue, FiniteF64, SlotValue, Taint, join_taint};
pub use value_store::{ObjectField, ValueStore};
pub use workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprBranch, ExprOp,
    ExprProgram, PathSegment, ResourceContract, SlotBranch, WorkflowError, WorkflowParts,
    check_expr_stack_bound,
};
