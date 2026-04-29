//! Hot in-memory execution core for Velvet Ballistics.
//!
//! This crate owns the compiled workflow IR, numeric identifiers, runtime slot
//! model, and synchronous state-machine loop. It intentionally has no async,
//! no storage, no HTTP, and no YAML dependencies.

pub mod engine;
pub mod error;
pub mod ids;
pub mod value;
pub mod workflow;

pub use engine::{EngineSignal, RunFrame, StepBudget, step_once};
pub use error::EngineError;
pub use ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
pub use value::{SlotValue, Taint};
pub use workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowError, WorkflowParts,
};
