// Test writer agent: vb-lp2v proof-admission acceptance scenarios
// Tests ArtifactEnvelopeError variants and AdmissionError variants via public API.

#[cfg(test)]
mod artifact_envelope_bdd_tests {
    use vb_core::budget::{AggregateResourceBudget, AggregateResourceCapacity};
    use vb_core::capability::CapabilitySet;
    use vb_core::ids::{ActionId, RunId, WorkflowDigest};
    use vb_core::policy::RuntimePolicy;
    use crate::admission::{
        admit_artifact_run, admit_run_with_budget, validate_accepted_artifact_envelope,
        AcceptedArtifactStore, AdmissionError,
        ArtifactEnvelopeError, ArtifactStore, REQUIRED_GATE_COUNT,
    };
    use vb_storage::admission::{AcceptedArtifact, VerificationProof};
    use vb_storage::types::EventSeq;

    /// Helper to build a minimal valid VerificationProof with all flags true.
    fn valid_proof(digest: WorkflowDigest) -> VerificationProof {
        VerificationProof {
            digest,
            gate_count: REQUIRED_GATE_COUNT,
            durable: true,
            bounded: true,
            taint_safe: true,
            retry_safe: true,
            idempotency_verified: true,
            replayable: true,
            idempotency_keyed: Box::new([]),
            idempotency_attested: Box::new([]),
            warnings: Vec::new(),
        }
    }

    /// Helper to build a minimal valid AcceptedArtifact.
    fn valid_artifact(digest: WorkflowDigest) -> AcceptedArtifact {
        AcceptedArtifact {
            digest,
            ir: Vec::new(),
            verification: valid_proof(digest),
            accepted_at_seq: EventSeq::new(0),
            required_capabilities: Box::new([]),
        }
    }

    /// Store that always returns ArtifactNotFound.
    struct NotFoundStore;
    impl AcceptedArtifactStore for NotFoundStore {
        fn load_accepted_artifact(
            &self,
            digest: WorkflowDigest,
        ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
            Err(ArtifactEnvelopeError::ArtifactNotFound { digest })
        }
    }

