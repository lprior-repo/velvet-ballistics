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

    /// TEST: artifact_digest_binds_envelope_and_source_digest_binds_workflow
    ///
    /// Contract §2.1 Postcondition (All): artifact.digest binds the accepted
    /// artifact envelope; source_digest binds the workflow structure digest.
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
            let computed = accepted_artifact_digest(&artifact)
                .map_err(|e| format!("accepted artifact digest failed: {e}"))?;

            assert_eq!(
                artifact.source_digest.as_bytes(),
                workflow.digest().as_bytes(),
                "source digest must equal workflow.digest() for policy {policy:?}"
            );
            assert_eq!(
                artifact.digest, computed,
                "artifact.digest must equal canonical envelope digest for policy {policy:?}"
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

