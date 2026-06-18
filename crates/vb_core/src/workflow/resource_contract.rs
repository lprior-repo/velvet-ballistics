#![forbid(unsafe_code)]
//! Explicit compiled resource bounds accepted at run admission.

use serde::{Deserialize, Serialize};

/// Explicit compiled resource bounds accepted at run admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceContract {
    /// Maximum node count admitted for this workflow.
    pub max_steps: u16,
    /// Maximum runtime slot count admitted for this workflow.
    pub max_slots: u16,
    /// Maximum constant-pool entries admitted for this workflow.
    pub max_constants: u16,
    /// Maximum accessor programs admitted for this workflow.
    pub max_accessors: u16,
    /// Maximum expression programs admitted for this workflow.
    pub max_expressions: u16,
    /// Maximum expression stack entries admitted for this workflow.
    pub max_expr_stack: u8,
    /// Maximum deterministic transitions per runtime tick.
    pub max_step_budget_per_tick: u64,
    /// Maximum transitions per runtime tick.
    pub max_transitions_per_tick: u64,
    /// Maximum input bytes accepted at admission.
    pub max_input_bytes: u32,
    /// Maximum output bytes produced by a run.
    pub max_output_bytes: u32,
    /// Maximum blob payload bytes.
    pub max_blob_bytes: u64,
    /// Maximum IPC payload bytes.
    pub max_ipc_payload_bytes: u32,
    /// Maximum retry attempts for action policies.
    pub max_retry_attempts: u16,
    /// Maximum branch fanout.
    pub max_fanout: u16,
    /// Maximum collect items.
    pub max_collect_items: u32,
    /// Maximum runtime queue depth.
    pub max_queue_depth: u32,
    /// Maximum journal batch bytes.
    pub max_journal_batch_bytes: u32,
    /// Whether secret-tainted results are allowed in answer payloads.
    pub allows_secret_results: bool,
}

impl ResourceContract {
    /// Conservative default used until policy profiles become user-configurable.
    ///
    /// NOTE: Any `*.removed`, `*.bak`, or `*.orig` tombstone files that
    /// previously shadowed this `DEFAULT` were cleaned up in bead
    /// `vb-dedup.6`; the canonical `DEFAULT` is defined here only.
    pub const DEFAULT: Self = Self {
        max_steps: 1_000,
        max_slots: 1_024,
        max_constants: 8_192,
        max_accessors: 8_192,
        max_expressions: 4_096,
        max_expr_stack: 64,
        max_step_budget_per_tick: 10_000,
        max_transitions_per_tick: 10_000,
        max_input_bytes: 1_048_576,
        max_output_bytes: 262_144,
        max_blob_bytes: 16_777_216,
        max_ipc_payload_bytes: 1_048_576,
        max_retry_attempts: 3,
        max_fanout: 64,
        max_collect_items: 1_024,
        max_queue_depth: 1_024,
        max_journal_batch_bytes: 1_048_576,
        allows_secret_results: false,
    };
}
