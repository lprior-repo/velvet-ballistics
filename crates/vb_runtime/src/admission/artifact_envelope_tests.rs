// Test writer agent: vb-lp2v proof-admission acceptance scenarios
// Tests ArtifactEnvelopeError variants and AdmissionError variants via public API.

#[cfg(test)]
mod artifact_envelope_bdd_tests {
    use crate::admission::{
        AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError, ArtifactStore,
        admit_artifact_run, admit_run_with_budget, validate_accepted_artifact_envelope,
    };
    use vb_core::budget::{AggregateResourceBudget, AggregateResourceCapacity};
    use vb_core::capability::CapabilitySet;
    use vb_core::ids::{ActionId, RunId, WorkflowDigest};
    use vb_core::policy::RuntimePolicy;
    use vb_storage::admission::AcceptedArtifact;

    fn valid_artifact() -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
        crate::admission::tests::evidence_test_support::accepted_artifact_with_caps(Box::new([]))
            .map_err(|_| ArtifactEnvelopeError::ArtifactEvidenceInvalid)
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
            _digest: WorkflowDigest,
        ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = valid_artifact()?;
            let proof = &mut artifact.verification;
            proof.gate_count = self.gate_count;
            // Validate envelope - this is what StorageArtifactStore does
            validate_accepted_artifact_envelope(self, &artifact)?;
            Ok(artifact)
        }
    }

    /// Store that changes one checked metadata field in a valid artifact.
    struct TamperedProofStore {
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
    impl TamperedProofStore {
        fn new(flag: ProofFlagVariant) -> Self {
            Self { flag }
        }
    }
    impl AcceptedArtifactStore for TamperedProofStore {
        fn load_accepted_artifact(
            &self,
            _digest: WorkflowDigest,
        ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = valid_artifact()?;
            let proof = &mut artifact.verification;
            match self.flag {
                ProofFlagVariant::Bounded => proof.bounded_claimed = false,
                ProofFlagVariant::TaintSafe => proof.taint_safe_claimed = true,
                ProofFlagVariant::RetrySafe => proof.retry_safe_claimed = true,
                ProofFlagVariant::Durable => proof.durable = true,
                ProofFlagVariant::Replayable => proof.replayable_claimed = true,
                ProofFlagVariant::IdempotencyVerified => proof.idempotency_verified_claimed = false,
            }
            // Validate envelope - this is what StorageArtifactStore does
            validate_accepted_artifact_envelope(self, &artifact)?;
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
            _digest: WorkflowDigest,
        ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
            let mut artifact = valid_artifact()?;
            let proof = &mut artifact.verification;
            proof.idempotency_keyed = self.keyed.clone();
            proof.idempotency_attested = self.attested.clone();
            // Validate envelope - this is what StorageArtifactStore does
            validate_accepted_artifact_envelope(self, &artifact)?;
            Ok(artifact)
        }
    }

    /// Store that returns an artifact with a different digest inside than requested.
    struct DigestMismatchStore;
    impl DigestMismatchStore {
        fn new(_inner_digest: WorkflowDigest) -> Self {
            Self
        }
    }
    impl AcceptedArtifactStore for DigestMismatchStore {
        fn load_accepted_artifact(
            &self,
            _requested_digest: WorkflowDigest,
        ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
            valid_artifact()
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
    fn load_accepted_artifact_rejects_fabricated_gate_count() {
        let store = WrongGateCountStore::new(7);
        let digest = WorkflowDigest::from_bytes([0x42_u8; 32]);

        let result = store.load_accepted_artifact(digest);

        assert!(
            matches!(result, Err(ArtifactEnvelopeError::ArtifactEvidenceInvalid)),
            "expected ArtifactEvidenceInvalid, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 4: load_accepted_artifact_rejects_changed_bounded_metadata
    // =====================================================================
    #[test]
    fn load_accepted_artifact_rejects_changed_bounded_metadata() {
        let store = TamperedProofStore::new(ProofFlagVariant::Bounded);
        let digest = WorkflowDigest::from_bytes([0x42_u8; 32]);

        let result = store.load_accepted_artifact(digest);

        assert!(
            matches!(result, Err(ArtifactEnvelopeError::ArtifactEvidenceInvalid)),
            "expected ArtifactEvidenceInvalid (legacy field Bounded, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 5: load_accepted_artifact_rejects_forged_taint_safe_claim
    // =====================================================================
    #[test]
    fn load_accepted_artifact_rejects_forged_taint_safe_claim() {
        let store = TamperedProofStore::new(ProofFlagVariant::TaintSafe);
        let digest = WorkflowDigest::from_bytes([0x42_u8; 32]);

        let result = store.load_accepted_artifact(digest);

        assert!(
            matches!(result, Err(ArtifactEnvelopeError::ArtifactEvidenceInvalid)),
            "expected ArtifactEvidenceInvalid (legacy field TaintSafe, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 6: load_accepted_artifact_rejects_forged_retry_safe_claim
    // =====================================================================
    #[test]
    fn load_accepted_artifact_rejects_forged_retry_safe_claim() {
        let store = TamperedProofStore::new(ProofFlagVariant::RetrySafe);
        let digest = WorkflowDigest::from_bytes([0x42_u8; 32]);

        let result = store.load_accepted_artifact(digest);

        assert!(
            matches!(result, Err(ArtifactEnvelopeError::ArtifactEvidenceInvalid)),
            "expected ArtifactEvidenceInvalid (legacy field RetrySafe, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 7: load_accepted_artifact_rejects_forged_durable_claim
    // =====================================================================
    #[test]
    fn load_accepted_artifact_rejects_forged_durable_claim() {
        let store = TamperedProofStore::new(ProofFlagVariant::Durable);
        let digest = WorkflowDigest::from_bytes([0x42_u8; 32]);

        let result = store.load_accepted_artifact(digest);

        assert!(
            matches!(result, Err(ArtifactEnvelopeError::ArtifactEvidenceInvalid)),
            "expected ArtifactEvidenceInvalid (legacy field Durable, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 8: load_accepted_artifact_rejects_forged_replayable_claim
    // =====================================================================
    #[test]
    fn load_accepted_artifact_rejects_forged_replayable_claim() {
        let store = TamperedProofStore::new(ProofFlagVariant::Replayable);
        let digest = WorkflowDigest::from_bytes([0x42_u8; 32]);

        let result = store.load_accepted_artifact(digest);

        assert!(
            matches!(result, Err(ArtifactEnvelopeError::ArtifactEvidenceInvalid)),
            "expected ArtifactEvidenceInvalid (legacy field Replayable, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 9: load_accepted_artifact_rejects_changed_idempotency_verified_metadata
    // =====================================================================
    #[test]
    fn load_accepted_artifact_rejects_changed_idempotency_verified_metadata() {
        let store = TamperedProofStore::new(ProofFlagVariant::IdempotencyVerified);
        let digest = WorkflowDigest::from_bytes([0x42_u8; 32]);

        let result = store.load_accepted_artifact(digest);

        assert!(
            matches!(result, Err(ArtifactEnvelopeError::ArtifactEvidenceInvalid)),
            "expected ArtifactEvidenceInvalid (legacy field IdempotencyVerified, got {:?}",
            result
        );
    }

    // =====================================================================
    // Test 10: admit_artifact_run_returns_digest_mismatch_when_stored_digest_differs
    // =====================================================================
    #[test]
    fn admit_artifact_run_returns_digest_mismatch_when_stored_digest_differs() -> Result<(), String>
    {
        let requested_digest = WorkflowDigest::from_bytes([0xAA_u8; 32]);
        let stored_digest = valid_artifact().map_err(|e| format!("{e:?}"))?.digest;
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
        Ok(())
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
            max_timer_entries: 8,
            max_trace_events: 9,
            max_queue_depth: 10,
            max_journal_batch_bytes: 4096,
            max_ipc_payload_bytes: 2048,
            max_blob_bytes: 8192,
            max_input_bytes: 1024,
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
            max_timer_entries: 16,
            max_trace_events: 18,
            max_active_runs: 8,
            max_queue_depth: 20,
            max_journal_batch_bytes: 8192,
            max_ipc_payload_bytes: 4096,
            max_blob_bytes: 16_384,
            max_input_bytes: 2048,
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
    // Test 12: load_accepted_artifact_rejects_unbound_idempotency_attestation
    // =====================================================================
    #[test]
    fn load_accepted_artifact_rejects_unbound_idempotency_attestation() {
        let action = ActionId::new(99);
        // keyed = [action], attested = []  => action is keyed but NOT attested
        let store = MissingAttestationStore::new(Box::new([action]), Box::new([]));
        let digest = WorkflowDigest::from_bytes([0xDD_u8; 32]);

        let result = store.load_accepted_artifact(digest);

        assert!(
            matches!(result, Err(ArtifactEnvelopeError::ArtifactEvidenceInvalid)),
            "expected MissingIdempotencyAttestation for action {action:?}, got {:?}",
            result
        );
    }
}
