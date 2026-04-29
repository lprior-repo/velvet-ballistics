//! Hot in-memory execution core for Velvet Ballastics.
//!
//! This crate owns the compiled workflow IR, numeric identifiers, runtime slot
//! model, and synchronous state-machine loop. It intentionally has no async,
//! no storage, no HTTP, and no YAML dependencies.

pub mod diagnostic;
pub mod engine;
pub mod error;
pub mod errors;
pub mod ids;
pub mod limits;
pub mod span;
pub mod value;
pub mod value_store;
pub mod workflow;

pub use diagnostic::{Diagnostic, DiagnosticCode, DiagnosticCodeParseError, Severity};
pub use engine::{EngineSignal, RunFrame, StepBudget, step_once};
pub use errors::{CoreError, CoreResult, EngineError};
pub use ids::{
    AccessorIdx, ActionId, BlobId, CheckedIndex, ConstIdx, ExprIdx, ListId, ObjectId, RunId, SeqNo,
    SlotIdx, StepIdx, SymbolId, WorkflowDigest, WorkflowId,
};
pub use span::{Located, SourceMap, Span, Spanned};
pub use value::{FiniteF64, SlotValue, Taint};
pub use value_store::{ObjectField, ValueStore};
pub use workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowError, WorkflowParts,
};
