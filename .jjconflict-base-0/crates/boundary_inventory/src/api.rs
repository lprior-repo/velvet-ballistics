use std::collections::HashSet;
use std::fs;
use std::path::Path;

use super::inventory::{BoundaryInventory, ValidatedBoundaryInventory};
use super::record::BoundaryRecord;
use super::status::{EvidenceRequirement, UnsafeIsolationStatus};
use super::types::{
    BoundaryCandidate, BoundaryClass, BoundaryExposure, BoundaryInventoryError, BoundaryRisk,
    ClassifiedBoundary, ClassifiedBoundaryInput, WorkspaceRoot,
};
use super::validation::validate_record;

pub fn discover_boundaries(
    workspace: WorkspaceRoot,
) -> Result<Vec<BoundaryCandidate>, BoundaryInventoryError> {
    if !workspace.path.exists() || required_surface_absent(&workspace) {
        return Err(BoundaryInventoryError::WorkspaceNotDiscoverable);
    }
    if decoder_surface_omitted(&workspace)? {
        return Err(BoundaryInventoryError::IncompleteDiscoveryInput);
    }
    let candidates = discover_marker_candidates(&workspace)?;
    if candidates.is_empty() {
        return Err(BoundaryInventoryError::IncompleteDiscoveryInput);
    }
    Ok(candidates)
}

pub fn classify_boundary(
    candidate: BoundaryCandidate,
) -> Result<ClassifiedBoundary, BoundaryInventoryError> {
    let class = class_from_marker(&candidate.marker)?;
    Ok(ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: stable_id(&class, &candidate.source_path),
        class,
        source_path: candidate.source_path,
        exposure: BoundaryExposure::risky(BoundaryRisk::Multiple),
    }))
}

pub fn required_evidence(
    boundary: ClassifiedBoundary,
) -> Result<EvidenceRequirement, BoundaryInventoryError> {
    if boundary.class == BoundaryClass::Unknown {
        return Err(BoundaryInventoryError::UnknownBoundaryClass);
    }
    if is_risky_boundary(&boundary) {
        return Ok(EvidenceRequirement::FuzzOrIsolationOrManualQa);
    }
    Err(BoundaryInventoryError::MissingEvidencePath)
}

pub fn validate_inventory(
    inventory: BoundaryInventory,
    workspace: WorkspaceRoot,
) -> Result<ValidatedBoundaryInventory, BoundaryInventoryError> {
    if inventory.schema_version != Some(1) {
        return Err(BoundaryInventoryError::SchemaVersionUnsupported);
    }
    validate_unique_ids(&inventory.records)?;
    for record in &inventory.records {
        validate_record(record, &workspace)?;
    }
    let review_status = first_review_status(&inventory.records).map(str::to_owned);
    Ok(ValidatedBoundaryInventory::from_validated_records(
        1,
        inventory.records,
        review_status,
    ))
}

pub fn inventory_completion_status(
    inventory: ValidatedBoundaryInventory,
) -> Result<UnsafeIsolationStatus, BoundaryInventoryError> {
    if inventory
        .records
        .iter()
        .any(|record| record.class == BoundaryClass::Unknown)
    {
        return Err(BoundaryInventoryError::UnknownBoundaryClass);
    }
    if inventory.records.iter().any(is_first_party_unsafe_record) {
        return Err(BoundaryInventoryError::UnsafeForbiddenViolation);
    }
    if inventory.records.is_empty() && inventory.discovered_boundary_count != 0 {
        return Err(BoundaryInventoryError::IncompleteDiscoveryInput);
    }
    Ok(UnsafeIsolationStatus::Complete {
        boundary_count: inventory.records.len(),
    })
}

fn is_risky_boundary(boundary: &ClassifiedBoundary) -> bool {
    boundary.exposure.risk != BoundaryRisk::None
        || matches!(
            boundary.class,
            BoundaryClass::GeneratedCode | BoundaryClass::UnsafeAdjacentDependency
        )
}

fn validate_unique_ids(records: &[BoundaryRecord]) -> Result<(), BoundaryInventoryError> {
    let mut seen = HashSet::new();
    for record in records {
        if !seen.insert(record.id.as_str()) {
            return Err(BoundaryInventoryError::DuplicateBoundaryId);
        }
    }
    Ok(())
}

fn is_first_party_unsafe_record(record: &BoundaryRecord) -> bool {
    record.class == BoundaryClass::UnsafeAdjacentDependency
        && record.source_path.starts_with("crates")
}

