use crate::support::*;

#[test]
fn doc_reconcile_error_preserves_out_of_scope_change_fields_when_constructed() {
    // Given
    let result: Result<(), DocReconcileError> = Err(DocReconcileError::OutOfScopeChange {
        change_kind: "runtime semantic change".to_owned(),
        path_or_operation: "vb_runtime::eval_expr".to_owned(),
    });

    // When
    let observed = result;

    // Then
    assert_eq!(
        observed,
        Err(DocReconcileError::OutOfScopeChange {
            change_kind: "runtime semantic change".to_owned(),
            path_or_operation: "vb_runtime::eval_expr".to_owned(),
        })
    );
}

#[test]
fn doc_reconcile_error_preserves_missing_traceability_clause_when_constructed() {
    // Given
    let result: Result<(), DocReconcileError> = Err(DocReconcileError::MissingTraceability {
        clause: "DRIFT-1 joined taint contract".to_owned(),
    });

    // When
    let observed = result;

    // Then
    assert_eq!(
        observed,
        Err(DocReconcileError::MissingTraceability {
            clause: "DRIFT-1 joined taint contract".to_owned(),
        })
    );
}

#[test]
fn master_doc_snapshot_for_workspace_text_preserves_runtime_path() {
    // Given
    let path = master_doc_path();

    // When
    let snapshot = MasterDocSnapshot::for_workspace_text(path.clone(), "contract text");

    // Then
    assert_eq!(snapshot.path, path);
}

#[test]
fn master_doc_snapshot_for_workspace_text_preserves_text_verbatim() {
    // Given
    let path = master_doc_path();

    // When
    let snapshot = MasterDocSnapshot::for_workspace_text(path, "line one\nline two");

    // Then
    assert_eq!(snapshot.text, "line one\nline two".to_owned());
}

#[test]
fn evidence_policy_strict_bounded_preserves_runtime_root() {
    // Given
    let root = workspace_root().join("nested-root");

    // When
    let policy = EvidencePolicy::strict_bounded(root.clone());

    // Then
    assert_eq!(policy.workspace_root, root);
}

#[test]
fn evidence_policy_strict_bounded_requires_concrete_artifact_or_pending_marker() {
    // Given
    let root = workspace_root().join("policy-root");

    // When
    let policy = EvidencePolicy::strict_bounded(root);

    // Then
    assert_eq!(
        policy.required,
        RequiredEvidence::ConcreteArtifactOrPendingMarker
    );
}
