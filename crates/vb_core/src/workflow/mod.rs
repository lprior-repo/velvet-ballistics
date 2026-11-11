#![forbid(unsafe_code)]
//! Compiled workflow IR.

use crate::budget::BudgetDimension;
use crate::errors::{CoreError, CoreResult};
use crate::ids::{
    AccessorIdx, ActionId, BranchCount, BranchIdx, ConstIdx, ExprIdx, FanoutLimit, MaxAttempts,
    RunId, SlotIdx, StepIdx, SymbolId, WorkflowDigest,
};
use crate::limits::{
    MAX_ACCESSORS, MAX_BLOB_BYTES, MAX_COLLECT_ITEMS, MAX_CONSTANTS, MAX_EXPRESSION_OPS,
    MAX_EXPRESSION_STACK, MAX_EXPRESSIONS, MAX_FANOUT, MAX_INPUT_BYTES, MAX_IPC_PAYLOAD_BYTES,
    MAX_JOURNAL_BATCH_BYTES, MAX_LIST_ITEMS_PER_VALUE, MAX_OBJECT_FIELDS_PER_VALUE,
    MAX_OUTPUT_BYTES, MAX_PATH_DEPTH, MAX_QUEUE_DEPTH, MAX_RETRY_ATTEMPTS, MAX_SLOTS_PER_WORKFLOW,
    MAX_STEP_BUDGET, MAX_STEPS_PER_WORKFLOW,
};
use crate::value::{ConstValue, SlotValue};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

mod bound_symbols;
pub mod source_metadata;
pub use bound_symbols::{VersionedWorkflowParts, WorkflowIrV2};

/// Immutable compiled workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledWorkflow {
    name: Box<str>,
    digest: WorkflowDigest,
    semantic_source_identity: Option<WorkflowDigest>,
    nodes: Box<[CompiledNode]>,
    expressions: Box<[ExprProgram]>,
    accessors: Box<[AccessorProgram]>,
    constants: Box<[ConstValue]>,
    slot_count: u16,
    symbols_count: u32,
    entry: StepIdx,
    resource_contract: ResourceContract,
    step_names: Box<[Box<str>]>,
    input_slots: Box<[CompiledInputSlot]>,
    loop_slots: Box<[LoopRegionSlot]>,
    source_symbols: Option<source_metadata::WorkflowSourceMetadata>,
}

impl CompiledWorkflow {
    /// Creates a compiled workflow after validating all numeric references.
    pub fn try_from_parts(parts: WorkflowParts) -> Result<Self, WorkflowError> {
        validate_parts(&parts)?;
        let loop_regions = loop_regions::validate(&parts)?;
        let nested_activation_entries =
            loop_regions::nested_activation_map(&parts, &loop_regions)?;
        let loop_slots = loop_regions::merge_slots(&loop_regions, &nested_activation_entries);
        validate_budget(&parts)?;
        validate_input_slots(&parts)?;
        Ok(Self {
            name: parts.name,
            digest: parts.digest,
            semantic_source_identity: parts.semantic_source_identity,
            nodes: parts.nodes,
            expressions: parts.expressions,
            accessors: parts.accessors,
            constants: parts.constants,
            slot_count: parts.slot_count,
            symbols_count: parts.symbols_count,
            entry: parts.entry,
            resource_contract: parts.resource_contract,
            step_names: parts.step_names,
            input_slots: parts.input_slots,
            loop_slots,
            source_symbols: None,
        })
    }

    /// Creates a compiled workflow without validation.
    ///
    /// Enabled by the `test-util` feature. For test use only.
    /// Production code must use [`Self::try_from_parts`].
    #[cfg(feature = "test-util")]
    pub fn from_parts_unchecked(parts: WorkflowParts) -> Self {
        Self {
            name: parts.name,
            digest: parts.digest,
            semantic_source_identity: parts.semantic_source_identity,
            nodes: parts.nodes,
            expressions: parts.expressions,
            accessors: parts.accessors,
            constants: parts.constants,
            slot_count: parts.slot_count,
            symbols_count: parts.symbols_count,
            entry: parts.entry,
            resource_contract: parts.resource_contract,
            step_names: parts.step_names,
            input_slots: parts.input_slots,
            loop_slots: Box::new([]),
            source_symbols: None,
        }
    }

    /// Creates a compiled workflow for Kani harnesses that need concrete runtime
    /// APIs without re-proving workflow validation internals in each harness.
    #[cfg(kani)]
    pub fn kani_from_parts_unchecked(parts: WorkflowParts) -> Self {
        Self {
            name: parts.name,
            digest: parts.digest,
            semantic_source_identity: parts.semantic_source_identity,
            nodes: parts.nodes,
            expressions: parts.expressions,
            accessors: parts.accessors,
            constants: parts.constants,
            slot_count: parts.slot_count,
            symbols_count: parts.symbols_count,
            entry: parts.entry,
            resource_contract: parts.resource_contract,
            step_names: parts.step_names,
            input_slots: parts.input_slots,
            loop_slots: Box::new([]),
            source_symbols: None,
        }
    }

    /// Workflow name retained for cold diagnostics.
    #[must_use]
    pub const fn name(&self) -> &str {
        &self.name
    }

    /// Compiled workflow digest.
    #[must_use]
    pub const fn digest(&self) -> WorkflowDigest {
        self.digest
    }

    /// Canonical identity of the normalized source semantics, when source-backed.
    ///
    /// This is distinct from a digest of raw source bytes. `None` denotes a
    /// compiled-IR-only or explicitly legacy artifact.
    #[must_use]
    pub const fn semantic_source_identity(&self) -> Option<WorkflowDigest> {
        self.semantic_source_identity
    }

    /// Entry step for new runs.
    #[must_use]
    pub const fn entry(&self) -> StepIdx {
        self.entry
    }

    /// Number of slots required by each run frame.
    #[must_use]
    pub const fn slot_count(&self) -> u16 {
        self.slot_count
    }

    /// Number of interned symbols referenced by this workflow.
    #[must_use]
    pub const fn symbols_count(&self) -> u32 {
        self.symbols_count
    }

    /// Number of compiled nodes.
    #[must_use]
    pub fn node_count(&self) -> u16 {
        u16::try_from(self.nodes.len()).map_or(u16::MAX, |value| value)
    }

    /// Returns a checked node reference.
    #[must_use]
    pub fn node(&self, step: StepIdx) -> Option<&CompiledNode> {
        self.nodes.get(step.as_usize())
    }

    /// Returns a checked expression program reference.
    #[must_use]
    pub fn expression(&self, expr: ExprIdx) -> Option<&ExprProgram> {
        self.expressions.get(expr.as_usize())
    }

    /// Returns a checked accessor program reference.
    #[must_use]
    pub fn accessor(&self, accessor: AccessorIdx) -> Option<&AccessorProgram> {
        self.accessors.get(accessor.as_usize())
    }

    /// Returns a checked constant reference.
    #[must_use]
    pub fn constant(&self, constant: ConstIdx) -> Option<&ConstValue> {
        self.constants.get(constant.as_usize())
    }

    /// Returns the human-readable step name for a given step index.
    #[must_use]
    pub fn step_name(&self, step: StepIdx) -> Option<&str> {
        self.step_names.get(step.as_usize()).map(|s| s.as_ref())
    }

    /// Explicit compiled resource bounds for admission and allocation.
    #[must_use]
    pub const fn resource_contract(&self) -> ResourceContract {
        self.resource_contract
    }

    /// Read-only reference to the compiled input slots.
    #[must_use]
    pub fn input_slots(&self) -> &[CompiledInputSlot] {
        self.input_slots.as_ref()
    }

    /// Returns the compiled loop region for a control step, if any.
    ///
    /// The cold constructor validates ownership and edge closure; this checked
    /// lookup returns its immutable body/control/done region for runtime use. Returns `None` for non-loop steps or when the cache
    /// was not populated (unchecked constructors).
    #[must_use]
    pub fn loop_region(&self, control: StepIdx) -> Option<&CompiledLoopRegion> {
        self.loop_slot(control)?.region()
    }

    /// Returns whether a step is a nested activation entry point.
    ///
    /// Returns `None` for out-of-range indices or when the cache was not
    /// populated (unchecked constructors). `true` marks a nested Start or a step in its
    /// validated inclusive body/control interval.
    #[must_use]
    pub fn nested_activation_blocked(&self, step: StepIdx) -> Option<bool> {
        Some(self.loop_slot(step)?.is_nested_entry())
    }

    /// Returns the total loop cache state for a step.
    ///
    /// Returns `None` for out-of-range indices or when the cache was not
    /// populated (unchecked constructors).
    #[must_use]
    pub fn loop_slot(&self, step: StepIdx) -> Option<&LoopRegionSlot> {
        self.loop_slots.get(step.as_usize())
    }

    /// Returns the legacy parts-only projection without bound symbol payloads.
    /// Use [`Self::to_versioned_parts`] to retain the bound representation.
    #[must_use]
    pub fn to_parts(&self) -> WorkflowParts {
        WorkflowParts {
            name: self.name.clone(),
            digest: self.digest,
            semantic_source_identity: self.semantic_source_identity,
            nodes: self.nodes.clone(),
            expressions: self.expressions.clone(),
            accessors: self.accessors.clone(),
            constants: self.constants.clone(),
            slot_count: self.slot_count,
            symbols_count: self.symbols_count,
            entry: self.entry,
            resource_contract: self.resource_contract,
            step_names: self.step_names.clone(),
            input_slots: self.input_slots.clone(),
        }
    }
    pub(crate) fn error_handler_for_body(&self, body_step: StepIdx) -> Option<&CompiledNode> {
        self.nodes.iter().find(|node| {
            matches!(node.kind, CompiledNodeKind::ErrorHandler { body, .. } if body == body_step)
        })
    }
}

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

/// Expression branch target used by final choose IR scaffolding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExprBranch {
    /// Expression condition index.
    pub condition: ExprIdx,
    /// Target node when the condition is true.
    pub target: StepIdx,
}

/// Materialized boolean-slot branch target used by final choose IR scaffolding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotBranch {
    /// Boolean condition slot.
    pub condition: SlotIdx,
    /// Target node when the condition is true.
    pub target: StepIdx,
}

/// Classification of a compiled input slot by its runtime value shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum InputSlotKind {
    /// Slot holds a null value.
    Null,
    /// Slot holds a boolean value.
    Bool,
    /// Slot holds a 64-bit signed integer value.
    I64,
    /// Slot holds a 64-bit float value.
    F64,
    /// Slot holds an interned symbol.
    Symbol,
    /// Slot holds a list handle.
    List,
    /// Slot holds an object handle.
    Object,
    /// Slot holds a blob handle.
    Blob,
    /// Slot accepts any recognized value shape (schema wildcard).
    Any,
}

