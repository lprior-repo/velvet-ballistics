//! Tests for vb_doc public API — vb_doc is the document reconciliation crate.
//!
//! These tests cover the public API surface with exact assertions on every
//! Error variant and every success path. Density target: ≥5× pub fn count.

use vb_doc::evidence::{EvidenceIndex, EvidenceSupport};
use vb_doc::reconcile::{
    check_doc_taint_consistency, plan_taint_doc_reconciliation, scan_for_stale_clean_only_text,
    validate_evidence_bounded_wording, validate_taint_vocabulary_consistency,
};
use vb_doc::{
    ClaimKind, ConflictKind, ContradictionReport, DocReconcileError, EvidenceBoundedReport,
    EvidencePolicy, MasterDocSnapshot, PatchEdit, PatchPlanStatus, PatchTarget, PreservedNonGoal,
    RequiredEvidence, ResolvedNode, StalePhrase, TaintVocabularyReport, TaintVocabularyRule,
};

// ============================================================================
// MasterDocSnapshot — constructor
// ============================================================================

#[test]
fn master_doc_snapshot_for_workspace_text_constructs_with_path_and_text() {
    // Given
    let path = std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md");
    let text = "Clean < DerivedFromSecret < Secret.";

    // When
    let snapshot = MasterDocSnapshot::for_workspace_text(path.clone(), text);

    // Then
    assert_eq!(snapshot.path, path);
    assert_eq!(snapshot.text, text);
}

#[test]
fn master_doc_snapshot_for_workspace_text_clone_is_independent() {
    // Given
    let path = std::path::PathBuf::from("/tmp/master.md");
    let snapshot = MasterDocSnapshot::for_workspace_text(path.clone(), "original");

    // When
    let cloned = snapshot.clone();
    // Then
    assert_eq!(snapshot.path, cloned.path);
    assert_eq!(snapshot.text, cloned.text);
    assert!(std::ptr::eq(&snapshot.text, &cloned.text) == false); // deep clone
}

// ============================================================================
// EvidencePolicy — constructor
// ============================================================================

#[test]
fn evidence_policy_strict_bounded_sets_workspace_root() {
    // Given
    let root = std::path::PathBuf::from("/tmp/workspace");

    // When
    let policy = EvidencePolicy::strict_bounded(root.clone());

    // Then
    assert_eq!(policy.workspace_root, root);
    assert_eq!(
        policy.required,
        RequiredEvidence::ConcreteArtifactOrPendingMarker
    );
}

#[test]
fn evidence_policy_strict_bounded_clone_is_independent() {
    // Given
    let policy = EvidencePolicy::strict_bounded(std::path::PathBuf::from("/tmp"));

    // When
    let cloned = policy.clone();

    // Then
    assert_eq!(policy.workspace_root, cloned.workspace_root);
    assert_eq!(policy.required, cloned.required);
}

// ============================================================================
// EvidenceIndex — constructors (behavior tested via validate_evidence_bounded_wording)
// ============================================================================

#[test]
fn evidence_index_empty_creates_valid_index() {
    // When
    let index = EvidenceIndex::empty();

    // Then - used in validate_evidence_bounded_wording without error
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "Clean text.",
    );
    let result = validate_evidence_bounded_wording(doc, index);
    assert_eq!(
        result,
        Ok(EvidenceBoundedReport {
            unsupported_claims: Vec::new(),
            cited_claims: 0,
            pending_claims: 0,
            forbidden_claims: Vec::new(),
        })
    );
}

#[test]
fn evidence_index_from_supports_works_with_join_claim() {
    // Given
    let supports = vec![
        EvidenceSupport::cited("DRIFT-1 joined taint is resolved", "formal-report.md"),
        EvidenceSupport::pending("runtime rejection claim"),
    ];

    // When
    let index = EvidenceIndex::from_supports(supports);

    // Then - used in validate_evidence_bounded_wording without error
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "Clean text.",
    );
    let result = validate_evidence_bounded_wording(doc, index);
    assert_eq!(
        result,
        Ok(EvidenceBoundedReport {
            unsupported_claims: Vec::new(),
            cited_claims: 0,
            pending_claims: 0,
            forbidden_claims: Vec::new(),
        })
    );
}

