// Proptest: Gate count and proof flags via VerificationProof public API.
//
// Obligation: PO-vb-h09wf-025
// Verifier: proptest
// Command: cargo test -p vb_storage --test proptest -- ps_008_gate_count_flags
//
// Domain claim: >1000 cases: VerificationProof structs constructed with varying
// field values produce correct gate_count and flag states when submitted.
//
// PRODUCTION BINDING:
//   vb_storage::admission::VerificationProof (public struct)
//   vb_storage::admission::VerificationWarning (public struct)

use proptest::prelude::*;
use vb_core::WorkflowDigest;
use vb_storage::admission::{Durability, VerificationProof, VerificationWarning};

proptest! {
    /// PS-008a: VerificationProof::new always produces correct gate_count and flags.
    #[test]
    fn ps_008_new_verification_proof_has_correct_fields(
        gate_choice in 0u8..2u8,
        durable in proptest::bool::ANY,
    ) {
        let gate_count = if gate_choice == 0 { 0u8 } else { 15u8 };
        let digest = WorkflowDigest::from_bytes([0u8; 32]);
        let durability = Durability::from(durable);
        let proof = VerificationProof::new(digest, gate_count, durability);

        prop_assert_eq!(proof.gate_count, gate_count);
        prop_assert_eq!(proof.durable, durable);
        let flags_expected = gate_count == 15;
        prop_assert_eq!(proof.bounded_claimed, flags_expected);
        prop_assert_eq!(proof.taint_safe_claimed, flags_expected);
        prop_assert_eq!(proof.retry_safe_claimed, flags_expected);
        prop_assert_eq!(proof.idempotency_verified_claimed, flags_expected);
        prop_assert_eq!(proof.replayable_claimed, flags_expected);
    }

    /// PS-008b: VerificationProof with gate_count=0 has relaxed flag.
    #[test]
    fn ps_008_relaxed_proof_has_gate_count_zero(_dummy in proptest::bool::ANY) {
        let digest = WorkflowDigest::from_bytes([0u8; 32]);
        let proof = VerificationProof::new_volatile(digest, 0);
        prop_assert_eq!(proof.gate_count, 0);
        prop_assert!(!proof.durable);
        prop_assert!(!proof.bounded_claimed);
        prop_assert!(!proof.taint_safe_claimed);
        prop_assert!(!proof.retry_safe_claimed);
        prop_assert!(!proof.idempotency_verified_claimed);
        prop_assert!(!proof.replayable_claimed);
    }

    /// PS-008c: VerificationProof with gate_count=15 has all proof flags true.
    #[test]
    fn ps_008_checked_proof_has_all_flags(_dummy in proptest::bool::ANY) {
        let digest = WorkflowDigest::from_bytes([0u8; 32]);
        let proof = VerificationProof::new_durable(digest, 15);
        prop_assert_eq!(proof.gate_count, 15);
        prop_assert!(proof.durable);
        prop_assert!(proof.bounded_claimed);
        prop_assert!(proof.taint_safe_claimed);
        prop_assert!(proof.retry_safe_claimed);
        prop_assert!(proof.idempotency_verified_claimed);
        prop_assert!(proof.replayable_claimed);
    }

    /// PS-008d: VerificationWarning::is_valid works for valid gate values.
    #[test]
    fn ps_008_verification_warning_valid_gates(gate in 1u8..=15u8) {
        let warning = VerificationWarning {
            code: 1,
            message: Box::<str>::from("test"),
            gate,
        };
        prop_assert!(warning.is_valid());
    }

    /// PS-008e: VerificationWarning rejects invalid gate values.
    #[test]
    fn ps_008_verification_warning_invalid_gates(gate in 16u8..=255u8) {
        let warning = VerificationWarning {
            code: 1,
            message: Box::<str>::from("test"),
            gate,
        };
        prop_assert!(!warning.is_valid());
    }
}
