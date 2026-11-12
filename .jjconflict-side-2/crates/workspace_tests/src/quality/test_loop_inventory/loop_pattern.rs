use super::{
    BehaviorEvidence, CaseEvidence, CaseLabel, DomainPath, ExceptionReason, ExceptionScope,
    FindingId, Location,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoopPatternKind {
    TableLoop,
    IteratorTableLoop,
    NestedOuterLoop,
    NestedInnerLoop,
    HelperDrivenTableLoop,
    TraceableMacroLoop,
    SafeLabeledLoop,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LabelEvidence {
    Absent,
    DuplicateCaseLabel {
        label: CaseLabel,
        behavior: Option<BehaviorEvidence>,
        case_count: usize,
    },
    BehaviorOnly {
        behavior: BehaviorEvidence,
    },
    CaseOnly {
        case: CaseLabel,
    },
    BehaviorAndCases {
        behavior: BehaviorEvidence,
        cases: CaseEvidence,
    },
    AcceptedExceptionEvidence {
        reason: ExceptionReason,
        scope: ExceptionScope,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopPattern {
    pub path: DomainPath,
    pub location: Location,
    pub kind: LoopPatternKind,
    pub assertion_count: u32,
    pub label_evidence: LabelEvidence,
}

impl LoopPattern {
    #[must_use]
    pub fn new(
        path: &str,
        location: Location,
        kind: LoopPatternKind,
        assertion_count: u32,
        label_evidence: LabelEvidence,
    ) -> Self {
        Self {
            path: DomainPath::new(path),
            location,
            kind,
            assertion_count,
            label_evidence,
        }
    }

    #[must_use]
    pub(crate) fn finding_id(&self) -> FindingId {
        FindingId(format!(
            "{}:{}:{}:{:?}",
            self.path.as_str(),
            self.location.line,
            self.location.column,
            self.kind
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LabelingPolicy {
    RequireBehaviorAndCaseIdentity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RiskReason {
    MissingCaseIdentity,
    MissingBehaviorIdentity,
    AcceptedExceptionRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispositionKind {
    RepairRequired,
    AcceptedException,
    SafeLabelingProven,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopRisk {
    Risky {
        finding_id: FindingId,
        reason: RiskReason,
        required_action: DispositionKind,
    },
    SafeLabelingProven {
        finding_id: FindingId,
        behavior_evidence: BehaviorEvidence,
        case_evidence: CaseEvidence,
    },
}