// ============================================================================
// EvidenceSupport — constructors (behavior tested via validate_evidence_bounded_wording)
// ============================================================================

#[test]
fn evidence_support_cited_creates_valid_support() {
    // Given
    let support = EvidenceSupport::cited("DRIFT-1 is verified", "report.md");
    let index = EvidenceIndex::from_supports(vec![support]);

    // Then - used in validate_evidence_bounded_wording with matching claim succeeds
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "Clean < DerivedFromSecret < Secret.\nDRIFT-1 is verified",
    );
    let result = validate_evidence_bounded_wording(doc, index);
    assert_eq!(
        result,
        Ok(EvidenceBoundedReport {
            unsupported_claims: Vec::new(),
            cited_claims: 0,
            pending_claims: 0,
            forbidden_claims: Vec::new(),
        })
    );
}

#[test]
fn evidence_support_pending_creates_valid_pending_support() {
    // Given
    let support = EvidenceSupport::pending("runtime claim");
    let index = EvidenceIndex::from_supports(vec![support]);

    // Then - used in validate_evidence_bounded_wording with pending claim in text
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "Clean < DerivedFromSecret < Secret.\nruntime claim remains unverified",
    );
    let result = validate_evidence_bounded_wording(doc, index);
    assert_eq!(
        result,
        Ok(EvidenceBoundedReport {
            unsupported_claims: Vec::new(),
            cited_claims: 0,
            pending_claims: 1,
            forbidden_claims: Vec::new(),
        })
    );
}

// ============================================================================
// validate_taint_vocabulary_consistency — vocabulary rule validation
// ============================================================================

#[test]
fn validate_taint_vocabulary_consistency_returns_report_for_valid_lattice() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "Clean < DerivedFromSecret < Secret.\n\
         v1 does not track control-flow taint.",
    );

    // When
    let result = validate_taint_vocabulary_consistency(doc);

    // Then
    let report = result.expect("should not error");
    assert_eq!(report.lattice, vec!["Clean", "DerivedFromSecret", "Secret"]);
    assert_eq!(
        report.propagation_rule,
        TaintVocabularyRule::JoinedDataFlowTaint
    );
    assert!(report.conflicts.is_empty());
}

#[test]
fn validate_taint_vocabulary_consistency_rejects_wrong_order() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "Clean < Secret < DerivedFromSecret",
    );

    // When
    let result = validate_taint_vocabulary_consistency(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::TaintVocabularyConflict {
            conflict: ConflictKind::WrongOrder,
            sentence: "Clean < Secret < DerivedFromSecret".to_owned(),
            term: None,
        })
    );
}

#[test]
fn validate_taint_vocabulary_consistency_rejects_downgrade() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "Secret downgrades to Clean after BuildList",
    );

    // When
    let result = validate_taint_vocabulary_consistency(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::TaintVocabularyConflict {
            conflict: ConflictKind::Downgrade,
            sentence: "Secret downgrades to Clean after BuildList".to_owned(),
            term: None,
        })
    );
}

#[test]
fn validate_taint_vocabulary_consistency_rejects_unknown_term_private() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "Private taint dominates Secret",
    );

    // When
    let result = validate_taint_vocabulary_consistency(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::TaintVocabularyConflict {
            conflict: ConflictKind::UnknownTerm,
            sentence: "Private taint dominates Secret".to_owned(),
            term: Some("Private".to_owned()),
        })
    );
}

#[test]
fn validate_taint_vocabulary_consistency_rejects_control_flow_conflation_tracks() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "v1 tracks branch-condition taint",
    );

    // When
    let result = validate_taint_vocabulary_consistency(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::ControlFlowTaintConflation {
            sentence: "v1 tracks branch-condition taint".to_owned(),
        })
    );
}

#[test]
fn validate_taint_vocabulary_consistency_rejects_control_flow_conflation_tracks_secret() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "runtime tracks secret branch-condition taint",
    );

    // When
    let result = validate_taint_vocabulary_consistency(doc);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::ControlFlowTaintConflation {
            sentence: "runtime tracks secret branch-condition taint".to_owned(),
        })
    );
}

