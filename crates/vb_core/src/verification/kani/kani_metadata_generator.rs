#![cfg(kani)]

use crate::ids::SymbolId;
use crate::workflow::source_metadata::{
    CompiledSecretRequirement, SourceMetadataError, WorkflowSourceMetadata,
};

#[derive(Clone, Copy, kani::Arbitrary)]
enum SymbolWidth {
    Empty,
    One,
    Two,
}

#[derive(Clone, Copy, kani::Arbitrary)]
struct MetadataSeed {
    present: bool,
    width: SymbolWidth,
    wants_requirement: bool,
    alias_second: bool,
    requirement_second: bool,
}

fn make_symbols(width: SymbolWidth) -> Box<[Box<str>]> {
    match width {
        SymbolWidth::Empty => Box::new([]),
        SymbolWidth::One => Box::new([super::super::kani_step_name(0)]),
        SymbolWidth::Two => Box::new([
            super::super::kani_step_name(0),
            super::super::kani_step_name(1),
        ]),
    }
}

fn reference_id(width: SymbolWidth, second: bool) -> SymbolId {
    SymbolId::new(if second && matches!(width, SymbolWidth::Two) {
        1
    } else {
        0
    })
}

fn make_requirements(seed: MetadataSeed) -> Box<[CompiledSecretRequirement]> {
    if matches!(seed.width, SymbolWidth::Empty) || !seed.wants_requirement {
        Box::new([])
    } else {
        let requirement = CompiledSecretRequirement::new(
            reference_id(seed.width, seed.alias_second),
            reference_id(seed.width, seed.requirement_second),
        );
        Box::new([requirement])
    }
}

fn try_metadata(
    seed: MetadataSeed,
) -> Result<Option<Box<WorkflowSourceMetadata>>, SourceMetadataError> {
    if !seed.present {
        return Ok(None);
    }
    let symbols = make_symbols(seed.width);
    let requirements = make_requirements(seed);
    WorkflowSourceMetadata::try_new(symbols, requirements).map(|metadata| Some(Box::new(metadata)))
}

fn checked_outcome(
    result: Result<Option<Box<WorkflowSourceMetadata>>, SourceMetadataError>,
) -> Option<Box<WorkflowSourceMetadata>> {
    kani::assert(result.is_ok(), "bounded metadata constructor must succeed");
    match result {
        Ok(metadata) => metadata,
        Err(_) => std::process::abort(),
    }
}

fn from_seed(seed: MetadataSeed) -> Option<Box<WorkflowSourceMetadata>> {
    checked_outcome(try_metadata(seed))
}

pub(super) fn metadata() -> Option<Box<WorkflowSourceMetadata>> {
    from_seed(kani::any::<MetadataSeed>())
}

#[path = "kani_metadata_generator_checks.rs"]
mod checks;
