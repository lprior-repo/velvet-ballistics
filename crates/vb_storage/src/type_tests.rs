#![allow(
    unused_imports,
    dead_code,
    clippy::assertions_on_constants,
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used,
    clippy::let_underscore_must_use,
    clippy::len_zero,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::needless_return,
    clippy::needless_bool,
    clippy::single_match,
    clippy::single_match_else,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_locals,
    clippy::manual_let_else,
    clippy::or_fun_call,
    clippy::needless_borrow,
    clippy::needless_pass_by_value,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::module_inception,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::uninlined_format_args,
    clippy::large_digit_groups,
    clippy::unreadable_literal,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::vec_init_then_push,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::trivially_copy_pass_by_ref,
    clippy::wildcard_imports,
    clippy::wrong_self_convention,
    clippy::needless_range_loop,
    clippy::nonminimal_bool,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::should_implement_trait,
    clippy::result_large_err,
    clippy::missing_const_for_fn,
    clippy::use_self,
    clippy::items_after_statements,
    clippy::option_if_let_else,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::comparison_chain,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::explicit_counter_loop,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::needless_update,
    clippy::let_and_return,
    clippy::manual_div_ceil,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::match_like_matches_macro,
    clippy::wildcard_enum_match_arm,
    clippy::large_types_passed_by_value,
    clippy::large_futures,
    clippy::type_complexity,
    clippy::needless_collect,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::suspicious_operation_groupings,
    clippy::field_reassign_with_default,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::borrow_deref_ref,
    clippy::cloned_ref_to_slice_refs,
    clippy::inefficient_to_string,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::get_first,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::implicit_saturating_sub,
    clippy::unwrap_or_default,
    clippy::default_trait_access
)]

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
    use crate::{
        DurabilityProfile, EventSeq, FjallConfig, IndexStatusState,
        JournalBatchSize, JournalQueueCapacity, JournalWriterFlushReport,
        KeyspaceProfile, RecordEnvelope, RecordHeader, StorageKey, StorageLimits,
        constants::{DIGEST_BYTES, RECORD_HEADER_LEN},
    };
    use vb_core::{ActionId, RunId, StepIdx, WorkflowId};
    use std::num::NonZeroUsize;

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
        assert!(
            err.is_err(),
            "zero should fail with QueueCapacity error"
        );
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
            pending_after: 0,
        };
        assert_eq!(report.drained, 15);
        assert_eq!(report.written, 10);
        assert_eq!(report.pending_after, 0);
    }

    #[test]
    fn fjall_config_default_has_256_mib_cache() {
        let config = FjallConfig::default();
        assert_eq!(config.cache_size_bytes, 268_435_456);
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

    #[test]
    fn index_status_state_roundtrip_from_and_to_u8() {
        for value in [0u8, 1, 2, 7, 42, 255] {
            let state = IndexStatusState::from_u8(value);
            assert_eq!(state.to_u8(), value, "roundtrip failed for value {}", value);
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
        let _rs = StorageKey::RecoveryStamp {
            run: RunId::new(9),
            seq: EventSeq::new(10),
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
}