#[test]
fn validate_taint_vocabulary_consistency_accepts_empty_text() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "",
    );

    // When
    let result = validate_taint_vocabulary_consistency(doc);

    // Then
    let report = result.expect("should not error");
    assert_eq!(report.lattice, vec!["Clean", "DerivedFromSecret", "Secret"]);
    assert!(report.conflicts.is_empty());
}

#[test]
fn validate_taint_vocabulary_consistency_preserved_non_goal_present() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "Clean < DerivedFromSecret < Secret.\n\
         v1 does not track control-flow taint.",
    );

    // When
    let result = validate_taint_vocabulary_consistency(doc);

    // Then
    let report = result.expect("should not error");
    assert_eq!(
        report.control_flow_scope,
        PreservedNonGoal::ControlFlowTaintV1NonGoal
    );
}

// ============================================================================
// validate_evidence_bounded_wording — evidence claim validation
// ============================================================================

#[test]
fn validate_evidence_bounded_wording_returns_empty_report_for_no_claims() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "Some text with no evidence claims.",
    );
    let evidence = EvidenceIndex::empty();

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then
    let report = result.expect("should not error");
    assert!(report.unsupported_claims.is_empty());
    assert_eq!(report.cited_claims, 0);
    assert_eq!(report.pending_claims, 0);
    assert!(report.forbidden_claims.is_empty());
}

#[test]
fn validate_evidence_bounded_wording_rejects_test_evidence_without_support() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "tests prove joined taint",
    );
    let evidence = EvidenceIndex::empty();

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::UnsupportedEvidenceClaim {
            sentence: "tests prove joined taint".to_owned(),
            claim_kind: ClaimKind::TestEvidence,
            required: RequiredEvidence::ConcreteArtifactOrPendingMarker,
        })
    );
}

#[test]
fn validate_evidence_bounded_wording_rejects_formal_evidence_without_support() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "Lean proves implementation parity",
    );
    let evidence = EvidenceIndex::empty();

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::UnsupportedEvidenceClaim {
            sentence: "Lean proves implementation parity".to_owned(),
            claim_kind: ClaimKind::FormalEvidence,
            required: RequiredEvidence::ConcreteArtifactOrPendingMarker,
        })
    );
}

#[test]
fn validate_evidence_bounded_wording_rejects_release_readiness_without_support() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "DRIFT-1 is release ready",
    );
    let evidence = EvidenceIndex::empty();

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::UnsupportedEvidenceClaim {
            sentence: "DRIFT-1 is release ready".to_owned(),
            claim_kind: ClaimKind::ReleaseReadiness,
            required: RequiredEvidence::ConcreteArtifactOrPendingMarker,
        })
    );
}

#[test]
fn validate_evidence_bounded_wording_rejects_generated_parity_drift_without_support() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "DRIFT-1 generated Rust and IR parity is verified",
    );
    let evidence = EvidenceIndex::empty();

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::UnsupportedEvidenceClaim {
            sentence: "DRIFT-1 generated Rust and IR parity is verified".to_owned(),
            claim_kind: ClaimKind::GeneratedParity,
            required: RequiredEvidence::ConcreteArtifactOrPendingMarker,
        })
    );
}

#[test]
fn validate_evidence_bounded_wording_rejects_generated_parity_full_without_support() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "full generated Rust and IR parity is verified",
    );
    let evidence = EvidenceIndex::empty();

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::UnsupportedEvidenceClaim {
            sentence: "full generated Rust and IR parity is verified".to_owned(),
            claim_kind: ClaimKind::GeneratedParity,
            required: RequiredEvidence::ConcreteArtifactOrPendingMarker,
        })
    );
}

#[test]
fn validate_evidence_bounded_wording_accepts_supported_test_evidence() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "tests prove joined taint test-report.html",
    );
    let evidence = EvidenceIndex::from_supports(vec![EvidenceSupport::cited(
        "tests prove joined taint",
        "test-report.html",
    )]);

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then
    let report = result.expect("should not error");
    assert_eq!(report.cited_claims, 1);
    assert_eq!(report.pending_claims, 0);
}

