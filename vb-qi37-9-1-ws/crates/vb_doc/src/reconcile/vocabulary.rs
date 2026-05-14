use crate::{
    ConflictKind, DocReconcileError, MasterDocSnapshot, PreservedNonGoal, TaintVocabularyReport,
    TaintVocabularyRule,
};

pub(super) fn validate(doc: MasterDocSnapshot) -> Result<TaintVocabularyReport, DocReconcileError> {
    reject_control_flow_conflation(&doc.text)?;
    reject_lattice_conflicts(&doc.text)?;
    Ok(taint_vocabulary_report())
}

fn taint_vocabulary_report() -> TaintVocabularyReport {
    TaintVocabularyReport {
        lattice: vec![
            "Clean".to_owned(),
            "DerivedFromSecret".to_owned(),
            "Secret".to_owned(),
        ],
        propagation_rule: TaintVocabularyRule::JoinedDataFlowTaint,
        conflicts: Vec::new(),
        control_flow_scope: PreservedNonGoal::ControlFlowTaintV1NonGoal,
    }
}

fn reject_control_flow_conflation(text: &str) -> Result<(), DocReconcileError> {
    if contains_control_flow_conflation(text) {
        Err(DocReconcileError::ControlFlowTaintConflation {
            sentence: text.to_owned(),
        })
    } else {
        Ok(())
    }
}

fn reject_lattice_conflicts(text: &str) -> Result<(), DocReconcileError> {
    if text.contains("Clean < Secret < DerivedFromSecret") {
        return Err(vocabulary_error(
            ConflictKind::WrongOrder,
            text.to_owned(),
            None,
        ));
    }
    if text.contains("Secret downgrades to Clean") {
        return Err(vocabulary_error(
            ConflictKind::Downgrade,
            text.to_owned(),
            None,
        ));
    }
    if text.contains("Private") {
        return Err(vocabulary_error(
            ConflictKind::UnknownTerm,
            text.to_owned(),
            Some("Private".to_owned()),
        ));
    }
    Ok(())
}

fn vocabulary_error(
    conflict: ConflictKind,
    sentence: String,
    term: Option<String>,
) -> DocReconcileError {
    DocReconcileError::TaintVocabularyConflict {
        conflict,
        sentence,
        term,
    }
}

fn contains_control_flow_conflation(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("tracks secret branch-condition taint")
        || lower.contains("tracks branch-condition taint")
}
