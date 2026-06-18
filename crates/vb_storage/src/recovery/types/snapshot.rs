#![forbid(unsafe_code)]
//! Durable snapshot types for run state checkpoints.

use serde::{Deserialize, Serialize};
use vb_core::{RunId, WorkflowDigest};

use crate::EventSeq;

/// Snapshot of a run's runtime state at a specific event sequence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSnapshot {
    /// Run identifier.
    pub run: RunId,
    /// Sequence number at which this snapshot was taken.
    pub seq: EventSeq,
    /// Compiled workflow digest.
    pub workflow: WorkflowDigest,
    /// Slot values at snapshot time, compact binary form.
    pub slots: Vec<u8>,
    /// Slot taint markers at snapshot time, compact binary form.
    pub taint: Vec<u8>,
}