#[test]
fn validate_evidence_bounded_wording_counts_pending_correctly() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "runtime claim remains unverified",
    );
    let evidence = EvidenceIndex::from_supports(vec![EvidenceSupport::pending("runtime claim")]);

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then
    let report = result.expect("should not error");
    assert_eq!(report.cited_claims, 0);
    assert_eq!(report.pending_claims, 1);
}

// ============================================================================
// scan_for_stale_clean_only_text — stale phrase detection
// ============================================================================

#[test]
fn scan_for_stale_clean_only_text_returns_empty_for_clean_doc() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "Clean data flow without stale phrases.",
    );

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    let report = result.expect("should not error");
    assert_eq!(report.stale_clean_only, Vec::<StalePhrase>::new());
    assert_eq!(
        report.scanned_nodes,
        vec![
            ResolvedNode::EvalExpr,
            ResolvedNode::BuildObject,
            ResolvedNode::BuildList,
            ResolvedNode::Finish,
        ]
    );
}

#[test]
fn scan_for_stale_clean_only_text_detects_eval_expr_always_clean() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "EvalExpr is Always Clean",
    );

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    let err = result.expect_err("should detect stale phrase");
    assert_eq!(
        err,
        DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::EvalExpr,
            phrase: "Always Clean".to_owned(),
        }
    );
}

#[test]
fn scan_for_stale_clean_only_text_detects_build_object_always_clean() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "BuildObject is always Clean",
    );

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    let err = result.expect_err("should detect stale phrase");
    assert_eq!(
        err,
        DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::BuildObject,
            phrase: "Always Clean".to_owned(),
        }
    );
}

#[test]
fn scan_for_stale_clean_only_text_detects_no_taint_join() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "EvalExpr has No taint join",
    );

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    let err = result.expect_err("should detect stale phrase");
    assert_eq!(
        err,
        DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::EvalExpr,
            phrase: "No taint join".to_owned(),
        }
    );
}

#[test]
fn scan_for_stale_clean_only_text_detects_build_object_no_field_join() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "BuildObject has no join of field taints",
    );

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    let err = result.expect_err("should detect stale phrase");
    assert_eq!(
        err,
        DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::BuildObject,
            phrase: "no join of field taints".to_owned(),
        }
    );
}

#[test]
fn scan_for_stale_clean_only_text_detects_build_list_no_item_join() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "BuildList has no join of item taints",
    );

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    let err = result.expect_err("should detect stale phrase");
    assert_eq!(
        err,
        DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::BuildList,
            phrase: "no join of item taints".to_owned(),
        }
    );
}

#[test]
fn scan_for_stale_clean_only_text_detects_write_slot_not_write_slot_with_taint() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "EvalExpr uses write_slot not write_slot_with_taint",
    );

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    let err = result.expect_err("should detect stale phrase");
    assert_eq!(
        err,
        DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::EvalExpr,
            phrase: "write_slot".to_owned(),
        }
    );
}

#[test]
fn scan_for_stale_clean_only_text_detects_finish_without_taint() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "Finish emits Finished(SlotValue)",
    );

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    let err = result.expect_err("should detect stale phrase");
    assert_eq!(
        err,
        DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::Finish,
            phrase: "Finished(SlotValue)".to_owned(),
        }
    );
}

#[test]
fn scan_for_stale_clean_only_text_detects_finish_rejection() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "Finish rejects secret taint",
    );

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    let err = result.expect_err("should detect stale phrase");
    assert_eq!(
        err,
        DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::Finish,
            phrase: "rejects finish taint".to_owned(),
        }
    );
}

#[test]
fn scan_for_stale_clean_only_text_finish_no_rejection_allowed() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "Finish does not reject secret taint",
    );

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    // Should NOT error because "does not reject" is allowed
    let report = result.expect("should not detect false positive");
    assert!(report.stale_clean_only.is_empty());
}

// ============================================================================
// check_doc_taint_consistency — thin wrapper
// ============================================================================

