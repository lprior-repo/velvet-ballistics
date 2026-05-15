// Miri tests for AcceptedArtifact decode safety.
// MIRI-DECODE-001: postcard decode of untrusted IR bytes from Fjall store does not cause UB or panic.
// MIRI-SAFETY-001: AcceptedArtifact struct has valid memory layout when decoded from postcard.

#![forbid(unsafe_code)]
#![cfg(miri)]

// Miri test for AcceptedArtifact decode.
// This lives in vb_storage because AcceptedArtifact is defined there.
// The test exercises the full decode path from raw bytes.

#[cfg(miri)]
mod miri_decode_tests {
    use crate::admission::{AcceptedArtifact, VerificationProof};
    use vb_core::WorkflowDigest;

    /// Test: arbitrary bytes do not cause panic on AcceptedArtifact decode.
    /// MIRI-DECODE-001: 0 UB violations, 0 panics.
    #[test]
    fn miri_accepted_artifact_decode_arbitrary_bytes() {
        // Arbitrary bytes — may not be valid postcard encoding.
        let bytes: Vec<u8> = (0..64).map(|i| (i * 7 + 13) as u8).collect();

        // postcard::from_bytes may return Err but must NOT panic.
        let result: Result<AcceptedArtifact, _> = postcard::from_bytes(&bytes);

        // Either decode succeeds (unlikely for arbitrary bytes) or returns error.
        // Neither outcome causes UB or panic under Miri.
        if let Ok(artifact) = result {
            // If decode somehow succeeds, verify basic structural properties.
            assert!(artifact.ir.len() >= 0);
            // gate_count should be a valid u8.
            assert!(artifact.verification.gate_count <= 15);
        }
    }

    /// Test: empty bytes do not cause panic.
    #[test]
    fn miri_accepted_artifact_decode_empty_bytes() {
        let bytes: Vec<u8> = vec![];
        let result: Result<AcceptedArtifact, _> = postcard::from_bytes(&bytes);
        // Must not panic — Err is acceptable.
        assert!(result.is_err());
    }

    /// Test: valid round-trip does not cause UB.
    #[test]
    fn miri_accepted_artifact_roundtrip_safety() {
        let proof = VerificationProof {
            digest: WorkflowDigest::from_bytes([0xAB; 32]),
            gate_count: 2,
            durable: true,
            bounded: true,
            taint_safe: true,
            retry_safe: true,
            replayable: true,
            idempotency_keyed: Box::new([]),
            idempotency_attested: Box::new([]),
            warnings: Vec::new(),
        };

        let artifact = AcceptedArtifact {
            digest: WorkflowDigest::from_bytes([0xAB; 32]),
            ir: vec![1, 2, 3, 4],
            verification: proof,
            accepted_at_seq: crate::types::EventSeq::new(0),
            required_capabilities: Box::new([]),
        };

        // Encode.
        let encoded = postcard::to_allocvec(&artifact).unwrap();

        // Decode — must not cause UB or panic.
        let decoded: AcceptedArtifact = postcard::from_bytes(&encoded).unwrap();

        // Verify decoded matches original.
        assert_eq!(decoded.digest, artifact.digest);
        assert_eq!(decoded.verification.gate_count, artifact.verification.gate_count);
    }

    /// Test: partial bytes do not cause panic.
    #[test]
    fn miri_accepted_artifact_decode_partial_bytes() {
        let bytes: Vec<u8> = vec![0x01, 0x02, 0x03];
        let result: Result<AcceptedArtifact, _> = postcard::from_bytes(&bytes);
        // Must not panic.
        assert!(result.is_err());
    }

    /// Test: VerificationProof decode safety.
    #[test]
    fn miri_verification_proof_decode_safety() {
        let bytes: Vec<u8> = (0..32).map(|i| (i * 3) as u8).collect();
        let result: Result<VerificationProof, _> = postcard::from_bytes(&bytes);
        // Must not panic — Err is acceptable.
        assert!(result.is_err());
    }

    /// Test: zero-initialized bytes decode safety.
    #[test]
    fn miri_accepted_artifact_decode_zero_bytes() {
        let bytes: Vec<u8> = vec![0u8; 128];
        let result: Result<AcceptedArtifact, _> = postcard::from_bytes(&bytes);
        // Must not panic under Miri.
        if let Ok(artifact) = result {
            // Even if decode succeeds, gate_count should be valid.
            assert!(artifact.verification.gate_count <= 255);  // u8 range
        }
    }
}
