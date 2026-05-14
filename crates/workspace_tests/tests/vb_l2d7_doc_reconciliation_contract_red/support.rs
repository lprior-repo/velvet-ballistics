pub(crate) use std::path::PathBuf;

pub(crate) use vb_doc::evidence::{EvidenceIndex, EvidenceSupport};
pub(crate) use vb_doc::reconcile::{
    plan_taint_doc_reconciliation, scan_for_stale_clean_only_text,
    validate_evidence_bounded_wording, validate_taint_vocabulary_consistency,
};
pub(crate) use vb_doc::{
    ClaimKind, ConflictKind, ContradictionReport, DocPatchPlan, DocReconcileError,
    EvidenceBoundedReport, EvidencePolicy, MasterDocSnapshot, PatchEdit, PatchPlanStatus,
    PatchTarget, PreservedNonGoal, RequiredEvidence, ResolvedNode, StalePhrase,
    TaintVocabularyReport, TaintVocabularyRule,
};

const MASTER_DOC_FILE: &str = "velvet-ballistics-MASTER.md";

pub(crate) fn workspace_root() -> PathBuf {
    std::env::temp_dir().join("vb_l2d7_contract_workspace")
}

pub(crate) fn master_doc_path() -> PathBuf {
    workspace_root().join(MASTER_DOC_FILE)
}

pub(crate) fn outside_doc_path() -> PathBuf {
    std::env::temp_dir()
        .join("vb_l2d7_contract_outside")
        .join(MASTER_DOC_FILE)
}

pub(crate) fn snapshot_with(text: &str) -> MasterDocSnapshot {
    MasterDocSnapshot::for_workspace_text(master_doc_path(), text)
}

pub(crate) fn outside_snapshot_with(text: &str) -> MasterDocSnapshot {
    MasterDocSnapshot::for_workspace_text(outside_doc_path(), text)
}

pub(crate) fn strict_policy() -> EvidencePolicy {
    EvidencePolicy::strict_bounded(workspace_root())
}

pub(crate) fn bounded_evidence() -> EvidenceIndex {
    EvidenceIndex::from_supports(vec![
        EvidenceSupport::cited(
            "DRIFT-1 joined taint is resolved",
            "formal-verification-report.md",
        ),
        EvidenceSupport::pending("runtime rejection claim remains unverified"),
    ])
}