/// A compiled input slot: numeric index plus production classification.
///
/// Source names are intentionally excluded; the compiler emits only numeric
/// references so the runtime never pays for string lookups on hot paths.
/// The `kind` field describes the runtime value shape held at that slot index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledInputSlot {
    /// Numeric slot index.
    pub slot: SlotIdx,
    /// Runtime value shape classification for this slot.
    pub kind: InputSlotKind,
    /// Retained author-declared schema constraints for this slot.
    ///
    /// Serde `default` tolerates pre-constraint artifacts the same way
    /// `WorkflowParts::input_slots` tolerates pre-v1 artifacts.
    #[serde(default)]
    pub constraint: CompiledInputConstraint,
}

/// Presence requirement for a compiled input slot.
///
/// Replaces the legacy `optional: bool` flag so the decision is named at
/// every use. The encoding is frozen: it serializes exactly as the legacy
/// boolean (`Required` as `false`, `Optional` as `true`) so canonical
/// digests and artifact bytes are unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Presence {
    /// The slot must be supplied (unless a default is declared).
    Required,
    /// The slot may be omitted at submission.
    Optional,
}

impl Presence {
    /// Whether the slot may be omitted.
    #[must_use]
    pub const fn is_optional(self) -> bool {
        matches!(self, Self::Optional)
    }

    /// Whether the slot must be supplied.
    #[must_use]
    pub const fn is_required(self) -> bool {
        matches!(self, Self::Required)
    }

    /// Legacy boolean encoding (`optional`).
    #[must_use]
    pub const fn as_bool(self) -> bool {
        self.is_optional()
    }

    /// Decode the legacy `optional` boolean.
    #[must_use]
    pub const fn from_bool(optional: bool) -> Self {
        if optional {
            Self::Optional
        } else {
            Self::Required
        }
    }
}

impl From<bool> for Presence {
    /// Converts the legacy `optional` boolean flag.
    fn from(optional: bool) -> Self {
        Self::from_bool(optional)
    }
}

impl Serialize for Presence {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(self.as_bool())
    }
}

impl<'de> Deserialize<'de> for Presence {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        bool::deserialize(deserializer).map(Self::from_bool)
    }
}

#[cfg(kani)]
impl kani::Arbitrary for Presence {
    fn any() -> Self {
        if kani::any() { Self::Optional } else { Self::Required }
    }
}

/// Nullability classification for a compiled input slot.
///
/// Replaces the legacy `nullable: bool` flag. The encoding is frozen: it
/// serializes exactly as the legacy boolean (`NonNullable` as `false`,
/// `Nullable` as `true`) so canonical digests and artifact bytes are
/// unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Nullability {
    /// Null values are rejected for non-null kinds.
    NonNullable,
    /// An explicit null value is accepted for non-null kinds.
    Nullable,
}

impl Nullability {
    /// Whether explicit null is accepted.
    #[must_use]
    pub const fn allows_null(self) -> bool {
        matches!(self, Self::Nullable)
    }

    /// Legacy boolean encoding (`nullable`).
    #[must_use]
    pub const fn as_bool(self) -> bool {
        self.allows_null()
    }

    /// Decode the legacy `nullable` boolean.
    #[must_use]
    pub const fn from_bool(nullable: bool) -> Self {
        if nullable {
            Self::Nullable
        } else {
            Self::NonNullable
        }
    }
}

impl From<bool> for Nullability {
    /// Converts the legacy `nullable` boolean flag.
    fn from(nullable: bool) -> Self {
        Self::from_bool(nullable)
    }
}

impl Serialize for Nullability {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(self.as_bool())
    }
}

impl<'de> Deserialize<'de> for Nullability {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        bool::deserialize(deserializer).map(Self::from_bool)
    }
}

#[cfg(kani)]
impl kani::Arbitrary for Nullability {
    fn any() -> Self {
        if kani::any() { Self::Nullable } else { Self::NonNullable }
    }
}

/// Inclusive bounded range for integer / length constraints.
///
/// Replaces the scattered `min` / `max` / `min_length` / `max_length`
/// option soup with a single total type whose invariants (ordering,
/// non-negativity) are checked in one place. `Bound::UNBOUNDED` is the
/// legacy `(None, None)` pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Bound {
    /// Inclusive lower bound.
    pub min: Option<i64>,
    /// Inclusive upper bound.
    pub max: Option<i64>,
}

impl Bound {
    /// Fully unbounded range.
    pub const UNBOUNDED: Self = Self { min: None, max: None };

    /// Constructs a bounded range.
    #[must_use]
    pub const fn new(min: Option<i64>, max: Option<i64>) -> Self {
        Self { min, max }
    }

    /// Lower bound, when declared.
    #[must_use]
    pub const fn lower(self) -> Option<i64> {
        self.min
    }

    /// Upper bound, when declared.
    #[must_use]
    pub const fn upper(self) -> Option<i64> {
        self.max
    }

    /// Whether no bound is declared on either end.
    #[must_use]
    pub const fn is_unbounded(self) -> bool {
        self.min.is_none() && self.max.is_none()
    }

    /// Whether `value` satisfies both declared ends.
    #[must_use]
    pub const fn contains(self, value: i64) -> bool {
        match self.min {
            Some(min) if value < min => return false,
            _ => {}
        }
        match self.max {
            Some(max) if value > max => return false,
            _ => {}
        }
        true
    }

    /// Structural inconsistency, when this range cannot hold any value.
    #[must_use]
    pub const fn inconsistency(self) -> bool {
        match (self.min, self.max) {
            (Some(min), Some(max)) => min > max,
            _ => false,
        }
    }
}

/// Object `extra` policy for a compiled input slot.
///
/// Replaces the legacy `allow_extra: Option<bool>` triple-state. The
/// encoding is frozen: it serializes exactly as the legacy `Option<bool>`
/// (`Unspecified` as `None`, `Allow` as `Some(true)`, `Reject` as
/// `Some(false)`) so canonical digests and artifact bytes are unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtraFields {
    /// No `extra` policy was declared.
    Unspecified,
    /// Additional object fields are allowed.
    Allow,
    /// Additional object fields are rejected.
    Reject,
}

impl ExtraFields {
    /// Whether extra fields are explicitly allowed.
    #[must_use]
    pub const fn allows_extra(self) -> bool {
        matches!(self, Self::Allow)
    }

    /// Whether extra fields are explicitly rejected.
    #[must_use]
    pub const fn rejects_extra(self) -> bool {
        matches!(self, Self::Reject)
    }

    /// Legacy `Option<bool>` encoding.
    #[must_use]
    pub const fn as_option(self) -> Option<bool> {
        match self {
            Self::Unspecified => None,
            Self::Allow => Some(true),
            Self::Reject => Some(false),
        }
    }

    /// Decode the legacy `Option<bool>` encoding.
    #[must_use]
    pub const fn from_option(value: Option<bool>) -> Self {
        match value {
            None => Self::Unspecified,
            Some(true) => Self::Allow,
            Some(false) => Self::Reject,
        }
    }
}

impl From<Option<bool>> for ExtraFields {
    /// Converts the legacy `allow_extra: Option<bool>` encoding.
    fn from(value: Option<bool>) -> Self {
        Self::from_option(value)
    }
}

impl Serialize for ExtraFields {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.as_option().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ExtraFields {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Option::<bool>::deserialize(deserializer).map(Self::from_option)
    }
}

#[cfg(kani)]
impl kani::Arbitrary for ExtraFields {
    fn any() -> Self {
        match kani::any::<u8>() % 3 {
            0 => Self::Unspecified,
            1 => Self::Allow,
            _ => Self::Reject,
        }
    }
}

/// Secrecy classification for a compiled input slot.
///
/// Replaces the legacy `secret: bool` flag with a named decision. The
/// encoding is frozen: it serializes exactly as the legacy boolean
/// (`NonSecret` as `false`, `Secret` as `true`) so canonical digests and
/// artifact bytes are unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Secretness {
    /// No secrecy marker is declared.
    NonSecret,
    /// Author-declared secrecy marker (matches compile-time taint facts).
    Secret,
}

impl Secretness {
    /// Whether the slot is marked secret.
    #[must_use]
    pub const fn is_secret(self) -> bool {
        matches!(self, Self::Secret)
    }

    /// Legacy boolean encoding (`secret`).
    #[must_use]
    pub const fn as_bool(self) -> bool {
        self.is_secret()
    }

    /// Decode the legacy `secret` boolean.
    #[must_use]
    pub const fn from_bool(secret: bool) -> Self {
        if secret {
            Self::Secret
        } else {
            Self::NonSecret
        }
    }
}

impl From<bool> for Secretness {
    /// Converts the legacy `secret` boolean flag.
    fn from(secret: bool) -> Self {
        Self::from_bool(secret)
    }
}

impl Serialize for Secretness {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bool(self.as_bool())
    }
}

impl<'de> Deserialize<'de> for Secretness {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        bool::deserialize(deserializer).map(Self::from_bool)
    }
}

#[cfg(kani)]
impl kani::Arbitrary for Secretness {
    fn any() -> Self {
        if kani::any() { Self::Secret } else { Self::NonSecret }
    }
}

/// Retained per-slot input schema constraints as bounded numerics.
///
/// Every field is `Copy` with no heap allocation so input-slot tables stay
/// numeric end to end. `unconstrained()` reproduces exact legacy behavior
/// (kind-only check, no presence requirement) for hand-built or
/// pre-constraint artifacts; only compiler-emitted slots carry strict
/// constraints. Nested object `fields` sub-schemas and non-scalar defaults
/// are not representable here; their full-fidelity identity is carried by
/// `WorkflowParts::semantic_source_identity` instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledInputConstraint {
    /// Presence requirement for this slot.
    pub presence: Presence,
    /// Nullability classification for this slot.
    pub nullability: Nullability,
    /// Scalar default filled when the slot is omitted (`None` when no
    /// materializable default was declared).
    pub default_value: Option<SlotValue>,
    /// Inclusive minimum for integer values (length bound for lists).
    pub min: Option<i64>,
    /// Inclusive maximum for integer values (length bound for lists).
    pub max: Option<i64>,
    /// Inclusive minimum text length (retained; enforced where payloads
    /// are visible).
    pub min_length: Option<i64>,
    /// Inclusive maximum text length (retained; enforced where payloads
    /// are visible).
    pub max_length: Option<i64>,
    /// Declared list element kind (`of`).
    pub element: Option<InputSlotKind>,
    /// Declared object `extra` policy.
    pub extra: ExtraFields,
    /// Secrecy classification for this slot.
    pub secretness: Secretness,
}

impl CompiledInputConstraint {
    /// Plain required slot with no further constraints.
    ///
    /// This is what the compiler emits for bare shorthand schemas such as
    /// `count: number`: the value must be supplied, with kind-only checking.
    #[must_use]
    pub const fn required() -> Self {
        Self {
            presence: Presence::Required,
            ..Self::unconstrained()
        }
    }

    /// Legacy-compatible constraint: kind-only check, nothing required.
    #[must_use]
    pub const fn unconstrained() -> Self {
        Self {
            presence: Presence::Optional,
            nullability: Nullability::NonNullable,
            default_value: None,
            min: None,
            max: None,
            min_length: None,
            max_length: None,
            element: None,
            extra: ExtraFields::Unspecified,
            secretness: Secretness::NonSecret,
        }
    }

