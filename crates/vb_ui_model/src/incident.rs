//! Incident and failure-console view types.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IncidentReportView {
    pub run_id: RunId,
    pub failure_step: StepIdx,
    pub failure_action: ActionId,
    pub failure_code: String,
    pub attempt: u16,
    pub timestamp: i64,
    pub severity: IncidentSeverity,
    pub safe_to_retry: bool,
    pub idempotency_key_required: bool,
    pub strict_durability: bool,
    pub replay_safe: bool,
    pub repair_hints: Vec<String>,
    pub evidence_chain: EvidenceChain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum IncidentSeverity {
    Warning = 0,
    Critical = 1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceChain {
    pub scheduled_durable: bool,
    pub completion_durable: bool,
    pub side_effect_certainty: f32,
    pub journal_tail: Option<SeqNo>,
}
