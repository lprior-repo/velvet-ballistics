use super::*;

pub fn classify_loop_pattern(
    pattern: LoopPattern,
    _policy: LabelingPolicy,
) -> Result<LoopRisk, InventoryError> {
    let finding_id = pattern.finding_id();
    classify_evidence(pattern.label_evidence, finding_id, pattern.kind)
}

fn classify_evidence(
    evidence: LabelEvidence,
    finding_id: FindingId,
    kind: LoopPatternKind,
) -> Result<LoopRisk, InventoryError> {
    match evidence {
        LabelEvidence::Absent => Ok(risky_for_kind(finding_id, kind)),
        LabelEvidence::DuplicateCaseLabel {
            label,
            behavior,
            case_count,
        } => ambiguous_loop_label(label, behavior, case_count),
        LabelEvidence::BehaviorOnly { behavior } => {
            ambiguous_loop_label(CaseLabel(behavior.0.clone()), Some(behavior), 0)
        }
        LabelEvidence::CaseOnly { case } => ambiguous_loop_label(case, None, 1),
        LabelEvidence::BehaviorAndCases { behavior, cases } => {
            safe_loop_risk(finding_id, behavior, cases)
        }
        LabelEvidence::AcceptedExceptionEvidence { .. } => accepted_exception_risk(finding_id),
    }
}

fn risky_for_kind(finding_id: FindingId, kind: LoopPatternKind) -> LoopRisk {
    let reason = if kind == LoopPatternKind::IteratorTableLoop {
        RiskReason::MissingBehaviorIdentity
    } else {
        RiskReason::MissingCaseIdentity
    };
    LoopRisk::Risky {
        finding_id,
        reason,
        required_action: DispositionKind::RepairRequired,
    }
}

fn ambiguous_loop_label(
    label: CaseLabel,
    behavior: Option<BehaviorEvidence>,
    case_count: usize,
) -> Result<LoopRisk, InventoryError> {
    Err(InventoryError::AmbiguousCaseLabel {
        label: label.0,
        behavior: behavior.map(|value| value.0),
        case_count,
    })
}

fn safe_loop_risk(
    finding_id: FindingId,
    behavior: BehaviorEvidence,
    cases: CaseEvidence,
) -> Result<LoopRisk, InventoryError> {
    if cases.0.is_empty() {
        validate_case_evidence(&behavior, &cases)?;
    }
    Ok(LoopRisk::SafeLabelingProven {
        finding_id,
        behavior_evidence: behavior,
        case_evidence: cases,
    })
}

fn accepted_exception_risk(finding_id: FindingId) -> Result<LoopRisk, InventoryError> {
    Ok(LoopRisk::Risky {
        finding_id,
        reason: RiskReason::AcceptedExceptionRequired,
        required_action: DispositionKind::AcceptedException,
    })
}
