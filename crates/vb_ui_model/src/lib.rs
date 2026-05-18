#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

//! Typed cold-path view models for Velvet Ballistics UI screens.
//!
//! This crate is intentionally decoupled from Makepad, tokio, and async runtimes.
//! All types are plain data — no fallible constructors or behavior methods.

extern crate alloc;

use alloc::boxed::Box;
use serde::{Deserialize, Serialize};

use crate::ai::AiContextView;
use crate::incident::IncidentReportView;
use crate::replay::ReplayReportView;
use crate::run::{RunInspectionView, RunSummaryView};
use crate::storage::StorageDoctorView;
use crate::system::{ActionDescriptionView, SystemStatusView};
use crate::verify::VerificationReportView;
use crate::workflow::WorkflowGraphView;

// Re-exported domain primitives from vb_core.
pub use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
pub use vb_core::capability::Capability;
pub use vb_core::ids::{
    ActionId, BlobId, RunId, SeqNo, SlotIdx, StepIdx, SymbolId, WorkflowDigest, WorkflowId,
};
pub use vb_core::value::Taint;

// Public modules.
pub mod ai;
pub mod canonical;
pub mod emitter;
pub mod envelope;
pub mod incident;
pub mod redact;
pub mod replay;
pub mod run;
pub mod storage;
pub mod system;
pub mod verify;
pub mod workflow;

// ---------------------------------------------------------------------------
// Top-level types (not specific to a single sub-domain)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum UiScreenKind {
    ExecutionOverview = 0,
    WorkflowGraphAuthoring = 1,
    ExecutionDetailsGraph = 2,
    VerificationCertificate = 3,
    ReplayTheater = 4,
    IncidentFailureConsole = 5,
    ActionRegistry = 6,
    StorageDoctorAiContext = 7,
}

/// Aggregate snapshot of the entire UI state, used for snapshot tests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiAppSnapshot {
    pub status: SystemStatusView,
    #[serde(bound = "")]
    pub active_runs: Box<[RunSummaryView]>,
    pub selected_run: Option<RunInspectionView>,
    pub selected_workflow: Option<WorkflowGraphView>,
    pub verification: Option<VerificationReportView>,
    pub replay: Option<ReplayReportView>,
    pub incident: Option<IncidentReportView>,
    #[serde(bound = "")]
    pub actions: Box<[ActionDescriptionView]>,
    pub storage: Option<StorageDoctorView>,
    pub ai_context: Option<AiContextView>,
}