    /// Store that returns a corrupt (non-postcard) payload.
    struct CorruptBytesStore;
    impl AcceptedArtifactStore for CorruptBytesStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
            // postcard::from_bytes will fail on non-valid bytes
            let corrupt_bytes = vec![0xFF, 0xFE, 0xFD];
            let artifact: Result<AcceptedArtifact, _> = postcard::from_bytes(&corrupt_bytes);
            artifact.map_err(|_| ArtifactEnvelopeError::PostcardDecodeFailed)
        }
    }

    /// Store that returns an artifact with wrong gate count.
    struct WrongGateCountStore {
        gate_count: u8,
    }
    impl WrongGateCountStore {
        fn new(gate_count: u8) -> Self {
            Self { gate_count }
        }
    }
    impl AcceptedArtifactStore for WrongGateCountStore {
        fn load_accepted_artifact(
            &self,
            digest: WorkflowDigest,
        ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
            let mut proof = valid_proof(digest);
            proof.gate_count = self.gate_count;
            let artifact = AcceptedArtifact {
                digest,
                ir: Vec::new(),
                verification: proof,
                accepted_at_seq: EventSeq::new(0),
                required_capabilities: Box::new([]),
            };
            // Validate envelope - this is what StorageArtifactStore does
            validate_accepted_artifact_envelope(&artifact)?;
            Ok(artifact)
        }
    }

    /// Store that returns an artifact with one proof flag set to false.
    struct ProofFlagFalseStore {
        flag: ProofFlagVariant,
    }
    enum ProofFlagVariant {
        Bounded,
        TaintSafe,
        RetrySafe,
        Durable,
        Replayable,
        IdempotencyVerified,
    }
    impl ProofFlagFalseStore {
        fn new(flag: ProofFlagVariant) -> Self {
            Self { flag }
        }
    }
    impl AcceptedArtifactStore for ProofFlagFalseStore {
        fn load_accepted_artifact(
            &self,
            digest: WorkflowDigest,
        ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
            let mut proof = valid_proof(digest);
            match self.flag {
                ProofFlagVariant::Bounded => proof.bounded = false,
                ProofFlagVariant::TaintSafe => proof.taint_safe = false,
                ProofFlagVariant::RetrySafe => proof.retry_safe = false,
                ProofFlagVariant::Durable => proof.durable = false,
                ProofFlagVariant::Replayable => proof.replayable = false,
                ProofFlagVariant::IdempotencyVerified => proof.idempotency_verified = false,
            }
            let artifact = AcceptedArtifact {
                digest,
                ir: Vec::new(),
                verification: proof,
                accepted_at_seq: EventSeq::new(0),
                required_capabilities: Box::new([]),
            };
            // Validate envelope - this is what StorageArtifactStore does
            validate_accepted_artifact_envelope(&artifact)?;
            Ok(artifact)
        }
    }

    /// Store that returns an artifact with keyed action not attested.
    struct MissingAttestationStore {
        keyed: Box<[ActionId]>,
        attested: Box<[ActionId]>,
    }
    impl MissingAttestationStore {
        fn new(keyed: Box<[ActionId]>, attested: Box<[ActionId]>) -> Self {
            Self { keyed, attested }
        }
    }
    impl AcceptedArtifactStore for MissingAttestationStore {
        fn load_accepted_artifact(
            &self,
            digest: WorkflowDigest,
        ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
            let mut proof = valid_proof(digest);
            proof.idempotency_keyed = self.keyed.clone();
            proof.idempotency_attested = self.attested.clone();
            let artifact = AcceptedArtifact {
                digest,
                ir: Vec::new(),
                verification: proof,
                accepted_at_seq: EventSeq::new(0),
                required_capabilities: Box::new([]),
            };
            // Validate envelope - this is what StorageArtifactStore does
            validate_accepted_artifact_envelope(&artifact)?;
            Ok(artifact)
        }
    }

    /// Store that returns an artifact with a different digest inside than requested.
    struct DigestMismatchStore {
        inner_digest: WorkflowDigest,
    }
    impl DigestMismatchStore {
        fn new(inner_digest: WorkflowDigest) -> Self {
            Self { inner_digest }
        }
    }
    impl AcceptedArtifactStore for DigestMismatchStore {
        fn load_accepted_artifact(
            &self,
            _requested_digest: WorkflowDigest,
        ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
            Ok(valid_artifact(self.inner_digest))
        }
    }

    /// A simple ArtifactStore that reports artifact existence by digest.
    struct SimpleArtifactStore {
        exists: bool,
    }
    impl SimpleArtifactStore {
        fn new(exists: bool) -> Self {
            Self { exists }
        }
    }
    impl ArtifactStore for SimpleArtifactStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            self.exists
        }
    }

    // =====================================================================
    // Test 1: load_accepted_artifact_returns_not_found_for_unknown_digest
    // =====================================================================
    #[test]
    fn load_accepted_artifact_returns_not_found_for_unknown_digest() {
        let store = NotFoundStore;
        let unknown_digest = WorkflowDigest::from_bytes([0x42_u8; 32]);

        let result = store.load_accepted_artifact(unknown_digest);

        assert!(
            matches!(
                result,
                Err(ArtifactEnvelopeError::ArtifactNotFound { digest })
                if digest == unknown_digest
            ),
            "expected ArtifactNotFound for unknown digest, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 2: load_accepted_artifact_returns_postcard_decode_failed_for_corrupt_bytes
    // =====================================================================
    #[test]
    fn load_accepted_artifact_returns_postcard_decode_failed_for_corrupt_bytes() {
        let store = CorruptBytesStore;
        let digest = WorkflowDigest::from_bytes([0x42_u8; 32]);

        let result = store.load_accepted_artifact(digest);

        assert!(
            matches!(result, Err(ArtifactEnvelopeError::PostcardDecodeFailed)),
            "expected PostcardDecodeFailed for corrupt bytes, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 3: load_accepted_artifact_returns_invalid_gate_count_when_gate_count_not_15
    // =====================================================================
    #[test]
    fn load_accepted_artifact_returns_invalid_gate_count_when_gate_count_not_15() {
        let store = WrongGateCountStore::new(7);
        let digest = WorkflowDigest::from_bytes([0x42_u8; 32]);

        let result = store.load_accepted_artifact(digest);

        assert!(
            matches!(
                result,
                Err(ArtifactEnvelopeError::InvalidGateCount {
                    found: 7,
                    required: REQUIRED_GATE_COUNT
                })
            ),
            "expected InvalidGateCount {{ found: 7, required: 15 }}, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 4: load_accepted_artifact_returns_missing_bounded_flag
    // =====================================================================
    #[test]
    fn load_accepted_artifact_returns_missing_bounded_flag() {
        let store = ProofFlagFalseStore::new(ProofFlagVariant::Bounded);
        let digest = WorkflowDigest::from_bytes([0x42_u8; 32]);

        let result = store.load_accepted_artifact(digest);

        assert!(
            matches!(result, Err(ArtifactEnvelopeError::MissingRequiredProofFlagBounded)),
            "expected MissingRequiredProofFlagBounded, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 5: load_accepted_artifact_returns_missing_taint_safe_flag
    // =====================================================================
    #[test]
    fn load_accepted_artifact_returns_missing_taint_safe_flag() {
        let store = ProofFlagFalseStore::new(ProofFlagVariant::TaintSafe);
        let digest = WorkflowDigest::from_bytes([0x42_u8; 32]);

        let result = store.load_accepted_artifact(digest);

        assert!(
            matches!(result, Err(ArtifactEnvelopeError::MissingRequiredProofFlagTaintSafe)),
            "expected MissingRequiredProofFlagTaintSafe, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 6: load_accepted_artifact_returns_missing_retry_safe_flag
    // =====================================================================
    #[test]
    fn load_accepted_artifact_returns_missing_retry_safe_flag() {
        let store = ProofFlagFalseStore::new(ProofFlagVariant::RetrySafe);
        let digest = WorkflowDigest::from_bytes([0x42_u8; 32]);

        let result = store.load_accepted_artifact(digest);

        assert!(
            matches!(result, Err(ArtifactEnvelopeError::MissingRequiredProofFlagRetrySafe)),
            "expected MissingRequiredProofFlagRetrySafe, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 7: load_accepted_artifact_returns_missing_durable_flag
    // =====================================================================
    #[test]
    fn load_accepted_artifact_returns_missing_durable_flag() {
        let store = ProofFlagFalseStore::new(ProofFlagVariant::Durable);
        let digest = WorkflowDigest::from_bytes([0x42_u8; 32]);

        let result = store.load_accepted_artifact(digest);

        assert!(
            matches!(result, Err(ArtifactEnvelopeError::MissingRequiredProofFlagDurable)),
            "expected MissingRequiredProofFlagDurable, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 8: load_accepted_artifact_returns_missing_replayable_flag
    // =====================================================================
    #[test]
    fn load_accepted_artifact_returns_missing_replayable_flag() {
        let store = ProofFlagFalseStore::new(ProofFlagVariant::Replayable);
        let digest = WorkflowDigest::from_bytes([0x42_u8; 32]);

        let result = store.load_accepted_artifact(digest);

        assert!(
            matches!(result, Err(ArtifactEnvelopeError::MissingRequiredProofFlagReplayable)),
            "expected MissingRequiredProofFlagReplayable, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 9: load_accepted_artifact_returns_missing_idempotency_verified_flag
    // =====================================================================
    #[test]
    fn load_accepted_artifact_returns_missing_idempotency_verified_flag() {
        let store = ProofFlagFalseStore::new(ProofFlagVariant::IdempotencyVerified);
        let digest = WorkflowDigest::from_bytes([0x42_u8; 32]);

        let result = store.load_accepted_artifact(digest);

        assert!(
            matches!(result, Err(ArtifactEnvelopeError::MissingRequiredProofFlagIdempotencyVerified)),
            "expected MissingRequiredProofFlagIdempotencyVerified, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 10: admit_artifact_run_returns_digest_mismatch_when_stored_digest_differs
    // =====================================================================
    #[test]
    fn admit_artifact_run_returns_digest_mismatch_when_stored_digest_differs() {
        let requested_digest = WorkflowDigest::from_bytes([0xAA_u8; 32]);
        let stored_digest = WorkflowDigest::from_bytes([0xBB_u8; 32]);
        let store = DigestMismatchStore::new(stored_digest);
        let run_id = RunId::new(1);
        let caps = CapabilitySet::empty();

        let result = admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            run_id,
            requested_digest,
            caps,
        );

        assert!(
            matches!(
                result,
                Err(AdmissionError::ArtifactDigestMismatch {
                    requested,
                    found
                }) if requested == requested_digest && found == stored_digest
            ),
            "expected ArtifactDigestMismatch with requested={requested_digest:?}, found={stored_digest:?}, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 11: admit_run_with_budget_returns_resource_capacity_exceeded_when_budget_too_large
    // =====================================================================
    #[test]
    fn admit_run_with_budget_returns_resource_capacity_exceeded_when_budget_too_large() {
        let digest = WorkflowDigest::from_bytes([0xCC_u8; 32]);
        let run_id = RunId::new(1);
        let caps = CapabilitySet::empty();
        let store = SimpleArtifactStore::new(true);

        // Request a budget where max_steps_executable=10 but capacity only allows 5.
        let requested_budget = AggregateResourceBudget {
            max_steps_executable: 10,
            max_action_tickets: 5,
            max_parallel_in_flight: 2,
            max_retries_per_action: 3,
            max_gather_pages: 4,
            max_gather_items: 5,
            max_for_each_iterations: 6,
            max_together_branches: 2,
            max_repeat_attempts: 3,
            max_run_time_seconds: 60,
            max_result_bytes: 1024,
            max_total_slots_written: 512,
            max_queue_depth: 10,
            max_journal_batch_bytes: 4096,
            max_step_budget_per_tick: 100,
            max_transitions_per_tick: 200,
        };

        // Capacity that is smaller than requested for max_steps_executable.
        let available_capacity = AggregateResourceCapacity {
            max_steps_executable: 5, // < requested 10
            max_action_tickets: 10,
            max_parallel_in_flight: 5,
            max_gather_pages: 10,
            max_gather_items: 10,
            max_result_bytes: 2048,
            max_total_slots_written: 1024,
            max_active_runs: 8,
            max_queue_depth: 20,
            max_journal_batch_bytes: 8192,
            max_step_budget_per_tick: 200,
            max_transitions_per_tick: 400,
        };

        let result = admit_run_with_budget(
            &store,
            RuntimePolicy::Strict,
            digest,
            run_id,
            caps,
            requested_budget,
            available_capacity,
        );

        assert!(
            matches!(
                result,
                Err(AdmissionError::ResourceCapacityExceeded {
                    resource: "max_steps_executable",
                    requested: 10,
                    available: 5
                })
            ),
            "expected ResourceCapacityExceeded for max_steps_executable, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 12: load_accepted_artifact_returns_missing_idempotency_attestation_when_keyed_not_attested
    // =====================================================================
    #[test]
    fn load_accepted_artifact_returns_missing_idempotency_attestation_when_keyed_not_attested() {
        let action = ActionId::new(99);
        // keyed = [action], attested = []  => action is keyed but NOT attested
        let store = MissingAttestationStore::new(
            Box::new([action]),
            Box::new([]),
        );
        let digest = WorkflowDigest::from_bytes([0xDD_u8; 32]);

        let result = store.load_accepted_artifact(digest);

        assert!(
            matches!(
                result,
                Err(ArtifactEnvelopeError::MissingIdempotencyAttestation { action: a })
                if a == action
            ),
            "expected MissingIdempotencyAttestation for action {action:?}, got {:?}",
            result
        );
    }
}
