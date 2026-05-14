use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepairMetadata {
    pub owner: OwnerName,
    pub bead: ReportAction,
}

impl RepairMetadata {
    #[must_use]
    pub fn new(owner: &str, bead: &str) -> Self {
        Self {
            owner: OwnerName::new(owner),
            bead: ReportAction::new(bead),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExceptionMetadata {
    pub reason: ExceptionReason,
    pub scope: ExceptionScope,
    pub owner: OwnerName,
    pub review_trigger: ReportAction,
}

impl ExceptionMetadata {
    pub fn new(
        reason: &str,
        scope: &str,
        owner: &str,
        review_trigger: &str,
    ) -> Result<Self, InventoryError> {
        validate_exception_parts(reason, scope, owner, review_trigger)?;
        Ok(Self {
            reason: ExceptionReason::new(reason),
            scope: ExceptionScope::new(scope),
            owner: OwnerName::new(owner),
            review_trigger: ReportAction::new(review_trigger),
        })
    }
}

fn validate_exception_parts(
    reason: &str,
    scope: &str,
    owner: &str,
    review_trigger: &str,
) -> Result<(), InventoryError> {
    if let Some(field) = first_empty_exception_part(reason, scope, owner, review_trigger) {
        Err(InventoryError::PolicyViolation {
            rule: "accepted_exception_metadata_complete".to_owned(),
            field: field.to_owned(),
        })
    } else {
        Ok(())
    }
}

fn first_empty_exception_part(
    reason: &str,
    scope: &str,
    owner: &str,
    review_trigger: &str,
) -> Option<&'static str> {
    if reason.is_empty() {
        Some("reason")
    } else if scope.is_empty() {
        Some("scope")
    } else if owner.is_empty() {
        Some("owner")
    } else if review_trigger.is_empty() {
        Some("review_trigger")
    } else {
        None
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssignmentEvidence {
    RepairEvidence(RepairMetadata),
    ExceptionEvidence(ExceptionMetadata),
    SafeLabelEvidence {
        behavior: BehaviorEvidence,
        cases: CaseEvidence,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Disposition {
    RepairRequired(RepairMetadata),
    AcceptedException(ExceptionMetadata),
    SafeLabelingProven {
        behavior_evidence: BehaviorEvidence,
        case_evidence: CaseEvidence,
    },
}

impl Disposition {
    #[must_use]
    pub(crate) fn kind(&self) -> DispositionKind {
        match self {
            Self::RepairRequired(_repair) => DispositionKind::RepairRequired,
            Self::AcceptedException(_exception) => DispositionKind::AcceptedException,
            Self::SafeLabelingProven { .. } => DispositionKind::SafeLabelingProven,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FindingRisk {
    Risky(RiskReason),
    NonRisky,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    pub id: FindingId,
    pub risk: FindingRisk,
    pub dispositions: Vec<Disposition>,
}

impl Finding {
    #[must_use]
    pub fn risky(id: &str, reason: RiskReason, disposition: Option<Disposition>) -> Self {
        let dispositions = match disposition {
            Some(value) => vec![value],
            None => Vec::new(),
        };
        Self {
            id: FindingId::new(id),
            risk: FindingRisk::Risky(reason),
            dispositions,
        }
    }

    #[must_use]
    pub fn risky_with_dispositions(
        id: &str,
        reason: RiskReason,
        dispositions: Vec<Disposition>,
    ) -> Self {
        Self {
            id: FindingId::new(id),
            risk: FindingRisk::Risky(reason),
            dispositions,
        }
    }

    #[must_use]
    pub fn non_risky(id: &str) -> Self {
        Self {
            id: FindingId::new(id),
            risk: FindingRisk::NonRisky,
            dispositions: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Inventory {
    pub findings: Vec<Finding>,
    pub(crate) baseline_finding_ids: Vec<FindingId>,
    pub(crate) current_finding_ids: Vec<FindingId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispositionSelection {
    Missing,
    Single(DispositionKind),
    Conflicting,
}

impl Inventory {
    #[must_use]
    pub fn from_findings(findings: Vec<Finding>) -> Self {
        Self {
            findings,
            baseline_finding_ids: Vec::new(),
            current_finding_ids: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_baseline_and_current(
        baseline_finding_ids: Vec<FindingId>,
        current_finding_ids: Vec<FindingId>,
    ) -> Self {
        Self {
            findings: Vec::new(),
            baseline_finding_ids,
            current_finding_ids,
        }
    }

    #[must_use]
    pub fn with_non_risky_count_and_one_unassigned_risky(
        extra_safe_count: usize,
        id: &str,
    ) -> Self {
        let mut findings = Vec::with_capacity(extra_safe_count.saturating_add(1));
        for n in 0..extra_safe_count {
            findings.push(Finding::non_risky(&format!(
                "tests/safe_{n}.rs:1:1:SafeLabeledLoop"
            )));
        }
        findings.push(Finding::risky(id, RiskReason::MissingCaseIdentity, None));
        Self::from_findings(findings)
    }

    pub fn symbolic_disposition_validate(
        selection: DispositionSelection,
    ) -> Result<(), InventoryError> {
        match selection {
            DispositionSelection::Single(_kind) => Ok(()),
            DispositionSelection::Missing | DispositionSelection::Conflicting => {
                Err(InventoryError::ConflictingDisposition {
                    finding_id: String::new(),
                    dispositions: Vec::new(),
                })
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SafeLabelInput {
    Complete {
        behavior: BehaviorEvidence,
        cases: CaseEvidence,
    },
    MissingBehavior {
        case_count: usize,
    },
    MissingCase {
        behavior: BehaviorEvidence,
    },
}

impl AssignmentEvidence {
    pub fn symbolic_safe_label_validate(input: SafeLabelInput) -> Result<(), InventoryError> {
        match input {
            SafeLabelInput::Complete { .. } => Ok(()),
            SafeLabelInput::MissingBehavior { case_count } => {
                ambiguous_label(String::new(), None, case_count)
            }
            SafeLabelInput::MissingCase { behavior } => {
                ambiguous_label(behavior.0.clone(), Some(behavior.0), 0)
            }
        }
    }
}

fn ambiguous_label(
    label: String,
    behavior: Option<String>,
    case_count: usize,
) -> Result<(), InventoryError> {
    Err(InventoryError::AmbiguousCaseLabel {
        label,
        behavior,
        case_count,
    })
}