    /// Whether submission must supply a value for this slot.
    ///
    /// A declared scalar default satisfies a missing input, so only slots
    /// that are neither optional nor defaulted require a value.
    #[must_use]
    pub const fn requires_value(&self) -> bool {
        self.presence.is_required() && self.default_value.is_none()
    }

    /// Value bound over `min` / `max`.
    #[must_use]
    pub const fn value_bound(&self) -> Bound {
        Bound::new(self.min, self.max)
    }

    /// Length bound over `min_length` / `max_length`.
    #[must_use]
    pub const fn length_bound(&self) -> Bound {
        Bound::new(self.min_length, self.max_length)
    }
}

impl Default for CompiledInputConstraint {
    /// Legacy-compatible default: same as `unconstrained()`.
    ///
    /// This keeps `#[serde(default)]` on pre-constraint artifacts
    /// permissive instead of flipping them to required.
    fn default() -> Self {
        Self::unconstrained()
    }
}

/// One structural inconsistency in a retained input constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum InputConstraintIssue {
    /// `min` exceeds `max`.
    MinExceedsMax,
    /// `min_length` exceeds `max_length`.
    MinLengthExceedsMaxLength,
    /// A length bound is negative.
    NegativeLengthBound,
}

/// Untrusted compiled workflow parts emitted by a compiler boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowParts {
    /// Workflow name retained for cold diagnostics.
    pub name: Box<str>,
    /// Compiled workflow digest.
    pub digest: WorkflowDigest,
    /// Canonical identity of normalized source semantics, when source-backed.
    ///
    /// This is distinct from a digest of raw source bytes. `None` denotes a
    /// compiled-IR-only or explicitly legacy artifact.
    pub semantic_source_identity: Option<WorkflowDigest>,
    /// Numeric nodes.
    pub nodes: Box<[CompiledNode]>,
    /// Expression bytecode table.
    pub expressions: Box<[ExprProgram]>,
    /// Accessor bytecode table.
    pub accessors: Box<[AccessorProgram]>,
    /// Constant pool.
    pub constants: Box<[ConstValue]>,
    /// Number of runtime slots.
    pub slot_count: u16,
    /// Number of interned symbols referenced by this workflow.
    pub symbols_count: u32,
    /// Entry step.
    pub entry: StepIdx,
    /// Explicit resource bounds carried with the compiled artifact.
    pub resource_contract: ResourceContract,
    /// Human-readable step names indexed by StepIdx.
    pub step_names: Box<[Box<str>]>,
    /// Compiled input slots emitted by the compiler.
    ///
    /// Serde `default` tolerates pre-v1 artifacts that omit this field.
    #[serde(default)]
    pub input_slots: Box<[CompiledInputSlot]>,
}

/// Bounded accessor program for slot-rooted path traversal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessorProgram {
    /// Root slot for the traversal.
    pub root: SlotIdx,
    /// Bounded path from root to selected value.
    pub path: Box<[PathSegment]>,
}

/// One path segment in an accessor program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PathSegment {
    /// Object field by interned symbol.
    Field(SymbolId),
    /// List index.
    Index(u32),
}

/// Workflow IR validation failures.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorkflowError {
    /// Bound symbol payloads failed metadata validation.
    #[error("invalid symbol metadata: {source}")]
    InvalidSymbolMetadata {
        /// Exact metadata validation failure.
        source: source_metadata::SourceMetadataError,
    },
    /// A step wrote to a repeat counter owned by a different step.
    #[error("step {writer:?} wrote to repeat counter slot {slot:?} owned by step {owner:?}")]
    RepeatCounterForeignWrite {
        /// Owner RepeatStart step.
        owner: StepIdx,
        /// Foreign writing step.
        writer: StepIdx,
        /// Reserved counter slot.
        slot: SlotIdx,
    },
    /// The compiler emitted no nodes.
    #[error("compiled workflow must contain at least one node")]
    EmptyNodes,
    /// Entry step is outside the node array.
    #[error("entry step {entry:?} is outside the node array")]
    EntryOutOfBounds {
        /// Invalid entry step.
        entry: StepIdx,
    },
    /// A node target step is outside the node array.
    #[error("node target step {step:?} is outside the node array")]
    StepOutOfBounds {
        /// Invalid target step.
        step: StepIdx,
    },
    /// A slot reference is outside the run frame.
    #[error("slot {slot:?} is outside slot_count")]
    SlotOutOfBounds {
        /// Invalid slot.
        slot: SlotIdx,
    },
    /// A constant reference is outside the constant pool.
    #[error("constant {constant:?} is outside the constant pool")]
    ConstOutOfBounds {
        /// Invalid constant.
        constant: ConstIdx,
    },
    /// Node identity does not match its table position.
    #[error("node id mismatch: expected {expected:?}, found {actual:?}")]
    NodeIdMismatch {
        /// Expected node id for this table position.
        expected: StepIdx,
        /// Actual node id emitted by the compiler.
        actual: StepIdx,
    },
    /// Expression program failed bytecode validation.
    #[error("expression program is invalid: {0}")]
    Expression(#[from] CoreError),
    /// Resource contract does not cover the compiled artifact.
    #[error("resource contract exceeded: {resource}")]
    ResourceContractExceeded {
        /// Resource dimension that exceeded its declared bound.
        resource: ContractResource,
    },
    /// Resource contract exceeds protocol hard limits.
    #[error("resource contract exceeds hard limit: {resource}")]
    ResourceContractTooLarge {
        /// Resource dimension that exceeds the protocol hard limit.
        resource: ContractResource,
    },
    /// Branching node has no branch and no otherwise route.
    #[error("branch table must contain a branch or otherwise target")]
    EmptyBranchTable,
    /// A node is not reachable from the entry step.
    #[error("node {step:?} is not reachable from the entry step")]
    UnreachableNode {
        /// Unreachable step index.
        step: StepIdx,
    },
    /// An edge points backward without being a recognized loop back-edge.
    #[error("backward edge from {from:?} to {to:?}")]
    BackwardEdge {
        /// Source step of the backward edge.
        from: StepIdx,
        /// Target step of the backward edge.
        to: StepIdx,
    },

    /// Reduce branch body and done targets are the same step.
    #[error("reduce node {step:?} aliases body and done at {target:?}")]
    ReduceBranchTargetAlias {
        /// The reduce node with the alias.
        step: StepIdx,
        /// The shared target step.
        target: StepIdx,
    },
    /// ForEachNext branch body and done targets are the same step.
    #[error("for_each_next node {step:?} aliases body and done at {target:?}")]
    ForEachBranchTargetAlias {
        /// The for_each_next node with the alias.
        step: StepIdx,
        /// The shared target step.
        target: StepIdx,
    },
    /// An inner loop exceeds its outer loop span.
    #[error("inner loop at {inner:?} exceeds outer loop done at {outer_done:?}")]
    ImproperLoopNesting {
        /// Inner loop start step.
        inner: StepIdx,
        /// Outer loop done step.
        outer_done: StepIdx,
    },
    /// Whole-workflow budget exceeded the boundedness policy.
    #[error("budget policy exceeded: {detail}")]
    BudgetPolicyExceeded {
        /// Budget dimension that failed the policy bound.
        detail: BudgetDimension,
    },
    /// Budget step count overflowed during computation.
    #[error("budget step count overflow: {actual} cannot be represented")]
    StepCountOverflow {
        /// The overflowing value.
        actual: u64,
    },
    /// Nesting depth overflowed `u16::MAX` during budget computation.
    #[error("nesting depth overflow: {depth} cannot be incremented past u16::MAX")]
    DepthOverflow {
        /// The actual pre-overflow depth value.
        depth: u16,
    },
    /// A symbol identifier exceeded the declared symbols table bound.
    #[error("symbol {symbol:?} exceeds symbols_count")]
    SymbolOutOfBounds {
        /// Invalid symbol identifier.
        symbol: SymbolId,
    },
    /// An accessor path exceeded the maximum allowed depth.
    #[error("accessor path depth {depth} exceeds maximum {max}")]
    AccessorPathTooDeep {
        /// Actual path depth.
        depth: usize,
        /// Maximum allowed path depth.
        max: usize,
    },
    /// A jump creates a cycle that would cause infinite execution.
    #[error(
        "jump cycle detected: {step:?} jumps to {target:?} which is already in the current traversal path"
    )]
    JumpCycle {
        /// Step issuing the jump.
        step: StepIdx,
        /// Jump target creating the cycle.
        target: StepIdx,
    },
    /// A `TogetherStart` node is reachable from the body of another
    /// `TogetherStart` node. Nested parallel sections are rejected because
    /// `compute_max_parallel_in_flight` reports a per-node maximum and
    /// therefore cannot represent the true peak concurrency of nested
    /// parallel branches, leading to spurious `ParallelLimitExceeded`
    /// failures at runtime. Surface this at compile time instead.
    #[error("nested TogetherStart: outer {outer:?} contains inner {inner:?}")]
    NestedTogether {
        /// The outer `TogetherStart` step that owns the branch body.
        outer: StepIdx,
        /// The inner `TogetherStart` step reachable from the outer branch.
        inner: StepIdx,
    },
    /// A compiled input slot references a slot index that exceeds the declared slot count.
    #[error("input slot {slot:?} kind {kind:?} is outside slot_count")]
    InputSlotOutOfBounds {
        /// The out-of-bounds input slot.
        slot: SlotIdx,
        /// The kind classification of the offending slot.
        kind: InputSlotKind,
    },
    /// A compiled input slot index is duplicated.
    #[error("duplicate input slot {slot:?} kind {kind:?}")]
    DuplicateInputSlot {
        /// The duplicated slot index.
        slot: SlotIdx,
        /// The kind classification of the duplicate.
        kind: InputSlotKind,
    },
    /// Compiled input slots are not in strictly increasing index order.
    #[error("input slots out of canonical order: {prev_slot:?} followed by {curr_slot:?}")]
    NonCanonicalInputSlotOrder {
        /// The preceding slot index.
        prev_slot: SlotIdx,
        /// The out-of-order slot index.
        curr_slot: SlotIdx,
    },
    /// A retained input constraint is structurally inconsistent.
    #[error("input slot {slot:?} has inconsistent constraint: {issue:?}")]
    InconsistentInputConstraint {
        /// The slot carrying the inconsistent constraint.
        slot: SlotIdx,
        /// The structural inconsistency found.
        issue: InputConstraintIssue,
    },
    /// A loop control step has no matching loop body or done target.
    #[error("loop control step {control:?} is unpaired")]
    LoopRegionUnpaired {
        /// The unpaired loop control step.
        control: StepIdx,
    },
    /// Loop regions cross each other, violating proper nesting.
    #[error("loop region at {control:?} crosses from {from:?} to {to:?}")]
    LoopRegionCrossing {
        /// The loop control step with the crossing region.
        control: StepIdx,
        /// The start of the crossing region.
        from: StepIdx,
        /// The end of the crossing region.
        to: StepIdx,
    },
    /// A loop control step references an unsupported node kind.
    #[error("loop control step {control:?} references unsupported step {step:?}")]
    LoopRegionUnsupported {
        /// The loop control step.
        control: StepIdx,
        /// The unsupported referenced step.
        step: StepIdx,
    },
}

