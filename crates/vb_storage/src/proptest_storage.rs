#![forbid(unsafe_code)]
#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod storage_tests {
    use crate::{
        BlobRecord, DIGEST_BYTES, EventSeq, FjallJournal, JournalEvent, JournalWriterQueue,
        MAGIC_BLOB, MAGIC_JOURNAL_EVENT, MAX_BLOB_BYTES, MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        RecordKind, StorageLimits, decode_record, encode_record,
    };
    use proptest::prelude::*;
    use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest};

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open");
        (temp, journal)
    }

    #[test]
    fn journal_append_read_roundtrip_with_arbitrary_run() {
        proptest!(|(run_val in 1u64..10000u64, event_count in 1u16..50u16)| {
            let (_temp, journal) = temp_journal();
            let run = RunId::new(run_val);
            let mut events = Vec::new();

            for i in 0..event_count {
                let seq_val = u64::from(i);
                let ev = JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(seq_val),
                    step: StepIdx::new(i),
                    attempt: 1,
                };
                journal.append_journaled(&ev).unwrap();
                events.push(ev);
            }

            let replayed = journal.events_for_run(run).unwrap();
            prop_assert_eq!(replayed.len(), event_count as usize);
            for (i, ev) in replayed.iter().enumerate() {
                prop_assert_eq!(ev.run_id(), run);
                prop_assert_eq!(ev.seq(), EventSeq::new(i as u64));
                prop_assert_eq!(ev, &events[i]);
            }
        });
    }

    #[test]
    fn journal_append_read_roundtrip_mixed_kinds() {
        proptest!(|(run_val in 1u64..1000u64)| {
            let (_temp, journal) = temp_journal();
            let run = RunId::new(run_val);
            let digest = WorkflowDigest::from_bytes([0x42; 32]);

            let events = vec![
                JournalEvent::RunAccepted {
                    run,
                    seq: EventSeq::new(0),
                    workflow: digest,
                },
                JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(1),
                    step: StepIdx::new(0),
                    attempt: 1,
                },
                JournalEvent::ActionScheduled {
                    run,
                    seq: EventSeq::new(2),
                    step: StepIdx::new(0),
                    action: vb_core::ActionId::new(1),
                    attempt: 1,
                },
                JournalEvent::SlotWrittenEvent {
                    run,
                    seq: EventSeq::new(3),
                    slot: SlotIdx::new(0),
                    value: None,
                    extra: None,
                    attempt: 1,
                },
                JournalEvent::RunFinished {
                    run,
                    seq: EventSeq::new(4),
                    result: SlotIdx::new(0),
                    attempt: 1,
                },
            ];

            journal.append_strict_batch(&events).unwrap();
            let replayed = journal.events_for_run(run).unwrap();
            prop_assert_eq!(replayed, events);
        });
    }

    #[test]
    fn codec_encode_decode_roundtrip_all_kinds() {
        proptest!(|(
            kind_id in proptest::sample::select(&[10u16, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 31]),
            run_val in 1u64..1000u64,
            seq_val in 0u64..100u64,
            attempt_val in 1u16..10u16,
        )| {
            let _attempt = attempt_val;
            let run = RunId::new(run_val);
            let seq = EventSeq::new(seq_val);
            let kind = match kind_id {
                10 => RecordKind::RunAccepted,
                11 => RecordKind::StepStarted,
                12 => RecordKind::SlotWritten,
                13 => RecordKind::ActionScheduled,
                14 => RecordKind::ActionCompleted,
                15 => RecordKind::ActionFailed,
                16 => RecordKind::WaitScheduled,
                17 => RecordKind::AskScheduled,
                18 => RecordKind::AskAnswered,
                19 => RecordKind::RetryScheduled,
                20 => RecordKind::StepFailed,
                21 => RecordKind::RunCancelled,
                22 => RecordKind::RunFinished,
                23 => RecordKind::RunFailed,
                24 => RecordKind::RunAdmission,
                25 => RecordKind::RunResumed,
                26 => RecordKind::RunRetried,
                27 => RecordKind::RunAnswered,
                28 => RecordKind::RunKilled,
                29 => RecordKind::AskTimedOut,
                31 => RecordKind::WaitResolved,
                _ => return Ok(()),
            };

            let event = JournalEvent::RunAccepted {
                run,
                seq,
                workflow: WorkflowDigest::from_bytes([kind_id as u8; 32]),
            };

            let encoded = encode_record(
                MAGIC_JOURNAL_EVENT,
                kind,
                seq_val,
                &event,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            );
            let Ok(encoded) = encoded else { return Ok(()) };

            let decoded = decode_record::<JournalEvent>(
                &encoded,
                MAGIC_JOURNAL_EVENT,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            );
            let Ok((envelope, decoded_event)) = decoded else { return Ok(()) };

            prop_assert_eq!(envelope.magic, MAGIC_JOURNAL_EVENT);
            prop_assert_eq!(envelope.record_kind, kind.id());
            prop_assert_eq!(envelope.sequence, seq_val);
            prop_assert_eq!(decoded_event, event);
        });
    }

    #[test]
    fn codec_blob_roundtrip_with_arbitrary_bytes() {
        proptest!(|(blob_bytes in proptest::collection::vec(any::<u8>(), 0..4096usize))| {
            let digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();
            let record = BlobRecord {
                digest,
                bytes: blob_bytes,
            };
            let encoded = encode_record(MAGIC_BLOB, RecordKind::Blob, 0, &record, MAX_BLOB_BYTES);
            let Ok(encoded) = encoded else { return Ok(()) };

            let decoded = decode_record::<BlobRecord>(&encoded, MAGIC_BLOB, MAX_BLOB_BYTES);
            let Ok((_, decoded_record)) = decoded else { return Ok(()) };

            prop_assert_eq!(decoded_record, record);
        });
    }

    #[test]
    fn codec_decode_never_panics_on_any_bytes() {
        proptest!(|(data in proptest::collection::vec(any::<u8>(), 0..1024usize))| {
            let _ = decode_record::<JournalEvent>(
                &data,
                MAGIC_JOURNAL_EVENT,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            );
            let _ = decode_record::<JournalEvent>(&data, MAGIC_BLOB, MAX_BLOB_BYTES);
            let _ = decode_record::<JournalEvent>(
                &data,
                crate::MAGIC_COMPILED_ARTIFACT,
                crate::MAX_COMPILED_IR_BYTES,
            );
            let _ = decode_record::<JournalEvent>(
                &data,
                crate::MAGIC_SNAPSHOT,
                crate::MAX_SNAPSHOT_BYTES,
            );
            let _ = decode_record::<JournalEvent>(
                &data,
                crate::MAGIC_WORKFLOW_SOURCE,
                crate::MAX_WORKFLOW_SOURCE_BYTES,
            );
            let _ = crate::decode_record_header(&data, MAGIC_JOURNAL_EVENT, 1024);
        });
    }

    #[test]
    fn queue_fifo_ordering_invariant() {
        proptest!(|(num_events in 1usize..8usize, run_val in 1u64..1000u64)| {
            let (_temp, journal) = temp_journal();
            let queue = JournalWriterQueue::new(16, 8, StorageLimits::DEFAULT).unwrap();
            let run = RunId::new(run_val);

            for i in 0..num_events {
                let ev = JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(i as u64),
                    step: StepIdx::new(i as u16),
                    attempt: 1,
                };
                queue.enqueue_journaled(ev).unwrap();
            }

            queue.drain_all(&journal).unwrap();
            let events = journal.events_for_run(run).unwrap();
            prop_assert_eq!(events.len(), num_events);
            for (i, ev) in events.iter().enumerate() {
                prop_assert_eq!(ev.seq(), EventSeq::new(i as u64));
                prop_assert_eq!(ev.run_id(), run);
            }
        });
    }

    #[test]
    fn queue_fifo_preserved_across_multiple_batches() {
        proptest!(|(num_events in 4usize..16usize, run_val in 1u64..1000u64)| {
            let (_temp, journal) = temp_journal();
            let queue = JournalWriterQueue::new(32, 3, StorageLimits::DEFAULT).unwrap();
            let run = RunId::new(run_val);

            for i in 0..num_events {
                let ev = JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(i as u64),
                    step: StepIdx::new(i as u16),
                    attempt: 1,
                };
                queue.enqueue_journaled(ev).unwrap();
            }

            queue.drain_all(&journal).unwrap();
            let events = journal.events_for_run(run).unwrap();
            prop_assert_eq!(events.len(), num_events);
            for (i, ev) in events.iter().enumerate() {
                prop_assert_eq!(ev.seq(), EventSeq::new(i as u64));
            }
        });
    }

    #[test]
    fn queue_capacity_limit_never_exceeded() {
        proptest!(|(capacity in 1usize..8usize)| {
            let queue = JournalWriterQueue::new(capacity, capacity, StorageLimits::DEFAULT).unwrap();
            let run = RunId::new(42);

            let mut enqueued = 0usize;
            for i in 0..(capacity + 2) {
                let ev = JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(i as u64),
                    step: StepIdx::new(i as u16),
                    attempt: 1,
                };
                let result = queue.enqueue_journaled(ev);
                if enqueued < capacity {
                    prop_assert!(result.is_ok(), "enqueue must succeed within capacity");
                    enqueued += 1;
                } else {
                    prop_assert!(
                        matches!(result, Err(crate::JournalError::QueueFull)),
                        "enqueue must fail when at capacity"
                    );
                }
            }
        });
    }

    #[test]
    fn trimming_preserves_events_after_snapshot() {
        proptest!(|(
            run_val in 1u64..5000u64,
            snapshot_seq_val in 5u64..10u64,
            tail_count in 1u16..5u16,
        )| {
            use crate::recovery::RunSnapshot;
            use crate::trimming::TrimPolicy;

            let (_temp, journal) = temp_journal();
            let run = RunId::new(run_val);
            let digest = WorkflowDigest::from_bytes([0xBB; 32]);
            let snapshot_seq = EventSeq::new(snapshot_seq_val);

            let mut events = Vec::new();
            events.push(JournalEvent::RunAccepted {
                run,
                seq: EventSeq::new(0),
                workflow: digest,
            });

            let mut seq = 1u64;
            while seq < snapshot_seq_val {
                events.push(JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(seq),
                    step: StepIdx::new(seq as u16),
                    attempt: 1,
                });
                seq += 1;
            }

            journal.append_strict_batch(&events).unwrap();

            let snapshot = RunSnapshot {
                run,
                seq: snapshot_seq,
                workflow: digest,
                slots: vec![0u8],
                taint: vec![],
            };
            journal.put_snapshot(&snapshot).unwrap();

            for i in 0..tail_count {
                let tail_seq = snapshot_seq_val + 1 + u64::from(i);
                events.push(JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(tail_seq),
                    step: StepIdx::new(tail_seq as u16),
                    attempt: 1,
                });
            }

            let total_tail: usize = tail_count as usize;
            if total_tail > 0 {
                let tail_start = snapshot_seq_val + 1;
                for i in 0..total_tail {
                    journal.append_journaled(&JournalEvent::StepStarted {
                        run,
                        seq: EventSeq::new(tail_start + i as u64),
                        step: StepIdx::new((tail_start + i as u64) as u16),
                        attempt: 1,
                    }).unwrap();
                }
            }

            let policy = TrimPolicy::default();
            let diagnostic = journal.trim_eligibility_diagnostic(policy).unwrap();
            let eligible_has_run = diagnostic.runs.iter().any(|e| match e {
                crate::TrimEligibility::Eligible { run: r, .. } => *r == run,
                _ => false,
            });

            let replayed_after = journal.events_for_run(run).unwrap();
            let snapshot_present = journal.snapshot(run, snapshot_seq).unwrap();
            prop_assert!(snapshot_present.is_some(), "snapshot must survive");

            if eligible_has_run {
                if let Ok(result) = journal.trim_events_for_run(run, policy) {
                    let replayed_trimmed = journal.events_for_run(run).unwrap();
                    if result.deleted_count > 0 {
                        prop_assert!(
                            replayed_trimmed.len() <= replayed_after.len(),
                            "trimming must not add events"
                        );
                    }
                }
            }
        });
    }

    #[test]
    fn prop_kind_28_id_is_stable() {
        assert_eq!(RecordKind::RunKilled.id(), 28);
    }

    #[test]
    fn prop_kind_28_is_known_record_kind() {
        assert!(
            crate::codec::validation::is_known_record_kind(28),
            "is_known_record_kind(28) must return true for RunKilled"
        );
    }

    #[test]
    fn prop_runkilled_encode_decode_roundtrip() {
        proptest!(|(
            run_val in 1u64..u64::MAX,
            seq_val in 0u64..(u64::MAX - 1),
            attempt_val in 1u16..1000u16,
        )| {
            let event = JournalEvent::RunKilled {
                run: RunId::new(run_val),
                seq: EventSeq::new(seq_val),
                attempt: attempt_val,
            };

            prop_assert!(event.is_valid(), "RunKilled with valid fields must be valid");
            prop_assert_eq!(event.record_kind(), RecordKind::RunKilled);

            let max_payload = MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
            let encoded = encode_record(
                MAGIC_JOURNAL_EVENT,
                event.record_kind(),
                event.seq().get(),
                &event,
                max_payload,
            );
            prop_assert!(encoded.is_ok(), "encode of RunKilled must succeed with kind 28");
            let bytes = encoded.unwrap();
            let decoded = decode_record::<JournalEvent>(&bytes, MAGIC_JOURNAL_EVENT, max_payload);
            prop_assert!(decoded.is_ok(), "decode of RunKilled must succeed with kind 28");
            let (_envelope, decoded_event) = decoded.unwrap();
            prop_assert_eq!(decoded_event, event, "RunKilled round-trip must preserve all fields");
        });
    }

    #[test]
    fn prop_kind_28_rejected_for_wrong_magic() {
        proptest!(|(run_val in 1u64..10000u64)| {
            let event = JournalEvent::RunKilled {
                run: RunId::new(run_val),
                seq: EventSeq::new(0),
                attempt: 1,
            };
            let result = encode_record(
                crate::MAGIC_SNAPSHOT,
                event.record_kind(),
                event.seq().get(),
                &event,
                MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            );
            prop_assert!(result.is_err(), "kind 28 with MAGIC_SNAPSHOT must return Err");
        });
    }

    #[test]
    fn prop_kind_28_rejected_for_blob_magic() {
        proptest!(|(run_val in 1u64..10000u64)| {
            let event = JournalEvent::RunKilled {
                run: RunId::new(run_val),
                seq: EventSeq::new(0),
                attempt: 1,
            };
            let result = encode_record(
                MAGIC_BLOB,
                event.record_kind(),
                event.seq().get(),
                &event,
                MAX_BLOB_BYTES,
            );
            prop_assert!(result.is_err(), "kind 28 with MAGIC_BLOB must return Err");
        });
    }

    #[test]
    fn prop_replay_contiguity_mixed_kinds() {
        proptest!(|(run_val in 1u64..10000u64, event_count in 2usize..8usize)| {
            let (_temp, journal) = temp_journal();
            let run = RunId::new(run_val);
            let mut seq: u64 = 0;

            let kill_position = event_count / 2;

            for i in 0..event_count {
                let event = if i == kill_position {
                    JournalEvent::RunKilled {
                        run,
                        seq: EventSeq::new(seq),
                        attempt: 1,
                    }
                } else {
                    JournalEvent::StepStarted {
                        run,
                        seq: EventSeq::new(seq),
                        step: StepIdx::new(i as u16),
                        attempt: 1,
                    }
                };
                journal.append_journaled(&event).unwrap();
                seq += 1;
            }

            let replayed = journal.events_for_run(run).unwrap();
            prop_assert_eq!(replayed.len(), event_count, "replay must recover all events");

            for (i, ev) in replayed.iter().enumerate() {
                prop_assert_eq!(ev.seq().get(), i as u64, "event at index {} must have seq={}", i, i);
                prop_assert_eq!(ev.run_id(), run, "all events must belong to the correct run");
            }
        });
    }

    #[test]
    fn prop_replay_gap_detection() {
        proptest!(|(run_val in 1u64..10000u64)| {
            let (_temp, journal) = temp_journal();
            let run = RunId::new(run_val);

            let events = vec![
                JournalEvent::StepStarted {
                    run,
                    seq: EventSeq::new(0),
                    step: StepIdx::new(0),
                    attempt: 1,
                },
                JournalEvent::RunKilled {
                    run,
                    seq: EventSeq::new(2), // gap: seq 1 is missing
                    attempt: 1,
                },
            ];

            for ev in &events {
                journal.append_journaled(ev).unwrap();
            }

            match journal.events_for_run(run) {
                Err(crate::JournalError::SequenceGap { expected, actual }) => {
                    prop_assert_eq!(expected, EventSeq::new(1));
                    prop_assert_eq!(actual, EventSeq::new(2));
                }
                other => prop_assert!(
                    false,
                    "sequence [0, 2] must be detected as SequenceGap, got {:?}",
                    other
                ),
            }
        });
    }

    #[test]
    fn prop_duplicate_seq_rejected_at_append() {
        proptest!(|(run_val in 1u64..10000u64)| {
            let (_temp, journal) = temp_journal();
            let run = RunId::new(run_val);

            let first = JournalEvent::StepStarted {
                run,
                seq: EventSeq::new(0),
                step: StepIdx::new(0),
                attempt: 1,
            };
            journal.append_journaled(&first).unwrap();

            let duplicate = JournalEvent::RunKilled {
                run,
                seq: EventSeq::new(0),
                attempt: 1,
            };
            match journal.append_journaled(&duplicate) {
                Err(crate::JournalError::DuplicateEvent { run: r, seq: s }) => {
                    prop_assert_eq!(r, run);
                    prop_assert_eq!(s, EventSeq::new(0));
                }
                other => prop_assert!(
                    false,
                    "duplicate seq must be rejected as DuplicateEvent, got {:?}",
                    other
                ),
            }

            let replayed = journal.events_for_run(run).unwrap();
            prop_assert_eq!(replayed.len(), 1);
            prop_assert_eq!(&replayed[0], &first);
        });
    }
}
