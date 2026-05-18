//! Integration tests for vb_doc + vb_boundary_inventory cross-crate scenarios.
//!
//! These tests exercise the integration between vb_doc (document reconciliation)
//! and vb_boundary_inventory (boundary inventory management) to ensure they
//! work correctly together.

use vb_boundary_inventory::boundary_inventory::{
    BoundaryClass, BoundaryRecord, BoundaryRecordDraft, BoundaryRecordParts, ClassifiedBoundaryInput,
    EvidenceKind, EvidenceReference, FieldState, FreshnessMarker, Owner, ReviewStatus,
    ThreatStatement, WorkspaceRoot,
};
use vb_doc::evidence::{EvidenceIndex, EvidenceSupport};
use vb_doc::reconcile::{
    check_doc_taint_consistency, plan_taint_doc_reconciliation, scan_for_stale_clean_only_text,
    validate_evidence_bounded_wording, validate_taint_vocabulary_consistency,
};
use vb_doc::{
    ClaimKind, ConflictKind, DocReconcileError, EvidencePolicy, MasterDocSnapshot,
    PatchPlanStatus, RequiredEvidence,
};

/// Helper to create a workspace root path.
fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from("/tmp/vb_integration_workspace")
}

/// Helper to create a master doc path inside the workspace.
fn master_doc_path() -> std::path::PathBuf {
    workspace_root().join("velvet-ballistics-MASTER.md")
}

/// Helper to create a master doc snapshot.
fn master_doc(text: &str) -> MasterDocSnapshot {
    MasterDocSnapshot::for_workspace_text(master_doc_path(), text)
}

/// Helper to create a strict evidence policy.
fn strict_policy() -> EvidencePolicy {
    EvidencePolicy::strict_bounded(workspace_root())
}

// ============================================================================
// Integration: vb_doc reconciliation with vb_boundary_inventory evidence
// ============================================================================

#[test]
fn doc_reconciliation_with_boundary_evidence_valid_lattice() {
    // Given - doc with valid taint lattice AND Finish taint
    let doc = master_doc(
        "Clean < DerivedFromSecret < Secret.\n\
         v1 does not track control-flow taint.\n\
         Finish emits EngineSignal::Finished(SlotValue, Taint).",
    );
    let policy = strict_policy();

    // When - run reconciliation
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then - should succeed
    let plan = result.expect("reconciliation should succeed");
    assert_eq!(plan.status, PatchPlanStatus::AlreadyConsistent);
}