fn first_review_status(records: &[BoundaryRecord]) -> Option<&str> {
    records.iter().find_map(BoundaryRecord::review_status)
}

fn required_surface_absent(workspace: &WorkspaceRoot) -> bool {
    required_surfaces()
        .iter()
        .any(|entry| !workspace.path.join(entry).exists())
}

fn required_surfaces() -> [&'static str; 4] {
    ["crates", "fuzz", "scripts", "Cargo.toml"]
}

fn decoder_surface_omitted(workspace: &WorkspaceRoot) -> Result<bool, BoundaryInventoryError> {
    let config = workspace.path.join("boundary-surfaces.txt");
    if !config.exists() {
        return Ok(false);
    }
    let content = fs::read_to_string(config)
        .map_err(|_error| BoundaryInventoryError::WorkspaceNotDiscoverable)?;
    Ok(!content
        .lines()
        .any(|line| line.trim() == "decoder-byte-ingest-boundary"))
}

fn discover_marker_candidates(
    workspace: &WorkspaceRoot,
) -> Result<Vec<BoundaryCandidate>, BoundaryInventoryError> {
    let mut candidates = Vec::new();
    for entry in candidate_roots() {
        collect_markers(
            &workspace.path.join(entry),
            Path::new(entry),
            &mut candidates,
        )?;
    }
    Ok(candidates)
}

fn candidate_roots() -> [&'static str; 4] {
    ["crates", "fuzz", "scripts", "Cargo.toml"]
}

fn collect_markers(
    absolute: &Path,
    relative: &Path,
    candidates: &mut Vec<BoundaryCandidate>,
) -> Result<(), BoundaryInventoryError> {
    if absolute.is_dir() {
        return collect_directory_markers(absolute, relative, candidates);
    }
    collect_file_markers(absolute, relative, candidates)
}

fn collect_directory_markers(
    absolute: &Path,
    relative: &Path,
    candidates: &mut Vec<BoundaryCandidate>,
) -> Result<(), BoundaryInventoryError> {
    let entries = fs::read_dir(absolute)
        .map_err(|_error| BoundaryInventoryError::WorkspaceNotDiscoverable)?;
    let mut paths = Vec::new();
    for entry in entries {
        paths.push(entry.map_err(|_error| BoundaryInventoryError::WorkspaceNotDiscoverable)?);
    }
    paths.sort_by_key(std::fs::DirEntry::path);
    for entry in paths {
        collect_markers(&entry.path(), &relative.join(entry.file_name()), candidates)?;
    }
    Ok(())
}

fn collect_file_markers(
    absolute: &Path,
    relative: &Path,
    candidates: &mut Vec<BoundaryCandidate>,
) -> Result<(), BoundaryInventoryError> {
    let content = fs::read_to_string(absolute)
        .map_err(|_error| BoundaryInventoryError::WorkspaceNotDiscoverable)?;
    for marker in marker_set() {
        if content.lines().any(|line| line.contains(marker)) {
            candidates.push(BoundaryCandidate::new(relative.to_path_buf(), marker));
        }
    }
    Ok(())
}

fn marker_set() -> [&'static str; 7] {
    [
        "extern-c-boundary",
        "foreign-function-boundary",
        "ipc-frame-boundary",
        "external-binary-boundary",
        "decoder-byte-ingest-boundary",
        "generated-interface-boundary",
        "unsafe-adjacent-dependency-boundary",
    ]
}

fn class_from_marker(marker: &str) -> Result<BoundaryClass, BoundaryInventoryError> {
    match marker {
        "extern-c-boundary" => Ok(BoundaryClass::CAbi),
        "foreign-function-boundary" => Ok(BoundaryClass::Ffi),
        "ipc-frame-boundary" => Ok(BoundaryClass::Ipc),
        "external-binary-boundary" => Ok(BoundaryClass::ExternalBinary),
        "decoder-byte-ingest-boundary" => Ok(BoundaryClass::Decoder),
        "generated-interface-boundary" => Ok(BoundaryClass::GeneratedCode),
        "unsafe-adjacent-dependency-boundary" => Ok(BoundaryClass::UnsafeAdjacentDependency),
        _unknown => Err(BoundaryInventoryError::UnknownBoundaryClass),
    }
}

fn stable_id(class: &BoundaryClass, source_path: &Path) -> String {
    let source = source_path.to_string_lossy();
    format!("vb-y1zq-{class:?}-{source}").replace(['/', '.', '_'], "-")
}