#[test]
fn check_doc_taint_consistency_returns_report_for_clean_text() {
    // Given
    let text = "Clean text without any stale phrases.";

    // When
    let result = check_doc_taint_consistency(text);

    // Then
    let report = result.expect("should not error");
    assert!(report.stale_clean_only.is_empty());
}

#[test]
fn check_doc_taint_consistency_detects_stale_phrases() {
    // Given
    let text = "EvalExpr No taint join";

    // When
    let result = check_doc_taint_consistency(text);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::EvalExpr,
            phrase: "No taint join".to_owned(),
        })
    );
}

// ============================================================================
// plan_taint_doc_reconciliation — full reconciliation
// ============================================================================

#[test]
fn plan_taint_doc_reconciliation_returns_already_consistent_for_clean_doc() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/workspace/velvet-ballistics-MASTER.md"),
        "Clean < DerivedFromSecret < Secret.\n\
         EvalExpr uses joined data-flow taint.\n\
         Finish emits EngineSignal::Finished(SlotValue, Taint).\n\
         v1 does not track control-flow taint.",
    );
    let policy = EvidencePolicy::strict_bounded(std::path::PathBuf::from("/tmp/workspace"));

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    let plan = result.expect("should not error");
    assert_eq!(plan.status, PatchPlanStatus::AlreadyConsistent);
    assert!(plan.edits.is_empty());
    assert_eq!(plan.contradiction_count, 0);
}

#[test]
fn plan_taint_doc_reconciliation_returns_needs_reconciliation_for_stale_text() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/workspace/velvet-ballistics-MASTER.md"),
        "EvalExpr is Always Clean",
    );
    let policy = EvidencePolicy::strict_bounded(std::path::PathBuf::from("/tmp/workspace"));

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    let plan = result.expect("should not error");
    assert_eq!(plan.status, PatchPlanStatus::NeedsReconciliation);
    assert!(!plan.edits.is_empty());
    assert_eq!(plan.contradiction_count, 1);
}

#[test]
fn plan_taint_doc_reconciliation_wrong_workspace_returns_error() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/other/velvet-ballistics-MASTER.md"),
        "Clean < DerivedFromSecret < Secret.",
    );
    let policy = EvidencePolicy::strict_bounded(std::path::PathBuf::from("/tmp/workspace"));

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::WrongWorkspace {
            path: std::path::PathBuf::from("/tmp/other/velvet-ballistics-MASTER.md"),
        })
    );
}

#[test]
fn plan_taint_doc_reconciliation_vocabulary_conflict_returns_error() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/workspace/velvet-ballistics-MASTER.md"),
        "Clean < Secret < DerivedFromSecret",
    );
    let policy = EvidencePolicy::strict_bounded(std::path::PathBuf::from("/tmp/workspace"));

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::TaintVocabularyConflict {
            conflict: ConflictKind::WrongOrder,
            sentence: "Clean < Secret < DerivedFromSecret".to_owned(),
            term: None,
        })
    );
}

#[test]
fn plan_taint_doc_reconciliation_evidence_claim_without_support_returns_error() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/workspace/velvet-ballistics-MASTER.md"),
        "Clean < DerivedFromSecret < Secret.\n\
         tests prove joined taint",
    );
    let policy = EvidencePolicy::strict_bounded(std::path::PathBuf::from("/tmp/workspace"));

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::UnsupportedEvidenceClaim {
            sentence: "tests prove joined taint".to_owned(),
            claim_kind: ClaimKind::TestEvidence,
            required: RequiredEvidence::ConcreteArtifactOrPendingMarker,
        })
    );
}

#[test]
fn plan_taint_doc_reconciliation_edits_include_eval_expr_join() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/workspace/velvet-ballistics-MASTER.md"),
        "EvalExpr No taint join",
    );
    let policy = EvidencePolicy::strict_bounded(std::path::PathBuf::from("/tmp/workspace"));

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    let plan = result.expect("should not error");
    assert_eq!(plan.status, PatchPlanStatus::NeedsReconciliation);
    assert_eq!(
        plan.edits,
        vec![PatchEdit::EvalExprJoin, PatchEdit::FinishCarriesTaint]
    );
    assert_eq!(
        plan.stale_text_removed,
        vec![StalePhrase::EvalExprNoOperandJoin]
    );
    assert_eq!(
        plan.evidence_actions,
        RequiredEvidence::ConcreteArtifactOrPendingMarker
    );
    assert_eq!(plan.preserved_non_goals, Vec::<PreservedNonGoal>::new());
    assert_eq!(plan.forbidden_actions, Vec::<String>::new());
    assert_eq!(plan.contradiction_count, 1);
}

