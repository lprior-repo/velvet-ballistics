#![no_main]

use libfuzzer_sys::fuzz_target;
use xtask::doc_reconcile::reconcile::check_doc_taint_consistency;
use xtask::doc_reconcile::{
    ClaimKind, ConflictKind, DocReconcileError, RequiredEvidence, ResolvedNode,
};

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let markdown = String::from_utf8_lossy(data);

    let result = check_doc_taint_consistency(&markdown);

    match result {
        Ok(report) => {
            assert!(
                !report.scanned_nodes.is_empty(),
                "scanned_nodes must be populated for Ok result"
            );
            assert!(
                report.no_join_claims.len() <= report.stale_clean_only.len(),
                "no_join_claims must be a subset of stale_clean_only"
            );
            assert!(
                report.write_slot_only_claims.len() <= report.stale_clean_only.len(),
                "write_slot_only_claims must be a subset of stale_clean_only"
            );
        }
        Err(error) => {
            let actual = std::mem::discriminant(&error);
            let known: &[std::mem::Discriminant<DocReconcileError>] = &[
                std::mem::discriminant(&DocReconcileError::WrongWorkspace {
                    path: std::path::PathBuf::new(),
                }),
                std::mem::discriminant(&DocReconcileError::OutOfScopeChange {
                    change_kind: String::new(),
                    path_or_operation: String::new(),
                }),
                std::mem::discriminant(&DocReconcileError::StaleCleanOnlyTaintText {
                    node: ResolvedNode::EvalExpr,
                    phrase: String::new(),
                }),
                std::mem::discriminant(&DocReconcileError::UnsupportedEvidenceClaim {
                    sentence: String::new(),
                    claim_kind: ClaimKind::TestEvidence,
                    required: RequiredEvidence::ConcreteArtifactOrPendingMarker,
                }),
                std::mem::discriminant(&DocReconcileError::TaintVocabularyConflict {
                    conflict: ConflictKind::WrongOrder,
                    sentence: String::new(),
                    term: None,
                }),
                std::mem::discriminant(&DocReconcileError::ControlFlowTaintConflation {
                    sentence: String::new(),
                }),
                std::mem::discriminant(&DocReconcileError::MissingTraceability {
                    clause: String::new(),
                }),
            ];
            assert!(
                known.contains(&actual),
                "DocReconcileError discriminant must match a known production variant"
            );
        }
    }
});
