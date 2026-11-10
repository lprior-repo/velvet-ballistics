    // =========================================================================
    // Section: Adversarial Key Encoding Tests
    // =========================================================================

    #[test]
    fn adversarial_key_prefix_isolation_proves_different_prefixes() {
        let digest = [0xAB; 32];
        let ws = workflow_source_key(digest).expect("ws");
        let ci = compiled_ir_key(digest).expect("ci");
        let bl = blob_key(digest).expect("bl");
        assert_ne!(ws[0], ci[0]);
        assert_ne!(ws[0], bl[0]);
        assert_eq!(ws[1..], ci[1..]);
        assert_eq!(ws[1..], bl[1..]);
    }

    #[test]
    fn adversarial_key_wrong_endianness_produces_different_keys() {
        let key = run_header_key(RunId::new(1)).expect("key");
        let mut le = [0u8; 9];
        le[0] = PREFIX_RUN_HEADER;
        le[1..9].copy_from_slice(&1u64.to_le_bytes());
        assert_ne!(key.as_slice(), le.as_slice());
        assert_eq!(key[1..9], 1u64.to_be_bytes());
    }

    #[test]
    fn adversarial_key_no_collision_different_runs_same_seq() {
        let k1 = run_event_key(RunId::new(100), EventSeq::new(5)).expect("k1");
        let k2 = run_event_key(RunId::new(200), EventSeq::new(5)).expect("k2");
        assert_ne!(k1.as_slice(), k2.as_slice());
    }

    #[test]
    fn adversarial_key_no_collision_same_run_different_seq() {
        let k1 = run_event_key(RunId::new(100), EventSeq::new(0)).expect("k1");
        let k2 = run_event_key(RunId::new(100), EventSeq::new(1)).expect("k2");
        assert_ne!(k1.as_slice(), k2.as_slice());
    }

    #[test]
    fn adversarial_key_no_collision_different_digests() {
        assert_ne!(
            blob_key([1u8; 32]).expect("k1").as_slice(),
            blob_key([2u8; 32]).expect("k2").as_slice()
        );
    }

    // =========================================================================
    // Section: Adversarial Journal / Replay Tests
    // =========================================================================

    #[test]
    fn adversarial_append_duplicate_sequence_rejected_with_exact_fields() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let run = RunId::new(50);
        assert!(
            journal
                .append_journaled(&JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: test_digest(1)
                })
                .is_ok()
        );
        let result = journal.append_journaled(&JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(0),
            step: StepIdx::new(0),
            attempt: 1,
        });
        let Err(JournalError::DuplicateEvent { run: r, seq: s }) = result else {
            panic!("expected DuplicateEvent, got {:?}", result)
        };
        assert_eq!(r, run);
        assert_eq!(s, EventSeq::new(0));
    }

    #[test]
    fn adversarial_read_events_with_sequence_gap_returns_exact_gap() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let run = RunId::new(777);
        assert!(
            journal
                .append_journaled(&JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: test_digest(1)
                })
                .is_ok()
        );
        assert!(
            journal
                .append_journaled(&JournalEvent::RunFinished {
                    run,
                    seq: EventSeq::new(5),
                    result: vb_core::SlotIdx::new(0),
                    attempt: 1,
                })
                .is_ok()
        );
        let Err(JournalError::SequenceGap { expected, actual }) = journal.events_for_run(run)
        else {
            panic!("expected SequenceGap")
        };
        assert_eq!(expected, EventSeq::new(1));
        assert_eq!(actual, EventSeq::new(5));
    }

    // =========================================================================
    // Section: Adversarial Blob / Snapshot / Size Boundary Tests
    // =========================================================================

    #[test]
    fn adversarial_put_blob_exceeding_max_returns_payload_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let bytes = vec![0u8; (MAX_BLOB_BYTES as usize).saturating_add(1)];
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&bytes).into();
        let record = BlobRecord { digest, bytes };
        assert!(matches!(
            journal.put_blob(&record),
            Err(JournalError::PayloadTooLarge { .. })
        ));
    }

    #[test]
    fn adversarial_blob_zero_length_round_trips() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let bytes: Vec<u8> = vec![];
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&bytes).into();
        let record = BlobRecord {
            digest,
            bytes: bytes.clone(),
        };
        journal
            .put_blob(&record)
            .expect("journal.put_blob must succeed");
        assert_eq!(journal.blob(digest).expect("ok"), Some(record));
    }

    #[test]
    fn adversarial_snapshot_exceeding_max_returns_payload_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let snap = RunSnapshot {
            run: RunId::new(888),
            seq: EventSeq::new(0),
            workflow: test_digest(1),
            slots: vec![0u8; (MAX_SNAPSHOT_BYTES as usize).saturating_add(1)],
            taint: Vec::new(),
        };
        assert!(matches!(
            journal.put_snapshot(&snap),
            Err(JournalError::PayloadTooLarge { .. })
        ));
    }

    #[test]
    fn adversarial_snapshot_corrupt_magic_returns_bad_magic() {
        let snap = RunSnapshot {
            run: RunId::new(889),
            seq: EventSeq::new(0),
            workflow: test_digest(1),
            slots: vec![1, 2, 3],
            taint: Vec::new(),
        };
        let mut enc = encode_record(
            MAGIC_SNAPSHOT,
            RecordKind::Snapshot,
            snap.seq.get(),
            &snap,
            MAX_SNAPSHOT_BYTES,
        )
        .expect("ok");
        if let Some(b) = enc.get_mut(0) {
            *b ^= 0xFF;
        }
        assert!(matches!(
            decode_record::<RunSnapshot>(&enc, MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES),
            Err(JournalError::BadMagic { .. })
        ));
    }

    #[test]
    fn adversarial_workflow_source_exceeding_max_returns_payload_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let source = vec![0u8; (MAX_WORKFLOW_SOURCE_BYTES as usize).saturating_add(1)];
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord { digest, source };
        assert!(matches!(
            journal.put_workflow_source(&record),
            Err(JournalError::PayloadTooLarge { .. })
        ));
    }

    #[test]
    fn adversarial_compiled_ir_exceeding_max_returns_payload_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let oversized_ir = vec![0u8; (MAX_COMPILED_IR_BYTES as usize).saturating_add(1)];
        let record = CompiledIrRecord {
            digest: compiled_ir_digest(&oversized_ir),
            ir: oversized_ir,
        };
        assert!(matches!(
            journal.put_compiled_ir(&record),
            Err(JournalError::PayloadTooLarge { .. })
        ));
    }

    // =========================================================================
    // Section: Adversarial Schema Migration Tests
    // =========================================================================

    #[test]
    fn adversarial_schema_migration_from_zero_exact_fields() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(11),
            seq: EventSeq::new(0),
            workflow: test_digest(11),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 4, &0u16.to_le_bytes());
        let Err(JournalError::MigrationRequired { from, to }) =
            decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        else {
            panic!("expected MigrationRequired")
        };
        assert_eq!(from, 0);
        assert_eq!(to, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn adversarial_schema_future_version_max_unsupported() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(12),
            seq: EventSeq::new(0),
            workflow: test_digest(12),
        };
        let encoded =
            encode_and_patch_field(&event, RecordKind::RunAccepted, 4, &u16::MAX.to_le_bytes());
        let Err(JournalError::UnsupportedSchemaVersion { version }) =
            decode_record::<JournalEvent>(&encoded, MAGIC_JOURNAL_EVENT, 128)
        else {
            panic!("expected UnsupportedSchemaVersion")
        };
        assert_eq!(version, u16::MAX);
    }

    // =========================================================================
    // Section: Adversarial Queue Tests
    // =========================================================================

    #[test]
    fn adversarial_queue_zero_capacity_returns_queue_capacity() {
        assert!(matches!(
            JournalWriterQueue::new(0, 1, StorageLimits::DEFAULT),
            Err(JournalError::QueueCapacity)
        ));
    }

    #[test]
    fn adversarial_queue_zero_batch_returns_queue_capacity() {
        assert!(matches!(
            JournalWriterQueue::new(1, 0, StorageLimits::DEFAULT),
            Err(JournalError::QueueCapacity)
        ));
    }

    #[test]
    fn adversarial_queue_full_returns_queue_full() {
        let queue = JournalWriterQueue::new(1, 1, StorageLimits::DEFAULT).expect("q");
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: test_digest(1),
        };
        queue
            .enqueue_journaled(event.clone())
            .expect("queue.enqueue_journaled must succeed");
        assert!(matches!(
            queue.enqueue_journaled(event),
            Err(JournalError::QueueFull)
        ));
    }

    #[test]
    fn journal_writer_queue_drain_all_flushes_until_empty() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let queue = JournalWriterQueue::new(4, 1, StorageLimits::DEFAULT).expect("q");
        let run = RunId::new(2);
        let workflow = test_digest(2);

        assert!(
            queue
                .enqueue_journaled(JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow,
                })
                .is_ok()
        );
        assert!(
            queue
                .enqueue_journaled(JournalEvent::RunCancelled {
                    run,
                    seq: EventSeq::new(1),
                    attempt: 1,
                    reason: None,
                })
                .is_ok()
        );

        assert!(matches!(
            queue.drain_all(&journal),
            Ok(report) if report.drained == 2 && report.written == 2
        ));
        assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() == 2));
    }

    #[test]
    fn journal_writer_queue_retains_events_when_append_fails() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT).expect("q");
        let run = RunId::new(3);
        let duplicate = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(3),
        };
        let conflicting_duplicate = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(33),
        };
        let next = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        };

        assert!(matches!(journal.append_journaled(&duplicate), Ok(())));
        assert!(matches!(
            queue.enqueue_journaled(conflicting_duplicate),
            Ok(())
        ));
        assert!(matches!(queue.enqueue_journaled(next), Ok(())));

        assert!(matches!(
            queue.flush_batch(&journal),
            Err(JournalError::DuplicateEvent { run: found, seq })
                if found == run && seq == EventSeq::new(0)
        ));
        assert!(matches!(
            queue.pending_profile_counts(),
            Ok(counts) if counts.journaled == 2 && counts.strict == 0
        ));
    }

    #[test]
    fn journal_writer_queue_rejects_same_flush_duplicate_keys() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT).expect("q");
        let run = RunId::new(30_010);
        let first = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(10),
        };
        let duplicate_key = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(11),
        };

        assert!(matches!(queue.enqueue_journaled(first), Ok(())));
        assert!(matches!(queue.enqueue_journaled(duplicate_key), Ok(())));

        assert!(matches!(
            queue.flush_batch(&journal),
            Err(JournalError::DuplicateStagedKey { run: found, seq })
                if found == run && seq == EventSeq::new(0)
        ));
        assert!(matches!(journal.events_for_run(run), Ok(events) if events.is_empty()));
        assert!(matches!(journal.run_header(run), Ok(None)));
        assert!(matches!(
            queue.pending_profile_counts(),
            Ok(counts) if counts.journaled == 2 && counts.strict == 0
        ));
    }

    #[test]
    fn journal_writer_queue_flush_persists_journaled_events_before_drain() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let path = temp_dir.path().to_path_buf();
        let journal = FjallJournal::open(&path, None).expect("opens");
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT).expect("q");
        let run = RunId::new(4);
        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(4),
        };
        let cancelled = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        };

        queue
            .enqueue_journaled(accepted)
            .expect("queue.enqueue_journaled must succeed");
        queue
            .enqueue_journaled(cancelled)
            .expect("queue.enqueue_journaled must succeed");
        assert!(matches!(
            queue.flush_batch(&journal),
            Ok(report) if report.drained == 2 && report.written == 2
        ));
        drop(journal);

        let reopened = FjallJournal::open(&path, None).expect("reopen");
        assert!(matches!(reopened.events_for_run(run), Ok(events) if events.len() == 2));
    }

    #[test]
    fn journal_writer_queue_shutdown_rejects_new_writes_after_durable_drain() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let queue = JournalWriterQueue::new(4, 1, StorageLimits::DEFAULT).expect("q");
        let run = RunId::new(5);
        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(5),
        };
        let cancelled = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        };

        queue
            .enqueue_journaled(accepted.clone())
            .expect("queue.enqueue_journaled must succeed");
        queue
            .enqueue_strict(cancelled)
            .expect("queue.enqueue_strict must succeed");
        assert!(matches!(
            queue.shutdown(&journal),
            Ok(report) if report.drained == 2 && report.written == 2
        ));
        assert!(matches!(
            queue.enqueue_journaled(accepted),
            Err(JournalError::QueueShutdown)
        ));
        assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() == 2));
    }

    #[test]
    fn journal_writer_queue_crash_window_retry_drains_already_written_same_event() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp_dir.path(), None).expect("opens");
        let queue = JournalWriterQueue::new(4, 2, StorageLimits::DEFAULT).expect("q");
        let run = RunId::new(6);
        let accepted = JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(0),
            workflow: test_digest(6),
        };
        let cancelled = JournalEvent::RunCancelled {
            run,
            seq: EventSeq::new(1),
            attempt: 1,
            reason: None,
        };

        journal
            .append_journaled(&accepted)
            .expect("journal.append_journaled must succeed");
        queue
            .enqueue_journaled(accepted)
            .expect("queue.enqueue_journaled must succeed");
        queue
            .enqueue_journaled(cancelled)
            .expect("queue.enqueue_journaled must succeed");

        // This models the crash window where a prior attempt reached Fjall before
        // the queue could durably drain. Retrying accepts the identical event only.
        assert!(matches!(
            queue.flush_batch(&journal),
            Ok(report) if report.drained == 2 && report.written == 2
        ));
        assert!(matches!(
            queue.pending_profile_counts(),
            Ok(counts) if counts.journaled == 0 && counts.strict == 0
        ));
        assert!(matches!(journal.events_for_run(run), Ok(events) if events.len() == 2));
    }

    // =========================================================================
    // Section: Adversarial Postcard / Encoding Edge Cases
    // =========================================================================

    #[test]
    fn adversarial_valid_header_garbage_postcard_returns_decode_failed() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(13),
            seq: EventSeq::new(0),
            workflow: test_digest(13),
        };
        let mut enc = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            event.seq().get(),
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("ok");
        if let Some(b) = enc.get_mut(60) {
            *b = 0xFF;
        }
        let digest_bytes = *blake3::hash(&enc[60..]).as_bytes();
        enc.get_mut(24..56)
            .expect("digest")
            .copy_from_slice(&digest_bytes);
        let cs = crc32c::crc32c(&enc[..56]);
        enc[56] = (cs & 0xFF) as u8;
        enc[57] = ((cs >> 8) & 0xFF) as u8;
        enc[58] = ((cs >> 16) & 0xFF) as u8;
        enc[59] = ((cs >> 24) & 0xFF) as u8;
        assert!(matches!(
            decode_record::<JournalEvent>(&enc, MAGIC_JOURNAL_EVENT, 128),
            Err(JournalError::PostcardDecodeFailed(_))
        ));
    }

    #[test]
    fn adversarial_run_header_wrong_magic_returns_bad_magic() {
        let record = RunHeaderRecord {
            run: RunId::new(123),
            workflow_id: WorkflowId::new(456),
            compiled_digest: test_digest(8),
            status: 1,
            accepted_at_ms: 1700000000,
        };
        let enc = encode_record(
            MAGIC_INDEX_RECORD,
            RecordKind::RunHeader,
            record.run.get(),
            &record,
            MAX_RUN_HEADER_BYTES,
        )
        .expect("ok");
        assert!(matches!(
            decode_record::<RunHeaderRecord>(&enc, MAGIC_BLOB, MAX_RUN_HEADER_BYTES),
            Err(JournalError::BadMagic { .. })
        ));
    }

    #[test]
    fn adversarial_decode_empty_returns_unexpected_eof() {
        assert!(matches!(
            decode_record::<JournalEvent>(&[][..], MAGIC_JOURNAL_EVENT, 128),
            Err(JournalError::UnexpectedEof)
        ));
    }

    #[test]
    fn adversarial_encode_empty_blob_succeeds() {
        assert!(
            encode_record(
                MAGIC_BLOB,
                RecordKind::Blob,
                0,
                &BlobRecord {
                    digest: [0; 32],
                    bytes: vec![]
                },
                MAX_BLOB_BYTES
            )
            .is_ok()
        );
    }

    #[test]
    fn adversarial_encode_empty_source_succeeds() {
        assert!(
            encode_record(
                MAGIC_WORKFLOW_SOURCE,
                RecordKind::WorkflowSource,
                0,
                &WorkflowSourceRecord {
                    digest: test_digest(0),
                    source: vec![]
                },
                MAX_WORKFLOW_SOURCE_BYTES
            )
            .is_ok()
        );
    }

    #[test]
    fn adversarial_encode_empty_ir_succeeds() {
        assert!(
            encode_record(
                MAGIC_COMPILED_ARTIFACT,
                RecordKind::CompiledIr,
                0,
                &CompiledIrRecord {
                    digest: test_digest(0),
                    ir: vec![]
                },
                MAX_COMPILED_IR_BYTES
            )
            .is_ok()
        );
    }

    #[test]
    fn journal_error_diagnostic_codes_are_unique() {
        let errors = [
            JournalError::KeyCapacity,
            JournalError::WriteLockPoisoned,
            JournalError::QueueCapacity,
            JournalError::QueueFull,
            JournalError::SequenceOverflow,
            JournalError::HeaderChecksumMismatch,
            JournalError::PayloadDigestMismatch,
            JournalError::UnexpectedEof,
            JournalError::PostcardDecodeFailed(postcard::Error::DeserializeBadVarint),
            JournalError::DuplicateEvent {
                run: RunId::new(1),
                seq: EventSeq::new(0),
            },
            JournalError::WrongRun {
                expected: RunId::new(1),
                actual: RunId::new(2),
            },
            JournalError::SequenceGap {
                expected: EventSeq::new(0),
                actual: EventSeq::new(1),
            },
            JournalError::BadMagic { found: 0 },
            JournalError::UnsupportedSchemaVersion { version: 0 },
            JournalError::MigrationRequired { from: 0, to: 1 },
            JournalError::UnknownRecordKind { kind: 0 },
            JournalError::RecordKindFamilyMismatch { magic: 0, kind: 0 },
            JournalError::HeaderLengthMismatch { found: 0 },
            JournalError::PayloadTooLarge { len: 0, max: 0 },
            JournalError::TooManyEvents {
                run: RunId::new(1),
                limit: 1,
                observed: 2,
            },
            JournalError::ReplayAllocationFailed {
                run: RunId::new(1),
                requested: 1,
            },
        ];
        let mut seen = std::collections::BTreeSet::new();
        for err in &errors {
            let code = err.diagnostic_code();
            assert!(seen.insert(code), "duplicate diagnostic code: {code}");
        }
        assert_eq!(seen.len(), errors.len());
    }

    #[test]
    fn journal_error_diagnostic_code_fjall() {
        // Fjall and Encode variants hold external errors; we verify via KeyCapacity
        assert_eq!(
            JournalError::KeyCapacity.diagnostic_code(),
            DiagnosticCode::new(0x4003)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_duplicate_event() {
        assert_eq!(
            JournalError::DuplicateEvent {
                run: RunId::new(42),
                seq: EventSeq::new(7),
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x4004)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_write_lock_poisoned() {
        assert_eq!(
            JournalError::WriteLockPoisoned.diagnostic_code(),
            DiagnosticCode::new(0x4005)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_queue_capacity() {
        assert_eq!(
            JournalError::QueueCapacity.diagnostic_code(),
            DiagnosticCode::new(0x4006)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_queue_full() {
        assert_eq!(
            JournalError::QueueFull.diagnostic_code(),
            DiagnosticCode::new(0x4007)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_wrong_run() {
        assert_eq!(
            JournalError::WrongRun {
                expected: RunId::new(1),
                actual: RunId::new(2),
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x4008)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_sequence_gap() {
        assert_eq!(
            JournalError::SequenceGap {
                expected: EventSeq::new(0),
                actual: EventSeq::new(1),
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x4009)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_sequence_overflow() {
        assert_eq!(
            JournalError::SequenceOverflow.diagnostic_code(),
            DiagnosticCode::new(0x400A)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_bad_magic() {
        assert_eq!(
            JournalError::BadMagic { found: 0xDEAD_BEEF }.diagnostic_code(),
            DiagnosticCode::new(0x400B)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_unsupported_schema_version() {
        assert_eq!(
            JournalError::UnsupportedSchemaVersion { version: 99 }.diagnostic_code(),
            DiagnosticCode::new(0x400C)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_migration_required() {
        assert_eq!(
            JournalError::MigrationRequired { from: 0, to: 1 }.diagnostic_code(),
            DiagnosticCode::new(0x400D)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_unknown_record_kind() {
        assert_eq!(
            JournalError::UnknownRecordKind { kind: 200 }.diagnostic_code(),
            DiagnosticCode::new(0x400E)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_record_kind_family_mismatch() {
        assert_eq!(
            JournalError::RecordKindFamilyMismatch {
                magic: MAGIC_JOURNAL_EVENT,
                kind: 1,
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x400F)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_header_length_mismatch() {
        assert_eq!(
            JournalError::HeaderLengthMismatch { found: 99 }.diagnostic_code(),
            DiagnosticCode::new(0x4010)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_payload_too_large() {
        assert_eq!(
            JournalError::PayloadTooLarge { len: 200, max: 10 }.diagnostic_code(),
            DiagnosticCode::new(0x4011)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_header_checksum_mismatch() {
        assert_eq!(
            JournalError::HeaderChecksumMismatch.diagnostic_code(),
            DiagnosticCode::new(0x4012)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_payload_digest_mismatch() {
        assert_eq!(
            JournalError::PayloadDigestMismatch.diagnostic_code(),
            DiagnosticCode::new(0x4013)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_unexpected_eof() {
        assert_eq!(
            JournalError::UnexpectedEof.diagnostic_code(),
            DiagnosticCode::new(0x4014)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_postcard_decode_failed() {
        assert_eq!(
            JournalError::PostcardDecodeFailed(postcard::Error::DeserializeBadVarint)
                .diagnostic_code(),
            DiagnosticCode::new(0x4015)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_too_many_events() {
        assert_eq!(
            JournalError::TooManyEvents {
                run: RunId::new(7),
                limit: 1,
                observed: 2,
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x401E)
        );
    }

    #[test]
    fn journal_error_diagnostic_code_replay_allocation_failed() {
        assert_eq!(
            JournalError::ReplayAllocationFailed {
                run: RunId::new(7),
                requested: 2,
            }
            .diagnostic_code(),
            DiagnosticCode::new(0x401F)
        );
    }
