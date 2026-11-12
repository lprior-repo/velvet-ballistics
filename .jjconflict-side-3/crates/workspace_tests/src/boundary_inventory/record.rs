use std::path::PathBuf;

use super::types::{BoundaryClass, EvidenceReference, FreshnessMarker, ReviewStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FieldState<T> {
    Present(T),
    Missing,
}

impl<T> From<Option<T>> for FieldState<T> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(inner) => Self::Present(inner),
            None => Self::Missing,
        }
    }
}

impl<T> FieldState<T> {
    pub(crate) fn as_ref(&self) -> FieldState<&T> {
        match self {
            Self::Present(value) => FieldState::Present(value),
            Self::Missing => FieldState::Missing,
        }
    }

    pub(crate) fn map<U>(self, convert: impl FnOnce(T) -> U) -> FieldState<U> {
        match self {
            Self::Present(value) => FieldState::Present(convert(value)),
            Self::Missing => FieldState::Missing,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Owner(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreatStatement(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReviewDecision {
    Approved,
    Waived { waiver: EvidenceReference },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryRecordParts {
    pub id: String,
    pub class: BoundaryClass,
    pub source_path: PathBuf,
    pub owner: FieldState<Owner>,
    pub threat: FieldState<ThreatStatement>,
    pub evidence: FieldState<EvidenceReference>,
    pub freshness: FreshnessMarker,
    pub review_status: FieldState<ReviewStatus>,
    pub waiver: FieldState<EvidenceReference>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryRecordDraft {
    pub id: String,
    pub class: BoundaryClass,
    pub source_path: PathBuf,
    pub owner: FieldState<Owner>,
    pub threat: FieldState<ThreatStatement>,
    pub evidence: FieldState<EvidenceReference>,
    pub freshness: FreshnessMarker,
    pub review_status: FieldState<ReviewStatus>,
    pub waiver: FieldState<EvidenceReference>,
}

pub type BoundaryRecord = BoundaryRecordDraft;

impl BoundaryRecordDraft {
    #[must_use]
    pub fn new(parts: BoundaryRecordParts) -> Self {
        Self {
            id: parts.id,
            class: parts.class,
            source_path: parts.source_path,
            owner: parts.owner,
            threat: parts.threat,
            evidence: parts.evidence,
            freshness: parts.freshness,
            review_status: parts.review_status,
            waiver: parts.waiver,
        }
    }

    #[must_use]
    pub fn review_status(&self) -> Option<&str> {
        match &self.review_status {
            FieldState::Present(status) => Some(status.serialized()),
            FieldState::Missing => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteBoundaryRecord {
    pub id: String,
    pub class: BoundaryClass,
    pub source_path: PathBuf,
    pub owner: Owner,
    pub threat: ThreatStatement,
    pub evidence: EvidenceReference,
    pub freshness: FreshnessMarker,
    pub review: ReviewDecision,
}

pub type ValidatedBoundaryRecord = CompleteBoundaryRecord;
