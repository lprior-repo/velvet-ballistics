    // =========================================================================
    // SECTION 2.1: submit_artifact — Policy Tier Behavior (Unit Tests)
    // =========================================================================

    /// TEST: submit_artifact Relaxed policy skips gate validation
    ///
    /// Contract §2.1 Precondition (Relaxed): no gate validation is performed.
    /// Contract §2.1 Postcondition (Relaxed): gate_count=0, durable=false.
    #[test]
    fn submit_artifact_relaxed_skips_gate_validation() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)
            .map_err(|e| format!("submit_artifact failed: {e}"))?;

        assert_eq!(
            result.verification.gate_count, 0,
            "Relaxed policy must skip gates and return gate_count=0"
        );
        assert!(
            !result.verification.durable,
            "Relaxed policy must have durable=false"
        );
        assert_eq!(
            result.verification.digest, result.digest,
            "proof digest must match artifact digest"
        );
        assert_eq!(
            result.source_digest,
            workflow.digest(),
            "source digest must match workflow digest"
        );
        Ok(())
    }

    /// TEST: submit_artifact Journaled policy enforces both gates
    ///
    /// Contract §2.1 Postcondition (Journaled): gate_count=2, durable=false.
    #[test]
    fn submit_artifact_journaled_enforces_both_gates() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .map_err(|e| format!("submit_artifact(journaled) failed: {e}"))?;

        assert_eq!(
            result.verification.gate_count, 0,
            "Journaled must pass typed checked evidence without a fabricated aggregate count"
        );
        assert!(
            !result.verification.durable,
            "Journaled must not be durable (no SyncAll)"
        );
        crate::admission::verify_accepted_artifact_for_policy(&result, RuntimePolicy::Journaled)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// TEST: submit_artifact Strict policy enforces gates plus SyncAll
    ///
    /// Strict persists checked evidence; proof bytes cannot certify the persistence acknowledgement.
    #[test]
    fn submit_artifact_strict_enforces_gates_plus_syncall() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
            .map_err(|e| format!("submit_artifact(strict) failed: {e}"))?;

        assert_eq!(
            result.verification.gate_count, 0,
            "Strict must pass typed checked evidence"
        );
        assert!(
            !result.verification.durable,
            "proof bytes cannot self-certify a durable acknowledgement"
        );
        crate::admission::verify_accepted_artifact_for_policy(&result, RuntimePolicy::Strict)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// TEST: submit_artifact Relaxed persists record
    ///
    /// Contract §2.1 Postcondition (Relaxed): journal.put_compiled_ir called exactly once.
    #[test]
    fn submit_artifact_relaxed_persists_record() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;
        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)
            .map_err(|e| format!("submit_artifact failed: {e}"))?;

        let loaded = journal
            .compiled_ir(artifact.digest)
            .map_err(|e| format!("read: {e}"))?;
        assert!(
            loaded.is_some(),
            "Relaxed policy must persist record to compiled_ir keyspace"
        );
        Ok(())
    }

    /// TEST: submit_artifact all policies bind source and artifact digests
    ///
    /// Contract §2.1 Postcondition (All): source_digest == workflow.digest()
    /// and artifact.digest binds the accepted-artifact envelope.
    #[test]
    fn submit_artifact_all_policies_set_correct_digest() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        for policy in [
            RuntimePolicy::Relaxed,
            RuntimePolicy::Journaled,
            RuntimePolicy::Strict,
        ] {
            let result = submit_artifact(&journal, &workflow, policy)
                .map_err(|e| format!("submit_artifact({policy:?}) failed: {e}"))?;
            let computed = accepted_artifact_digest(&result)
                .map_err(|e| format!("accepted artifact digest failed: {e}"))?;
            assert_eq!(
                result.source_digest,
                workflow.digest(),
                "source digest must equal workflow.digest() for policy {policy:?}"
            );
            assert_eq!(
                result.digest, computed,
                "artifact.digest must equal canonical envelope digest for policy {policy:?}"
            );
        }
        Ok(())
    }

    /// TEST: submit_artifact all policies return non-empty ir
    ///
    /// Contract §2.1 Postcondition (All): artifact.ir is non-empty postcard-encoded bytes.
    #[test]
    fn submit_artifact_all_policies_return_nonempty_ir() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        for policy in [
            RuntimePolicy::Relaxed,
            RuntimePolicy::Journaled,
            RuntimePolicy::Strict,
        ] {
            let result = submit_artifact(&journal, &workflow, policy)
                .map_err(|e| format!("submit_artifact({policy:?}) failed: {e}"))?;
            assert!(
                !result.ir.is_empty(),
                "artifact.ir must be non-empty for policy {policy:?}"
            );
        }
        Ok(())
    }

    /// TEST: submit_artifact accepted_at_seq is valid EventSeq
    ///
    /// Contract §2.1 Postcondition (All): accepted_at_seq is a valid EventSeq.
    #[test]
    fn accepted_at_seq_is_valid_event_seq() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
            .map_err(|e| format!("submit_artifact failed: {e}"))?;

        // accepted_at_seq must be a valid EventSeq (non-null, properly constructed)
        assert_eq!(
            result.accepted_at_seq.get(),
            1,
            "accepted_at_seq should be the first issued sequence number (1)"
        );
        Ok(())
    }

    // =========================================================================
    // SECTION 2.2: admit_compiled_artifact — Admission Gate (Unit Tests)
    // =========================================================================

    /// TEST: admit_compiled_artifact structure gate enforced
    ///
    /// Contract §2.2: ArtifactMalformed when try_from_parts fails.
    #[test]
    fn admit_compiled_artifact_structure_gate_enforced() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let result = admit_compiled_artifact(&journal, &workflow)
            .map_err(|e| format!("admit_compiled_artifact failed: {e}"))?;
        let expected = compiled_record_digest(&workflow)?;

        assert_eq!(
            result, expected,
            "admit_compiled_artifact must return compiled payload digest on success"
        );
        Ok(())
    }

    /// TEST: admit_compiled_artifact idempotent on duplicate
    ///
    /// Contract §2.2 Postcondition: On duplicate admission, returns same digest without error.
    #[test]
    fn admit_compiled_artifact_idempotent_on_duplicate() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let digest_a = admit_compiled_artifact(&journal, &workflow)
            .map_err(|e| format!("first admit failed: {e}"))?;
        let digest_b = admit_compiled_artifact(&journal, &workflow)
            .map_err(|e| format!("second admit failed: {e}"))?;

        assert_eq!(
            digest_a, digest_b,
            "duplicate admission must return same digest (idempotent)"
        );
        Ok(())
    }

    /// TEST: admit_compiled_artifact puts record with matching digest
    ///
    /// Contract §2.2 Postcondition: journal.put_compiled_ir called where record.digest == workflow.digest().
    #[test]
    fn admit_compiled_artifact_puts_record_with_matching_digest() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;
        let expected_digest = compiled_record_digest(&workflow)?;

        admit_compiled_artifact(&journal, &workflow).map_err(|e| format!("admit failed: {e}"))?;

        let loaded = journal
            .compiled_ir(expected_digest)
            .map_err(|e| format!("read: {e}"))?;
        assert!(
            loaded.is_some(),
            "compiled_ir must contain record with matching digest"
        );
        Ok(())
    }

    /// TEST: admit_compiled_artifact returns compiled payload digest
    ///
    /// Contract §2.2 Postcondition: Returns BLAKE3 of persisted compiled payload.
    #[test]
    fn admit_compiled_artifact_returns_workflow_digest() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let result = admit_compiled_artifact(&journal, &workflow)
            .map_err(|e| format!("admit failed: {e}"))?;

        assert_eq!(
            result,
            compiled_record_digest(&workflow)?,
            "returned digest must equal compiled payload digest"
        );
        Ok(())
    }

    // =========================================================================
    // SECTION 2.3: verify_content_digest — Checksum Verification (Unit Tests)
    // =========================================================================

    /// TEST: verify_content_digest accepts matching hash
    ///
    /// Contract §2.3 Postcondition (Success): blake3::hash(content) == expected.
    #[test]
    fn verify_content_digest_accepts_matching_hash() -> Result<(), String> {
        let content = b"test content for hashing".to_vec();
        let expected: [u8; 32] = blake3::hash(&content).into();

        let result = crate::journal::verify_content_digest(&content, &expected);
        assert!(
            result.is_ok(),
            "verify_content_digest must accept content matching expected hash"
        );
        Ok(())
    }

    /// TEST: verify_content_digest rejects mismatched hash
    ///
    /// Contract §2.3 Postcondition (Failure): Returns PayloadDigestMismatch.
    #[test]
    fn verify_content_digest_rejects_mismatched_hash() -> Result<(), String> {
        let content = b"test content".to_vec();
        let wrong_digest: [u8; 32] = [0xFF; 32];

        let result = crate::journal::verify_content_digest(&content, &wrong_digest);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "mismatched hash must return PayloadDigestMismatch"
        );
        Ok(())
    }

    /// TEST: verify_content_digest never panics
    ///
    /// Contract §2.3: Function returns Result — no panics on any input.
    #[test]
    fn verify_content_digest_never_panics() -> Result<(), String> {
        let content = b"any content".to_vec();
        let digest = [0x42u8; 32];

        // This must not panic — must return Result
        let result =
            std::panic::catch_unwind(|| crate::journal::verify_content_digest(&content, &digest));
        assert!(
            result.is_ok(),
            "verify_content_digest must not panic on any input"
        );
        Ok(())
    }

    // =========================================================================
    // SECTION 2.4: VerificationProof Invariants (Unit Tests)
    // =========================================================================

    /// TEST: gate_count zero for Relaxed
    ///
    /// Contract §3.2: Relaxed → gate_count = 0.
    #[test]
    fn gate_count_zero_for_relaxed() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)
            .map_err(|e| format!("submit failed: {e}"))?;

        assert_eq!(
            result.verification.gate_count, 0,
            "Relaxed policy must have gate_count == 0"
        );
        Ok(())
    }

    /// TEST: gate_count two for Journaled
    ///
    /// Contract §3.2: Journaled → gate_count = 2.
    #[test]
    fn gate_count_two_for_journaled() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .map_err(|e| format!("submit failed: {e}"))?;

        assert_eq!(
            result.verification.gate_count, 0,
            "Journaled policy must have no fabricated aggregate gate count"
        );
        crate::admission::verify_accepted_artifact_for_policy(&result, RuntimePolicy::Journaled)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// TEST: checked evidence replaces aggregate gate counts for Strict
    ///
    /// Strict carries typed evidence with a conservative legacy gate count.
    #[test]
    fn strict_uses_checked_evidence_without_fabricated_count() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
            .map_err(|e| format!("submit failed: {e}"))?;

        assert_eq!(
            result.verification.gate_count, 0,
            "Strict policy must have no fabricated aggregate gate count"
        );
        crate::admission::verify_accepted_artifact_for_policy(&result, RuntimePolicy::Strict)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// TEST: proof metadata does not certify durability for any policy
    ///
    /// Durability is an executed journal barrier result.
    #[test]
    fn proof_metadata_does_not_certify_durability() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let relaxed = submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)
            .map_err(|e| format!("relaxed failed: {e}"))?;
        let journaled = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .map_err(|e| format!("journaled failed: {e}"))?;
        let strict = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
            .map_err(|e| format!("strict failed: {e}"))?;

        assert!(
            !relaxed.verification.durable,
            "Relaxed must have durable == false"
        );
        assert!(
            !journaled.verification.durable,
            "Journaled must have durable == false"
        );
        assert!(
            !strict.verification.durable,
            "durability is an executed persistence result, not a proof flag"
        );
        crate::admission::verify_accepted_artifact_for_policy(&strict, RuntimePolicy::Strict)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // =========================================================================
    // SECTION 2.5: Digest Forgery Prevention (BH-01 through BH-04)
    // =========================================================================

    /// TEST: forged_workflow_source_digest_rejected (BH-01)
    ///
    /// Contract §6 BH-01: Direct put_workflow_source rejects forged digest.
    #[test]
    fn forged_workflow_source_digest_rejected() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let forged_digest = WorkflowDigest::from_bytes([0xDE; 32]);
        let record = WorkflowSourceRecord {
            digest: forged_digest,
            source: b"this will not hash to 0xDE..DE".to_vec(),
        };

        let result = journal.put_workflow_source(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "forged workflow source digest must be rejected"
        );
        Ok(())
    }

    /// TEST: forged_blob_digest_rejected (BH-01)
    ///
    /// Contract §6 BH-01: Direct put_blob rejects forged digest.
    #[test]
    fn forged_blob_digest_rejected() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let forged_digest = [0xAB; 32];
        let record = BlobRecord {
            digest: forged_digest,
            bytes: b"these bytes won't hash to 0xAB..AB".to_vec(),
        };

        let result = journal.put_blob(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "forged blob digest must be rejected"
        );
        Ok(())
    }

    /// TEST: batch_forged_workflow_source_digest_rejected (BH-02)
    ///
    /// Contract §6 BH-02: Batch put_workflow_source rejects forged digest.
    #[test]
    fn batch_forged_workflow_source_digest_rejected() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let forged_digest = WorkflowDigest::from_bytes([0xFF; 32]);
        let mut batch = journal.batch();

        let result = batch.put_workflow_source(&WorkflowSourceRecord {
            digest: forged_digest,
            source: b"not the right hash".to_vec(),
        });

        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "batch forged workflow source digest must be rejected"
        );
        Ok(())
    }

    /// TEST: batch_forged_blob_digest_rejected (BH-02)
    ///
    /// Contract §6 BH-02: Batch put_blob rejects forged digest.
    #[test]
    fn batch_forged_blob_digest_rejected() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let mut batch = journal.batch();

        let result = batch.put_blob(&BlobRecord {
            digest: [0x11; 32],
            bytes: b"wrong hash".to_vec(),
        });

        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "batch forged blob digest must be rejected"
        );
        Ok(())
    }

    /// TEST: all_zero_digest_rejects_nonempty_content (BH-14)
    ///
    /// Contract §6 BH-14: All-zero 32-byte digest rejects non-empty content.
    #[test]
    fn all_zero_digest_rejects_nonempty_content() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let content = b"some content that definitely won't hash to zeros";
        let zero_digest = [0u8; 32];
        let record = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes(zero_digest),
            source: content.to_vec(),
        };

        let result = journal.put_workflow_source(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "all-zero digest must reject non-empty content"
        );
        Ok(())
    }