/// Bounded postfix expression bytecode program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExprProgram {
    /// Postfix bytecode operations.
    pub ops: Box<[ExprOp]>,
    /// Maximum stack entries required by this program.
    pub max_stack: u8,
}

impl ExprProgram {
    /// Builds a program and computes the exact required stack depth.
    pub fn try_from_ops(ops: Box<[ExprOp]>) -> CoreResult<Self> {
        let max_stack = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK)?;
        Ok(Self { ops, max_stack })
    }

    /// Builds a program from untrusted parts and rejects stale stack metadata.
    pub fn try_from_parts(ops: Box<[ExprOp]>, max_stack: u8) -> CoreResult<Self> {
        let computed = check_expr_stack_bound(&ops, max_stack)?;
        if computed == max_stack {
            Ok(Self { ops, max_stack })
        } else {
            Err(CoreError::InvalidCompiledWorkflow {
                reason: "expression max_stack mismatch",
            })
        }
    }
}

/// Postfix expression bytecode operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ExprOp {
    /// Push a runtime slot value.
    LoadSlot(SlotIdx),
    /// Push a constant-pool value.
    LoadConst(ConstIdx),
    /// Push a value resolved by an accessor program.
    LoadAccessor(AccessorIdx),
    /// Equality comparison.
    Eq,
    /// Inequality comparison.
    NotEq,
    /// Greater-than comparison.
    Gt,
    /// Greater-than-or-equal comparison.
    Gte,
    /// Less-than comparison.
    Lt,
    /// Less-than-or-equal comparison.
    Lte,
    /// Boolean conjunction.
    And,
    /// Boolean disjunction.
    Or,
    /// Boolean negation.
    Not,
    /// Numeric addition.
    Add,
    /// Numeric subtraction.
    Sub,
    /// Numeric multiplication.
    Mul,
    /// Numeric division.
    Div,
    /// `contains` helper.
    Contains,
    /// `starts_with` helper.
    StartsWith,
    /// `ends_with` helper.
    EndsWith,
    /// `has` helper.
    Has,
    /// `exists` helper.
    Exists,
    /// `length` helper.
    Length,
    /// `empty` helper.
    Empty,
    /// `append` helper.
    Append,
    /// `append_if` helper.
    AppendIf,
    /// `merge` helper.
    Merge,
    /// `sum` helper.
    Sum,
    /// `count` helper.
    Count,
    /// `unique` helper.
    Unique,
}

/// Validates stack effects and returns the exact required stack depth.
pub fn check_expr_stack_bound(ops: &[ExprOp], capacity: u8) -> CoreResult<u8> {
    validate_expr_op_count(ops)?;
    let mut depth = 0u8;
    let mut required = 0u8;
    for op in ops {
        depth = apply_expr_stack_effect(depth, *op)?;
        required = required.max(depth);
        validate_expr_stack_capacity(required, capacity)?;
    }
    validate_expr_final_depth(depth)?;
    Ok(required)
}

/// One compiled state-machine node.
///
/// Data routing (`output`) and control routing (`routing`) are separate
/// planes. Field order is frozen: it defines the canonical postcard
/// artifact bytes, so fields are never reordered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledNode {
    /// Step index of this node.
    pub id: StepIdx,
    /// Where this node writes its result.
    pub output: NodeOutput,
    /// Control and error routing for this node.
    pub routing: NodeRouting,
    /// Node behavior.
    pub kind: CompiledNodeKind,
}

/// Where a node writes its result, when it writes one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeOutput {
    /// The node writes nothing.
    Discard,
    /// The node writes its result into this slot.
    Slot(SlotIdx),
}

impl NodeOutput {
    /// Output slot, when this node writes one.
    #[must_use]
    pub const fn slot(self) -> Option<SlotIdx> {
        match self {
            Self::Discard => None,
            Self::Slot(slot) => Some(slot),
        }
    }
}

/// Control-flow routing for a node: fallthrough plus failure handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRouting {
    /// Fallthrough step after this node completes.
    pub next: NodeContinuation,
    /// Where failures of this node are routed.
    pub error: ErrorRouting,
}

/// Fallthrough continuation after a node completes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeContinuation {
    /// No fallthrough step; stepping off raises `MissingNextStep`.
    End,
    /// Fall through to this step.
    Step(StepIdx),
}

impl NodeContinuation {
    /// Fallthrough target, or `None` when this node ends the path.
    #[must_use]
    pub const fn step(self) -> Option<StepIdx> {
        match self {
            Self::End => None,
            Self::Step(step) => Some(step),
        }
    }
}

impl From<Option<StepIdx>> for NodeContinuation {
    fn from(next: Option<StepIdx>) -> Self {
        match next {
            Some(step) => Self::Step(step),
            None => Self::End,
        }
    }
}

impl From<Option<SlotIdx>> for NodeOutput {
    fn from(output: Option<SlotIdx>) -> Self {
        match output {
            Some(slot) => Self::Slot(slot),
            None => Self::Discard,
        }
    }
}

/// Failure routing for a node.
///
/// The serialized form is the legacy `(on_error, error_slot)` option pair,
/// not the enum discriminant: `None` encodes as two absent options and
/// `Handler` as two present values. This keeps postcard bytes identical to
/// the pre-enum layout so existing artifacts and digests keep verifying.
/// Reads are lenient in exactly one case: a legacy present-slot-without-handler
/// pair decodes to `None`, matching the old runtime, which failed the run
/// without touching the slot when no handler was configured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorRouting {
    /// No error handler; failures fail the run.
    None,
    /// Route failures to a handler step, optionally recording into a slot.
    Handler {
        /// Handler step receiving control on failure.
        handler: StepIdx,
        /// Slot receiving failure information before routing.
        error_slot: Option<SlotIdx>,
    },
}

impl Serialize for ErrorRouting {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match *self {
            Self::None => (None::<StepIdx>, None::<SlotIdx>).serialize(serializer),
            Self::Handler {
                handler,
                error_slot,
            } => (Some(handler), error_slot).serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ErrorRouting {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (handler, error_slot) =
            <(Option<StepIdx>, Option<SlotIdx>)>::deserialize(deserializer)?;
        Ok(match handler {
            Some(handler) => Self::Handler {
                handler,
                error_slot,
            },
            None => Self::None,
        })
    }
}

impl ErrorRouting {
    /// Handler step, when routing is configured.
    #[must_use]
    pub const fn handler(self) -> Option<StepIdx> {
        match self {
            Self::None => None,
            Self::Handler { handler, .. } => Some(handler),
        }
    }

