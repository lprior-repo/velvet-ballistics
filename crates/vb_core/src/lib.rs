//! Hot in-memory execution core for Velvet Ballastics.
//!
//! This crate owns the compiled workflow IR, numeric identifiers, runtime slot
//! model, and synchronous state-machine loop. It intentionally has no async,
//! no storage, no HTTP, and no YAML dependencies.

pub mod action;
pub mod diagnostic;
pub mod engine;
pub mod error;
pub mod errors;
pub mod frame;
pub mod ids;
pub mod limits;
pub mod span;
pub mod value;
pub mod value_store;
pub mod workflow;

pub use action::{
    ActionContract, ActionError, ActionFailure, ActionFailureCode, ActionInput, ActionOutcome,
    ActionOutput, ActionOutputReady, ActionResult, ActionTicket, Idempotency,
    propagate_action_taint,
};
pub use diagnostic::{Diagnostic, DiagnosticCode, DiagnosticCodeParseError, Severity};
pub use engine::{
    EngineSignal, StepBudget, build_list, build_object, drive_deterministic, eval_accessor,
    eval_expr, new_run_frame, run_until_blocked, step_once, validate_compiled_workflow,
    validate_node_bounds, validate_resource_contract, validate_transition_target,
};
pub use errors::{CoreError, CoreResult, EngineError};
pub use frame::{RunFrame, StepState};
pub use ids::{
    AccessorIdx, ActionId, BlobId, CheckedIndex, ConstIdx, ExprIdx, ListId, ObjectId, RunId, SeqNo,
    SlotIdx, StepIdx, SymbolId, WorkflowDigest, WorkflowId,
};
pub use span::{Located, SourceMap, Span, Spanned};
pub use value::{ConstValue, FiniteF64, SlotValue, Taint};
pub use value_store::{ObjectField, ValueStore};
pub use workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprBranch, ExprOp,
    ExprProgram, PathSegment, ResourceContract, SlotBranch, WorkflowError, WorkflowParts,
    check_expr_stack_bound,
};
