use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportFinding {
    pub path: DomainPath,
    pub location: ReportLocation,
    pub kind: LoopPatternKind,
    pub risk_reason: RiskReason,
    pub disposition: DispositionKind,
    pub owner: OwnerName,
    pub action: ReportAction,
    pub reason: Option<ExceptionReason>,
    pub scope: Option<ExceptionScope>,
    pub review_trigger: Option<ReportAction>,
    pub behavior_evidence: Option<BehaviorEvidence>,
    pub case_evidence: CaseEvidence,
}

impl ReportFinding {
    #[must_use]
    pub fn repair_required(summary: FindingSummary, repair: RepairMetadata) -> Self {
        Self {
            path: summary.path,
            location: summary.location,
            kind: summary.kind,
            risk_reason: summary.risk_reason,
            disposition: DispositionKind::RepairRequired,
            owner: repair.owner,
            action: repair.bead,
            reason: None,
            scope: None,
            review_trigger: None,
            behavior_evidence: None,
            case_evidence: CaseEvidence(Vec::new()),
        }
    }

    #[must_use]
    pub fn accepted_exception(path: &str, location: &str, exception: ExceptionMetadata) -> Self {
        Self {
            path: DomainPath::new(path),
            location: ReportLocation::new(location),
            kind: LoopPatternKind::TableLoop,
            risk_reason: RiskReason::AcceptedExceptionRequired,
            disposition: DispositionKind::AcceptedException,
            owner: exception.owner,
            action: exception.review_trigger.clone(),
            reason: Some(exception.reason),
            scope: Some(exception.scope),
            review_trigger: Some(exception.review_trigger),
            behavior_evidence: None,
            case_evidence: CaseEvidence(Vec::new()),
        }
    }

    #[must_use]
    pub fn safe_labeling(
        path: &str,
        location: &str,
        behavior_evidence: &str,
        case_evidence: Vec<String>,
    ) -> Self {
        Self {
            path: DomainPath::new(path),
            location: ReportLocation::new(location),
            kind: LoopPatternKind::SafeLabeledLoop,
            risk_reason: RiskReason::MissingCaseIdentity,
            disposition: DispositionKind::SafeLabelingProven,
            owner: OwnerName::new(""),
            action: ReportAction::new(""),
            reason: None,
            scope: None,
            review_trigger: None,
            behavior_evidence: Some(BehaviorEvidence::new(behavior_evidence)),
            case_evidence: CaseEvidence(case_evidence),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InventoryReport {
    pub risky_count: usize,
    pub findings: Vec<ReportFinding>,
    pub mutation_evidence: MutationEvidence,
    pub mutation_improvement_claim: Option<MutationImprovementClaim>,
}

impl InventoryReport {
    #[must_use]
    pub fn from_findings(
        risky_count: usize,
        findings: Vec<ReportFinding>,
        mutation_evidence: MutationEvidence,
        mutation_improvement_claim: Option<MutationImprovementClaim>,
    ) -> Self {
        Self {
            risky_count,
            findings,
            mutation_evidence,
            mutation_improvement_claim,
        }
    }
}
