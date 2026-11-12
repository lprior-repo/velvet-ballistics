#![forbid(unsafe_code)]
//! Explicit compiled resource bounds accepted at run admission.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
    pub allows_secret_results: SecretResultPolicy,
}

/// Resource dimension named by contract validation failures.
///
/// Exhaustive classification backing `WorkflowError::ResourceContractExceeded`
/// and `WorkflowError::ResourceContractTooLarge`. The canonical names are
/// frozen: diagnostics, tests, and admission mapping match on them, so
/// renaming a string is a behavior change. Two legacy names break the `max_`
/// convention (`branch_count`, `loop_regions`); they are preserved exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractResource {
    /// Step count bound.
    Steps,
    /// Slot count bound.
    Slots,
    /// Constant-pool entry bound.
    Constants,
    /// Accessor program bound.
    Accessors,
    /// Expression program bound.
    Expressions,
    /// Expression stack bound.
    ExprStack,
    /// Per-tick step budget bound.
    StepBudgetPerTick,
    /// Per-tick transition bound.
    TransitionsPerTick,
    /// Input byte bound.
    InputBytes,
    /// Output byte bound.
    OutputBytes,
    /// Blob byte bound.
    BlobBytes,
    /// IPC payload byte bound.
    IpcPayloadBytes,
    /// Retry attempt bound.
    RetryAttempts,
    /// Branch fanout bound.
    Fanout,
    /// Collection item bound.
    CollectItems,
    /// Queue depth bound.
    QueueDepth,
    /// Journal batch byte bound.
    JournalBatchBytes,
    /// Per-branch node count bound.
    BranchCount,
    /// Loop region table bound.
    LoopRegions,
    /// Build-list item bound.
    ListItems,
    /// Build-object field bound.
    ObjectFields,
}

impl ContractResource {
    /// Canonical resource name. Frozen: diagnostics match on these strings.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Steps => "max_steps",
            Self::Slots => "max_slots",
            Self::Constants => "max_constants",
            Self::Accessors => "max_accessors",
            Self::Expressions => "max_expressions",
            Self::ExprStack => "max_expr_stack",
            Self::StepBudgetPerTick => "max_step_budget_per_tick",
            Self::TransitionsPerTick => "max_transitions_per_tick",
            Self::InputBytes => "max_input_bytes",
            Self::OutputBytes => "max_output_bytes",
            Self::BlobBytes => "max_blob_bytes",
            Self::IpcPayloadBytes => "max_ipc_payload_bytes",
            Self::RetryAttempts => "max_retry_attempts",
            Self::Fanout => "max_fanout",
            Self::CollectItems => "max_collect_items",
            Self::QueueDepth => "max_queue_depth",
            Self::JournalBatchBytes => "max_journal_batch_bytes",
            Self::BranchCount => "branch_count",
            Self::LoopRegions => "loop_regions",
            Self::ListItems => "list_items",
            Self::ObjectFields => "object_fields",
        }
    }
}

impl core::fmt::Display for ContractResource {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Policy deciding whether secret-tainted results may appear in answers.
///
/// Replaces a bare boolean so call sites name the decision. The encoding is
/// frozen: it serializes exactly as the legacy boolean (`Allow` as `true`,
/// `Deny` as `false`) so canonical digests and artifact bytes are unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretResultPolicy {
    /// Secret-tainted results are allowed in answer payloads.
    Allow,
    /// Secret-tainted results are rejected from answer payloads.
    Deny,
}

impl SecretResultPolicy {
    /// Whether secret-tainted results are allowed in answer payloads.
    #[must_use]
    pub const fn allows(self) -> bool {
        match self {
            Self::Allow => true,
            Self::Deny => false,
        }
    }
}

impl Serialize for SecretResultPolicy {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(self.allows())
    }
}

impl<'de> Deserialize<'de> for SecretResultPolicy {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let allowed = bool::deserialize(deserializer)?;
        Ok(if allowed { Self::Allow } else { Self::Deny })
    }
}

#[cfg(kani)]
impl kani::Arbitrary for SecretResultPolicy {
    fn any() -> Self {
        if kani::any() { Self::Allow } else { Self::Deny }
    }
}

impl ResourceContract {
    /// Conservative default used until policy profiles become user-configurable.
    pub const DEFAULT: Self = Self {
        max_steps: 10_000,
        max_slots: 1_024,
        max_constants: u16::MAX,
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
        allows_secret_results: SecretResultPolicy::Deny,
    };
}
