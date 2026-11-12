//! Immutable, name-only secret input bindings and restored-root checks.
use crate::workflow::{CompiledNodeKind as K, CompiledWorkflow, WorkflowError, WorkflowParts};
use crate::{
    CompiledInputConstraint, CoreError, CoreResult, InputSlotKind, SlotIdx, SlotValue, Taint,
};

impl super::CompiledSecretRequirement {
    /// Creates a table-relative presence-only requirement without an input root.
    #[must_use]
    pub const fn new(alias: crate::SymbolId, requirement: crate::SymbolId) -> Self {
        Self {
            alias,
            requirement,
            slot: None,
        }
    }

    /// Binds a presence-only requirement to an immutable secret input root.
    #[must_use]
    pub const fn with_slot(
        alias: crate::SymbolId,
        requirement: crate::SymbolId,
        slot: SlotIdx,
    ) -> Self {
        Self {
            alias,
            requirement,
            slot: Some(slot),
        }
    }

    /// Input root carrying the opaque requirement reference, when executable.
    #[must_use]
    pub const fn slot(self) -> Option<SlotIdx> {
        self.slot
    }
}

/// Checks secret roots against their compiler-retained declaration and writers.
pub fn validate(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let Some(metadata) = &parts.source_metadata else {
        return Ok(());
    };
    for (index, binding) in metadata.secret_requirements().iter().enumerate() {
        let Some(slot) = binding.slot() else {
            continue;
        };
        if metadata
            .secret_requirements()
            .iter()
            .take(index)
            .any(|previous| previous.slot() == Some(slot))
        {
            return Err(invalid());
        }
        let expected = CompiledInputConstraint {
            secretness: true.into(),
            default_cold: Some(crate::InputDefaultValue::Text(
                metadata
                    .symbol(binding.requirement())
                    .ok_or_else(invalid)?
                    .into(),
            )),
            ..CompiledInputConstraint::required()
        };
        if !parts.input_slots.iter().any(|input| {
            input.slot == slot
                && input.kind == InputSlotKind::Symbol
                && input.constraint == expected
        }) || parts.nodes.iter().any(|node| writes(parts, node, slot))
        {
            return Err(invalid());
        }
    }
    Ok(())
}

/// Validates canonical supplied or recovered roots without granting host presence.
pub fn validate_roots(
    workflow: &CompiledWorkflow,
    roots: &[(SlotIdx, SlotValue, Taint)],
) -> CoreResult<()> {
    let Some(metadata) = workflow.source_metadata() else {
        return Ok(());
    };
    for binding in metadata.secret_requirements() {
        let Some(slot) = binding.slot() else {
            continue;
        };
        if let Some((_, value, taint)) = roots.iter().find(|(candidate, _, _)| *candidate == slot)
            && (*value != SlotValue::Symbol(binding.requirement()) || *taint != Taint::Secret)
        {
            return Err(CoreError::InternalInvariantViolation {
                reason: "secret_input_binding".into(),
            });
        }
    }
    Ok(())
}

/// Restored frames must retain every immutable secret root initialized at admission.
pub fn validate_runtime_roots(
    workflow: &CompiledWorkflow,
    roots: &[(SlotIdx, SlotValue, Taint)],
) -> CoreResult<()> {
    validate_roots(workflow, roots)?;
    if workflow.source_metadata().is_some_and(|metadata| {
        metadata.secret_requirements().iter().any(|binding| {
            binding
                .slot()
                .is_some_and(|slot| !roots.iter().any(|(candidate, _, _)| *candidate == slot))
        })
    }) {
        return Err(CoreError::InternalInvariantViolation {
            reason: "secret_input_binding".into(),
        });
    }
    Ok(())
}

fn writes(parts: &WorkflowParts, node: &crate::CompiledNode, slot: SlotIdx) -> bool {
    if node.output.slot() == Some(slot) || node.routing.error.error_slot() == Some(slot) {
        return true;
    }
    match node.kind {
        K::ForEachStart { item_slot, .. } => item_slot == slot,
        K::ForEachNext { iterator_slot, .. } | K::ReduceNext { iterator_slot, .. } => {
            iterator_slot == slot
        }
        K::TogetherBranch { accumulator, .. } | K::TogetherJoin { accumulator, .. } => {
            accumulator == slot
        }
        K::CollectStart { source, .. } => node.output.slot().is_none() && source == slot,
        K::CollectNext { collector_slot, .. } => collector_slot == slot,
        K::ReduceStart {
            input, accumulator, ..
        } => input == slot || accumulator == slot,
        K::RepeatCheck { attempt_slot, .. } => attempt_slot == slot,
        K::RetryCheck { policy_slot, .. } => policy_slot == slot,
        K::Ask { .. } => node
            .routing
            .next
            .step()
            .and_then(|next| parts.nodes.get(next.as_usize()))
            .is_some_and(|resume| matches!(resume.kind, K::AskResume {answer} if answer == slot)),
        K::ErrorHandler { error_slot, .. } => error_slot.slot() == Some(slot),
        _ => false,
    }
}

fn invalid() -> WorkflowError {
    WorkflowError::SourceMetadata {
        detail: "invalid immutable secret input binding".into(),
    }
}
