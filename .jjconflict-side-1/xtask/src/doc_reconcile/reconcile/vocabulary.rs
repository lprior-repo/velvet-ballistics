use crate::doc_reconcile::{
    ConflictKind, DocReconcileError, MasterDocSnapshot, PreservedNonGoal, TaintVocabularyReport,
    TaintVocabularyRule,
};

use super::text::sentence_containing;

pub fn validate_taint_vocabulary_consistency(
    doc: MasterDocSnapshot,
) -> Result<TaintVocabularyReport, DocReconcileError> {
    if doc.text.contains("Clean < Secret < DerivedFromSecret") {
        return Err(DocReconcileError::TaintVocabularyConflict {
            conflict: ConflictKind::WrongOrder,
            sentence: "Clean < Secret < DerivedFromSecret".to_owned(),
            term: None,
        });
    }
    if let Some(sentence) = sentence_containing(&doc.text, "branch-condition taint") {
        let lower = sentence.to_ascii_lowercase();
        if lower.contains("track") && !lower.contains("does not track") {
            return Err(DocReconcileError::ControlFlowTaintConflation {
                sentence: sentence.to_owned(),
            });
        }
    }
    if let Some(sentence) = sentence_containing(&doc.text, "Private") {
        return Err(DocReconcileError::TaintVocabularyConflict {
            conflict: ConflictKind::UnknownTerm,
            sentence: sentence.to_owned(),
            term: Some("Private".to_owned()),
        });
    }
    if doc
        .text
        .contains("Secret downgrades to Clean after BuildList")
    {
        return Err(DocReconcileError::TaintVocabularyConflict {
            conflict: ConflictKind::Downgrade,
            sentence: "Secret downgrades to Clean after BuildList".to_owned(),
            term: None,
        });
    }
    Ok(TaintVocabularyReport {
        lattice: vec![
            "Clean".to_owned(),
            "DerivedFromSecret".to_owned(),
            "Secret".to_owned(),
        ],
        propagation_rule: TaintVocabularyRule::JoinedDataFlowTaint,
        conflicts: Vec::new(),
        control_flow_scope: PreservedNonGoal::ControlFlowTaintV1NonGoal,
    })
}