    /// Failure-information slot, when configured.
    #[must_use]
    pub const fn error_slot(self) -> Option<SlotIdx> {
        match self {
            Self::None => None,
            Self::Handler { error_slot, .. } => error_slot,
        }
    }
}

#[cfg(kani)]
impl kani::Arbitrary for NodeOutput {
    fn any() -> Self {
        if kani::any() {
            Self::Slot(SlotIdx::new(kani::any()))
        } else {
            Self::Discard
        }
    }
}

#[cfg(kani)]
impl kani::Arbitrary for NodeContinuation {
    fn any() -> Self {
        if kani::any() {
            Self::Step(StepIdx::new(kani::any()))
        } else {
            Self::End
        }
    }
}

#[cfg(kani)]
impl kani::Arbitrary for ErrorRouting {
    fn any() -> Self {
        if kani::any() {
            Self::Handler {
                handler: StepIdx::new(kani::any()),
                error_slot: kani::any(),
            }
        } else {
            Self::None
        }
    }
}

#[cfg(kani)]
impl kani::Arbitrary for NodeRouting {
    fn any() -> Self {
        Self {
            next: kani::any(),
            error: kani::any(),
        }
    }
}

/// Hot-path node variants. All references are numeric.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
#[non_exhaustive]
pub enum CompiledNodeKind {
    /// No-op transition to `next`.
    Nop,
    /// Write a constant-pool value into the output slot.
    SetConst {
        /// Constant-pool index.
        value: ConstIdx,
    },
    /// Copy one slot into the output slot.
    Copy {
        /// Source slot.
        source: SlotIdx,
    },
    /// Evaluate expression bytecode into `output`.
    EvalExpr { expr: ExprIdx },
    /// Build an object handle from numeric field and slot references.
    BuildObject {
        fields: Box<[(crate::ids::SymbolId, SlotIdx)]>,
    },
    /// Build a list handle from numeric slot references.
    BuildList { items: Box<[SlotIdx]> },
    /// Schedule an external action and suspend.
    Do { action: ActionId, input: SlotIdx },
    /// Branch using a pre-materialized boolean condition slot.
    Choose {
        /// Ordered expression branches.
        branches: Box<[ExprBranch]>,
        /// Target when no branch condition is true.
        otherwise: BranchContinuation,
    },
    /// Branch using pre-materialized boolean condition slots.
    ChooseSlot {
        /// Ordered slot branches.
        branches: Box<[SlotBranch]>,
        /// Target when no branch condition is true.
        otherwise: BranchContinuation,
    },
    /// Start a bounded for-each loop.
    ForEachStart {
        input: SlotIdx,
        item_slot: SlotIdx,
        limit: FanoutLimit,
        body: StepIdx,
        done: StepIdx,
    },
    /// Advance a bounded for-each loop.
    ForEachNext {
        iterator_slot: SlotIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Join a for-each loop output.
    ForEachJoin { output: SlotIdx },
    /// Start bounded parallel branches.
    TogetherStart {
        branches: Box<[StepIdx]>,
        join: StepIdx,
    },
    /// Execute one together branch.
    TogetherBranch {
        branch: BranchIdx,
        entry: StepIdx,
        join: StepIdx,
        /// Slot holding the accumulator list (shared with TogetherStart).
        accumulator: SlotIdx,
    },
    /// Join together branches.
    TogetherJoin {
        branch_count: BranchCount,
        /// Slot holding the accumulator list (shared with TogetherStart).
        accumulator: SlotIdx,
    },
    /// Start bounded collection.
    CollectStart {
        source: SlotIdx,
        limit: FanoutLimit,
        page_size: u32,
        body: StepIdx,
        done: StepIdx,
    },
    /// Process one collection page.
    CollectPage {
        collector_slot: SlotIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Advance collection.
    CollectNext {
        collector_slot: SlotIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Finish collection.
    CollectFinish { collector_slot: SlotIdx },
    /// Start bounded reduction.
    ReduceStart {
        input: SlotIdx,
        accumulator: SlotIdx,
        initial: ConstIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Advance reduction.
    ReduceNext {
        iterator_slot: SlotIdx,
        accumulator: SlotIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Finish reduction.
    ReduceFinish { accumulator: SlotIdx },
    /// Start bounded repeat.
    RepeatStart {
        max_attempts: MaxAttempts,
        body: StepIdx,
        done: StepIdx,
    },
    /// Execute repeat attempt.
    RepeatAttempt {
        attempt_slot: SlotIdx,
        body: StepIdx,
        done: StepIdx,
    },
    /// Check repeat state.
    RepeatCheck {
        attempt_slot: SlotIdx,
        done: StepIdx,
    },
    /// Finish repeat.
    RepeatFinish { result: SlotIdx },
    /// Wait until a deadline slot.
    WaitUntil { deadline_slot: SlotIdx },
    /// Wait for an event slot.
    WaitEvent {
        event: SlotIdx,
        timeout_slot: Deadline,
    },
    /// Ask for external input.
    Ask {
        prompt: SlotIdx,
        timeout_slot: Deadline,
    },
    /// Resume an ask.
    AskResume { answer: SlotIdx },
    /// Check retry policy.
    RetryCheck {
        policy_slot: SlotIdx,
        body: StepIdx,
        exhausted: StepIdx,
    },
    /// Run error handler.
    ErrorHandler {
        /// Body step to execute.
        body: StepIdx,
        /// Handler step to route to on body failure.
        handler: StepIdx,
        /// Optional slot to write failed step index for handler consumption.
        error_slot: ErrorSink,
    },
    /// Jump to a numeric target.
    Jump { target: StepIdx },
    /// Finish the run with the selected result slot.
    Finish {
        /// Result slot.
        result: SlotIdx,
    },
}

mod repeat_counter_ownership;

mod loop_regions;
pub use loop_regions::{CompiledLoopRegion, LoopRegionSlot};

mod route_enums;
pub use route_enums::{BranchContinuation, Deadline, ErrorSink};

mod node_assembly;
pub use node_assembly::{node, node_with_handler};

fn validate_parts(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    if parts.nodes.is_empty() {
        return Err(WorkflowError::EmptyNodes);
    }
    validate_resource_contract(parts)?;
    validate_entry(parts.entry, parts.nodes.len())?;
    validate_expressions(&parts.expressions, parts.accessors.len())?;
    validate_accessors(&parts.accessors, parts.slot_count)?;
    for (index, node) in parts.nodes.iter().enumerate() {
        validate_node_id(node, index)?;
        validate_node(node, parts)?;
    }
    validate_accessor_paths(&parts.accessors, parts.symbols_count)?;
    validate_constants_symbols(&parts.constants, parts.symbols_count)?;
    validate_build_object_symbols(&parts.nodes, parts.symbols_count)?;
    validate_reachability(parts)?;
    validate_forward_edges(parts)?;
    repeat_counter_ownership::validate(parts)?;
    // RE-001: reject nested `TogetherStart`. Walked inline here (the
    // helper lives in `crate::engine::validate` to avoid an
    // engine->workflow dependency cycle) so `try_from_parts` rejects
    // the shape at the source. The helper returns
    // `WorkflowError::NestedTogether { outer, inner }`.
    crate::engine::validate::validate_no_nested_together(parts)?;
    Ok(())
}

fn validate_input_slots(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let slot_count = parts.slot_count;
    let mut prev_slot: Option<SlotIdx> = None;
    for input_slot in parts.input_slots.as_ref() {
        let current = input_slot.slot;
        let current_value = current.get();
        if current_value >= slot_count {
            return Err(WorkflowError::InputSlotOutOfBounds {
                slot: current,
                kind: input_slot.kind,
            });
        }
        if let Some(prev) = prev_slot {
            let prev_value = prev.get();
            if current_value == prev_value {
                return Err(WorkflowError::DuplicateInputSlot {
                    slot: current,
                    kind: input_slot.kind,
                });
            }
            if current_value < prev_value {
                return Err(WorkflowError::NonCanonicalInputSlotOrder {
                    prev_slot: prev,
                    curr_slot: current,
                });
            }
        }
        validate_input_constraint(current, &input_slot.constraint)?;
        prev_slot = Some(current);
    }
    Ok(())
}

fn validate_input_constraint(
    slot: SlotIdx,
    constraint: &CompiledInputConstraint,
) -> Result<(), WorkflowError> {
    let inconsistent =
        |issue: InputConstraintIssue| WorkflowError::InconsistentInputConstraint { slot, issue };
    if constraint.value_bound().inconsistency() {
        return Err(inconsistent(InputConstraintIssue::MinExceedsMax));
    }
    if constraint.length_bound().inconsistency() {
        return Err(inconsistent(
            InputConstraintIssue::MinLengthExceedsMaxLength,
        ));
    }
    if constraint.length_bound().lower().is_some_and(|bound| bound < 0)
        || constraint.length_bound().upper().is_some_and(|bound| bound < 0)
    {
        return Err(inconsistent(InputConstraintIssue::NegativeLengthBound));
    }
    Ok(())
}

fn validate_budget(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    use crate::budget::{BoundednessPolicy, WholeWorkflowBudget};

    let budget = WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)?;

    validate_budget_result(BoundednessPolicy::DEFAULT.validate(&budget))
}

fn validate_budget_result(
    result: Result<(), crate::budget::BudgetError>,
) -> Result<(), WorkflowError> {
    match result {
        Ok(()) => Ok(()),
        Err(error) => Err(WorkflowError::BudgetPolicyExceeded {
            detail: error.dimension(),
        }),
    }
}

fn validate_node_id(node: &CompiledNode, index: usize) -> Result<(), WorkflowError> {
    if node.id.as_usize() == index {
        Ok(())
    } else {
        Err(WorkflowError::NodeIdMismatch {
            expected: StepIdx::new(u16::try_from(index).map_err(|_| {
                WorkflowError::ResourceContractExceeded {
                    resource: ContractResource::Steps,
                }
            })?),
            actual: node.id,
        })
    }
}

/// Validates every declared resource-contract limit against its hard limit.
///
/// This is the single canonical implementation of the declared-vs-hard-limit
/// (`TooLarge`) rule shared by admission (`validate_resource_contract`, below)
/// and the public engine pre-check
/// (`engine::validate::validate_resource_contract`). Checks run in
/// `ResourceContract` field order and report the first over-limit dimension.
/// Actual-usage (`Exceeded`) checks stay with the callers: admission applies
/// them after this gate, while the engine pre-check is bounds-only by design.
pub(crate) fn check_resource_contract_limits(
    contract: &ResourceContract,
) -> Result<(), WorkflowError> {
    if usize::from(contract.max_steps) > MAX_STEPS_PER_WORKFLOW {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Steps,
        });
    }
    if usize::from(contract.max_slots) > MAX_SLOTS_PER_WORKFLOW {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Slots,
        });
    }
    if usize::from(contract.max_constants) > MAX_CONSTANTS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Constants,
        });
    }
    if usize::from(contract.max_accessors) > MAX_ACCESSORS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Accessors,
        });
    }
    if usize::from(contract.max_expressions) > MAX_EXPRESSIONS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Expressions,
        });
    }
    if contract.max_expr_stack > MAX_EXPRESSION_STACK {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::ExprStack,
        });
    }
    if contract.max_step_budget_per_tick > MAX_STEP_BUDGET {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::StepBudgetPerTick,
        });
    }
    if contract.max_transitions_per_tick > MAX_STEP_BUDGET {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::TransitionsPerTick,
        });
    }
    if contract.max_input_bytes > MAX_INPUT_BYTES {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::InputBytes,
        });
    }
    if contract.max_output_bytes > MAX_OUTPUT_BYTES {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::OutputBytes,
        });
    }
    if contract.max_blob_bytes > MAX_BLOB_BYTES {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::BlobBytes,
        });
    }
    if contract.max_ipc_payload_bytes > MAX_IPC_PAYLOAD_BYTES {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::IpcPayloadBytes,
        });
    }
    if contract.max_retry_attempts > MAX_RETRY_ATTEMPTS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::RetryAttempts,
        });
    }
    if contract.max_fanout > MAX_FANOUT {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Fanout,
        });
    }
    if contract.max_collect_items > MAX_COLLECT_ITEMS {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::CollectItems,
        });
    }
    if contract.max_queue_depth > MAX_QUEUE_DEPTH {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::QueueDepth,
        });
    }
    if contract.max_journal_batch_bytes > MAX_JOURNAL_BATCH_BYTES {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::JournalBatchBytes,
        });
    }
    Ok(())
}

fn validate_resource_contract(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let contract = parts.resource_contract;
    check_resource_contract_limits(&contract)?;
    validate_resource_usage(parts, contract)?;
    validate_expr_stack_usage(parts.expressions.as_ref(), contract.max_expr_stack)?;
    validate_transitions_nonzero(contract.max_transitions_per_tick)
}

/// Rejects a zero `max_transitions_per_tick` declaration.
///
/// The over-limit half of the old transitions check now lives in the
/// canonical [`check_resource_contract_limits`] gate above; only the
/// admission-only non-zero rule remains here.
fn validate_transitions_nonzero(max_transitions_per_tick: u64) -> Result<(), WorkflowError> {
    if max_transitions_per_tick == 0 {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::TransitionsPerTick,
        });
    }
    Ok(())
}

/// Validates actual resource usage against the declared contract.
///
/// Runs after the canonical [`check_resource_contract_limits`] gate, so every
/// `declared` value handled here is already known to sit within its hard
/// limit; only the actual-vs-declared (`Exceeded`) direction remains.
fn validate_resource_usage(
    parts: &WorkflowParts,
    contract: ResourceContract,
) -> Result<(), WorkflowError> {
    check_actual_within_declared(
        ContractResource::Steps,
        parts.nodes.len(),
        usize::from(contract.max_steps),
    )?;
    check_actual_within_declared(
        ContractResource::Slots,
        usize::from(parts.slot_count),
        usize::from(contract.max_slots),
    )?;
    check_actual_within_declared(
        ContractResource::Constants,
        parts.constants.len(),
        usize::from(contract.max_constants),
    )?;
    check_actual_within_declared(
        ContractResource::Accessors,
        parts.accessors.len(),
        usize::from(contract.max_accessors),
    )?;
    check_actual_within_declared(
        ContractResource::Expressions,
        parts.expressions.len(),
        usize::from(contract.max_expressions),
    )
}

fn check_actual_within_declared(
    resource: ContractResource,
    actual: usize,
    declared: usize,
) -> Result<(), WorkflowError> {
    if actual > declared {
        Err(WorkflowError::ResourceContractExceeded { resource })
    } else {
        Ok(())
    }
}

fn validate_expr_stack_usage(
    expressions: &[ExprProgram],
    max_expr_stack: u8,
) -> Result<(), WorkflowError> {
    if expressions
        .iter()
        .any(|expression| expression.max_stack > max_expr_stack)
    {
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::ExprStack,
        })
    } else {
        Ok(())
    }
}

fn validate_entry(entry: StepIdx, node_count: usize) -> Result<(), WorkflowError> {
    validate_step(entry, node_count).map_err(|_| WorkflowError::EntryOutOfBounds { entry })
}

fn validate_node(node: &CompiledNode, parts: &WorkflowParts) -> Result<(), WorkflowError> {
    validate_node_common(node, parts)?;
    validate_node_kind(&node.kind, parts)?;
    validate_reduce_branch_targets(node)?;
    validate_for_each_next_targets(node)
}

