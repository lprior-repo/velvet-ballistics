use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MutationEvidence {
    NotProvided,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedFinding {
    pub path: DomainPath,
    pub location: ReportLocation,
    pub kind: LoopPatternKind,
    pub risk_reason: RiskReason,
    pub disposition: Disposition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FindingSummary {
    pub path: DomainPath,
    pub location: ReportLocation,
    pub kind: LoopPatternKind,
    pub risk_reason: RiskReason,
}

impl FindingSummary {
    #[must_use]
    pub fn new(path: &str, location: &str, kind: LoopPatternKind, risk_reason: RiskReason) -> Self {
        Self {
            path: DomainPath::new(path),
            location: ReportLocation::new(location),
            kind,
            risk_reason,
        }
    }
}

impl ValidatedFinding {
    #[must_use]
    pub fn repair_required(summary: FindingSummary, repair: RepairMetadata) -> Self {
        Self {
            path: summary.path,
            location: summary.location,
            kind: summary.kind,
            risk_reason: summary.risk_reason,
            disposition: Disposition::RepairRequired(repair),
        }
    }

    #[must_use]
    pub fn accepted_exception(path: &str, location: &str, exception: ExceptionMetadata) -> Self {
        Self {
            path: DomainPath::new(path),
            location: ReportLocation::new(location),
            kind: LoopPatternKind::TableLoop,
            risk_reason: RiskReason::AcceptedExceptionRequired,
            disposition: Disposition::AcceptedException(exception),
        }
    }

    pub fn safe_labeling(
        path: &str,
        location: &str,
        behavior_evidence: &str,
        case_evidence: Vec<String>,
    ) -> Result<Self, InventoryError> {
        Ok(Self {
            path: DomainPath::new(path),
            location: ReportLocation::new(location),
            kind: LoopPatternKind::SafeLabeledLoop,
            risk_reason: RiskReason::MissingCaseIdentity,
            disposition: Disposition::SafeLabelingProven {
                behavior_evidence: BehaviorEvidence::new(behavior_evidence),
                case_evidence: CaseEvidence::new(case_evidence)?,
            },
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedInventory {
    pub risky_count: usize,
    pub repair_required_count: usize,
    pub accepted_exception_count: usize,
    pub safe_labeling_count: usize,
    pub finding_ids: Vec<FindingId>,
    pub findings: Vec<ValidatedFinding>,
    pub mutation_evidence: MutationEvidence,
    pub mutation_improvement_claim: Option<MutationImprovementClaim>,
    pub(crate) policy_violation: Option<(String, String)>,
}

impl ValidatedInventory {
    #[must_use]
    pub fn summary(
        risky_count: usize,
        repair_required_count: usize,
        accepted_exception_count: usize,
        safe_labeling_count: usize,
        finding_ids: Vec<FindingId>,
    ) -> Self {
        Self {
            risky_count,
            repair_required_count,
            accepted_exception_count,
            safe_labeling_count,
            finding_ids,
            findings: Vec::new(),
            mutation_evidence: MutationEvidence::NotProvided,
            mutation_improvement_claim: None,
            policy_violation: None,
        }
    }

    pub fn with_findings(
        findings: Vec<ValidatedFinding>,
        mutation_evidence: MutationEvidence,
        mutation_improvement_claim: Option<String>,
    ) -> Result<Self, InventoryError> {
        for finding in &findings {
            validate_validated_finding(finding)?;
        }
        Ok(Self {
            risky_count: 0,
            repair_required_count: 0,
            accepted_exception_count: 0,
            safe_labeling_count: 0,
            finding_ids: Vec::new(),
            findings,
            mutation_evidence,
            mutation_improvement_claim: mutation_improvement_claim.map(MutationImprovementClaim),
            policy_violation: None,
        })
    }

    #[must_use]
    pub fn with_policy_violation(rule: &str, field: &str) -> Self {
        Self {
            risky_count: 0,
            repair_required_count: 0,
            accepted_exception_count: 0,
            safe_labeling_count: 0,
            finding_ids: Vec::new(),
            findings: Vec::new(),
            mutation_evidence: MutationEvidence::NotProvided,
            mutation_improvement_claim: None,
            policy_violation: Some((rule.to_owned(), field.to_owned())),
        }
    }
}
