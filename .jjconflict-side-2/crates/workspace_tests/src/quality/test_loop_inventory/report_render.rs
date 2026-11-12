use super::*;

pub fn render_inventory_report(
    inventory: ValidatedInventory,
) -> Result<InventoryReport, InventoryError> {
    if let Some((rule, field)) = inventory.policy_violation {
        return Err(InventoryError::PolicyViolation { rule, field });
    }
    let mut findings = Vec::with_capacity(inventory.findings.len());
    for finding in inventory.findings {
        validate_validated_finding(&finding)?;
        findings.push(report_finding(finding));
    }
    Ok(InventoryReport::from_findings(
        inventory.risky_count,
        findings,
        inventory.mutation_evidence,
        inventory.mutation_improvement_claim,
    ))
}

fn report_finding(finding: ValidatedFinding) -> ReportFinding {
    let (summary, disposition) = split_validated_finding(finding);
    match disposition {
        Disposition::RepairRequired(repair) => report_repair(summary, repair),
        Disposition::AcceptedException(exception) => report_exception(summary, exception),
        Disposition::SafeLabelingProven {
            behavior_evidence,
            case_evidence,
        } => report_safe_label(summary, behavior_evidence, case_evidence),
    }
}

fn split_validated_finding(finding: ValidatedFinding) -> (FindingSummary, Disposition) {
    let ValidatedFinding {
        path,
        location,
        kind,
        risk_reason,
        disposition,
    } = finding;
    (
        FindingSummary {
            path,
            location,
            kind,
            risk_reason,
        },
        disposition,
    )
}

fn report_repair(summary: FindingSummary, repair: RepairMetadata) -> ReportFinding {
    ReportFinding::repair_required(summary, repair)
}

fn report_exception(summary: FindingSummary, exception: ExceptionMetadata) -> ReportFinding {
    ReportFinding::accepted_exception(summary.path.as_str(), summary.location.as_str(), exception)
}

fn report_safe_label(
    summary: FindingSummary,
    behavior_evidence: BehaviorEvidence,
    case_evidence: CaseEvidence,
) -> ReportFinding {
    ReportFinding::safe_labeling(
        summary.path.as_str(),
        summary.location.as_str(),
        &behavior_evidence.0,
        case_evidence.0,
    )
}