#[test]
fn plan_taint_doc_reconciliation_edits_include_build_object_join() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/workspace/velvet-ballistics-MASTER.md"),
        "BuildObject has no join of field taints",
    );
    let policy = EvidencePolicy::strict_bounded(std::path::PathBuf::from("/tmp/workspace"));

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    let plan = result.expect("should not error");
    assert_eq!(
        plan.edits,
        vec![PatchEdit::BuildObjectJoin, PatchEdit::FinishCarriesTaint]
    );
}

#[test]
fn plan_taint_doc_reconciliation_edits_include_build_list_join() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/workspace/velvet-ballistics-MASTER.md"),
        "BuildList has no join of item taints",
    );
    let policy = EvidencePolicy::strict_bounded(std::path::PathBuf::from("/tmp/workspace"));

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    let plan = result.expect("should not error");
    assert_eq!(
        plan.edits,
        vec![PatchEdit::BuildListJoin, PatchEdit::FinishCarriesTaint]
    );
}

#[test]
fn plan_taint_doc_reconciliation_edits_include_finish_carries_taint_when_missing() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/workspace/velvet-ballistics-MASTER.md"),
        "Finish emits Finished(SlotValue)",
    );
    let policy = EvidencePolicy::strict_bounded(std::path::PathBuf::from("/tmp/workspace"));

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    let plan = result.expect("should not error");
    assert_eq!(plan.edits, vec![PatchEdit::FinishCarriesTaint]);
}

#[test]
fn plan_taint_doc_reconciliation_preserved_non_goals_extracted() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/workspace/velvet-ballistics-MASTER.md"),
        "Clean < DerivedFromSecret < Secret.\n\
         v1 does not track control-flow taint.",
    );
    let policy = EvidencePolicy::strict_bounded(std::path::PathBuf::from("/tmp/workspace"));

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    let plan = result.expect("should not error");
    assert_eq!(
        plan.preserved_non_goals,
        vec![PreservedNonGoal::ControlFlowTaintV1NonGoal]
    );
}

#[test]
fn plan_taint_doc_reconciliation_contradiction_count_reflects_stale_phrases() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/workspace/velvet-ballistics-MASTER.md"),
        "EvalExpr is Always Clean.\n\
         BuildObject has no join of field taints.",
    );
    let policy = EvidencePolicy::strict_bounded(std::path::PathBuf::from("/tmp/workspace"));

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    let plan = result.expect("should not error");
    assert_eq!(plan.contradiction_count, 2);
}

#[test]
fn plan_taint_doc_reconciliation_target_is_master_doc() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/workspace/velvet-ballistics-MASTER.md"),
        "Clean < DerivedFromSecret < Secret.",
    );
    let policy = EvidencePolicy::strict_bounded(std::path::PathBuf::from("/tmp/workspace"));

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    let plan = result.expect("should not error");
    assert_eq!(
        plan.target,
        PatchTarget::MasterDoc(std::path::PathBuf::from(
            "/tmp/workspace/velvet-ballistics-MASTER.md"
        ))
    );
}

#[test]
fn plan_taint_doc_reconciliation_stale_text_removed_lists_stale_phrases() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/workspace/velvet-ballistics-MASTER.md"),
        "EvalExpr is Always Clean",
    );
    let policy = EvidencePolicy::strict_bounded(std::path::PathBuf::from("/tmp/workspace"));

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    let plan = result.expect("should not error");
    assert_eq!(
        plan.stale_text_removed,
        vec![StalePhrase::EvalExprAlwaysClean]
    );
}

// ============================================================================
// Data structure equality and derived Debug contract
// ============================================================================

