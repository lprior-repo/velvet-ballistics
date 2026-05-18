//! AI context panel view types.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use vb_core::ids::RunId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiContextPanel {
    pub safe_for_model: bool,
    pub secrets_redacted: bool,
    pub blobs_summarized: bool,
    pub suggested_commands: Vec<SuggestedCommand>,
    pub failure_summary: String,
    pub replay_safety: ReplaySafety,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuggestedCommand {
    pub cmd: String,
    pub desc: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ReplaySafety {
    Safe,
    Unsafe { reason: String },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiContextView {
    pub run_id: RunId,
    pub safe_for_model: bool,
    pub secrets_redacted: bool,
    pub blobs_summarized: bool,
    pub failure_summary: Option<String>,
    pub replay_safe: bool,
    pub suggested_commands: Box<[String]>,
    pub last_cert_check: Option<i64>,
    pub last_replay_check: Option<i64>,
    pub last_crash_lab_fixture: Option<String>,
    pub incomplete_evidence_warnings: Box<[String]>,
}
