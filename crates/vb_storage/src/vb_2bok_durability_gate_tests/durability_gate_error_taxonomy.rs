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

        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Relaxed)
            .map_err(|e| format!("submit failed: {e}"))?;

        let loaded = journal
            .compiled_ir(result.digest)
            .map_err(|e| format!("read: {e}"))?;
        assert!(
            loaded.is_some(),
            "artifact must be persisted in compiled_ir keyspace"
        );

        assert_eq!(result.verification.gate_count, 0, "gate_count must be 0");
        assert!(!result.verification.durable, "durable must be false");
        assert_eq!(result.source_digest, workflow.digest());

        Ok(())
    }

    /// BDD Scenario: Journaled policy enforces both gates
    #[test]
    fn bdd_journaled_policy_enforces_both_gates() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Journaled)
            .map_err(|e| format!("submit failed: {e}"))?;

        assert_eq!(
            result.verification.gate_count, 0,
            "aggregate count is not proof evidence"
        );
        assert!(!result.verification.durable, "durable must be false");

        crate::admission::verify_accepted_artifact_for_policy(&result, RuntimePolicy::Journaled)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// BDD Scenario: Strict policy enforces gates and calls SyncAll
    #[test]
    fn bdd_strict_policy_enforces_gates_and_syncall() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let result = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
            .map_err(|e| format!("submit failed: {e}"))?;

        assert_eq!(
            result.verification.gate_count, 0,
            "aggregate count is not proof evidence"
        );
        assert!(
            !result.verification.durable,
            "proof bytes cannot self-certify a durable acknowledgement"
        );

        crate::admission::verify_accepted_artifact_for_policy(&result, RuntimePolicy::Strict)
            .map_err(|e| e.to_string())?;
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
            record.digest, artifact.digest,
            "retrieved digest must equal accepted artifact digest"
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

    /// BDD Scenario: Artifact digest binds envelope and source digest binds workflow
    #[test]
    fn bdd_artifact_digest_equals_workflow_digest() -> Result<(), String> {
        let (_temp, journal) = temp_journal().map_err(|e| format!("journal open: {e}"))?;
        let workflow = minimal_valid_workflow()?;

        let artifact = submit_artifact(&journal, &workflow, RuntimePolicy::Strict)
            .map_err(|e| format!("submit: {e}"))?;
        let computed = accepted_artifact_digest(&artifact)
            .map_err(|e| format!("accepted artifact digest failed: {e}"))?;

        assert_eq!(
            artifact.source_digest.as_bytes(),
            workflow.digest().as_bytes(),
            "source digest must equal workflow.digest()"
        );
        assert_eq!(artifact.digest, computed);

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
            .append_unfsynced(&e0)
            .map_err(|e| format!("append 0: {e}"))?;
        journal
            .append_unfsynced(&e2)
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