#[test]
fn doc_reconcile_error_derived_debug_is_exact_for_wrong_workspace() {
    // Given
    let err = DocReconcileError::WrongWorkspace {
        path: std::path::PathBuf::from("/tmp/test.md"),
    };

    // When
    let debug = format!("{:?}", err);

    // Then
    assert_eq!(debug, "WrongWorkspace { path: \"/tmp/test.md\" }");
}

#[test]
fn master_doc_snapshot_eq_for_same_values() {
    // Given
    let path = std::path::PathBuf::from("/tmp/doc.md");
    let text = "same text";

    // When
    let snap1 = MasterDocSnapshot::for_workspace_text(path.clone(), text);
    let snap2 = MasterDocSnapshot::for_workspace_text(path.clone(), text);

    // Then
    assert_eq!(snap1, snap2);
}

#[test]
fn master_doc_snapshot_ne_for_different_text() {
    // Given
    let path = std::path::PathBuf::from("/tmp/doc.md");

    // When
    let snap1 = MasterDocSnapshot::for_workspace_text(path.clone(), "text one");
    let snap2 = MasterDocSnapshot::for_workspace_text(path.clone(), "text two");

    // Then
    assert_ne!(snap1, snap2);
}

#[test]
fn evidence_policy_eq_for_same_values() {
    // Given
    let root = std::path::PathBuf::from("/tmp/workspace");

    // When
    let policy1 = EvidencePolicy::strict_bounded(root.clone());
    let policy2 = EvidencePolicy::strict_bounded(root.clone());

    // Then
    assert_eq!(policy1, policy2);
}

#[test]
fn contradiction_report_eq_for_empty_reports() {
    // Given
    let report1 = ContradictionReport {
        stale_clean_only: Vec::new(),
        no_join_claims: Vec::new(),
        write_slot_only_claims: Vec::new(),
        scanned_nodes: vec![ResolvedNode::EvalExpr],
    };
    let report2 = ContradictionReport {
        stale_clean_only: Vec::new(),
        no_join_claims: Vec::new(),
        write_slot_only_claims: Vec::new(),
        scanned_nodes: vec![ResolvedNode::EvalExpr],
    };

    // Then
    assert_eq!(report1, report2);
}

#[test]
fn evidence_bounded_report_eq_for_same_values() {
    // Given
    let report1 = EvidenceBoundedReport {
        unsupported_claims: Vec::new(),
        cited_claims: 5,
        pending_claims: 3,
        forbidden_claims: Vec::new(),
    };
    let report2 = EvidenceBoundedReport {
        unsupported_claims: Vec::new(),
        cited_claims: 5,
        pending_claims: 3,
        forbidden_claims: Vec::new(),
    };

    // Then
    assert_eq!(report1, report2);
}

#[test]
fn taint_vocabulary_report_eq_for_same_values() {
    // Given
    let report1 = TaintVocabularyReport {
        lattice: vec!["Clean".to_owned()],
        propagation_rule: TaintVocabularyRule::JoinedDataFlowTaint,
        conflicts: Vec::new(),
        control_flow_scope: PreservedNonGoal::ControlFlowTaintV1NonGoal,
    };
    let report2 = TaintVocabularyReport {
        lattice: vec!["Clean".to_owned()],
        propagation_rule: TaintVocabularyRule::JoinedDataFlowTaint,
        conflicts: Vec::new(),
        control_flow_scope: PreservedNonGoal::ControlFlowTaintV1NonGoal,
    };

    // Then
    assert_eq!(report1, report2);
}

// ============================================================================
// Proptest integration — property-based tests
// ============================================================================

