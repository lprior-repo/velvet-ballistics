use proptest::prelude::*;

use crate::support::*;

proptest! {
    #[test]
    fn plan_taint_doc_reconciliation_contract_properties(fragment in ".{0,4096}") {
        // Given
        let doc = snapshot_with(&fragment);

        // When
        let first = plan_taint_doc_reconciliation(doc.clone(), strict_policy());
        let second = plan_taint_doc_reconciliation(doc, strict_policy());

        // Then
        prop_assert_eq!(first, second);
    }

    #[test]
    fn validate_evidence_bounded_wording_claim_combinations(
        (sentence, claim_kind) in prop_oneof![
            Just(("tests prove joined taint".to_owned(), ClaimKind::TestEvidence)),
            Just(("Lean proves implementation parity".to_owned(), ClaimKind::FormalEvidence)),
            Just(("DRIFT-1 generated Rust and IR parity is verified".to_owned(), ClaimKind::GeneratedParity)),
            Just(("DRIFT-1 is release ready".to_owned(), ClaimKind::ReleaseReadiness)),
        ]
    ) {
        // Given
        let doc = snapshot_with(&sentence);
        let evidence = EvidenceIndex::empty();

        // When
        let result = validate_evidence_bounded_wording(doc, evidence);

        // Then
        prop_assert_eq!(
            result,
            Err(DocReconcileError::UnsupportedEvidenceClaim {
                sentence,
                claim_kind,
                required: RequiredEvidence::ConcreteArtifactOrPendingMarker,
            })
        );
    }
}
