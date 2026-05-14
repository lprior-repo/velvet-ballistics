use std::path::Path;
pub(crate) use std::path::PathBuf;

pub(crate) use crate::boundary_inventory::{
    BoundaryCandidate, BoundaryClass, BoundaryExposure, BoundaryInventory, BoundaryInventoryError,
    BoundaryRecord, BoundaryRecordParts, BoundaryRisk, ClassifiedBoundary, ClassifiedBoundaryInput,
    EvidenceKind, EvidenceReference, EvidenceRequirement, FieldState, FreshnessMarker, Owner,
    ReviewStatus, ThreatStatement, UnsafeIsolationStatus, ValidatedBoundaryInventory,
    WorkspaceRoot, classify_boundary, discover_boundaries, inventory_completion_status,
    parse_inventory, required_evidence, validate_evidence_reference_bytes, validate_inventory,
};

pub(crate) fn workspace(name: &str) -> WorkspaceRoot {
    WorkspaceRoot::new(PathBuf::from("tests/fixtures/vb_y1zq").join(name))
}

pub(crate) fn candidate(source_path: &str, marker: &str) -> BoundaryCandidate {
    BoundaryCandidate::new(source_path, marker)
}

pub(crate) fn classified(class: BoundaryClass, source_path: &str) -> ClassifiedBoundary {
    ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: stable_id(&class, source_path),
        class,
        source_path: PathBuf::from(source_path),
        exposure: BoundaryExposure::risky(BoundaryRisk::Multiple),
    })
}

pub(crate) fn stable_id(class: &BoundaryClass, source_path: &str) -> String {
    format!("vb-y1zq-{class:?}-{source_path}").replace(['/', '.', '_'], "-")
}

pub(crate) fn evidence(path: &str) -> EvidenceReference {
    EvidenceReference::repo_local(PathBuf::from(path), EvidenceKind::Fuzz)
}

pub(crate) fn valid_record(class: BoundaryClass, source_path: &str) -> BoundaryRecord {
    BoundaryRecord::new(BoundaryRecordParts {
        id: stable_id(&class, source_path),
        class,
        source_path: PathBuf::from(source_path),
        owner: FieldState::Present(Owner(String::from("boundary-owner"))),
        threat: FieldState::Present(ThreatStatement(String::from(
            "hostile external bytes cross a trust boundary",
        ))),
        evidence: FieldState::Present(evidence("formal-verification-report.md")),
        freshness: FreshnessMarker::new(10, 10, 10),
        review_status: FieldState::Present(ReviewStatus::Approved),
        waiver: FieldState::Missing,
    })
}

pub(crate) fn inventory_with(record: BoundaryRecord) -> BoundaryInventory {
    BoundaryInventory::new(
        Some(1),
        vec![record],
        Some(evidence("proof-obligations.jsonl")),
    )
}

pub(crate) fn validated_empty_with_status(status: &str) -> ValidatedBoundaryInventory {
    ValidatedBoundaryInventory {
        schema_version: 1,
        records: Vec::new(),
        discovered_boundary_count: 0,
        review_status: Some(status.to_string()),
    }
}

pub(crate) fn validated_empty_with_count(
    discovered_boundary_count: usize,
) -> ValidatedBoundaryInventory {
    ValidatedBoundaryInventory {
        schema_version: 1,
        records: Vec::new(),
        discovered_boundary_count,
        review_status: None,
    }
}

pub(crate) fn validated_with_schema_status_and_count(
    schema_version: u32,
    status: Option<&str>,
    discovered_boundary_count: usize,
) -> ValidatedBoundaryInventory {
    ValidatedBoundaryInventory {
        schema_version,
        records: Vec::new(),
        discovered_boundary_count,
        review_status: status.map(str::to_string),
    }
}

pub(crate) fn validated_with_records(records: Vec<BoundaryRecord>) -> ValidatedBoundaryInventory {
    ValidatedBoundaryInventory {
        schema_version: 1,
        discovered_boundary_count: records.len(),
        records,
        review_status: None,
    }
}

pub(crate) fn validated_with_records_and_status(
    records: Vec<BoundaryRecord>,
    status: &str,
) -> ValidatedBoundaryInventory {
    ValidatedBoundaryInventory {
        schema_version: 1,
        discovered_boundary_count: records.len(),
        records,
        review_status: Some(status.to_string()),
    }
}

pub(crate) fn record_classes(inventory: &BoundaryInventory) -> Vec<BoundaryClass> {
    inventory
        .records
        .iter()
        .map(|record| record.class)
        .collect()
}

pub(crate) fn temp_workspace_missing_required_surfaces() -> Result<tempfile::TempDir, std::io::Error>
{
    tempfile::tempdir()
}

pub(crate) fn create_workspace_with_surfaces() -> Result<tempfile::TempDir, std::io::Error> {
    let dir = tempfile::tempdir()?;
    std::fs::create_dir(dir.path().join("crates"))?;
    std::fs::create_dir(dir.path().join("fuzz"))?;
    std::fs::create_dir(dir.path().join("scripts"))?;
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "# unsafe-adjacent-dependency-boundary\n",
    )?;
    std::fs::write(
        dir.path().join("boundary-surfaces.txt"),
        "decoder-byte-ingest-boundary\n",
    )?;
    Ok(dir)
}

pub(crate) fn write_file(root: &Path, rel: &str, content: &str) -> Result<(), std::io::Error> {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)
}

pub(crate) fn candidate_pairs(candidates: Vec<BoundaryCandidate>) -> Vec<(String, String)> {
    candidates
        .into_iter()
        .map(|candidate| {
            (
                candidate.source_path.display().to_string(),
                candidate.marker,
            )
        })
        .collect()
}

pub(crate) fn discover_temp_with_files(
    files: &[(&str, &str)],
) -> Result<Vec<(String, String)>, String> {
    let dir = create_workspace_with_surfaces().map_err(|error| error.to_string())?;
    for (path, content) in files {
        write_file(dir.path(), path, content).map_err(|error| error.to_string())?;
    }
    discover_boundaries(WorkspaceRoot::new(dir.path().to_path_buf()))
        .map(candidate_pairs)
        .map_err(|error| format!("{error:?}"))
}

pub(crate) fn discover_workspace_with_missing_surfaces_and_omitted_decoder_config()
-> Result<Result<Vec<BoundaryCandidate>, BoundaryInventoryError>, String> {
    let dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    std::fs::write(
        dir.path().join("boundary-surfaces.txt"),
        "ipc-frame-boundary\n",
    )
    .map_err(|error| error.to_string())?;
    Ok(discover_boundaries(WorkspaceRoot::new(
        dir.path().to_path_buf(),
    )))
}
