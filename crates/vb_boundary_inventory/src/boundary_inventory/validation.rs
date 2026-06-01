use std::path::{Path, PathBuf};

use super::record::{BoundaryRecord, FieldState};
use super::types::{
    BoundaryClass, BoundaryInventoryError, EvidenceKind, EvidenceReference, FreshnessMarker,
    ReviewStatus, WorkspaceRoot,
};

pub fn validate_evidence_reference_bytes(
    bytes: &[u8],
) -> Result<EvidenceReference, BoundaryInventoryError> {
    let text =
        std::str::from_utf8(bytes).map_err(|_error| BoundaryInventoryError::InvalidEvidencePath)?;
    evidence_reference_from_text(text)
}

fn evidence_reference_from_text(text: &str) -> Result<EvidenceReference, BoundaryInventoryError> {
    if text.starts_with("external:") && text.contains("#sha256=") {
        return Ok(EvidenceReference::ExternalProvenance(text.to_owned()));
    }
    if text.starts_with("external:") {
        return Err(BoundaryInventoryError::InvalidEvidencePath);
    }
    if valid_bead_id(text) {
        return Ok(EvidenceReference::ExternalProvenance(text.to_owned()));
    }
    repo_local_evidence_from_text(text)
}

fn repo_local_evidence_from_text(text: &str) -> Result<EvidenceReference, BoundaryInventoryError> {
    let path = PathBuf::from(text);
    validate_repo_path_shape(&path)?;
    validate_manifest_relative_path_exists(&path)?;
    Ok(EvidenceReference::repo_local(
        path,
        EvidenceKind::Provenance,
    ))
}

pub(crate) fn validate_record(
    record: &BoundaryRecord,
    workspace: &WorkspaceRoot,
) -> Result<(), BoundaryInventoryError> {
    if record.class == BoundaryClass::Unknown {
        return Err(BoundaryInventoryError::UnknownBoundaryClass);
    }
    validate_owner_and_threat(record)?;
    validate_source_path(&record.source_path)?;
    validate_freshness(record.freshness)?;
    let evidence = required_evidence(record)?;
    validate_evidence_reference(evidence, workspace)?;
    validate_review_status(
        record.review_status.as_ref(),
        record.waiver.as_ref(),
        workspace,
    )
}

fn validate_owner_and_threat(record: &BoundaryRecord) -> Result<(), BoundaryInventoryError> {
    if field_text_is_missing(record.owner.as_ref().map(|owner| owner.0.as_str())) {
        return Err(BoundaryInventoryError::MissingOwner);
    }
    if field_text_is_missing(record.threat.as_ref().map(|threat| threat.0.as_str())) {
        return Err(BoundaryInventoryError::MissingThreat);
    }
    Ok(())
}

fn required_evidence(
    record: &BoundaryRecord,
) -> Result<&EvidenceReference, BoundaryInventoryError> {
    match record.evidence.as_ref() {
        FieldState::Present(value) => Ok(value),
        FieldState::Missing => Err(BoundaryInventoryError::MissingEvidencePath),
    }
}

fn field_text_is_missing(field: FieldState<&str>) -> bool {
    match field {
        FieldState::Present(value) => value.is_empty(),
        FieldState::Missing => true,
    }
}

fn validate_source_path(path: &Path) -> Result<(), BoundaryInventoryError> {
    if path.as_os_str().is_empty() {
        return Err(BoundaryInventoryError::InventoryParseFailure);
    }
    let allowed = ["crates", "scripts", "fuzz", "Cargo.toml"];
    if allowed.iter().any(|prefix| path.starts_with(prefix)) {
        return Ok(());
    }
    Err(BoundaryInventoryError::WorkspaceNotDiscoverable)
}

fn validate_freshness(freshness: FreshnessMarker) -> Result<(), BoundaryInventoryError> {
    if freshness.evidence_version < freshness.source_version
        || freshness.evidence_version < freshness.schema_version
    {
        return Err(BoundaryInventoryError::StaleEvidence);
    }
    Ok(())
}

fn validate_review_status(
    status: FieldState<&ReviewStatus>,
    waiver: FieldState<&EvidenceReference>,
    workspace: &WorkspaceRoot,
) -> Result<(), BoundaryInventoryError> {
    match status {
        FieldState::Present(ReviewStatus::Approved) => Ok(()),
        FieldState::Present(ReviewStatus::Waived) => match waiver {
            FieldState::Present(reference) => validate_evidence_reference(reference, workspace),
            FieldState::Missing => Err(BoundaryInventoryError::ReviewStatusInvalid),
        },
        FieldState::Present(ReviewStatus::Other(_)) | FieldState::Missing => {
            Err(BoundaryInventoryError::ReviewStatusInvalid)
        }
    }
}

fn validate_evidence_reference(
    evidence: &EvidenceReference,
    workspace: &WorkspaceRoot,
) -> Result<(), BoundaryInventoryError> {
    match evidence {
        EvidenceReference::RepoLocal { path, kind: _ } => validate_repo_path(path, workspace),
        EvidenceReference::FreeText(_) => Err(BoundaryInventoryError::InvalidEvidencePath),
        EvidenceReference::ExternalProvenance(value) => validate_external_reference(value),
    }
}

fn validate_external_reference(value: &str) -> Result<(), BoundaryInventoryError> {
    if (value.starts_with("external:") && value.contains("#sha256=")) || valid_bead_id(value) {
        Ok(())
    } else {
        Err(BoundaryInventoryError::InvalidEvidencePath)
    }
}

fn validate_repo_path(
    path: &Path,
    workspace: &WorkspaceRoot,
) -> Result<(), BoundaryInventoryError> {
    validate_repo_path_shape(path)?;
    if workspace.path.join(path).exists() {
        return Ok(());
    }
    Err(BoundaryInventoryError::InvalidEvidencePath)
}

fn validate_manifest_relative_path_exists(path: &Path) -> Result<(), BoundaryInventoryError> {
    if PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(path)
        .exists()
    {
        return Ok(());
    }
    Err(BoundaryInventoryError::InvalidEvidencePath)
}

fn validate_repo_path_shape(path: &Path) -> Result<(), BoundaryInventoryError> {
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(BoundaryInventoryError::InvalidEvidencePath);
    }
    Ok(())
}

fn valid_bead_id(value: &str) -> bool {
    let mut parts = value.split('-');
    matches!(parts.next(), Some("vb"))
        && matches!(parts.next(), Some(part) if valid_bead_suffix(part))
        && parts.next().is_none()
}

fn valid_bead_suffix(part: &str) -> bool {
    !part.is_empty()
        && part
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
}
