// Stub for vb_doc reconcile module — functionality deferred to implementation phase.
use crate::doc_reconcile::{
    ContradictionReport, DocPatchPlan, DocReconcileError, EvidenceBoundedReport,
    EvidencePolicy, MasterDocSnapshot, TaintVocabularyReport,
};

pub fn plan_taint_doc_reconciliation(
    doc: MasterDocSnapshot,
    policy: EvidencePolicy,
) -> Result<DocPatchPlan, DocReconcileError> {
    Err(DocReconcileError::OutOfScopeChange {
        change_kind: String::new(),
        path_or_operation: String::new(),
    })
}

pub fn scan_for_stale_clean_only_text(
    doc: MasterDocSnapshot,
) -> Result<ContradictionReport, DocReconcileError> {
    Err(DocReconcileError::OutOfScopeChange {
        change_kind: String::new(),
        path_or_operation: String::new(),
    })
}

pub fn check_doc_taint_consistency(
    text: &str,
) -> Result<ContradictionReport, DocReconcileError> {
    let _ = text;
    Err(DocReconcileError::OutOfScopeChange {
        change_kind: String::new(),
        path_or_operation: String::new(),
    })
}

pub fn validate_evidence_bounded_wording(
    doc: MasterDocSnapshot,
    evidence: crate::doc_reconcile::evidence::EvidenceIndex,
) -> Result<EvidenceBoundedReport, DocReconcileError> {
    let _ = (doc, evidence);
    Err(DocReconcileError::OutOfScopeChange {
        change_kind: String::new(),
        path_or_operation: String::new(),
    })
}

pub fn validate_taint_vocabulary_consistency(
    doc: MasterDocSnapshot,
) -> Result<TaintVocabularyReport, DocReconcileError> {
    Err(DocReconcileError::OutOfScopeChange {
        change_kind: String::new(),
        path_or_operation: String::new(),
    })
}