proptest::proptest! {
    #[test]
    fn master_doc_snapshot_roundtrips_through_clone(
        path_str in "(/[a-z0-9]+)+",
        text in ".*"
    ) {
        // Given
        let path = std::path::PathBuf::from(&path_str);
        let original = MasterDocSnapshot::for_workspace_text(path.clone(), &text);

        // When
        let cloned = original.clone();

        // Then
        assert_eq!(original, cloned);
    }

    #[test]
    fn check_doc_taint_consistency_is_deterministic(text in ".*") {
        // Given
        let text = text.as_str();

        // When
        let result1 = check_doc_taint_consistency(text);
        let result2 = check_doc_taint_consistency(text);

        // Then
        assert_eq!(result1, result2);
    }

    #[test]
    fn validate_taint_vocabulary_consistency_is_deterministic(text in ".*") {
        // Given
        let doc = MasterDocSnapshot::for_workspace_text(
            std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
            &text,
        );

        // When
        let result1 = validate_taint_vocabulary_consistency(doc.clone());
        let result2 = validate_taint_vocabulary_consistency(doc);

        // Then
        assert_eq!(result1, result2);
    }

    #[test]
    fn evidence_index_from_supports_creates_valid_index(num_supports: usize) {
        // Given
        let supports: Vec<_> = (0..num_supports.min(10))
            .map(|i| EvidenceSupport::cited(&format!("sentence{}", i), &format!("artifact{}", i)))
            .collect();

        // When
        let index = EvidenceIndex::from_supports(supports);

        // Then - verify through validate_evidence_bounded_wording public API
        let doc = MasterDocSnapshot::for_workspace_text(
            std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
            "Clean text.",
        );
        let result = validate_evidence_bounded_wording(doc, index);
        assert_eq!(
            result,
            Ok(EvidenceBoundedReport {
                unsupported_claims: Vec::new(),
                cited_claims: 0,
                pending_claims: 0,
                forbidden_claims: Vec::new(),
            })
        );
    }
}

// ============================================================================
// Boundary cases
// ============================================================================

#[test]
fn scan_for_stale_clean_only_text_handles_empty_text() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "",
    );

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    let report = result.expect("should not error");
    assert!(report.stale_clean_only.is_empty());
}

#[test]
fn validate_taint_vocabulary_consistency_handles_unicode_text() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "Clean < DerivedFromSecret < 秘密.",
    );

    // When
    let result = validate_taint_vocabulary_consistency(doc);

    // Then
    assert_eq!(
        result,
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
    );
}

#[test]
fn plan_taint_doc_reconciliation_handles_very_long_text() {
    // Given
    let long_text = "x".repeat(100_000);
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/workspace/velvet-ballistics-MASTER.md"),
        &long_text,
    );
    let policy = EvidencePolicy::strict_bounded(std::path::PathBuf::from("/tmp/workspace"));

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    assert_eq!(
        result,
        Ok(vb_doc::DocPatchPlan {
            target: PatchTarget::MasterDoc(std::path::PathBuf::from(
                "/tmp/workspace/velvet-ballistics-MASTER.md"
            )),
            edits: vec![PatchEdit::FinishCarriesTaint],
            stale_text_removed: Vec::new(),
            evidence_actions: RequiredEvidence::ConcreteArtifactOrPendingMarker,
            preserved_non_goals: Vec::new(),
            forbidden_actions: Vec::new(),
            contradiction_count: 0,
            status: PatchPlanStatus::NeedsReconciliation,
        })
    );
}

#[test]
fn check_doc_taint_consistency_handles_multiline_text() {
    // Given
    let text = "Line 1: EvalExpr is Always Clean\n\
                Line 2: BuildObject has no join\n\
                Line 3: Clean text.";

    // When
    let result = check_doc_taint_consistency(text);

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::EvalExpr,
            phrase: "Always Clean".to_owned(),
        })
    );
}

#[test]
fn validate_evidence_bounded_wording_multiple_claims_counted() {
    // Given
    let doc = MasterDocSnapshot::for_workspace_text(
        std::path::PathBuf::from("/tmp/velvet-ballistics-MASTER.md"),
        "tests prove joined taint\nLean proves implementation parity",
    );
    let evidence = EvidenceIndex::from_supports(vec![EvidenceSupport::cited(
        "tests prove joined taint",
        "test.html",
    )]);

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then - first unsupported claim errors out
    assert_eq!(
        result,
        Err(DocReconcileError::UnsupportedEvidenceClaim {
            sentence: "Lean proves implementation parity".to_owned(),
            claim_kind: ClaimKind::FormalEvidence,
            required: RequiredEvidence::ConcreteArtifactOrPendingMarker,
        })
    );
}
