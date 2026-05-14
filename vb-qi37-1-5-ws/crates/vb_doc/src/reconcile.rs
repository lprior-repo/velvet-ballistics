use std::path::PathBuf;

use crate::evidence::EvidenceIndex;
use crate::{
    ContradictionReport, DocPatchPlan, DocReconcileError, EvidenceBoundedReport, EvidencePolicy,
    MasterDocSnapshot, PatchEdit, PatchPlanStatus, PatchTarget, TaintVocabularyReport,
};

mod contradictions;
mod evidence_claims;
mod vocabulary;
mod workspace;

const MASTER_DOC_FILE: &str = "velvet-ballistics-MASTER.md";

pub fn plan_taint_doc_reconciliation(
    doc: MasterDocSnapshot,
    policy: EvidencePolicy,
) -> Result<DocPatchPlan, DocReconcileError> {
    workspace::validate_workspace(&doc.path, &policy.workspace_root, MASTER_DOC_FILE)?;
    validate_taint_vocabulary_consistency(doc.clone())?;
    validate_evidence_bounded_wording(doc.clone(), EvidenceIndex::empty())?;
    let contradictions = contradictions::collect(&doc.text);
    let edits = edits_for(&contradictions, &doc.text);
    let status = status_for(&edits);
    Ok(DocPatchPlan {
        target: PatchTarget::MasterDoc(doc.path),
        edits,
        stale_text_removed: contradictions.all_stale_phrases,
        evidence_actions: policy.required,
        preserved_non_goals: workspace::preserved_non_goals(&doc.text),
        forbidden_actions: Vec::new(),
        contradiction_count: contradictions.count,
        status,
    })
}

pub fn scan_for_stale_clean_only_text(
    doc: MasterDocSnapshot,
) -> Result<ContradictionReport, DocReconcileError> {
    let contradictions = contradictions::collect(&doc.text);
    if let Some(finding) = contradictions.first_error {
        Err(finding)
    } else {
        Ok(ContradictionReport {
            stale_clean_only: Vec::new(),
            no_join_claims: Vec::new(),
            write_slot_only_claims: Vec::new(),
            scanned_nodes: contradictions::scanned_nodes(),
        })
    }
}

pub fn check_doc_taint_consistency(text: &str) -> Result<ContradictionReport, DocReconcileError> {
    scan_for_stale_clean_only_text(MasterDocSnapshot::for_workspace_text(
        PathBuf::from(MASTER_DOC_FILE),
        text,
    ))
}

pub fn validate_evidence_bounded_wording(
    doc: MasterDocSnapshot,
    evidence: EvidenceIndex,
) -> Result<EvidenceBoundedReport, DocReconcileError> {
    evidence_claims::validate(doc, evidence)
}

pub fn validate_taint_vocabulary_consistency(
    doc: MasterDocSnapshot,
) -> Result<TaintVocabularyReport, DocReconcileError> {
    vocabulary::validate(doc)
}

fn status_for(edits: &[PatchEdit]) -> PatchPlanStatus {
    if edits.is_empty() {
        PatchPlanStatus::AlreadyConsistent
    } else {
        PatchPlanStatus::NeedsReconciliation
    }
}

fn edits_for(contradictions: &contradictions::Contradictions, text: &str) -> Vec<PatchEdit> {
    let mut edits = Vec::new();
    if contradictions.has_eval_expr {
        edits.push(PatchEdit::EvalExprJoin);
    }
    if contradictions.has_build_object {
        edits.push(PatchEdit::BuildObjectJoin);
    }
    if contradictions.has_build_list {
        edits.push(PatchEdit::BuildListJoin);
    }
    if !text.contains("Finish emits EngineSignal::Finished(SlotValue, Taint)") {
        edits.push(PatchEdit::FinishCarriesTaint);
    }
    edits
}
