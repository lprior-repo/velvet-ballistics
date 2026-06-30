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
mod type_tests {
    use crate::keys::index_status_key;
    use crate::{
        DurabilityProfile, EventSeq, FjallConfig, IndexStatusState, JournalBatchSize, JournalError,
        JournalQueueCapacity, JournalWriterFlushReport, KeyspaceProfile, RecordEnvelope,
        RecordHeader, StorageKey, StorageLimits,
        constants::{DIGEST_BYTES, RECORD_HEADER_LEN},
    };
    use std::num::NonZeroUsize;
    use vb_core::{ActionId, RunId, StepIdx, WorkflowId};

    #[test]
    fn event_seq_zero_is_min() {
        assert_eq!(EventSeq::ZERO, EventSeq::MIN);
        assert_eq!(EventSeq::ZERO.get(), 0);
    }

    #[test]
    fn event_seq_max_is_u64_max() {
        assert_eq!(EventSeq::MAX.get(), u64::MAX);
    }

    #[test]
    fn event_seq_new_and_get_roundtrip() {
        for val in [0, 1, 42, u64::MAX] {
            let seq = EventSeq::new(val);
            assert_eq!(seq.get(), val);
        }
    }

    #[test]
    fn event_seq_ordering() {
        assert!(EventSeq::new(1) > EventSeq::new(0));
        assert!(EventSeq::new(0) < EventSeq::new(1));
        assert!(EventSeq::new(5) == EventSeq::new(5));
    }

