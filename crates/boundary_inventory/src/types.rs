use std::path::PathBuf;

pub type DiscoveryEvidence = EvidenceReference;
pub type OptionalDiscoveryEvidence = Option<DiscoveryEvidence>;
pub type ReviewSummary = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BoundaryInventoryError {
    WorkspaceNotDiscoverable,
    IncompleteDiscoveryInput,
    UnknownBoundaryClass,
    UnsafeForbiddenViolation,
    MissingOwner,
    MissingThreat,
    MissingEvidencePath,
    InvalidEvidencePath,
    StaleEvidence,
    DuplicateBoundaryId,
    InventoryParseFailure,
    SchemaVersionUnsupported,
    ReviewStatusInvalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BoundaryClass {
    CAbi,
    Ffi,
    Ipc,
    ExternalBinary,
    Decoder,
    GeneratedCode,
    UnsafeAdjacentDependency,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRoot {
    pub(crate) path: PathBuf,
}

impl WorkspaceRoot {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryCandidate {
    pub source_path: PathBuf,
    pub marker: String,
}

impl BoundaryCandidate {
    #[must_use]
    pub fn new(source_path: impl Into<PathBuf>, marker: impl Into<String>) -> Self {
        Self {
            source_path: source_path.into(),
            marker: marker.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassifiedBoundary {
    pub id: String,
    pub class: BoundaryClass,
    pub source_path: PathBuf,
    pub exposure: BoundaryExposure,
}

impl ClassifiedBoundary {
    #[must_use]
    pub fn new(input: ClassifiedBoundaryInput) -> Self {
        Self {
            id: input.id,
            class: input.class,
            source_path: input.source_path,
            exposure: input.exposure,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassifiedBoundaryInput {
    pub id: String,
    pub class: BoundaryClass,
    pub source_path: PathBuf,
    pub exposure: BoundaryExposure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundaryExposure {
    pub risk: BoundaryRisk,
}

impl BoundaryExposure {
    #[must_use]
    pub const fn none() -> Self {
        Self {
            risk: BoundaryRisk::None,
        }
    }

    #[must_use]
    pub const fn risky(risk: BoundaryRisk) -> Self {
        Self { risk }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BoundaryRisk {
    None,
    ExternalBytes,
    ProcessLimit,
    LanguageLimit,
    Multiple,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EvidenceKind {
    Fuzz,
    Isolation,
    ManualQa,
    Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EvidenceReference {
    RepoLocal { path: PathBuf, kind: EvidenceKind },
    FreeText(String),
    ExternalProvenance(String),
}

impl EvidenceReference {
    #[must_use]
    pub fn repo_local(path: PathBuf, kind: EvidenceKind) -> Self {
        Self::RepoLocal { path, kind }
    }

    #[must_use]
    pub fn free_text(text: impl Into<String>) -> Self {
        Self::FreeText(text.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshnessMarker {
    pub(crate) source_version: u64,
    pub(crate) schema_version: u64,
    pub(crate) evidence_version: u64,
}

impl FreshnessMarker {
    #[must_use]
    pub fn new(source_version: u64, schema_version: u64, evidence_version: u64) -> Self {
        Self {
            source_version,
            schema_version,
            evidence_version,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReviewStatus {
    Approved,
    Waived,
    Other(String),
}

impl ReviewStatus {
    #[must_use]
    pub fn from_serialized(value: impl Into<String>) -> Self {
        match value.into().as_str() {
            "approved" => Self::Approved,
            "waived" => Self::Waived,
            other => Self::Other(other.to_owned()),
        }
    }

    #[must_use]
    pub fn serialized(&self) -> &str {
        match self {
            Self::Approved => "approved",
            Self::Waived => "waived",
            Self::Other(value) => value.as_str(),
        }
    }
}
