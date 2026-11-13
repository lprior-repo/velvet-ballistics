#![forbid(unsafe_code)]
//! Shared checked construction for legacy and concurrent execution profiles.

use super::super::WorkflowIrProfile;
use super::super::ir_profile::validate_profile;
use super::*;

impl CompiledWorkflow {
    pub(super) fn try_from_parts_for_profile(
        parts: WorkflowParts,
        profile: WorkflowIrProfile,
    ) -> Result<Self, WorkflowError> {
        validate_profile(&parts, profile)?;
        let loop_slots = Self::validate_shared_parts(&parts)?;
        let concurrent_requirements = Self::checked_concurrent_requirements(&parts, profile)?;
        Ok(Self::from_validated_parts(
            parts,
            loop_slots,
            profile,
            concurrent_requirements,
        ))
    }

    fn validate_shared_parts(
        parts: &WorkflowParts,
    ) -> Result<Box<[LoopRegionSlot]>, WorkflowError> {
        validate_parts(parts)?;
        Self::validate_source_count(parts)?;
        let loop_slots = Self::validated_loop_slots(parts)?;
        validate_budget(parts)?;
        source_metadata::resources::validate(parts)?;
        validate_input_slots(parts)?;
        source_metadata::secrets::validate(parts)?;
        retry_policy::validate(parts)?;
        slot_flow::validate_slot_flow(parts)?;
        Ok(loop_slots)
    }

    fn validate_source_count(parts: &WorkflowParts) -> Result<(), WorkflowError> {
        if let Some(metadata) = &parts.source_metadata {
            metadata
                .validate_symbols_count(parts.symbols_count)
                .map_err(|error| WorkflowError::SourceMetadata {
                    detail: error.to_string().into(),
                })?;
        }
        Ok(())
    }

    fn validated_loop_slots(parts: &WorkflowParts) -> Result<Box<[LoopRegionSlot]>, WorkflowError> {
        let loop_regions = loop_regions::validate(parts)?;
        cursor_ownership::validate(parts, &loop_regions)?;
        let nested_activation_entries = loop_regions::nested_activation_map(parts, &loop_regions)?;
        let loop_slots = loop_regions::merge_slots(&loop_regions, &nested_activation_entries);
        Ok(loop_slots)
    }

    fn checked_concurrent_requirements(
        parts: &WorkflowParts,
        profile: WorkflowIrProfile,
    ) -> Result<Option<ConcurrentRequirements>, WorkflowError> {
        if profile == WorkflowIrProfile::LegacyV2 {
            return Ok(None);
        }
        let super::super::concurrent_analyze::ConcurrentValidation {
            regions,
            requirements,
        } = super::super::concurrent_analyze::validate_after_common(parts)?;
        drop(regions);
        Ok(Some(ConcurrentRequirements::new(
            requirements.frames,
            requirements.groups,
        )))
    }

    fn from_validated_parts(
        parts: WorkflowParts,
        loop_slots: Box<[LoopRegionSlot]>,
        profile: WorkflowIrProfile,
        concurrent_requirements: Option<ConcurrentRequirements>,
    ) -> Self {
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
            loop_slots,
            profile,
            concurrent_requirements,
        }
    }
}
