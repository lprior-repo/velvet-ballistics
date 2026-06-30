mod evidence;
mod scan;
mod text;
mod vocabulary;

use crate::doc_reconcile::{
    ContradictionReport, DocPatchPlan, DocReconcileError, EvidencePolicy, MasterDocSnapshot,
    PatchPlanStatus, PatchTarget,
};

pub use evidence::validate_evidence_bounded_wording;
pub use vocabulary::validate_taint_vocabulary_consistency;

use scan::{collect_contradictions, edits_for, first_stale_error, preserved_non_goals};

pub fn plan_taint_doc_reconciliation(
    doc: MasterDocSnapshot,
    policy: EvidencePolicy,
) -> Result<DocPatchPlan, DocReconcileError> {
    if !doc.path.starts_with(&policy.workspace_root) {
        return Err(DocReconcileError::WrongWorkspace { path: doc.path });
    }
    validate_taint_vocabulary_consistency(doc.clone())?;
    validate_evidence_bounded_wording(
        doc.clone(),
        crate::doc_reconcile::evidence::EvidenceIndex::empty(),
    )?;
    let report = collect_contradictions(&doc.text);
    let edits = edits_for(&doc.text);
    let status = if edits.is_empty() {
        PatchPlanStatus::AlreadyConsistent
    } else {
        PatchPlanStatus::NeedsReconciliation
    };
    Ok(DocPatchPlan {
        target: PatchTarget::MasterDoc(doc.path),
        edits,
        stale_text_removed: report.stale_clean_only.clone(),
        evidence_actions: policy.required,
        preserved_non_goals: preserved_non_goals(&doc.text),
        forbidden_actions: Vec::new(),
        contradiction_count: report.stale_clean_only.len(),
        status,
    })
}

pub fn scan_for_stale_clean_only_text(
    doc: MasterDocSnapshot,
) -> Result<ContradictionReport, DocReconcileError> {
    if let Some(error) = first_stale_error(&doc.text) {
        Err(error)
    } else {
        Ok(collect_contradictions(&doc.text))
    }
}

pub fn check_doc_taint_consistency(text: &str) -> Result<ContradictionReport, DocReconcileError> {
    if let Some(error) = first_stale_error(text) {
        Err(error)
    } else {
        Ok(collect_contradictions(text))
    }
}