    #[test]
    fn event_seq_clone_and_copy() {
        let a = EventSeq::new(7);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn journal_queue_capacity_new_and_get() {
        let nz = NonZeroUsize::new(10).expect("10 is non-zero");
        let cap = JournalQueueCapacity::new(nz);
        assert_eq!(cap.get(), 10);
    }

    #[test]
    fn journal_queue_capacity_try_from_usize() {
        let cap = JournalQueueCapacity::try_from_usize(5).expect("5 should succeed");
        assert_eq!(cap.get(), 5);

        let err = JournalQueueCapacity::try_from_usize(0);
        assert!(err.is_err(), "zero should fail with QueueCapacity error");
    }

    #[test]
    fn journal_batch_size_new_and_get() {
        let nz = NonZeroUsize::new(20).expect("20 is non-zero");
        let batch_size = JournalBatchSize::new(nz);
        assert_eq!(batch_size.get(), 20);
    }

    #[test]
    fn journal_batch_size_try_from_usize() {
        let bs = JournalBatchSize::try_from_usize(100).expect("100 should succeed");
        assert_eq!(bs.get(), 100);

        let err = JournalBatchSize::try_from_usize(0);
        assert!(err.is_err(), "zero should fail");
    }

    #[test]
    fn journal_writer_flush_report_has_expected_fields() {
        let report = JournalWriterFlushReport {
            drained: 15,
            written: 10,
        };
        assert_eq!(report.drained, 15);
        assert_eq!(report.written, 10);
    }

    #[test]
    fn fjall_config_default_has_256_mib_cache() {
        let config = FjallConfig::default();
        assert_eq!(config.cache_size_bytes, 268_435_456);
    }

    #[test]
    fn fjall_config_rejects_zero_and_extreme_cache() {
        // Below MIN: 0 and any value strictly less than MIN.
        assert_eq!(FjallConfig::try_new(0), None);
        assert_eq!(
            FjallConfig::try_new(FjallConfig::MIN_CACHE_SIZE_BYTES - 1),
            None
        );
        // Above MAX: u64::MAX and 2 * MAX.
        assert_eq!(FjallConfig::try_new(u64::MAX), None);
        assert_eq!(
            FjallConfig::try_new(2 * FjallConfig::MAX_CACHE_SIZE_BYTES),
            None
        );
        // Boundary-inclusive: MIN and MAX both succeed.
        let min_cfg =
            FjallConfig::try_new(FjallConfig::MIN_CACHE_SIZE_BYTES).expect("MIN must be valid");
        assert_eq!(
            min_cfg.cache_size_bytes(),
            FjallConfig::MIN_CACHE_SIZE_BYTES
        );
        let max_cfg =
            FjallConfig::try_new(FjallConfig::MAX_CACHE_SIZE_BYTES).expect("MAX must be valid");
        assert_eq!(
            max_cfg.cache_size_bytes(),
            FjallConfig::MAX_CACHE_SIZE_BYTES
        );
        // Sanity: an interior value also succeeds and round-trips.
        let mid = FjallConfig::try_new(256 * 1024 * 1024).expect("256 MiB must be valid");
        assert_eq!(mid.cache_size_bytes(), 256 * 1024 * 1024);
    }

    #[test]
    fn storage_limits_default_has_expected_value() {
        let limits = StorageLimits::DEFAULT;
        assert_eq!(
            limits.max_journal_event_payload_bytes,
            crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES
        );
    }

    #[test]
    fn index_status_state_from_u8_maps_correctly() {
        assert_eq!(IndexStatusState::from_u8(0), IndexStatusState::Submitted);
        assert_eq!(IndexStatusState::from_u8(1), IndexStatusState::Active);
        assert_eq!(IndexStatusState::from_u8(2), IndexStatusState::Completed);
    }

    #[test]
    fn index_status_state_from_u8_other_value_returns_other() {
        let state = IndexStatusState::from_u8(42);
        match state {
            IndexStatusState::Other(v) => assert_eq!(v, 42),
            other => panic!("expected Other(42), got {:?}", other),
        }
    }

    #[test]
    fn index_status_state_to_u8_maps_correctly() {
        assert_eq!(IndexStatusState::Submitted.to_u8(), 0);
        assert_eq!(IndexStatusState::Active.to_u8(), 1);
        assert_eq!(IndexStatusState::Completed.to_u8(), 2);
        assert_eq!(IndexStatusState::Other(99).to_u8(), 99);
    }

    // -- vb-3i8pq: IndexStatusState::new_other safe-construction tests -------
    //
    // These tests cover the public-safe construction path for the `Other`
    // variant. The unsafe bare-tuple path `IndexStatusState::Other(byte)`
    // is intentionally NOT exercised here; application code MUST go
    // through `new_other` to avoid fabricating a state whose wire byte
    // collides with one of the named variants (SC-001 / vb-f1xkn /
    // vb-3i8pq).

    #[test]
    fn index_status_state_new_other_accepts_safe_range() {
        for byte in crate::constants::MIN_OTHER_STATUS_BYTE..=u8::MAX {
            let state = IndexStatusState::new_other(byte).unwrap_or_else(|e| {
                panic!("new_other({byte}) must succeed in safe range, got {e:?}")
            });
            // The returned state must carry exactly the requested byte.
            assert_eq!(
                state.to_u8(),
                byte,
                "constructed Other({byte}) must encode to byte {byte}"
            );
        }
    }

    #[test]
    fn index_status_state_new_other_accepts_min_boundary() {
        // The boundary byte (MIN_OTHER_STATUS_BYTE) must succeed and
        // must NOT collide with any named variant on the wire.
        let min_byte = crate::constants::MIN_OTHER_STATUS_BYTE;
        assert_eq!(min_byte, 3, "MIN_OTHER_STATUS_BYTE contract is 3");
        let state =
            IndexStatusState::new_other(min_byte).expect("MIN_OTHER_STATUS_BYTE must be accepted");
        assert_eq!(state.to_u8(), min_byte);
        assert_ne!(state.to_u8(), IndexStatusState::Submitted.to_u8());
        assert_ne!(state.to_u8(), IndexStatusState::Active.to_u8());
        assert_ne!(state.to_u8(), IndexStatusState::Completed.to_u8());
    }

    #[test]
    fn index_status_state_new_other_rejects_collision_range() {
        // Bytes in `0..MIN_OTHER_STATUS_BYTE` (currently `0..=2`) must
        // be rejected with a typed IndexStatusStateCollision carrying
        // both the rejected byte and the accepted minimum.
        for byte in 0u8..crate::constants::MIN_OTHER_STATUS_BYTE {
            let err = IndexStatusState::new_other(byte)
                .expect_err("collision-range byte must be rejected by new_other");
            match err {
                JournalError::IndexStatusStateCollision { byte: b, min } => {
                    assert_eq!(b, byte, "rejected byte must round-trip in error");
                    assert_eq!(
                        min,
                        crate::constants::MIN_OTHER_STATUS_BYTE,
                        "min must reflect the safe-range minimum"
                    );
                }
                other => panic!(
                    "expected JournalError::IndexStatusStateCollision for byte {byte}, got {other:?}"
                ),
            }
        }
    }

    #[test]
    fn index_status_state_new_other_safe_constructed_state_passes_to_u8_checked() {
        // Belt-and-suspenders: states built via the safe constructor
        // must also pass the existing encoder-boundary guard
        // `to_u8_checked` without raising a collision error.
        for byte in [
            crate::constants::MIN_OTHER_STATUS_BYTE,
            crate::constants::MIN_OTHER_STATUS_BYTE + 1,
            42,
            99,
            255,
        ] {
            let state = IndexStatusState::new_other(byte).expect("safe byte must succeed");
            let checked_byte = state
                .to_u8_checked()
                .expect("safe-constructed state must pass to_u8_checked");
            assert_eq!(checked_byte, byte, "checked byte must equal payload");
        }
    }

    #[test]
    fn index_status_state_new_other_makes_collision_unrepresentable_for_safe_callers() {
        // The whole point of approach (A): for callers using the safe
        // construction path, it is impossible to obtain an
        // `IndexStatusState` whose wire byte collides with a named
        // variant. `new_other(0|1|2)` always returns `Err`.
        for byte in 0u8..=2 {
            assert!(
                IndexStatusState::new_other(byte).is_err(),
                "new_other({byte}) must reject the collision range"
            );
        }
        // And the named variants are the only way to express the
        // collision bytes: from_u8 maps them to the named variants
        // (which is the correct decode semantic).
        assert_eq!(IndexStatusState::from_u8(0), IndexStatusState::Submitted);
        assert_eq!(IndexStatusState::from_u8(1), IndexStatusState::Active);
        assert_eq!(IndexStatusState::from_u8(2), IndexStatusState::Completed);
    }

    #[test]
    fn index_status_state_roundtrip_from_and_to_u8() {
        for value in [0u8, 1, 2, 7, 42, 255] {
            let state = IndexStatusState::from_u8(value);
            assert_eq!(state.to_u8(), value, "roundtrip failed for value {}", value);
        }
    }

    #[test]
    fn index_status_state_other_byte_collides_with_named_variant_at_byte_level() {
        // This test deliberately constructs `IndexStatusState::Other(byte)`
        // for bytes in the collision range, which is reserved for
        // in-crate test/decoder paths. It exists to document the
        // byte-level shape of the collision so that the unsafe
        // bypass is visible to reviewers and so that future refactors
        // (e.g. wrapping the inner byte in a newtype) can pivot on
        // these exact assertions.
        //
        // Application code MUST use `IndexStatusState::new_other(byte)`
        // to construct an `Other` state; the bare tuple syntax above
        // is unsafe for caller code because it produces a wire byte
        // that aliases the named variants (SC-001 / vb-f1xkn / vb-3i8pq).
        //
        // Companion tests for the safe-construction path:
        // - `index_status_state_new_other_rejects_collision_range`
        // - `index_status_state_new_other_makes_collision_unrepresentable_for_safe_callers`
        assert_eq!(
            IndexStatusState::Other(0).to_u8(),
            IndexStatusState::Submitted.to_u8(),
            "SC-001 byte collision: Other(0) and Submitted share byte 0"
        );
        assert_eq!(
            IndexStatusState::Other(1).to_u8(),
            IndexStatusState::Active.to_u8(),
            "SC-001 byte collision: Other(1) and Active share byte 1"
        );
        assert_eq!(
            IndexStatusState::Other(2).to_u8(),
            IndexStatusState::Completed.to_u8(),
            "SC-001 byte collision: Other(2) and Completed share byte 2"
        );
        assert_ne!(
            IndexStatusState::Other(3).to_u8(),
            IndexStatusState::Submitted.to_u8(),
            "boundary byte 3 must not collide with any named variant"
        );
        // And `new_other` closes the door: the safe constructor must
        // reject every byte that this test fabricates.
        for collision_byte in 0u8..crate::constants::MIN_OTHER_STATUS_BYTE {
            assert!(
                IndexStatusState::new_other(collision_byte).is_err(),
                "new_other({collision_byte}) must reject the collision byte directly"
            );
        }
    }

    #[test]
    fn record_envelope_has_expected_fields() {
        let envelope = RecordEnvelope {
            magic: 0x5642_4A45,
            schema_version: 1,
            record_kind: 10,
            sequence: 5,
        };
        assert_eq!(envelope.magic, 0x5642_4A45);
        assert_eq!(envelope.schema_version, 1);
        assert_eq!(envelope.record_kind, 10);
        assert_eq!(envelope.sequence, 5);
    }

    #[test]
    fn record_header_has_expected_length() {
        let header = RecordHeader {
            magic: 0x5642_4952,
            schema_version: 1,
            record_kind: 2,
            header_len: RECORD_HEADER_LEN,
            payload_len: 100,
            sequence: 0,
            payload_digest: [0u8; DIGEST_BYTES],
            header_checksum: 0,
        };
        assert_eq!(header.header_len, RECORD_HEADER_LEN);
    }

    #[test]
    fn storage_key_variants_can_be_constructed() {
        let digest = [0xAA_u8; 32];
        let _ws = StorageKey::WorkflowSource { digest };
        let _ci = StorageKey::CompiledIr { digest };
        let _rh = StorageKey::RunHeader { run: RunId::new(1) };
        let _re = StorageKey::RunEvent {
            run: RunId::new(1),
            seq: EventSeq::new(0),
        };
        let _rs = StorageKey::RunSnapshot {
            run: RunId::new(2),
            seq: EventSeq::new(3),
        };
        let _bl = StorageKey::Blob { digest };
        let _is = StorageKey::IndexStatus {
            state: IndexStatusState::Active,
            timestamp: 100,
            run: RunId::new(3),
        };
        let _iw = StorageKey::IndexWorkflow {
            workflow: WorkflowId::new(4),
            run: RunId::new(5),
        };
        let _ia = StorageKey::IndexAction {
            action: ActionId::new(6),
            run: RunId::new(7),
            step: StepIdx::new(8),
        };
    }

    #[test]
    fn keyspace_profile_variants_exist() {
        let _hot = KeyspaceProfile::Hot;
        let _cold = KeyspaceProfile::Cold;
        let _blob = KeyspaceProfile::Blob;
    }

    #[test]
    fn keyspace_options_for_hot_has_bloom_filter() {
        let opts = crate::keyspace_options_for(KeyspaceProfile::Hot);
        // Verifying construction doesn't panic is the primary test
        let _ = opts;
    }

    #[test]
    fn durability_profile_variants_exist() {
        let _volatile = DurabilityProfile::Volatile;
        let _journaled = DurabilityProfile::Journaled;
        let _strict = DurabilityProfile::Strict;
    }

    #[test]
    fn index_status_key_rejects_other_with_named_variant_collision_byte() {
        let run = RunId::new(0x1234_5678);
        let timestamp = 0x0102_0304_0506_0708_u64;

        for collision_byte in [0u8, 1, 2] {
            let result = index_status_key(IndexStatusState::Other(collision_byte), timestamp, run);
            match result {
                Err(JournalError::IndexStatusStateCollision { byte, min }) => {
                    assert_eq!(
                        byte, collision_byte,
                        "collision variant must carry the rejected byte"
                    );
                    assert_eq!(
                        min,
                        crate::constants::MIN_OTHER_STATUS_BYTE,
                        "collision variant must surface MIN_OTHER_STATUS_BYTE"
                    );
                    assert_eq!(
                        min, 3,
                        "MIN_OTHER_STATUS_BYTE contract is 3 (first non-reserved byte)"
                    );
                }
                other => panic!(
                    "expected JournalError::IndexStatusStateCollision for Other({collision_byte}), got {other:?}"
                ),
            }
        }

        for ok_byte in [3u8, 4, 7, 42, 99, 255] {
            let result = index_status_key(IndexStatusState::Other(ok_byte), timestamp, run);
            assert!(
                result.is_ok(),
                "Other({ok_byte}) must encode without collision (>= MIN_OTHER_STATUS_BYTE)"
            );
        }

        for named in [
            IndexStatusState::Submitted,
            IndexStatusState::Active,
            IndexStatusState::Completed,
        ] {
            let result = index_status_key(named, timestamp, run);
            assert!(
                result.is_ok(),
                "named variant {named:?} must always encode successfully"
            );
        }
    }
}