fn validate_reduce_branch_targets(node: &CompiledNode) -> Result<(), WorkflowError> {
    match &node.kind {
        CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. }
            if body == done =>
        {
            Err(WorkflowError::ReduceBranchTargetAlias {
                step: node.id,
                target: *body,
            })
        }
        _ => Ok(()),
    }
}

fn validate_for_each_next_targets(node: &CompiledNode) -> Result<(), WorkflowError> {
    match &node.kind {
        CompiledNodeKind::ForEachNext { body, done, .. } if body == done => {
            Err(WorkflowError::ForEachBranchTargetAlias {
                step: node.id,
                target: *body,
            })
        }
        _ => Ok(()),
    }
}

fn validate_node_common(node: &CompiledNode, parts: &WorkflowParts) -> Result<(), WorkflowError> {
    validate_optional_slot(node.output.slot(), parts.slot_count)?;
    validate_optional_step(node.routing.next.step(), parts.nodes.len())?;
    validate_optional_step(node.routing.error.handler(), parts.nodes.len())?;
    validate_optional_slot(node.routing.error.error_slot(), parts.slot_count)
}

fn validate_node_kind(kind: &CompiledNodeKind, parts: &WorkflowParts) -> Result<(), WorkflowError> {
    match kind {
        CompiledNodeKind::Nop => Ok(()),
        CompiledNodeKind::SetConst { value } => validate_const(*value, parts.constants.len()),
        CompiledNodeKind::Copy { source } => validate_slot(*source, parts.slot_count),
        CompiledNodeKind::EvalExpr { expr } => validate_expr(*expr, parts.expressions.len()),
        CompiledNodeKind::BuildObject { fields } => validate_build_object(fields, parts),
        CompiledNodeKind::BuildList { items } => validate_build_list(items, parts.slot_count),
        CompiledNodeKind::Do { action: _, input } => validate_slot(*input, parts.slot_count),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => validate_slot_choose(branches, *otherwise, parts),
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => validate_expr_choose(branches, *otherwise, parts),
        CompiledNodeKind::ForEachStart {
            input,
            item_slot,
            limit: _,
            body,
            done,
        } => validate_for_each_start(*input, *item_slot, *body, *done, parts),
        CompiledNodeKind::ForEachNext {
            iterator_slot,
            body,
            done,
        } => validate_slot_and_steps(*iterator_slot, *body, *done, parts),
        CompiledNodeKind::ForEachJoin { output } => validate_slot(*output, parts.slot_count),
        CompiledNodeKind::TogetherStart { branches, join } => {
            validate_together(branches, *join, parts)
        }
        CompiledNodeKind::TogetherBranch {
            branch: _,
            entry,
            join,
            accumulator,
        } => {
            validate_two_steps(*entry, *join, parts)?;
            validate_slot(*accumulator, parts.slot_count)
        }
        CompiledNodeKind::TogetherJoin {
            branch_count,
            accumulator,
        } => {
            validate_nonzero_u16(branch_count.get(), ContractResource::BranchCount)?;
            validate_slot(*accumulator, parts.slot_count)
        }
        CompiledNodeKind::CollectStart {
            source,
            limit: _,
            page_size: _,
            body,
            done,
        } => validate_slot_and_steps(*source, *body, *done, parts),
        CompiledNodeKind::CollectPage {
            collector_slot,
            body,
            done,
        }
        | CompiledNodeKind::CollectNext {
            collector_slot,
            body,
            done,
        } => validate_slot_and_steps(*collector_slot, *body, *done, parts),
        CompiledNodeKind::CollectFinish { collector_slot } => {
            validate_slot(*collector_slot, parts.slot_count)
        }
        CompiledNodeKind::ReduceStart {
            input,
            accumulator,
            initial,
            body,
            done,
        } => validate_reduce_start(*input, *accumulator, *initial, *body, *done, parts),
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            body,
            done,
        } => validate_reduce_next(*iterator_slot, *accumulator, *body, *done, parts),
        CompiledNodeKind::ReduceFinish { accumulator } => {
            validate_slot(*accumulator, parts.slot_count)
        }
        CompiledNodeKind::RepeatStart {
            max_attempts,
            body,
            done,
        } => validate_repeat_start(max_attempts.get(), *body, *done, parts),
        CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body,
            done,
        } => validate_slot_and_steps(*attempt_slot, *body, *done, parts),
        CompiledNodeKind::RepeatCheck { attempt_slot, done } => {
            validate_slot(*attempt_slot, parts.slot_count)?;
            validate_step(*done, parts.nodes.len())
        }
        CompiledNodeKind::RepeatFinish { result } => validate_slot(*result, parts.slot_count),
        CompiledNodeKind::WaitUntil { deadline_slot } => {
            validate_slot(*deadline_slot, parts.slot_count)
        }
        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            validate_slot(*event, parts.slot_count)?;
            validate_optional_slot(timeout_slot.slot(), parts.slot_count)
        }
        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            validate_slot(*prompt, parts.slot_count)?;
            validate_optional_slot(timeout_slot.slot(), parts.slot_count)
        }
        CompiledNodeKind::AskResume { answer } => validate_slot(*answer, parts.slot_count),
        CompiledNodeKind::RetryCheck {
            policy_slot,
            body,
            exhausted,
        } => validate_slot_and_steps(*policy_slot, *body, *exhausted, parts),
        CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            validate_two_steps(*body, *handler, parts)
        }
        CompiledNodeKind::Jump { target } => validate_step(*target, parts.nodes.len()),
        CompiledNodeKind::Finish { result } => validate_slot(*result, parts.slot_count),
    }
}

fn validate_optional_slot(slot: Option<SlotIdx>, slot_count: u16) -> Result<(), WorkflowError> {
    slot.map_or(Ok(()), |value| validate_slot(value, slot_count))
}

fn validate_slots(slots: &[SlotIdx], slot_count: u16) -> Result<(), WorkflowError> {
    for slot in slots {
        validate_slot(*slot, slot_count)?;
    }
    Ok(())
}

fn validate_build_list(items: &[SlotIdx], slot_count: u16) -> Result<(), WorkflowError> {
    if items.len() > MAX_LIST_ITEMS_PER_VALUE {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::ListItems,
        });
    }
    validate_slots(items, slot_count)
}

fn validate_build_object(
    fields: &[(crate::ids::SymbolId, SlotIdx)],
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    if fields.len() > MAX_OBJECT_FIELDS_PER_VALUE {
        return Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::ObjectFields,
        });
    }
    for (_, slot) in fields {
        validate_slot(*slot, parts.slot_count)?;
    }
    Ok(())
}

fn validate_for_each_start(
    input: SlotIdx,
    item_slot: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(input, parts.slot_count)?;
    validate_slot(item_slot, parts.slot_count)?;
    validate_two_steps(body, done, parts)
}

fn validate_slot_and_steps(
    slot: SlotIdx,
    first: StepIdx,
    second: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(slot, parts.slot_count)?;
    validate_two_steps(first, second, parts)
}

fn validate_two_steps(
    first: StepIdx,
    second: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_step(first, parts.nodes.len())?;
    validate_step(second, parts.nodes.len())
}

fn validate_together(
    branches: &[StepIdx],
    join: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_branch_route(branches.len(), Some(join))?;
    for branch in branches {
        validate_step(*branch, parts.nodes.len())?;
    }
    validate_step(join, parts.nodes.len())
}

fn validate_nonzero_u16(value: u16, resource: ContractResource) -> Result<(), WorkflowError> {
    if value == 0 {
        Err(WorkflowError::ResourceContractExceeded { resource })
    } else {
        Ok(())
    }
}

fn validate_reduce_start(
    input: SlotIdx,
    accumulator: SlotIdx,
    initial: ConstIdx,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(input, parts.slot_count)?;
    validate_slot(accumulator, parts.slot_count)?;
    validate_const(initial, parts.constants.len())?;
    validate_two_steps(body, done, parts)
}

fn validate_reduce_next(
    iterator_slot: SlotIdx,
    accumulator: SlotIdx,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_slot(iterator_slot, parts.slot_count)?;
    validate_slot(accumulator, parts.slot_count)?;
    validate_two_steps(body, done, parts)
}

fn validate_repeat_start(
    max_attempts: u16,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_nonzero_u16(max_attempts, ContractResource::RetryAttempts)?;
    validate_two_steps(body, done, parts)
}

fn validate_optional_step_continuation(
    continuation: BranchContinuation,
    node_count: usize,
) -> Result<(), WorkflowError> {
    validate_optional_step(continuation.step(), node_count)
}

fn validate_slot_choose(
    branches: &[SlotBranch],
    otherwise: BranchContinuation,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_branch_route(branches.len(), otherwise.step())?;
    branches.iter().try_for_each(|branch| {
        validate_slot(branch.condition, parts.slot_count)?;
        validate_step(branch.target, parts.nodes.len())
    })?;
    validate_optional_step_continuation(otherwise, parts.nodes.len())
}

fn validate_expr_choose(
    branches: &[ExprBranch],
    otherwise: BranchContinuation,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_branch_route(branches.len(), otherwise.step())?;
    branches.iter().try_for_each(|branch| {
        validate_expr(branch.condition, parts.expressions.len())?;
        validate_step(branch.target, parts.nodes.len())
    })?;
    validate_optional_step_continuation(otherwise, parts.nodes.len())
}

fn validate_branch_route(
    branch_count: usize,
    otherwise: Option<StepIdx>,
) -> Result<(), WorkflowError> {
    if branch_count == 0 && otherwise.is_none() {
        Err(WorkflowError::EmptyBranchTable)
    } else {
        Ok(())
    }
}

fn validate_optional_step(step: Option<StepIdx>, node_count: usize) -> Result<(), WorkflowError> {
    step.map_or(Ok(()), |target| validate_step(target, node_count))
}

fn validate_expr(expr: ExprIdx, expression_count: usize) -> Result<(), WorkflowError> {
    if expr.as_usize() < expression_count {
        Ok(())
    } else {
        Err(WorkflowError::Expression(CoreError::ExprOutOfBounds {
            expr,
        }))
    }
}

fn validate_step(step: StepIdx, node_count: usize) -> Result<(), WorkflowError> {
    if step.as_usize() < node_count {
        Ok(())
    } else {
        Err(WorkflowError::StepOutOfBounds { step })
    }
}

fn validate_slot(slot: SlotIdx, slot_count: u16) -> Result<(), WorkflowError> {
    if slot.as_usize() < usize::from(slot_count) {
        Ok(())
    } else {
        Err(WorkflowError::SlotOutOfBounds { slot })
    }
}

fn validate_const(constant: ConstIdx, const_count: usize) -> Result<(), WorkflowError> {
    if constant.as_usize() < const_count {
        Ok(())
    } else {
        Err(WorkflowError::ConstOutOfBounds { constant })
    }
}

fn validate_expressions(
    expressions: &[ExprProgram],
    accessor_count: usize,
) -> Result<(), WorkflowError> {
    for expression in expressions {
        ExprProgram::try_from_parts(expression.ops.clone(), expression.max_stack)?;
        validate_expression_accessors(expression, accessor_count)?;
    }
    Ok(())
}

