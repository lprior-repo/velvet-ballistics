#![forbid(unsafe_code)]
//! Immutable compiled workflow assembled from validated parts.

use super::validate::{validate_budget, validate_input_slots, validate_parts};
use super::{
    AccessorProgram, CompiledInputSlot, CompiledLoopRegion, CompiledNode, CompiledNodeKind,
    ExprProgram, LoopRegionSlot, ResourceContract, WorkflowError, WorkflowParts,
};
use super::{cursor_ownership, loop_regions, retry_policy, slot_flow, source_metadata};
use crate::ids::{AccessorIdx, ConstIdx, ExprIdx, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::ConstValue;

/// Immutable compiled workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledWorkflow {
    name: Box<str>,
    digest: WorkflowDigest,
    semantic_source_identity: Option<WorkflowDigest>,
    source_metadata: Option<Box<source_metadata::WorkflowSourceMetadata>>,
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
}

impl CompiledWorkflow {
    /// Creates a compiled workflow after validating all numeric references.
    pub fn try_from_parts(parts: WorkflowParts) -> Result<Self, WorkflowError> {
        validate_parts(&parts)?;
        if let Some(metadata) = &parts.source_metadata {
            metadata
                .validate_symbols_count(parts.symbols_count)
                .map_err(|error| WorkflowError::SourceMetadata {
                    detail: error.to_string().into(),
                })?;
        }
        let loop_regions = loop_regions::validate(&parts)?;
        cursor_ownership::validate(&parts, &loop_regions)?;
        let nested_activation_entries = loop_regions::nested_activation_map(&parts, &loop_regions)?;
        let loop_slots = loop_regions::merge_slots(&loop_regions, &nested_activation_entries);
        validate_budget(&parts)?;
        source_metadata::resources::validate(&parts)?;
        validate_input_slots(&parts)?;
        source_metadata::secrets::validate(&parts)?;
        retry_policy::validate(&parts)?;
        slot_flow::validate_slot_flow(&parts)?;
        Ok(Self {
            name: parts.name,
            digest: parts.digest,
            semantic_source_identity: parts.semantic_source_identity,
            source_metadata: parts.source_metadata,
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
            source_metadata: parts.source_metadata,
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
            source_metadata: parts.source_metadata,
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

    /// Converts back to the serializable parts representation for artifact emission.
    #[must_use]
    pub fn to_parts(&self) -> WorkflowParts {
        WorkflowParts {
            name: self.name.clone(),
            digest: self.digest,
            semantic_source_identity: self.semantic_source_identity,
            source_metadata: self.source_metadata.clone(),
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
    /// Immutable compiled source symbol payloads.
    #[must_use]
    pub fn source_metadata(&self) -> Option<&source_metadata::WorkflowSourceMetadata> {
        self.source_metadata.as_deref()
    }

    /// Resolves a node's direct error slot before considering a legacy wrapper fallback.
    #[must_use]
    pub fn resolved_error_slot(&self, node: &CompiledNode) -> Option<SlotIdx> {
        resolved_error_slot(&self.nodes, node)
    }
}

// A direct error slot takes precedence; legacy wrappers provide only a fallback.
pub(crate) fn resolved_error_slot(nodes: &[CompiledNode], node: &CompiledNode) -> Option<SlotIdx> {
    node.routing.error.error_slot().or_else(|| {
        nodes.iter().find(|candidate| {
            matches!(candidate.kind, CompiledNodeKind::ErrorHandler { body, .. } if body == node.id)
        }).and_then(|wrapper| wrapper.routing.error.error_slot())
    })
}
