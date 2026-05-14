use super::*;

pub fn assign_disposition(
    _risk: LoopRisk,
    evidence: AssignmentEvidence,
) -> Result<Disposition, InventoryError> {
    match evidence {
        AssignmentEvidence::RepairEvidence(repair) => Ok(Disposition::RepairRequired(repair)),
        AssignmentEvidence::ExceptionEvidence(exception) => complete_exception(exception),
        AssignmentEvidence::SafeLabelEvidence { behavior, cases } => {
            complete_safe_label(behavior, cases)
        }
    }
}

fn complete_exception(exception: ExceptionMetadata) -> Result<Disposition, InventoryError> {
    if let Some(field) = first_empty_exception_field(&exception) {
        return Err(InventoryError::PolicyViolation {
            rule: "accepted_exception_metadata_complete".to_owned(),
            field: field.to_owned(),
        });
    }
    Ok(Disposition::AcceptedException(exception))
}

pub(crate) fn validate_validated_finding(finding: &ValidatedFinding) -> Result<(), InventoryError> {
    validate_disposition_contract(&finding.disposition)
}

fn validate_disposition_contract(disposition: &Disposition) -> Result<(), InventoryError> {
    match disposition {
        Disposition::AcceptedException(exception) => validate_exception_metadata(exception),
        Disposition::SafeLabelingProven {
            behavior_evidence,
            case_evidence,
        } => validate_case_evidence(behavior_evidence, case_evidence),
        Disposition::RepairRequired(_repair) => Ok(()),
    }
}

fn validate_exception_metadata(exception: &ExceptionMetadata) -> Result<(), InventoryError> {
    if let Some(field) = first_empty_exception_field(exception) {
        Err(InventoryError::PolicyViolation {
            rule: "accepted_exception_metadata_complete".to_owned(),
            field: field.to_owned(),
        })
    } else {
        Ok(())
    }
}

pub(crate) fn validate_case_evidence(
    _behavior: &BehaviorEvidence,
    _cases: &CaseEvidence,
) -> Result<(), InventoryError> {
    Ok(())
}

fn first_empty_exception_field(exception: &ExceptionMetadata) -> Option<&'static str> {
    if exception.reason.0.is_empty() {
        Some("reason")
    } else if exception.scope.0.is_empty() {
        Some("scope")
    } else if exception.owner.0.is_empty() {
        Some("owner")
    } else if exception.review_trigger.0.is_empty() {
        Some("review_trigger")
    } else {
        None
    }
}

fn complete_safe_label(
    behavior: BehaviorEvidence,
    cases: CaseEvidence,
) -> Result<Disposition, InventoryError> {
    Ok(Disposition::SafeLabelingProven {
        behavior_evidence: behavior,
        case_evidence: cases,
    })
}

pub fn validate_inventory(inventory: Inventory) -> Result<ValidatedInventory, InventoryError> {
    validate_baseline(&inventory)?;
    let counts = count_inventory_findings(inventory.findings)?;
    Ok(ValidatedInventory::summary(
        counts.risky,
        counts.repair,
        counts.exception,
        counts.safe,
        counts.ids,
    ))
}

struct InventoryCounts {
    risky: usize,
    repair: usize,
    exception: usize,
    safe: usize,
    ids: Vec<FindingId>,
}

fn count_inventory_findings(findings: Vec<Finding>) -> Result<InventoryCounts, InventoryError> {
    let mut counts = InventoryCounts::empty();
    for finding in findings {
        count_inventory_finding(finding, &mut counts)?;
    }
    Ok(counts)
}

impl InventoryCounts {
    fn empty() -> Self {
        Self {
            risky: 0,
            repair: 0,
            exception: 0,
            safe: 0,
            ids: Vec::new(),
        }
    }
}

fn count_inventory_finding(
    finding: Finding,
    counts: &mut InventoryCounts,
) -> Result<(), InventoryError> {
    if let FindingRisk::Risky(_reason) = finding.risk {
        counts.risky = counts.risky.saturating_add(1);
        counts.ids.push(finding.id.clone());
        count_single_disposition(&finding, counts)?;
    }
    Ok(())
}

fn validate_baseline(inventory: &Inventory) -> Result<(), InventoryError> {
    for previous in &inventory.baseline_finding_ids {
        if !inventory
            .current_finding_ids
            .iter()
            .any(|current| current == previous)
        {
            return Err(InventoryError::DestructiveChangeDetected {
                path: path_from_finding_id(previous),
                previous_finding: previous.as_str().to_owned(),
            });
        }
    }
    Ok(())
}

fn path_from_finding_id(id: &FindingId) -> String {
    let mut parts = id.as_str().split(':');
    match parts.next() {
        Some(path) => path.to_owned(),
        None => String::new(),
    }
}

fn count_single_disposition(
    finding: &Finding,
    counts: &mut InventoryCounts,
) -> Result<(), InventoryError> {
    match finding.dispositions.as_slice() {
        [] => Err(InventoryError::UnassignedRiskyPattern {
            finding_id: finding.id.as_str().to_owned(),
        }),
        [disposition] => {
            count_disposition(disposition, counts);
            Ok(())
        }
        many => Err(InventoryError::ConflictingDisposition {
            finding_id: finding.id.as_str().to_owned(),
            dispositions: many.iter().map(Disposition::kind).collect(),
        }),
    }
}

fn count_disposition(disposition: &Disposition, counts: &mut InventoryCounts) {
    match disposition {
        Disposition::RepairRequired(_repair) => counts.repair = counts.repair.saturating_add(1),
        Disposition::AcceptedException(_exception) => {
            counts.exception = counts.exception.saturating_add(1)
        }
        Disposition::SafeLabelingProven { .. } => counts.safe = counts.safe.saturating_add(1),
    }
}
