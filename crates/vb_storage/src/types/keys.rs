#![forbid(unsafe_code)]
//! Storage key variants.

use vb_core::{ActionId, RunId, WorkflowId};

use super::seq::EventSeq;

/// Key variants supported by the durable storage contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StorageKey {
    /// Workflow source bytes by digest.
    WorkflowSource {
        digest: [u8; crate::constants::DIGEST_BYTES],
    },
    /// Compiled IR bytes by digest.
    CompiledIr {
        digest: [u8; crate::constants::DIGEST_BYTES],
    },
    /// Run metadata by run id.
    RunHeader { run: RunId },
    /// Run event by run id and sequence.
    RunEvent { run: RunId, seq: EventSeq },
    /// Run snapshot by run id and sequence.
    RunSnapshot { run: RunId, seq: EventSeq },
    /// Blob bytes by digest.
    Blob {
        digest: [u8; crate::constants::DIGEST_BYTES],
    },
    /// Status index marker.
    IndexStatus {
        /// State marker; use `IndexStatusState` for type-safe construction.
        state: super::index::IndexStatusState,
        timestamp: u64,
        run: RunId,
    },
    /// Workflow/run index marker.
    IndexWorkflow { workflow: WorkflowId, run: RunId },
    /// Pending action index marker.
    IndexAction {
        action: ActionId,
        run: RunId,
        step: vb_core::StepIdx,
    },
    /// Recovery-stamp progress marker.
    RecoveryStamp { run: RunId, seq: EventSeq },
    /// Sequence-gap marker written by `inject_seq_gap`.
    RunSeqGap { run: RunId, seq: EventSeq },
}