fn validate_expression_accessors(
    expression: &ExprProgram,
    accessor_count: usize,
) -> Result<(), WorkflowError> {
    for op in expression.ops.as_ref() {
        if let ExprOp::LoadAccessor(accessor) = op {
            validate_accessor(*accessor, accessor_count)?;
        }
    }
    Ok(())
}

fn validate_accessors(accessors: &[AccessorProgram], slot_count: u16) -> Result<(), WorkflowError> {
    for accessor in accessors {
        validate_slot(accessor.root, slot_count)?;
    }
    Ok(())
}

fn validate_accessor(accessor: AccessorIdx, accessor_count: usize) -> Result<(), WorkflowError> {
    if accessor.as_usize() < accessor_count {
        Ok(())
    } else {
        Err(WorkflowError::Expression(
            CoreError::InvalidCompiledWorkflow {
                reason: "accessor index out of bounds",
            },
        ))
    }
}

/// Validates accessor paths: depth limits, reserved index values, and SymbolId bounds.
fn validate_accessor_paths(
    accessors: &[AccessorProgram],
    symbols_count: u32,
) -> Result<(), WorkflowError> {
    for accessor in accessors {
        let path_len = accessor.path.len();
        if path_len > MAX_PATH_DEPTH {
            return Err(WorkflowError::AccessorPathTooDeep {
                depth: path_len,
                max: MAX_PATH_DEPTH,
            });
        }
        for segment in accessor.path.as_ref() {
            match *segment {
                PathSegment::Field(symbol) => {
                    validate_symbol(symbol, symbols_count)?;
                }
                PathSegment::Index(index) => {
                    if index == u32::MAX {
                        return Err(WorkflowError::Expression(
                            CoreError::InvalidCompiledWorkflow {
                                reason: "accessor path index uses reserved value u32::MAX",
                            },
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Validates SymbolId values in the constant pool against the declared symbols count.
fn validate_constants_symbols(
    constants: &[ConstValue],
    symbols_count: u32,
) -> Result<(), WorkflowError> {
    for constant in constants {
        if let ConstValue::Symbol(symbol) = *constant {
            validate_symbol(symbol, symbols_count)?;
        }
    }
    Ok(())
}

/// Validates SymbolId values in BuildObject fields across all nodes.
fn validate_build_object_symbols(
    nodes: &[CompiledNode],
    symbols_count: u32,
) -> Result<(), WorkflowError> {
    for node in nodes {
        if let CompiledNodeKind::BuildObject { fields } = &node.kind {
            for (symbol, _slot) in fields.as_ref() {
                validate_symbol(*symbol, symbols_count)?;
            }
        }
    }
    Ok(())
}

/// Validates that a symbol identifier falls within the declared symbols table bound.
fn validate_symbol(symbol: SymbolId, symbols_count: u32) -> Result<(), WorkflowError> {
    if symbol.get() < symbols_count {
        Ok(())
    } else {
        Err(WorkflowError::SymbolOutOfBounds { symbol })
    }
}

/// Check A: every node must be reachable from the entry step via a forward walk
/// following `next` edges and kind-specific targets.
fn validate_reachability(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let node_count = parts.nodes.len();
    if node_count == 0 {
        return Ok(());
    }

    let mut visited: Vec<bool> = vec![false; node_count];
    let mut queue: Vec<usize> = Vec::new();

    let entry_usize = parts.entry.as_usize();
    if entry_usize >= node_count {
        return Ok(());
    }
    let Some(entry_flag) = visited.get_mut(entry_usize) else {
        return Err(WorkflowError::EntryOutOfBounds { entry: parts.entry });
    };
    *entry_flag = true;
    queue.push(entry_usize);

    let mut head = 0usize;
    while head < queue.len() {
        let current = match queue.get(head) {
            Some(&v) => v,
            None => break,
        };
        head = match head.checked_add(1) {
            Some(v) => v,
            None => break,
        };

        let mut targets: Vec<StepIdx> = Vec::new();
        let node = match parts.nodes.get(current) {
            Some(n) => n,
            None => break,
        };
        if let Some(next) = node.routing.next.step() {
            targets.push(next);
        }
        if let Some(handler) = node.routing.error.handler() {
            targets.push(handler);
        }
        collect_node_targets(&node.kind, &mut targets);

        for target in targets {
            let target_usize = target.as_usize();
            if target_usize < node_count {
                let Some(flag) = visited.get_mut(target_usize) else {
                    continue;
                };
                if !*flag {
                    *flag = true;
                    queue.push(target_usize);
                }
            }
        }
    }

    for (index, was_visited) in visited.iter().enumerate() {
        if !was_visited {
            return Err(WorkflowError::UnreachableNode {
                step: StepIdx::new(u16::try_from(index).map_err(|_| {
                    WorkflowError::ResourceContractExceeded {
                        resource: ContractResource::Steps,
                    }
                })?),
            });
        }
    }
    Ok(())
}

/// Collects all StepIdx targets referenced by a node kind (branch targets,
/// loop body/done, jump target, etc.) but NOT the `next` field.
fn collect_node_targets(kind: &CompiledNodeKind, targets: &mut Vec<StepIdx>) {
    match kind {
        CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::BuildObject { .. }
        | CompiledNodeKind::BuildList { .. }
        | CompiledNodeKind::Do { .. }
        | CompiledNodeKind::ForEachJoin { .. }
        | CompiledNodeKind::CollectFinish { .. }
        | CompiledNodeKind::ReduceFinish { .. }
        | CompiledNodeKind::RepeatFinish { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. }
        | CompiledNodeKind::Finish { .. }
        | CompiledNodeKind::TogetherJoin { .. }
        | CompiledNodeKind::WaitEvent { .. } => {}
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            collect_choose_slot_targets(branches, *otherwise, targets);
        }
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            collect_choose_expr_targets(branches, *otherwise, targets);
        }
        CompiledNodeKind::ForEachStart { body, done, .. }
        | CompiledNodeKind::ForEachNext { body, done, .. }
        | CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::CollectPage { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. }
        | CompiledNodeKind::RepeatStart { body, done, .. }
        | CompiledNodeKind::RepeatAttempt { body, done, .. }
        | CompiledNodeKind::RetryCheck {
            body,
            exhausted: done,
            ..
        } => {
            targets.push(*body);
            targets.push(*done);
        }
        CompiledNodeKind::RepeatCheck { done, .. } => {
            targets.push(*done);
        }
        CompiledNodeKind::TogetherStart { branches, join } => {
            collect_together_start_targets(branches, *join, targets);
        }
        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            targets.push(*entry);
            targets.push(*join);
        }
        CompiledNodeKind::ErrorHandler { body, handler, .. } => {
            targets.push(*body);
            targets.push(*handler);
        }
        CompiledNodeKind::Jump { target } => {
            targets.push(*target);
        }
    }
}

fn collect_choose_slot_targets(
    branches: &[SlotBranch],
    otherwise: BranchContinuation,
    targets: &mut Vec<StepIdx>,
) {
    for branch in branches {
        targets.push(branch.target);
    }
    if let Some(fallback) = otherwise.step() {
        targets.push(fallback);
    }
}

fn collect_choose_expr_targets(
    branches: &[ExprBranch],
    otherwise: BranchContinuation,
    targets: &mut Vec<StepIdx>,
) {
    for branch in branches {
        targets.push(branch.target);
    }
    if let Some(fallback) = otherwise.step() {
        targets.push(fallback);
    }
}

fn collect_together_start_targets(branches: &[StepIdx], join: StepIdx, targets: &mut Vec<StepIdx>) {
    for branch in branches {
        targets.push(*branch);
    }
    targets.push(join);
}

/// Check B: all edges must point forward except recognized loop back-edges.
/// Check D: loop spans must be properly nested (no overlapping loops).
fn validate_forward_edges(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let mut loop_spans: Vec<(usize, usize)> = Vec::new();

    for (index, node) in parts.nodes.iter().enumerate() {
        let current_id = StepIdx::new(u16::try_from(index).map_err(|_| {
            WorkflowError::ResourceContractExceeded {
                resource: ContractResource::Steps,
            }
        })?);

        if let Some(next) = node.routing.next.step() {
            if matches!(node.kind, CompiledNodeKind::RepeatCheck { .. }) && next.as_usize() <= index
            {
                repeat_check_validate::validate(parts, index, node, next)?;
            } else {
                validate_forward_target(next, index, current_id)?;
            }
        }

        if let Some(handler) = node.routing.error.handler() {
            validate_forward_target(handler, index, current_id)?;
        }

        validate_kind_edges(&node.kind, index, current_id)?;

        push_loop_span(&node.kind, index, &mut loop_spans)?;
    }
    Ok(())
}

/// Validates that kind-specific edges respect the forward-only rule.
fn validate_kind_edges(
    kind: &CompiledNodeKind,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    match kind {
        CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::BuildObject { .. }
        | CompiledNodeKind::BuildList { .. }
        | CompiledNodeKind::Do { .. }
        | CompiledNodeKind::ForEachJoin { .. }
        | CompiledNodeKind::TogetherJoin { .. }
        | CompiledNodeKind::CollectFinish { .. }
        | CompiledNodeKind::ReduceFinish { .. }
        | CompiledNodeKind::RepeatFinish { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. }
        | CompiledNodeKind::Finish { .. } => Ok(()),
        CompiledNodeKind::Jump { target } => validate_forward_target(*target, ci, cid),
        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => validate_choose_slot_edges(branches, otherwise, ci, cid),
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => validate_choose_expr_edges(branches, otherwise, ci, cid),
        CompiledNodeKind::ForEachStart { body, done, .. } => {
            validate_loop_start_edges(*body, *done, ci, cid)
        }
        CompiledNodeKind::ForEachNext { done, .. } => validate_loop_done_only(*done, ci, cid),
        CompiledNodeKind::TogetherStart { branches, join } => {
            validate_together_start_edges(branches, *join, ci, cid)
        }
        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            validate_together_branch_edges(*entry, *join, ci, cid)
        }
        CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::RepeatStart { body, done, .. } => {
            validate_loop_start_edges(*body, *done, ci, cid)
        }
        CompiledNodeKind::CollectPage { done, .. }
        | CompiledNodeKind::CollectNext { done, .. }
        | CompiledNodeKind::ReduceNext { done, .. }
        | CompiledNodeKind::RepeatAttempt { done, .. } => validate_loop_done_only(*done, ci, cid),
        CompiledNodeKind::RepeatCheck { done, .. } => validate_forward_target(*done, ci, cid),
        CompiledNodeKind::RetryCheck { exhausted, .. } => {
            validate_loop_done_only(*exhausted, ci, cid)
        }
        CompiledNodeKind::ErrorHandler { handler, .. } => {
            validate_loop_done_only(*handler, ci, cid)
        }
    }
}

fn validate_choose_slot_edges(
    branches: &[SlotBranch],
    otherwise: &BranchContinuation,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    for branch in branches {
        validate_forward_target(branch.target, ci, cid)?;
    }
    if let Some(fallback) = otherwise.step() {
        validate_forward_target(fallback, ci, cid)?;
    }
    Ok(())
}

fn validate_choose_expr_edges(
    branches: &[ExprBranch],
    otherwise: &BranchContinuation,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    for branch in branches {
        validate_forward_target(branch.target, ci, cid)?;
    }
    if let Some(fallback) = otherwise.step() {
        validate_forward_target(fallback, ci, cid)?;
    }
    Ok(())
}

fn validate_loop_done_only(done: StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    validate_forward_target(done, ci, cid)
}

/// Validates RepeatCheck backward next-edges against RepeatStart loop owners.
/// RepeatCheck.next must target the owning RepeatStart's body step, and the
/// check must match the owner's attempt slot and done target.
mod repeat_check_validate {
    use super::{CompiledNode, CompiledNodeKind, StepIdx, WorkflowError, WorkflowParts};

    /// Finds the nearest active enclosing structured start.
    /// Active means the structured start's done/join target extends beyond check_index.
    fn nearest_active_owner(
        parts: &WorkflowParts,
        check_index: usize,
    ) -> Option<(usize, &CompiledNode)> {
        for (i, node) in parts.nodes.iter().take(check_index).enumerate().rev() {
            let end_target = match &node.kind {
                CompiledNodeKind::ForEachStart { done, .. }
                | CompiledNodeKind::CollectStart { done, .. }
                | CompiledNodeKind::ReduceStart { done, .. }
                | CompiledNodeKind::RepeatStart { done, .. } => *done,
                CompiledNodeKind::TogetherStart { join, .. } => *join,
                _ => continue,
            };
            if end_target.as_usize() > check_index {
                return Some((i, node));
            }
        }
        None
    }

    pub(super) fn validate(
        parts: &WorkflowParts,
        check_index: usize,
        check: &CompiledNode,
        back_target: StepIdx,
    ) -> Result<(), WorkflowError> {
        let Some((owner_index, owner)) = nearest_active_owner(parts, check_index) else {
            return Err(WorkflowError::BackwardEdge {
                from: check.id,
                to: back_target,
            });
        };
        if !matches!(owner.kind, CompiledNodeKind::RepeatStart { .. }) {
            return Err(WorkflowError::BackwardEdge {
                from: check.id,
                to: back_target,
            });
        }
        let owner_body = match &owner.kind {
            CompiledNodeKind::RepeatStart { body, .. } => *body,
            _ => {
                return Err(WorkflowError::BackwardEdge {
                    from: check.id,
                    to: back_target,
                });
            }
        };
        let owner_done = match &owner.kind {
            CompiledNodeKind::RepeatStart { done, .. } => *done,
            _ => {
                return Err(WorkflowError::BackwardEdge {
                    from: check.id,
                    to: back_target,
                });
            }
        };
        let check_attempt = match &check.kind {
            CompiledNodeKind::RepeatCheck { attempt_slot, .. } => *attempt_slot,
            _ => {
                return Err(WorkflowError::BackwardEdge {
                    from: check.id,
                    to: back_target,
                });
            }
        };
        let check_done = match &check.kind {
            CompiledNodeKind::RepeatCheck { done, .. } => *done,
            _ => {
                return Err(WorkflowError::BackwardEdge {
                    from: check.id,
                    to: back_target,
                });
            }
        };
        if owner.output.slot() != Some(check_attempt) {
            return Err(WorkflowError::BackwardEdge {
                from: check.id,
                to: back_target,
            });
        }
        if owner_body != back_target {
            return Err(WorkflowError::BackwardEdge {
                from: check.id,
                to: back_target,
            });
        }
        if owner_done != check_done {
            return Err(WorkflowError::BackwardEdge {
                from: check.id,
                to: back_target,
            });
        }
        if owner_index >= check_index {
            return Err(WorkflowError::BackwardEdge {
                from: check.id,
                to: back_target,
            });
        }
        if owner_body.as_usize() >= check_index {
            return Err(WorkflowError::BackwardEdge {
                from: check.id,
                to: back_target,
            });
        }
        if check_index >= owner_done.as_usize() {
            return Err(WorkflowError::BackwardEdge {
                from: check.id,
                to: back_target,
            });
        }
        Ok(())
    }
}

/// Validates that both `body` and `done` edges of a `*Start` loop node point
/// strictly forward from the current node. `body` here is the entry into the
/// loop body (a forward edge); only `*Next` carries the back-edge in its
/// `body` slot, so `*Start` must require `body` to be forward.
fn validate_loop_start_edges(
    body: StepIdx,
    done: StepIdx,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    validate_forward_target(body, ci, cid)?;
    validate_forward_target(done, ci, cid)
}

fn validate_together_start_edges(
    branches: &[StepIdx],
    join: StepIdx,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    for branch in branches {
        validate_forward_target(*branch, ci, cid)?;
    }
    validate_forward_target(join, ci, cid)
}

fn validate_together_branch_edges(
    entry: StepIdx,
    join: StepIdx,
    ci: usize,
    cid: StepIdx,
) -> Result<(), WorkflowError> {
    validate_forward_target(entry, ci, cid)?;
    validate_forward_target(join, ci, cid)
}

/// Validates a target step is strictly forward from the current node.
fn validate_forward_target(target: StepIdx, ci: usize, cid: StepIdx) -> Result<(), WorkflowError> {
    if target.as_usize() > ci {
        Ok(())
    } else {
        Err(WorkflowError::BackwardEdge {
            from: cid,
            to: target,
        })
    }
}

/// Tracks loop spans for nesting validation (Check D).
fn push_loop_span(
    kind: &CompiledNodeKind,
    ci: usize,
    spans: &mut Vec<(usize, usize)>,
) -> Result<(), WorkflowError> {
    let done_usize: Option<usize> = match kind {
        CompiledNodeKind::ForEachStart { done, .. }
        | CompiledNodeKind::CollectStart { done, .. }
        | CompiledNodeKind::ReduceStart { done, .. }
        | CompiledNodeKind::RepeatStart { done, .. } => Some(done.as_usize()),
        CompiledNodeKind::TogetherStart { join, .. } => Some(join.as_usize()),
        _ => None,
    };

    let Some(done_idx) = done_usize else {
        return Ok(());
    };

    // Retire finished spans BEFORE the nesting comparison so that
    // sequential loops are not rejected as improperly nested.
    while spans
        .last()
        .is_some_and(|&(_, done): &(usize, usize)| done <= ci)
    {
        spans.pop();
    }

    match spans.last().copied() {
        Some((_outer_start, outer_done)) if done_idx > outer_done => {
            return Err(WorkflowError::ImproperLoopNesting {
                inner: StepIdx::new(u16::try_from(ci).map_err(|_| {
                    WorkflowError::ResourceContractExceeded {
                        resource: ContractResource::Steps,
                    }
                })?),
                outer_done: StepIdx::new(u16::try_from(outer_done).map_err(|_| {
                    WorkflowError::ResourceContractExceeded {
                        resource: ContractResource::Steps,
                    }
                })?),
            });
        }
        _ => {}
    }

    spans.push((ci, done_idx));
    Ok(())
}

fn validate_expr_op_count(ops: &[ExprOp]) -> CoreResult<()> {
    if ops.len() > MAX_EXPRESSION_OPS {
        Err(CoreError::ResourceLimitExceeded {
            resource: "expression ops",
        })
    } else {
        Ok(())
    }
}

fn apply_expr_stack_effect(depth: u8, op: ExprOp) -> CoreResult<u8> {
    let effect = expr_stack_effect(op);
    let consumed = depth
        .checked_sub(effect.pop)
        .ok_or(CoreError::ExpressionStackUnderflow)?;
    consumed
        .checked_add(effect.push)
        .ok_or(CoreError::ExpressionStackOverflow {
            max: MAX_EXPRESSION_STACK,
        })
}

fn validate_expr_stack_capacity(required: u8, capacity: u8) -> CoreResult<()> {
    if required <= capacity && required <= MAX_EXPRESSION_STACK {
        Ok(())
    } else {
        Err(CoreError::ExpressionStackOverflow { max: capacity })
    }
}

fn validate_expr_final_depth(depth: u8) -> CoreResult<()> {
    match depth {
        0 => Err(CoreError::ExpressionStackUnderflow),
        1 => Ok(()),
        _ => Err(CoreError::InvalidCompiledWorkflow {
            reason: "expression leaves non-single result",
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StackEffect {
    pop: u8,
    push: u8,
}

const fn expr_stack_effect(op: ExprOp) -> StackEffect {
    match op {
        ExprOp::LoadSlot(_) | ExprOp::LoadConst(_) | ExprOp::LoadAccessor(_) => effect(0, 1),
        ExprOp::Not
        | ExprOp::Exists
        | ExprOp::Length
        | ExprOp::Empty
        | ExprOp::Sum
        | ExprOp::Count
        | ExprOp::Unique => effect(1, 1),
        ExprOp::AppendIf => effect(3, 1),
        _ => effect(2, 1),
    }
}

const fn effect(pop: u8, push: u8) -> StackEffect {
    StackEffect { pop, push }
}

// ============================================================================
// Lifecycle state machine
// ============================================================================

/// Lifecycle state of a run derived from journal event replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LifecycleState {
    /// Run accepted but not yet active.
    Pending,
    /// Run is actively executing.
    Active,
    /// Run is waiting for an external answer.
    WaitingAnswer,
    /// Run was cancelled.
    Cancelled,
    /// Run completed successfully.
    Completed,
    /// Run failed.
    Failed,
}

impl LifecycleState {
    /// Returns true if this is a terminal state.
    /// Note: Failed is NOT terminal because retry can transition from Failed.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Cancelled | Self::Completed)
    }
}

/// Lifecycle command issued by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LifecycleCommand {
    /// Cancel the run.
    Cancel,
    /// Resume a waiting run.
    Resume,
    /// Retry a failed run.
    Retry,
    /// Answer a waiting run's question.
    Answer,
}

/// Checks if a lifecycle state transition is valid for the given command.
#[must_use]
pub const fn check_lifecycle_transition(state: LifecycleState, cmd: LifecycleCommand) -> bool {
    match (state, cmd) {
        // Cancel is valid from Active or WaitingAnswer
        (LifecycleState::Active, LifecycleCommand::Cancel) => true,
        (LifecycleState::WaitingAnswer, LifecycleCommand::Cancel) => true,
        // Resume is valid from WaitingAnswer
        (LifecycleState::WaitingAnswer, LifecycleCommand::Resume) => true,
        // Retry is valid from Failed
        (LifecycleState::Failed, LifecycleCommand::Retry) => true,
        // Answer is valid from WaitingAnswer
        (LifecycleState::WaitingAnswer, LifecycleCommand::Answer) => true,
        // All other transitions are invalid
        _ => false,
    }
}

/// Run state snapshot returned by replay.
#[derive(Debug, Clone)]
pub struct RunState {
    /// Current lifecycle state.
    pub lifecycle: LifecycleState,
    /// Run identifier.
    pub run_id: RunId,
}

impl RunState {
    /// Returns true if this run is in a terminal state.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        self.lifecycle.is_terminal()
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
mod proptest_collect_budget;

#[cfg(test)]
mod proptest_collect_traversal;
