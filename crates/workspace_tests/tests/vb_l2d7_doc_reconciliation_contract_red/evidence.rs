use crate::support::*;

#[test]
fn validate_evidence_bounded_wording_returns_report_when_all_claims_are_cited_or_pending() {
    // Given
    let doc = snapshot_with(
        "DRIFT-1 joined taint is resolved [formal-verification-report.md].\n\
         Runtime rejection claim remains pending/unverified.",
    );
    let evidence = bounded_evidence();
    let expected = EvidenceBoundedReport {
        unsupported_claims: Vec::new(),
        cited_claims: 1,
        pending_claims: 1,
        forbidden_claims: Vec::new(),
    };

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then
    assert_eq!(result, Ok(expected));
}

#[test]
fn validate_evidence_bounded_wording_reports_uncited_test_claim() {
    // Given
    let sentence = "tests prove joined taint";
    let doc = snapshot_with(sentence);

    // When
    let result = validate_evidence_bounded_wording(doc, EvidenceIndex::empty());

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::UnsupportedEvidenceClaim {
            sentence: sentence.to_owned(),
            claim_kind: ClaimKind::TestEvidence,
            required: RequiredEvidence::ConcreteArtifactOrPendingMarker,
        })
    );
}

#[test]
fn validate_evidence_bounded_wording_reports_uncited_formal_claim() {
    // Given
    let sentence = "Lean proves implementation parity";
    let doc = snapshot_with(sentence);

    // When
    let result = validate_evidence_bounded_wording(doc, EvidenceIndex::empty());

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::UnsupportedEvidenceClaim {
            sentence: sentence.to_owned(),
            claim_kind: ClaimKind::FormalEvidence,
            required: RequiredEvidence::ConcreteArtifactOrPendingMarker,
        })
    );
}

#[test]
fn validate_evidence_bounded_wording_reports_uncited_release_claim() {
    // Given
    let sentence = "DRIFT-1 is release ready";
    let doc = snapshot_with(sentence);

    // When
    let result = validate_evidence_bounded_wording(doc, EvidenceIndex::empty());

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::UnsupportedEvidenceClaim {
            sentence: sentence.to_owned(),
            claim_kind: ClaimKind::ReleaseReadiness,
            required: RequiredEvidence::ConcreteArtifactOrPendingMarker,
        })
    );
}

#[test]
fn validate_evidence_bounded_wording_reports_uncited_generated_parity_claim() {
    // Given
    let sentence = "full generated Rust and IR parity is verified";
    let doc = snapshot_with(sentence);

    // When
    let result = validate_evidence_bounded_wording(doc, EvidenceIndex::empty());

    // Then
    assert_eq!(
        result,
        Err(DocReconcileError::UnsupportedEvidenceClaim {
            sentence: sentence.to_owned(),
            claim_kind: ClaimKind::GeneratedParity,
            required: RequiredEvidence::ConcreteArtifactOrPendingMarker,
        })
    );
}

#[test]
fn evidence_index_empty_leaves_supported_report_counts_at_zero() {
    // Given
    let doc = snapshot_with("Plain wording without evidence claims.");

    // When
    let result = validate_evidence_bounded_wording(doc, EvidenceIndex::empty());

    // Then
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
fn evidence_support_cited_satisfies_exact_sentence_with_artifact() {
    // Given
    let sentence = "tests prove joined taint";
    let artifact = "nextest-report.txt";
    let doc = snapshot_with("tests prove joined taint [nextest-report.txt]");
    let evidence = EvidenceIndex::from_supports(vec![EvidenceSupport::cited(sentence, artifact)]);

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then
    assert_eq!(
        result,
        Ok(EvidenceBoundedReport {
            unsupported_claims: Vec::new(),
            cited_claims: 1,
            pending_claims: 0,
            forbidden_claims: Vec::new(),
        })
    );
}

#[test]
fn evidence_support_cited_does_not_count_without_artifact_reference() {
    // Given
    let doc = snapshot_with("Plain implementation note.");
    let evidence = EvidenceIndex::from_supports(vec![EvidenceSupport::cited(
        "Plain implementation note.",
        "missing-artifact.txt",
    )]);

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then
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
fn evidence_support_pending_counts_pending_wording() {
    // Given
    let doc = snapshot_with("Runtime rejection claim remains pending.");
    let evidence = EvidenceIndex::from_supports(vec![EvidenceSupport::pending(
        "Runtime rejection claim remains pending.",
    )]);

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then
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

#[test]
fn evidence_support_pending_counts_unverified_wording() {
    // Given
    let doc = snapshot_with("Runtime rejection claim remains unverified.");
    let evidence = EvidenceIndex::from_supports(vec![EvidenceSupport::pending(
        "Runtime rejection claim remains unverified.",
    )]);

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then
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

#[test]
fn validate_evidence_bounded_wording_accepts_cited_generated_parity_claim() {
    // Given
    let sentence = "DRIFT-1 generated Rust and IR parity is verified";
    let artifact = "formal-verification-report.md";
    let doc = snapshot_with(
        "DRIFT-1 generated Rust and IR parity is verified [formal-verification-report.md]",
    );
    let evidence = EvidenceIndex::from_supports(vec![EvidenceSupport::cited(sentence, artifact)]);

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then
    assert_eq!(
        result,
        Ok(EvidenceBoundedReport {
            unsupported_claims: Vec::new(),
            cited_claims: 1,
            pending_claims: 0,
            forbidden_claims: Vec::new(),
        })
    );
}

#[test]
fn validate_evidence_bounded_wording_accepts_pending_release_claim() {
    // Given
    let sentence = "DRIFT-1 is release ready";
    let doc = snapshot_with("DRIFT-1 is release ready pending final verification.");
    let evidence = EvidenceIndex::from_supports(vec![EvidenceSupport::pending(sentence)]);

    // When
    let result = validate_evidence_bounded_wording(doc, evidence);

    // Then
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
