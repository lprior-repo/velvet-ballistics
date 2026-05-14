//! Verification certificate view types.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use vb_core::ids::{WorkflowDigest, WorkflowId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationReportView {
    pub workflow_id: WorkflowId,
    pub workflow_digest: WorkflowDigest,
    pub passed: bool,
    pub warnings: Vec<String>,
    pub certificate: VerificationCertificate,
    pub gate_results: Vec<GateResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationCertificate {
    pub structure: bool,
    pub boundedness: bool,
    pub resources: bool,
    pub taint: bool,
    pub action_policy: bool,
    pub durability: bool,
    pub idempotency: bool,
    pub capability: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateResult {
    pub name: String,
    pub passed: bool,
    pub detail: Option<String>,
}
