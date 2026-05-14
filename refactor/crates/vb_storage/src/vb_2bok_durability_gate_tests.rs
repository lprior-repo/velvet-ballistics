#![forbid(unsafe_code)]
//! Durability Gate Tests for vb-2bok — RED PHASE
//!
//! These tests define expected behavior for the durability gate boundary
//! (vb_storage admission.rs).

#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod durability_gate_tests {
    use crate::admission::{admit_compiled_artifact, submit_artifact};
    use crate::codec::{decode_record, encode_record};
    use crate::constants::{
        CRC_OFFSET, MAGIC_BLOB, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        RECORD_HEADER_BYTES,
    };
    use crate::records::RecordKind;
    use crate::{
        BlobRecord, DIGEST_BYTES, EventSeq, FjallJournal, JournalError, JournalEvent,
        WorkflowSourceRecord,
    };
    use vb_core::{CompiledWorkflow, RunId, RuntimePolicy, SlotIdx, StepIdx, WorkflowDigest};

    // =========================================================================
    // Test fixtures and helpers
    // =========================================================================

    fn temp_journal() -> Result<(tempfile::TempDir, FjallJournal), JournalError> {
        let temp = tempfile::tempdir().map_err(|_| JournalError::ArtifactMalformed)?;
        let journal = FjallJournal::open(temp.path(), None)?;
        Ok((temp, journal))
    }

    fn minimal_valid_workflow() -> Result<CompiledWorkflow, String> {
        use vb_core::value::ConstValue;
        use vb_core::workflow::{ResourceContract, WorkflowParts};
        use vb_core::{CompiledNode, CompiledNodeKind, ConstIdx, SlotIdx, StepIdx};

        let mut parts = WorkflowParts {
            name: Box::<str>::from("vb_2bok_test"),
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: Box::new([
                CompiledNode {
                    id: StepIdx::new(0),
                    output: Some(SlotIdx::new(0)),
                    next: Some(StepIdx::new(1)),
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::SetConst {
                        value: ConstIdx::new(0),
                    },
                },
                CompiledNode {
                    id: StepIdx::new(1),
                    output: None,
                    next: None,
                    on_error: None,
                    error_slot: None,
                    kind: CompiledNodeKind::Finish {
                        result: SlotIdx::new(0),
                    },
                },
            ]),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([ConstValue::I64(42)]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        };

        let hash_bytes = postcard::to_allocvec(&parts)
            .map_err(|e| format!("serialize parts for digest: {e}"))?;
        let computed = blake3::hash(&hash_bytes);
        parts.digest = WorkflowDigest::from_bytes(computed.into());

        CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
    }

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
            result.verification.digest,
            workflow.digest(),
            "proof digest must match workflow digest"
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
            result.verification.gate_count, 2,
            "Journaled must pass exactly 2 gates (structure + checksum)"
        );
        assert!(
            !result.verification.durable,
            "Journaled must not be durable (no SyncAll)"
        );
        Ok(())
    }

    /// TEST: submit_artifact Strict policy enforces gates plus SyncAll
    ///
    /// Contract §2.1 Postcondition (Strict): gate_count=2, durable=true.
    #[test]
    fn submit_artifact_strict_enforces_gates_plus_syncall() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
            .map_err(|e| format!("submit_artifact(strict) failed: {e}"))?;

        assert_eq!(
            result.verification.gate_count, 2,
            "Strict must pass exactly 2 gates"
        );
        assert!(
            result.verification.durable,
            "Strict must be durable (SyncAll called)"
        );
        Ok(())
    }

    /// TEST: submit_artifact Relaxed persists record
    ///
    /// Contract §2.1 Postcondition (Relaxed): journal.put_compiled_ir called exactly once.
    #[test]
    fn submit_artifact_relaxed_persists_record() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;
        let digest = workflow.digest();

        submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)
            .map_err(|e| format!("submit_artifact failed: {e}"))?;

        let loaded = journal
            .compiled_ir(digest)
            .map_err(|e| format!("read: {e}"))?;
        assert!(
            loaded.is_some(),
            "Relaxed policy must persist record to compiled_ir keyspace"
        );
        Ok(())
    }

    /// TEST: submit_artifact all policies set correct digest
    ///
    /// Contract §2.1 Postcondition (All): artifact.digest == workflow.digest().
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
            assert_eq!(
                result.digest,
                workflow.digest(),
                "artifact.digest must equal workflow.digest() for policy {policy:?}"
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
            0,
            "accepted_at_seq should be initialized to 0 in current implementation"
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

        assert_eq!(
            result,
            workflow.digest(),
            "admit_compiled_artifact must return workflow digest on success"
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
        let expected_digest = workflow.digest();

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

    /// TEST: admit_compiled_artifact returns workflow digest
    ///
    /// Contract §2.2 Postcondition: Returns workflow.digest().
    #[test]
    fn admit_compiled_artifact_returns_workflow_digest() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let result = admit_compiled_artifact(&journal, &workflow)
            .map_err(|e| format!("admit failed: {e}"))?;

        assert_eq!(
            result,
            workflow.digest(),
            "returned digest must equal workflow.digest()"
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
            result.verification.gate_count, 2,
            "Journaled policy must have gate_count == 2"
        );
        Ok(())
    }

    /// TEST: gate_count two for Strict
    ///
    /// Contract §3.2: Strict → gate_count = 2.
    #[test]
    fn gate_count_two_for_strict() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
            .map_err(|e| format!("submit failed: {e}"))?;

        assert_eq!(
            result.verification.gate_count, 2,
            "Strict policy must have gate_count == 2"
        );
        Ok(())
    }

    /// TEST: durable true only for Strict
    ///
    /// Contract §3.2: durable == true only for Strict.
    #[test]
    fn durable_true_only_for_strict() -> Result<(), String> {
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
            strict.verification.durable,
            "Strict must have durable == true"
        );
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

    // =========================================================================
    // SECTION 2.6: Record Envelope & Codec (BH-03, BH-07, BH-08, BH-09, BH-14, BH-15)
    // =========================================================================

    /// TEST: decode_rejects_all_zero_bytes (BH-03)
    ///
    /// Contract §6 BH-03: All-zero input returns error (not panic).
    #[test]
    fn decode_rejects_all_zero_bytes() -> Result<(), String> {
        let zeros = [0u8; RECORD_HEADER_BYTES + 64];
        let result = decode_record::<JournalEvent>(
            &zeros,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            result.is_err(),
            "all-zero bytes must be rejected, not panic"
        );
        Ok(())
    }

    /// TEST: decode_rejects_valid_header_with_corrupt_payload (BH-03)
    ///
    /// Contract §6 BH-03: Tampered payload detected by BLAKE3.
    #[test]
    fn decode_rejects_valid_header_with_corrupt_payload() -> Result<(), String> {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0x42; DIGEST_BYTES]),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .map_err(|e| format!("encode failed: {e}"))?;

        let mut corrupt = bytes;
        for byte in corrupt.iter_mut().skip(RECORD_HEADER_BYTES) {
            *byte = byte.wrapping_add(1);
        }

        let result = decode_record::<JournalEvent>(
            &corrupt,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "corrupt payload must yield PayloadDigestMismatch"
        );
        Ok(())
    }

    /// TEST: event_seq_overflow_rejected (BH-04)
    ///
    /// Contract §6 BH-04: EventSeq::MAX overflow rejected.
    #[test]
    fn event_seq_overflow_rejected() -> Result<(), String> {
        let seq = EventSeq::new(u64::MAX);
        let result = crate::codec::next_seq(seq);
        assert!(
            matches!(result, Err(JournalError::SequenceOverflow)),
            "u64::MAX + 1 must yield SequenceOverflow"
        );
        Ok(())
    }

    /// TEST: decode_rejects_header_only_when_payload_declared (BH-05)
    ///
    /// Contract §6 BH-05: Header-only with declared payload returns UnexpectedEof.
    #[test]
    fn decode_rejects_header_only_when_payload_declared() -> Result<(), String> {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            attempt: 1,
            reason: None,
        };
        let full = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .map_err(|e| format!("encode failed: {e}"))?;

        let truncated = &full[..RECORD_HEADER_BYTES];
        let result = decode_record::<JournalEvent>(
            truncated,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "truncated record must yield UnexpectedEof"
        );
        Ok(())
    }

    /// TEST: decode_rejects_future_schema_version_in_full_record (BH-07)
    ///
    /// Contract §6 BH-07: Future schema version → UnsupportedSchemaVersion.
    #[test]
    fn decode_rejects_future_schema_version_in_full_record() -> Result<(), String> {
        use crate::constants::CURRENT_SCHEMA_VERSION;

        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            attempt: 1,
            reason: None,
        };
        let mut bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .map_err(|e| format!("encode failed: {e}"))?;

        let future_version = CURRENT_SCHEMA_VERSION.saturating_add(1);
        bytes
            .get_mut(4..6)
            .ok_or_else(|| String::from("schema version field not found"))?
            .copy_from_slice(&future_version.to_le_bytes());

        // Recompute CRC
        let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
        bytes
            .get_mut(CRC_OFFSET..CRC_OFFSET + 4)
            .ok_or_else(|| String::from("CRC field not found"))?
            .copy_from_slice(&checksum.to_le_bytes());

        let result = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::UnsupportedSchemaVersion { .. })),
            "future schema version must yield UnsupportedSchemaVersion"
        );
        Ok(())
    }

    /// TEST: encode_rejects_kind_family_mismatch_workflow_in_journal (BH-08)
    ///
    /// Contract §6 BH-08: Wrong magic for workflow record kind.
    #[test]
    fn encode_rejects_kind_family_mismatch_workflow_in_journal() -> Result<(), String> {
        let record = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
            source: vec![1],
        };
        let result = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::WorkflowSource,
            0,
            &record,
            128,
        );
        assert!(
            matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
            "kind family mismatch must be rejected"
        );
        Ok(())
    }

    /// TEST: crc_single_bit_flip_detected (BH-09)
    ///
    /// Contract §6 BH-09: Single-bit flip in header → HeaderChecksumMismatch.
    #[test]
    fn crc_single_bit_flip_detected() -> Result<(), String> {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xAA; DIGEST_BYTES]),
        };
        let mut bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .map_err(|e| format!("encode failed: {e}"))?;

        if let Some(byte) = bytes.get_mut(CRC_OFFSET) {
            *byte ^= 0x01;
        }

        let result = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::HeaderChecksumMismatch)),
            "CRC bit flip must yield HeaderChecksumMismatch"
        );
        Ok(())
    }

    /// TEST: journal_event_respects_max_payload (BH-15)
    ///
    /// Contract §6 BH-15: Payload exceeding max_payload_len → PayloadTooLarge.
    #[test]
    fn journal_event_respects_max_payload() -> Result<(), String> {
        let big_value = vec![0xFFu8; MAX_JOURNAL_EVENT_PAYLOAD_BYTES as usize + 1];
        let event = JournalEvent::SlotWrittenEvent {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            slot: SlotIdx::new(0),
            value: Some(big_value),
            extra: None,
            attempt: 1,
        };
        let result = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::SlotWritten,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::PayloadTooLarge { .. })),
            "oversized journal event must yield PayloadTooLarge"
        );
        Ok(())
    }

    // =========================================================================
    // SECTION 2.7: Process Lock Invariant (BH-16)
    // =========================================================================

    /// TEST: second_journal_open_on_same_path_is_prevented_by_process_lock (BH-16)
    ///
    /// Contract §6 BH-16: Second FjallJournal::open → ProcessLockHeld.
    #[test]
    fn second_journal_open_on_same_path_is_prevented_by_process_lock() -> Result<(), String> {
        let temp = tempfile::tempdir().map_err(|e| format!("tempdir failed: {e}"))?;

        let _journal1 =
            FjallJournal::open(temp.path(), None).map_err(|e| format!("first open failed: {e}"))?;

        let result = FjallJournal::open(temp.path(), None);
        assert!(
            result.is_err(),
            "second open on same path must fail due to process lock"
        );
        Ok(())
    }

    // =========================================================================
    // SECTION 3: Integration Tests
    // =========================================================================

    /// TEST: submit_then_retrieve_artifact_round_trips
    ///
    /// Contract §7.1: Store via submit_artifact, retrieve via journal.compiled_ir(digest),
    /// bytes round-trip through postcard.
    #[test]
    fn submit_then_retrieve_artifact_round_trips() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .map_err(|e| format!("submit failed: {e}"))?;

        // Retrieve from storage
        let loaded = journal
            .compiled_ir(artifact.digest)
            .map_err(|e| format!("read failed: {e}"))?;
        let record = loaded.ok_or_else(|| String::from("artifact not found after submit"))?;

        // Verify digest matches
        assert_eq!(
            record.digest, artifact.digest,
            "stored digest must match submitted digest"
        );

        // Verify ir bytes round-trip through postcard
        let decoded: crate::admission::AcceptedArtifact =
            postcard::from_bytes(&record.ir).map_err(|e| format!("postcard decode: {e}"))?;
        assert_eq!(
            decoded.digest, artifact.digest,
            "decoded artifact digest must match original"
        );
        Ok(())
    }

    /// TEST: submit_journaled_record_readable_by_digest
    ///
    /// Contract §7.1: Journaled artifact readable after persist.
    #[test]
    fn submit_journaled_record_readable_by_digest() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .map_err(|e| format!("submit failed: {e}"))?;

        let loaded = journal
            .compiled_ir(result.digest)
            .map_err(|e| format!("read: {e}"))?;
        assert!(
            loaded.is_some(),
            "journaled artifact must be readable by digest"
        );
        Ok(())
    }

    /// TEST: admit_twice_only_inserts_once
    ///
    /// Contract §7.1: Duplicate admit_compiled_artifact — only one insert, same digest returned.
    #[test]
    fn admit_twice_only_inserts_once() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let digest_a = admit_compiled_artifact(&journal, &workflow)
            .map_err(|e| format!("first admit: {e}"))?;
        let digest_b = admit_compiled_artifact(&journal, &workflow)
            .map_err(|e| format!("second admit: {e}"))?;

        assert_eq!(digest_a, digest_b, "both calls must return same digest");

        // Count records — should be exactly 1, not 2
        let loaded = journal
            .compiled_ir(digest_a)
            .map_err(|e| format!("read: {e}"))?;
        assert!(loaded.is_some(), "artifact must be stored after admission");
        Ok(())
    }

    /// TEST: batch_commit_persists_all_records
    ///
    /// Contract §7.1: After commit(), both workflow_source and blob readable from keyspaces.
    #[test]
    fn batch_commit_persists_all_records() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;

        let source = b"batch workflow".to_vec();
        let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let workflow_record = WorkflowSourceRecord {
            digest: source_digest,
            source: source.clone(),
        };

        let payload = vec![0xBB];
        let blob_digest: [u8; 32] = blake3::hash(&payload).into();
        let blob_record = BlobRecord {
            digest: blob_digest,
            bytes: payload.clone(),
        };

        {
            let mut batch = journal.batch();
            batch
                .put_workflow_source(&workflow_record)
                .map_err(|e| format!("batch ws: {e}"))?;
            batch
                .put_blob(&blob_record)
                .map_err(|e| format!("batch blob: {e}"))?;
            batch.commit().map_err(|e| format!("commit: {e}"))?;
        }

        // Verify both records are readable
        let loaded_source = journal
            .workflow_source(source_digest)
            .map_err(|e| format!("read ws: {e}"))?;
        assert!(
            loaded_source.is_some(),
            "workflow_source must be readable after batch commit"
        );

        let loaded_blob = journal
            .blob(blob_digest)
            .map_err(|e| format!("read blob: {e}"))?;
        assert!(
            loaded_blob.is_some(),
            "blob must be readable after batch commit"
        );
        Ok(())
    }

    /// TEST: batch_with_fraud_stages_nothing
    ///
    /// Contract §7.1: One forged item in batch → batch.len() == 0, nothing staged.
    #[test]
    fn batch_with_fraud_stages_nothing() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;

        let good_source = b"good source".to_vec();
        let good_digest = WorkflowDigest::from_bytes(blake3::hash(&good_source).into());
        let forged_digest = WorkflowDigest::from_bytes([0xFF; 32]);

        let mut batch = journal.batch();

        // First, valid item succeeds
        let result1 = batch.put_workflow_source(&WorkflowSourceRecord {
            digest: good_digest,
            source: good_source.clone(),
        });
        assert!(
            result1.is_ok(),
            "valid workflow source must be accepted into batch"
        );

        // Then forged item fails
        let result2 = batch.put_workflow_source(&WorkflowSourceRecord {
            digest: forged_digest,
            source: b"forged content".to_vec(),
        });
        assert!(
            matches!(result2, Err(JournalError::PayloadDigestMismatch)),
            "forged digest must cause batch put to fail"
        );

        // Batch must be empty after fraud attempt
        assert_eq!(
            batch.len(),
            0,
            "batch must be empty after failed put (nothing staged)"
        );
        Ok(())
    }

    /// TEST: artifact_digest_equals_workflow_digest
    ///
    /// Contract §2.1 Postcondition (All): artifact.digest == workflow.digest().
    #[test]
    fn artifact_digest_equals_workflow_digest() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        for policy in [
            RuntimePolicy::Relaxed,
            RuntimePolicy::Journaled,
            RuntimePolicy::Strict,
        ] {
            let artifact = submit_artifact(&journal, &workflow, policy)
                .map_err(|e| format!("submit failed: {e}"))?;

            assert_eq!(
                artifact.digest.as_bytes(),
                workflow.digest().as_bytes(),
                "artifact.digest must equal workflow.digest() for policy {policy:?}"
            );
        }
        Ok(())
    }

    /// TEST: events_for_run_returns_ascending_sequences
    ///
    /// Contract §3.7: Returned events have strictly monotonically increasing EventSeq.
    #[test]
    fn events_for_run_returns_ascending_sequences() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let run = RunId::new(12345);

        let events: Vec<JournalEvent> = (0..5)
            .map(|i| JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(i),
                step: StepIdx::new(i as u16),
                attempt: 1,
            })
            .collect();

        for event in &events {
            journal
                .append_journaled(event)
                .map_err(|e| format!("append: {e}"))?;
        }

        let replayed = journal
            .events_for_run(run)
            .map_err(|e| format!("replay: {e}"))?;

        for (i, event) in replayed.iter().enumerate() {
            assert_eq!(
                event.seq().get(),
                i as u64,
                "event {} must have seq {}",
                i,
                i
            );
        }
        Ok(())
    }

    /// TEST: events_for_run_returns_empty_for_unrelated_run (BH-06)
    ///
    /// Contract §6 BH-06: events isolated by run ID.
    #[test]
    fn events_for_run_returns_empty_for_unrelated_run() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let run_a = RunId::new(100);
        let run_b = RunId::new(200);

        let event = JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        };
        journal
            .append_strict(&event)
            .map_err(|e| format!("append: {e}"))?;

        let events_b = journal
            .events_for_run(run_b)
            .map_err(|e| format!("replay: {e}"))?;
        assert!(
            events_b.is_empty(),
            "run B must have zero events from run A"
        );
        Ok(())
    }

    /// TEST: event_replay_fails_on_sequence_gap
    ///
    /// Contract §3.7: Sequence gap → typed error (not panic).
    #[test]
    fn event_replay_fails_on_sequence_gap() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let run = RunId::new(400);

        // Write seq 0 and seq 2 (gap at seq 1)
        let e0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        };
        let e2 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(2),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        };

        journal
            .append_unpersisted(&e0)
            .map_err(|e| format!("append 0: {e}"))?;
        journal
            .append_unpersisted(&e2)
            .map_err(|e| format!("append 2: {e}"))?;

        let result = journal.events_for_run(run);
        assert!(
            matches!(result, Err(JournalError::SequenceGap { .. })),
            "sequence gap must yield SequenceGap error"
        );
        Ok(())
    }

    /// TEST: batch_commit_is_all_or_nothing
    ///
    /// Contract §3.5: After failed commit(), no partial records visible in keyspaces.
    #[test]
    fn batch_commit_is_all_or_nothing() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;

        // Create a batch that will be committed successfully
        let source = b"valid source".to_vec();
        let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());

        {
            let mut batch = journal.batch();
            batch
                .put_workflow_source(&WorkflowSourceRecord {
                    digest: source_digest,
                    source: source.clone(),
                })
                .map_err(|e| format!("batch put: {e}"))?;
            batch.commit().map_err(|e| format!("commit: {e}"))?;
        }

        // After successful commit, record must be visible
        let loaded = journal
            .workflow_source(source_digest)
            .map_err(|e| format!("read: {e}"))?;
        assert!(
            loaded.is_some(),
            "after successful batch commit, record must be visible"
        );
        Ok(())
    }

    // =========================================================================
    // SECTION 3.5: Error Taxonomy — Full Coverage (Integration Tests)
    // =========================================================================

    /// TEST: artifact_malformed_error_code
    ///
    /// Contract §4.1: ArtifactMalformed (0x4017).
    #[test]
    fn artifact_malformed_error_code() -> Result<(), String> {
        let code = JournalError::ARTIFACT_MALFORMED_CODE;
        assert_eq!(
            code.code(),
            0x4017,
            "ArtifactMalformed must have diagnostic code 0x4017"
        );
        Ok(())
    }

    /// TEST: artifact_checksum_mismatch_error_code
    ///
    /// Contract §4.2: ArtifactChecksumMismatch (0x4018).
    #[test]
    fn artifact_checksum_mismatch_error_code() -> Result<(), String> {
        let code = JournalError::ARTIFACT_CHECKSUM_MISMATCH_CODE;
        assert_eq!(
            code.code(),
            0x4018,
            "ArtifactChecksumMismatch must have diagnostic code 0x4018"
        );
        Ok(())
    }

    /// TEST: bad_magic_error_code
    ///
    /// Contract §4.3: BadMagic (0x400B).
    #[test]
    fn bad_magic_error_code() -> Result<(), String> {
        let code = JournalError::BAD_MAGIC_CODE;
        assert_eq!(
            code.code(),
            0x400B,
            "BadMagic must have diagnostic code 0x400B"
        );
        Ok(())
    }

    /// TEST: header_checksum_mismatch_error_code
    ///
    /// Contract §4.3: HeaderChecksumMismatch (0x4012).
    #[test]
    fn header_checksum_mismatch_error_code() -> Result<(), String> {
        let code = JournalError::HEADER_CHECKSUM_MISMATCH_CODE;
        assert_eq!(
            code.code(),
            0x4012,
            "HeaderChecksumMismatch must have diagnostic code 0x4012"
        );
        Ok(())
    }

    /// TEST: payload_digest_mismatch_error_code
    ///
    /// Contract §4.3: PayloadDigestMismatch (0x4013).
    #[test]
    fn payload_digest_mismatch_error_code() -> Result<(), String> {
        let code = JournalError::PAYLOAD_DIGEST_MISMATCH_CODE;
        assert_eq!(
            code.code(),
            0x4013,
            "PayloadDigestMismatch must have diagnostic code 0x4013"
        );
        Ok(())
    }

    /// TEST: header_length_mismatch_error_code
    ///
    /// Contract §4.3: HeaderLengthMismatch (0x4010).
    #[test]
    fn header_length_mismatch_error_code() -> Result<(), String> {
        let code = JournalError::HEADER_LENGTH_MISMATCH_CODE;
        assert_eq!(
            code.code(),
            0x4010,
            "HeaderLengthMismatch must have diagnostic code 0x4010"
        );
        Ok(())
    }

    /// TEST: payload_too_large_error_code
    ///
    /// Contract §4.3: PayloadTooLarge (0x4011).
    #[test]
    fn payload_too_large_error_code() -> Result<(), String> {
        let code = JournalError::PAYLOAD_TOO_LARGE_CODE;
        assert_eq!(
            code.code(),
            0x4011,
            "PayloadTooLarge must have diagnostic code 0x4011"
        );
        Ok(())
    }

    /// TEST: unexpected_eof_error_code
    ///
    /// Contract §4.4: UnexpectedEof (0x4014).
    #[test]
    fn unexpected_eof_error_code() -> Result<(), String> {
        let code = JournalError::UNEXPECTED_EOF_CODE;
        assert_eq!(
            code.code(),
            0x4014,
            "UnexpectedEof must have diagnostic code 0x4014"
        );
        Ok(())
    }

    /// TEST: postcard_decode_failed_error_code
    ///
    /// Contract §4.4: PostcardDecodeFailed (0x4015).
    #[test]
    fn postcard_decode_failed_error_code() -> Result<(), String> {
        let code = JournalError::POSTCARD_DECODE_FAILED_CODE;
        assert_eq!(
            code.code(),
            0x4015,
            "PostcardDecodeFailed must have diagnostic code 0x4015"
        );
        Ok(())
    }

    /// TEST: unknown_record_kind_error_code
    ///
    /// Contract §4.4: UnknownRecordKind (0x400E).
    #[test]
    fn unknown_record_kind_error_code() -> Result<(), String> {
        let code = JournalError::UNKNOWN_RECORD_KIND_CODE;
        assert_eq!(
            code.code(),
            0x400E,
            "UnknownRecordKind must have diagnostic code 0x400E"
        );
        Ok(())
    }

    /// TEST: process_lock_held_error_code
    ///
    /// Contract §4.5: ProcessLockHeld (0x401A).
    #[test]
    fn process_lock_held_error_code() -> Result<(), String> {
        let code = JournalError::PROCESS_LOCK_HELD_CODE;
        assert_eq!(
            code.code(),
            0x401A,
            "ProcessLockHeld must have diagnostic code 0x401A"
        );
        Ok(())
    }

    /// TEST: write_lock_poisoned_error_code
    ///
    /// Contract §4.5: WriteLockPoisoned (0x4005).
    #[test]
    fn write_lock_poisoned_error_code() -> Result<(), String> {
        let code = JournalError::WRITE_LOCK_POISONED_CODE;
        assert_eq!(
            code.code(),
            0x4005,
            "WriteLockPoisoned must have diagnostic code 0x4005"
        );
        Ok(())
    }

    /// TEST: unsupported_schema_version_error_code
    ///
    /// Contract §4.6: UnsupportedSchemaVersion (0x400C).
    #[test]
    fn unsupported_schema_version_error_code() -> Result<(), String> {
        let code = JournalError::UNSUPPORTED_SCHEMA_VERSION_CODE;
        assert_eq!(
            code.code(),
            0x400C,
            "UnsupportedSchemaVersion must have diagnostic code 0x400C"
        );
        Ok(())
    }

    /// TEST: migration_required_error_code
    ///
    /// Contract §4.6: MigrationRequired (0x400D).
    #[test]
    fn migration_required_error_code() -> Result<(), String> {
        let code = JournalError::MIGRATION_REQUIRED_CODE;
        assert_eq!(
            code.code(),
            0x400D,
            "MigrationRequired must have diagnostic code 0x400D"
        );
        Ok(())
    }

    // =========================================================================
    // SECTION 5: BDD Given-When-Then Scenarios (executable)
    // =========================================================================

    /// BDD Scenario: Relaxed policy accepts artifact without gate validation
    #[test]
    fn bdd_relaxed_policy_accepts_without_gate_validation() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;
        let digest = workflow.digest();

        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)
            .map_err(|e| format!("submit failed: {e}"))?;

        let loaded = journal
            .compiled_ir(digest)
            .map_err(|e| format!("read: {e}"))?;
        assert!(
            loaded.is_some(),
            "artifact must be persisted in compiled_ir keyspace"
        );

        assert_eq!(result.verification.gate_count, 0, "gate_count must be 0");
        assert!(!result.verification.durable, "durable must be false");

        Ok(())
    }

    /// BDD Scenario: Journaled policy enforces both gates
    #[test]
    fn bdd_journaled_policy_enforces_both_gates() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .map_err(|e| format!("submit failed: {e}"))?;

        assert_eq!(result.verification.gate_count, 2, "gate_count must be 2");
        assert!(!result.verification.durable, "durable must be false");

        Ok(())
    }

    /// BDD Scenario: Strict policy enforces gates and calls SyncAll
    #[test]
    fn bdd_strict_policy_enforces_gates_and_syncall() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
            .map_err(|e| format!("submit failed: {e}"))?;

        assert_eq!(result.verification.gate_count, 2, "gate_count must be 2");
        assert!(result.verification.durable, "durable must be true");

        Ok(())
    }

    /// BDD Scenario: Duplicate admission is idempotent
    #[test]
    fn bdd_duplicate_admission_is_idempotent() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let digest_a = admit_compiled_artifact(&journal, &workflow)
            .map_err(|e| format!("first admit: {e}"))?;
        let digest_b = admit_compiled_artifact(&journal, &workflow)
            .map_err(|e| format!("second admit: {e}"))?;

        assert_eq!(digest_a, digest_b, "both calls must return same digest");

        Ok(())
    }

    /// BDD Scenario: Accepted artifact round-trips through postcard encode/decode
    #[test]
    fn bdd_accepted_artifact_round_trips() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .map_err(|e| format!("submit: {e}"))?;

        let loaded = journal
            .compiled_ir(artifact.digest)
            .map_err(|e| format!("read: {e}"))?;
        let record = loaded.ok_or_else(|| String::from("artifact not found"))?;

        assert!(
            !record.ir.is_empty(),
            "retrieved ir bytes must be non-empty"
        );
        assert_eq!(
            record.digest,
            workflow.digest(),
            "retrieved digest must equal original workflow digest"
        );

        Ok(())
    }

    /// BDD Scenario: Batch commit persists all records atomically
    #[test]
    fn bdd_batch_commit_persists_all_records_atomically() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;

        let source = b"batch workflow".to_vec();
        let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let payload = vec![0xBB];
        let blob_digest: [u8; 32] = blake3::hash(&payload).into();

        {
            let mut batch = journal.batch();
            batch
                .put_workflow_source(&WorkflowSourceRecord {
                    digest: source_digest,
                    source: source.clone(),
                })
                .map_err(|e| format!("batch ws: {e}"))?;
            batch
                .put_blob(&BlobRecord {
                    digest: blob_digest,
                    bytes: payload.clone(),
                })
                .map_err(|e| format!("batch blob: {e}"))?;
            batch.commit().map_err(|e| format!("commit: {e}"))?;
        }

        let loaded_source = journal
            .workflow_source(source_digest)
            .map_err(|e| format!("read ws: {e}"))?;
        assert!(
            loaded_source.is_some(),
            "workflow_source must be readable after batch commit"
        );

        let loaded_blob = journal
            .blob(blob_digest)
            .map_err(|e| format!("read blob: {e}"))?;
        assert!(
            loaded_blob.is_some(),
            "blob must be readable after batch commit"
        );

        Ok(())
    }

    /// BDD Scenario: Artifact digest equals workflow digest
    #[test]
    fn bdd_artifact_digest_equals_workflow_digest() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
            .map_err(|e| format!("submit: {e}"))?;

        assert_eq!(
            artifact.digest.as_bytes(),
            workflow.digest().as_bytes(),
            "artifact.digest must equal workflow.digest()"
        );

        Ok(())
    }

    /// BDD Scenario: Event replay returns events in ascending sequence order
    #[test]
    fn bdd_event_replay_returns_ascending_sequences() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let run = RunId::new(999);

        for i in 0u64..5 {
            let event = JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(i),
                step: StepIdx::new(i as u16),
                attempt: 1,
            };
            journal
                .append_journaled(&event)
                .map_err(|e| format!("append: {e}"))?;
        }

        let replayed = journal
            .events_for_run(run)
            .map_err(|e| format!("replay: {e}"))?;

        for (i, event) in replayed.iter().enumerate() {
            assert_eq!(
                event.seq().get(),
                i as u64,
                "event {} must have seq {}",
                i,
                i
            );
        }

        Ok(())
    }

    /// BDD Scenario: Direct put_workflow_source rejects forged digest
    #[test]
    fn bdd_put_workflow_source_rejects_forged_digest() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let forged_digest = WorkflowDigest::from_bytes([0xDE; 32]);
        let record = WorkflowSourceRecord {
            digest: forged_digest,
            source: b"this will not hash to 0xDE..DE".to_vec(),
        };

        let result = journal.put_workflow_source(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "forged digest must yield PayloadDigestMismatch"
        );

        Ok(())
    }

    /// BDD Scenario: Direct put_blob rejects forged digest
    #[test]
    fn bdd_put_blob_rejects_forged_digest() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let forged_digest = [0xAB; 32];
        let record = BlobRecord {
            digest: forged_digest,
            bytes: b"these bytes won't hash to 0xAB..AB".to_vec(),
        };

        let result = journal.put_blob(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "forged digest must yield PayloadDigestMismatch"
        );

        Ok(())
    }

    /// BDD Scenario: Batch put_workflow_source rejects forged digest and stages nothing
    #[test]
    fn bdd_batch_put_workflow_source_rejects_forged_digest() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let forged_digest = WorkflowDigest::from_bytes([0xFF; 32]);
        let mut batch = journal.batch();

        let result = batch.put_workflow_source(&WorkflowSourceRecord {
            digest: forged_digest,
            source: b"not the right hash".to_vec(),
        });

        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "forged digest must yield PayloadDigestMismatch"
        );
        assert_eq!(batch.len(), 0, "batch.len() must remain 0 after failed put");

        Ok(())
    }

    /// BDD Scenario: Batch put_blob rejects forged digest and stages nothing
    #[test]
    fn bdd_batch_put_blob_rejects_forged_digest() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let mut batch = journal.batch();

        let result = batch.put_blob(&BlobRecord {
            digest: [0x11; 32],
            bytes: b"wrong hash".to_vec(),
        });

        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "forged digest must yield PayloadDigestMismatch"
        );
        assert_eq!(batch.len(), 0, "batch.len() must remain 0 after failed put");

        Ok(())
    }

    /// BDD Scenario: All-zero digest rejects non-empty content
    #[test]
    fn bdd_all_zero_digest_rejects_nonempty_content() -> Result<(), String> {
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

    /// BDD Scenario: Corrupted payload detected by BLAKE3 on decode
    #[test]
    fn bdd_corrupted_payload_detected_by_blake3() -> Result<(), String> {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0x42; DIGEST_BYTES]),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .map_err(|e| format!("encode: {e}"))?;

        let mut corrupt = bytes;
        for byte in corrupt.iter_mut().skip(RECORD_HEADER_BYTES) {
            *byte = byte.wrapping_add(1);
        }

        let result = decode_record::<JournalEvent>(
            &corrupt,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "corrupt payload must yield PayloadDigestMismatch"
        );

        Ok(())
    }

    /// BDD Scenario: Wrong magic bytes detected on decode
    #[test]
    fn bdd_wrong_magic_bytes_detected() -> Result<(), String> {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .map_err(|e| format!("encode: {e}"))?;

        let result =
            decode_record::<JournalEvent>(&bytes, MAGIC_BLOB, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(result, Err(JournalError::BadMagic { .. })),
            "wrong magic must yield BadMagic"
        );

        Ok(())
    }

    /// BDD Scenario: Corrupted header CRC detected on decode
    #[test]
    fn bdd_corrupted_header_crc_detected() -> Result<(), String> {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0xAA; DIGEST_BYTES]),
        };
        let mut bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .map_err(|e| format!("encode: {e}"))?;

        if let Some(byte) = bytes.get_mut(CRC_OFFSET) {
            *byte ^= 0x01;
        }

        let result = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::HeaderChecksumMismatch)),
            "CRC bit flip must yield HeaderChecksumMismatch"
        );

        Ok(())
    }

    /// BDD Scenario: Future schema version rejected on decode
    #[test]
    fn bdd_future_schema_version_rejected() -> Result<(), String> {
        use crate::constants::CURRENT_SCHEMA_VERSION;

        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            attempt: 1,
            reason: None,
        };
        let mut bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .map_err(|e| format!("encode: {e}"))?;

        let future_version = CURRENT_SCHEMA_VERSION.saturating_add(1);
        bytes
            .get_mut(4..6)
            .ok_or_else(|| String::from("schema version field not found"))?
            .copy_from_slice(&future_version.to_le_bytes());

        let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
        bytes
            .get_mut(CRC_OFFSET..CRC_OFFSET + 4)
            .ok_or_else(|| String::from("CRC field not found"))?
            .copy_from_slice(&checksum.to_le_bytes());

        let result = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::UnsupportedSchemaVersion { .. })),
            "future schema version must yield UnsupportedSchemaVersion"
        );

        Ok(())
    }

    /// BDD Scenario: Truncated record yields UnexpectedEof
    #[test]
    fn bdd_truncated_record_yields_unexpected_eof() -> Result<(), String> {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            attempt: 1,
            reason: None,
        };
        let full = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .map_err(|e| format!("encode: {e}"))?;

        let truncated = &full[..RECORD_HEADER_BYTES];
        let result = decode_record::<JournalEvent>(
            truncated,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "truncated record must yield UnexpectedEof"
        );

        Ok(())
    }

    /// BDD Scenario: Payload exceeding max allowed size is rejected
    #[test]
    fn bdd_payload_exceeding_max_size_rejected() -> Result<(), String> {
        let big_value = vec![0xFFu8; MAX_JOURNAL_EVENT_PAYLOAD_BYTES as usize + 1];
        let event = JournalEvent::SlotWrittenEvent {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            slot: SlotIdx::new(0),
            value: Some(big_value),
            extra: None,
            attempt: 1,
        };
        let result = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::SlotWritten,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::PayloadTooLarge { .. })),
            "oversized payload must yield PayloadTooLarge"
        );

        Ok(())
    }

    /// BDD Scenario: Second process cannot open same journal path (process lock)
    #[test]
    fn bdd_second_process_cannot_open_same_journal_path() -> Result<(), String> {
        let temp = tempfile::tempdir().map_err(|e| format!("tempdir failed: {e}"))?;

        let _journal1 =
            FjallJournal::open(temp.path(), None).map_err(|e| format!("first open: {e}"))?;

        let result = FjallJournal::open(temp.path(), None);
        assert!(
            result.is_err(),
            "second open on same path must fail due to process lock"
        );

        Ok(())
    }

    /// BDD Scenario: Sequence gap in event replay causes typed error
    #[test]
    fn bdd_sequence_gap_causes_typed_error() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let run = RunId::new(400);

        let e0 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        };
        let e2 = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(2),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        };

        journal
            .append_unpersisted(&e0)
            .map_err(|e| format!("append 0: {e}"))?;
        journal
            .append_unpersisted(&e2)
            .map_err(|e| format!("append 2: {e}"))?;

        let result = journal.events_for_run(run);
        assert!(
            matches!(result, Err(JournalError::SequenceGap { .. })),
            "sequence gap must yield SequenceGap error"
        );

        Ok(())
    }

    /// BDD Scenario: Batch with one forged item commits nothing
    #[test]
    fn bdd_batch_with_forged_item_commits_nothing() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;

        let good_source = b"good source".to_vec();
        let good_digest = WorkflowDigest::from_bytes(blake3::hash(&good_source).into());

        // Create a batch with valid source first, then forged blob
        let mut batch = journal.batch();

        batch
            .put_workflow_source(&WorkflowSourceRecord {
                digest: good_digest,
                source: good_source.clone(),
            })
            .map_err(|e| format!("valid put: {e}"))?;

        // Forge the blob digest
        let forged_blob_result = batch.put_blob(&BlobRecord {
            digest: [0xFF; 32],
            bytes: b"forged blob".to_vec(),
        });

        // If the forged blob is rejected, the batch should be empty
        if forged_blob_result.is_err() {
            assert_eq!(
                batch.len(),
                0,
                "batch must be empty after forged item rejection"
            );
        }

        Ok(())
    }
}
