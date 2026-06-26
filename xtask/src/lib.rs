#![forbid(unsafe_code)]

pub mod benchmark_metadata;
pub mod contracts;
pub mod doc_evidence;
pub mod doc_reconcile;
pub mod evidence;
pub mod evidence_gate;

pub mod boundary_inventory;

mod command_family;
mod dependency_boundary;
mod error;
mod parser;
mod registry;
mod routing;
mod status;

pub use benchmark_metadata::{
    BenchmarkMetadata, EvidenceError, IpcBenchmarkError, RecoveryBenchmarkError,
    RuntimeBenchmarkError, StorageBenchmarkError, YamlBenchmarkError, baseline_within_budget,
    budget_utilization_percent, capture_metadata, check_evidence_gate, latency_within_budget,
    result_exceeds_threshold,
};
pub use command_family::CommandFamily;
pub use dependency_boundary::{WorkspaceManifest, assert_runtime_dependency_boundary};
pub use error::XtaskCommandError;
pub use parser::{XtaskCommand, parse_xtask_command};
pub use registry::validate_command_registry;
pub use registry::{CommandFamilySpec, ValidatedCommandRegistry, required_command_families};
pub use routing::{XtaskEnvironment, placeholder_status, route_command};
pub use status::{DeferredReason, OutputFormat, StructuredStatus, render_structured_status};

// New modules for proof/test orchestrator (vb-i7xn)
pub mod discovery;
pub mod lanes;
pub mod logger;
pub mod profiles;
pub mod proof_orchestrator;
pub mod scheduler;
pub mod summary;