#[test]
fn doc_reconciliation_with_boundary_evidence_vocabulary_conflict() {
    // Given - doc with vocabulary conflict
    let doc = master_doc("Clean < Secret < DerivedFromSecret");
    let policy = strict_policy();

    // When - run reconciliation
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then - should detect vocabulary conflict
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
fn doc_reconciliation_detects_stale_phrases() {
    // Given - doc with stale phrase
    let doc = master_doc("EvalExpr No taint join");
    let policy = strict_policy();

    // When - run reconciliation
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then - should need reconciliation
    let plan = result.expect("reconciliation should succeed");
    assert_eq!(plan.status, PatchPlanStatus::NeedsReconciliation);
    assert!(!plan.edits.is_empty());
}

#[test]
fn doc_reconciliation_preserves_non_goals() {
    // Given - doc with non-goal statement
    let doc = master_doc(
        "Clean < DerivedFromSecret < Secret.\n\
         v1 does not track control-flow taint.",
    );
    let policy = strict_policy();

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    let plan = result.expect("should succeed");
    assert!(plan
        .preserved_non_goals
        .contains(&vb_doc::PreservedNonGoal::ControlFlowTaintV1NonGoal));
}

// ============================================================================
// Integration: vb_doc evidence with vb_boundary_inventory evidence requirements
// ============================================================================

#[test]
fn evidence_bounded_wording_with_boundary_evidence_support() {
    // Given - doc with test evidence claim and supporting evidence
    let doc = master_doc(
        "Clean < DerivedFromSecret < Secret.\n\
         tests prove joined taint test-report.html",
    );
    let evidence = EvidenceIndex::from_supports(vec![
        EvidenceSupport::cited("tests prove joined taint", "test-report.html"),
    ]);

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then
    let report = result.expect("should not error");
    assert_eq!(report.cited_claims, 1);
}

#[test]
fn evidence_bounded_wording_rejects_unsupported_claim() {
    // Given - doc with claim but no supporting evidence
    let doc = master_doc("tests prove joined taint");
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
fn evidence_bounded_wording_counts_pending_correctly() {
    // Given
    let doc = master_doc("Clean < DerivedFromSecret < Secret.\nruntime claim remains unverified");
    let evidence = EvidenceIndex::from_supports(vec![
        EvidenceSupport::pending("runtime claim"),
    ]);

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then
    let report = result.expect("should not error");
    assert_eq!(report.pending_claims, 1);
}

// ============================================================================
// Integration: scan_for_stale_clean_only_text with boundary inventory context
// ============================================================================

#[test]
fn scan_detects_eval_expr_stale_phrase() {
    // Given
    let doc = master_doc("EvalExpr is Always Clean");

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    let err = result.expect_err("should detect stale phrase");
    assert_eq!(
        err,
        DocReconcileError::StaleCleanOnlyTaintText {
            node: vb_doc::ResolvedNode::EvalExpr,
            phrase: "Always Clean".to_owned(),
        }
    );
}

#[test]
fn scan_detects_build_object_stale_phrase() {
    // Given
    let doc = master_doc("BuildObject has no join of field taints");

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    let err = result.expect_err("should detect stale phrase");
    assert_eq!(
        err,
        DocReconcileError::StaleCleanOnlyTaintText {
            node: vb_doc::ResolvedNode::BuildObject,
            phrase: "no join of field taints".to_owned(),
        }
    );
}

#[test]
fn scan_detects_build_list_stale_phrase() {
    // Given
    let doc = master_doc("BuildList has no join of item taints");

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    let err = result.expect_err("should detect stale phrase");
    assert_eq!(
        err,
        DocReconcileError::StaleCleanOnlyTaintText {
            node: vb_doc::ResolvedNode::BuildList,
            phrase: "no join of item taints".to_owned(),
        }
    );
}

#[test]
fn scan_detects_finish_without_taint() {
    // Given
    let doc = master_doc("Finish emits Finished(SlotValue)");

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    let err = result.expect_err("should detect stale phrase");
    assert_eq!(
        err,
        DocReconcileError::StaleCleanOnlyTaintText {
            node: vb_doc::ResolvedNode::Finish,
            phrase: "Finished(SlotValue)".to_owned(),
        }
    );
}

#[test]
fn scan_accepts_clean_document() {
    // Given
    let doc = master_doc("Clean data flow with no stale phrases.");

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    let report = result.expect("should not detect false positive");
    assert!(report.stale_clean_only.is_empty());
}

// ============================================================================
// Integration: check_doc_taint_consistency wrapper
// ============================================================================

#[test]
fn check_taint_consistency_returns_report_for_clean_text() {
    // Given
    let text = "Clean text without any stale phrases.";

    // When
    let result = check_doc_taint_consistency(text);

    // Then
    let report = result.expect("should not error");
    assert!(report.stale_clean_only.is_empty());
}

#[test]
fn check_taint_consistency_detects_stale_phrases() {
    // Given
    let text = "EvalExpr No taint join";

    // When
    let result = check_doc_taint_consistency(text);

    // Then
    assert!(matches!(result, Err(DocReconcileError::StaleCleanOnlyTaintText { .. })));
}

// ============================================================================
// Integration: validate_taint_vocabulary_consistency with vb_boundary_inventory
// ============================================================================

#[test]
fn validate_vocabulary_returns_report_for_valid_lattice() {
    // Given
    let doc = master_doc(
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
        vb_doc::TaintVocabularyRule::JoinedDataFlowTaint
    );
    assert!(report.conflicts.is_empty());
}

#[test]
fn validate_vocabulary_rejects_control_flow_conflation() {
    // Given
    let doc = master_doc("v1 tracks branch-condition taint");

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
fn validate_vocabulary_rejects_unknown_term() {
    // Given
    let doc = master_doc("Private taint dominates Secret");

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
fn validate_vocabulary_rejects_downgrade() {
    // Given
    let doc = master_doc("Secret downgrades to Clean after BuildList");

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

// ============================================================================
// Integration: wrong workspace detection
// ============================================================================

#[test]
fn reconciliation_rejects_wrong_workspace() {
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

// ============================================================================
// Integration: plan_taint_doc_reconciliation full workflow
// ============================================================================

#[test]
fn plan_reconciliation_includes_all_edit_types() {
    // Given - doc with multiple stale phrases
    let doc = master_doc(
        "EvalExpr No taint join\n\
         BuildObject has no join of field taints\n\
         BuildList has no join of item taints\n\
         Finish emits Finished(SlotValue)",
    );
    let policy = strict_policy();

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    let plan = result.expect("should not error");
    assert!(plan.edits.contains(&vb_doc::PatchEdit::EvalExprJoin));
    assert!(plan.edits.contains(&vb_doc::PatchEdit::BuildObjectJoin));
    assert!(plan.edits.contains(&vb_doc::PatchEdit::BuildListJoin));
    assert!(plan.edits.contains(&vb_doc::PatchEdit::FinishCarriesTaint));
}

#[test]
fn plan_reconciliation_counts_contradictions_correctly() {
    // Given
    let doc = master_doc(
        "EvalExpr is Always Clean.\n\
         BuildObject has no join of field taints.",
    );
    let policy = strict_policy();

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    let plan = result.expect("should not error");
    assert_eq!(plan.contradiction_count, 2);
}

#[test]
fn plan_reconciliation_target_is_master_doc() {
    // Given
    let doc = master_doc("Clean < DerivedFromSecret < Secret.");
    let policy = strict_policy();

    // When
    let result = plan_taint_doc_reconciliation(doc, policy);

    // Then
    let plan = result.expect("should not error");
    assert_eq!(
        plan.target,
        vb_doc::PatchTarget::MasterDoc(master_doc_path())
    );
}

// ============================================================================
// Boundary inventory integration - smoke tests for vb_boundary_inventory types
// ============================================================================

#[test]
fn boundary_record_draft_construction() {
    // Given - parts for a boundary record
    let parts = BoundaryRecordParts {
        id: "test-boundary-1".into(),
        class: BoundaryClass::Ffi,
        source_path: std::path::PathBuf::from("/src/ffi.rs"),
        owner: FieldState::Present(Owner("test-owner".into())),
        threat: FieldState::Present(ThreatStatement("test threat".into())),
        evidence: FieldState::Present(EvidenceReference::repo_local(
            std::path::PathBuf::from("/evidence/fuzz.html"),
            EvidenceKind::Fuzz,
        )),
        freshness: FreshnessMarker::new(1, 1, 1),
        review_status: FieldState::Present(ReviewStatus::Approved),
        waiver: FieldState::Missing,
    };

    // When
    let record: BoundaryRecord = BoundaryRecordDraft::new(parts);

    // Then - should be constructable
    assert_eq!(record.id, "test-boundary-1");
    assert!(matches!(record.review_status, FieldState::Present(ReviewStatus::Approved)));
}

#[test]
fn boundary_record_review_status_query() {
    // Given
    let parts = BoundaryRecordParts {
        id: "test-boundary-1".into(),
        class: BoundaryClass::Ipc,
        source_path: std::path::PathBuf::from("/src/ipc.rs"),
        owner: FieldState::Present(Owner("test-owner".into())),
        threat: FieldState::Present(ThreatStatement("test threat".into())),
        evidence: FieldState::Present(EvidenceReference::repo_local(
            std::path::PathBuf::from("/evidence/isolation.html"),
            EvidenceKind::Isolation,
        )),
        freshness: FreshnessMarker::new(1, 1, 1),
        review_status: FieldState::Present(ReviewStatus::Approved),
        waiver: FieldState::Missing,
    };
    let record: BoundaryRecord = BoundaryRecordDraft::new(parts);

    // When
    let status = record.review_status();

    // Then
    assert_eq!(status, Some("approved"));
}

#[test]
fn evidence_reference_free_text() {
    // Given
    let reference = EvidenceReference::free_text("manual inspection completed 2024-01-01");

    // Then
    assert!(matches!(reference, EvidenceReference::FreeText(_)));
}

#[test]
fn evidence_kind_variants() {
    // Given
    let fuzz = EvidenceKind::Fuzz;
    let isolation = EvidenceKind::Isolation;
    let manual_qa = EvidenceKind::ManualQa;
    let provenance = EvidenceKind::Provenance;

    // Then
    assert!(matches!(fuzz, EvidenceKind::Fuzz));
    assert!(matches!(isolation, EvidenceKind::Isolation));
    assert!(matches!(manual_qa, EvidenceKind::ManualQa));
    assert!(matches!(provenance, EvidenceKind::Provenance));
}

#[test]
fn boundary_class_variants() {
    // Given
    let cabi = BoundaryClass::CAbi;
    let ffi = BoundaryClass::Ffi;
    let ipc = BoundaryClass::Ipc;
    let external_binary = BoundaryClass::ExternalBinary;
    let decoder = BoundaryClass::Decoder;
    let generated_code = BoundaryClass::GeneratedCode;
    let unsafe_adjacent = BoundaryClass::UnsafeAdjacentDependency;
    let unknown = BoundaryClass::Unknown;

    // Then
    assert!(matches!(cabi, BoundaryClass::CAbi));
    assert!(matches!(ffi, BoundaryClass::Ffi));
    assert!(matches!(ipc, BoundaryClass::Ipc));
    assert!(matches!(external_binary, BoundaryClass::ExternalBinary));
    assert!(matches!(decoder, BoundaryClass::Decoder));
    assert!(matches!(generated_code, BoundaryClass::GeneratedCode));
    assert!(matches!(unsafe_adjacent, BoundaryClass::UnsafeAdjacentDependency));
    assert!(matches!(unknown, BoundaryClass::Unknown));
}

#[test]
fn freshness_marker_construction() {
    // Given
    let marker = FreshnessMarker::new(1, 2, 3);

    // Then - should be constructable (internal fields are pub(crate))
    // We can verify through Debug format
    assert!(format!("{:?}", marker).contains("1"));
    assert!(format!("{:?}", marker).contains("2"));
    assert!(format!("{:?}", marker).contains("3"));
}

#[test]
fn field_state_present_and_missing() {
    // Given
    let present: FieldState<i32> = FieldState::Present(42);
    let missing: FieldState<i32> = FieldState::Missing;

    // Then
    assert!(matches!(present, FieldState::Present(42)));
    assert!(matches!(missing, FieldState::Missing));
}

#[test]
fn workspace_root_construction() {
    // Given
    let root = WorkspaceRoot::new(std::path::PathBuf::from("/tmp/workspace"));

    // Then - should be constructable
    assert!(format!("{:?}", root).contains("workspace"));
}

#[test]
fn owner_and_threat_statement() {
    // Given
    let owner = Owner("security-team".into());
    let threat = ThreatStatement("data exfiltration risk".into());

    // Then
    assert_eq!(owner.0, "security-team");
    assert_eq!(threat.0, "data exfiltration risk");
}

// ============================================================================
// Cross-crate data flow: vb_doc evidence + vb_boundary_inventory evidence
// ============================================================================

#[test]
fn doc_evidence_policy_with_boundary_inventory_evidence() {
    // Given - vb_doc evidence policy
    let policy = EvidencePolicy::strict_bounded(workspace_root());

    // Then - should be compatible with RequiredEvidence
    assert!(matches!(
        policy.required,
        RequiredEvidence::ConcreteArtifactOrPendingMarker
    ));
}

#[test]
fn scan_nodes_returns_all_expected_nodes() {
    // Given
    let doc = master_doc("Clean text");

    // When
    let result = scan_for_stale_clean_only_text(doc);

    // Then
    let report = result.expect("should not error");
    assert!(report.scanned_nodes.contains(&vb_doc::ResolvedNode::EvalExpr));
    assert!(report.scanned_nodes.contains(&vb_doc::ResolvedNode::BuildObject));
    assert!(report.scanned_nodes.contains(&vb_doc::ResolvedNode::BuildList));
    assert!(report.scanned_nodes.contains(&vb_doc::ResolvedNode::Finish));
}

#[test]
fn classified_boundary_construction() {
    // Given
    let input = ClassifiedBoundaryInput {
        id: "boundary-1".into(),
        class: BoundaryClass::Ffi,
        source_path: std::path::PathBuf::from("/src/ffi.rs"),
        exposure: vb_boundary_inventory::boundary_inventory::BoundaryExposure::none(),
    };

    // When
    let classified = vb_boundary_inventory::boundary_inventory::ClassifiedBoundary::new(input);

    // Then
    assert_eq!(classified.id, "boundary-1");
    assert!(matches!(classified.class, BoundaryClass::Ffi));
}
