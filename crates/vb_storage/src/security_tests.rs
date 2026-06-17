#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables,
)]

#![forbid(unsafe_code)]
//! BLACKHAT security tests for vb_storage.
//!
//! Documents and verifies security properties across the storage layer including:
//! - Digest forgery prevention (direct and batch paths)
//! - Deserialization attack resistance
//! - Integer overflow/underflow boundary conditions
//! - Truncation and corruption detection
//! - Data corruption vector resistance
//! - Cross-run isolation enforcement
//! - Sequence integrity enforcement

/// Module marker ensuring the security test module is always compiled.
#[cfg(not(test))]
const _BLACKHAT_SECURITY_TESTS_LOADED: () = ();

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod tests {
    use crate::{
        BlobRecord, DIGEST_BYTES, EventSeq, FjallJournal, JournalError, WorkflowSourceRecord,
        codec::{
            decode_record, decode_record_header, encode_record, encode_record_header,
            verify_digest_match,
        },
        constants::{
            CRC_OFFSET, CURRENT_SCHEMA_VERSION, MAGIC_BLOB, MAGIC_COMPILED_ARTIFACT,
            MAGIC_INDEX_RECORD, MAGIC_JOURNAL_EVENT, MAGIC_SNAPSHOT, MAX_BLOB_BYTES,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_WORKFLOW_SOURCE_BYTES, RECORD_HEADER_BYTES,
        },
        events::JournalEvent,
        records::RecordKind,
    };
    use vb_core::{RunId, SlotIdx, WorkflowDigest};

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
        (temp, journal)
    }

    // =========================================================================
    // BH-01: Digest forgery prevention -- direct write path
    // =========================================================================

    /// SECURITY: put_workflow_source rejects a forged digest where source bytes
    /// do not hash to the claimed digest.
    #[test]
    fn forged_workflow_source_digest_rejected() {
        let (_temp, journal) = temp_journal();
        let forged_digest = WorkflowDigest::from_bytes([0xDE; 32]);
        let record = WorkflowSourceRecord {
            digest: forged_digest,
            source: b"this will not hash to 0xDE..DE".to_vec(),
        };
        let result = journal.put_workflow_source(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "forged workflow source digest must be rejected, got {:?}",
            result
        );
    }

    /// SECURITY: put_blob rejects a forged digest where blob bytes do not hash
    /// to the claimed digest.
    #[test]
    fn forged_blob_digest_rejected() {
        let (_temp, journal) = temp_journal();
        let forged_digest = [0xAB; 32];
        let record = BlobRecord {
            digest: forged_digest,
            bytes: b"these bytes won't hash to 0xAB..AB".to_vec(),
        };
        let result = journal.put_blob(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "forged blob digest must be rejected, got {:?}",
            result
        );
    }

    // =========================================================================
    // BH-02: Digest forgery prevention -- batch write path (FIXED)
    //
    // Previously, JournalWriteBatch::put_workflow_source and put_blob skipped
    // the verify_content_digest check that their direct counterparts enforce.
    // This meant a batch commit could persist records with forged digests,
    // allowing an attacker to substitute content while preserving a trusted
    // digest key. The fix adds verify_content_digest to both batch methods.
    // =========================================================================

    /// SECURITY: batch put_workflow_source rejects a forged digest.
    #[test]
    fn batch_forged_workflow_source_digest_rejected() {
        let (_temp, journal) = temp_journal();
        let forged_digest = WorkflowDigest::from_bytes([0xFF; 32]);
        let mut batch = journal.batch();
        let result = batch.put_workflow_source(&WorkflowSourceRecord {
            digest: forged_digest,
            source: b"not the right hash".to_vec(),
        });
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "batch forged workflow source digest must be rejected, got {:?}",
            result
        );
    }

    /// SECURITY: batch put_blob rejects a forged digest.
    #[test]
    fn batch_forged_blob_digest_rejected() {
        let (_temp, journal) = temp_journal();
        let mut batch = journal.batch();
        let result = batch.put_blob(&BlobRecord {
            digest: [0x11; 32],
            bytes: b"wrong hash".to_vec(),
        });
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "batch forged blob digest must be rejected, got {:?}",
            result
        );
    }

    /// SECURITY: put_workflow_source accepts a record with a correct digest.
    #[test]
    fn valid_workflow_source_digest_accepted() {
        let (_temp, journal) = temp_journal();
        let source = b"valid content".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let record = WorkflowSourceRecord { digest, source };
        journal
            .put_workflow_source(&record)
            .expect("correct digest must be accepted");
    }

    /// SECURITY: put_blob accepts a record with a correct digest.
    #[test]
    fn valid_blob_digest_accepted() {
        let (_temp, journal) = temp_journal();
        let bytes = b"valid blob content".to_vec();
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&bytes).into();
        let record = BlobRecord { digest, bytes };
        journal
            .put_blob(&record)
            .expect("correct digest must be accepted");
    }

    /// SECURITY: batch put_workflow_source accepts correct digest.
    #[test]
    fn batch_valid_workflow_source_digest_accepted() {
        let (_temp, journal) = temp_journal();
        let source = b"valid batch content".to_vec();
        let digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let mut batch = journal.batch();
        batch
            .put_workflow_source(&WorkflowSourceRecord { digest, source })
            .expect("correct digest in batch must be accepted");
    }

    /// SECURITY: batch put_blob accepts correct digest.
    #[test]
    fn batch_valid_blob_digest_accepted() {
        let (_temp, journal) = temp_journal();
        let bytes = b"valid batch blob".to_vec();
        let digest: [u8; DIGEST_BYTES] = blake3::hash(&bytes).into();
        let mut batch = journal.batch();
        batch
            .put_blob(&BlobRecord { digest, bytes })
            .expect("correct digest in batch must be accepted");
    }

    // =========================================================================
    // BH-03: Deserialization attack resistance
    //
    // Postcard deserialization of untrusted bytes is the primary attack
    // surface. The codec layer validates header integrity (magic, CRC,
    // schema version, kind family) before any postcard deserialization.
    // These tests verify that crafted malicious inputs are rejected at
    // the header boundary.
    // =========================================================================

    /// SECURITY: Completely zeroed bytes (a common fuzzing output) are rejected.
    #[test]
    fn decode_rejects_all_zero_bytes() {
        let zeros = [0u8; RECORD_HEADER_BYTES + 64];
        let result = decode_record::<JournalEvent>(
            &zeros,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            result.is_err(),
            "all-zero bytes must be rejected, got {:?}",
            result
        );
    }

    /// SECURITY: All-0xFF bytes are rejected.
    #[test]
    fn decode_rejects_all_ff_bytes() {
        let ff = [0xFFu8; RECORD_HEADER_BYTES + 64];
        let result = decode_record::<JournalEvent>(
            &ff,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            result.is_err(),
            "all-0xFF bytes must be rejected, got {:?}",
            result
        );
    }

    /// SECURITY: Valid header with every payload byte flipped fails BLAKE3 check.
    #[test]
    fn decode_rejects_valid_header_with_corrupt_payload() {
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
        .expect("encode should succeed");

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
            "corrupt payload must yield PayloadDigestMismatch, got {:?}",
            result
        );
    }

    /// SECURITY: Wrong magic is rejected before postcard deserialization.
    #[test]
    fn decode_rejects_valid_postcard_with_wrong_magic_before_deserialization() {
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
        .expect("encode should succeed");

        let result =
            decode_record::<JournalEvent>(&bytes, MAGIC_BLOB, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
        assert!(
            matches!(result, Err(JournalError::BadMagic { .. })),
            "wrong magic must fail at BadMagic before postcard decode, got {:?}",
            result
        );
    }

    // =========================================================================
    // BH-04: Integer overflow boundary conditions
    // =========================================================================

    /// SECURITY: Event sequence overflow at u64::MAX is detected.
    #[test]
    fn event_seq_overflow_rejected() {
        let seq = EventSeq::new(u64::MAX);
        let result = crate::codec::next_seq(seq);
        assert!(
            matches!(result, Err(JournalError::SequenceOverflow)),
            "u64::MAX + 1 must yield SequenceOverflow, got {:?}",
            result
        );
    }

    /// SECURITY: Non-empty payload with max=0 is rejected.
    #[test]
    fn encode_rejects_payload_at_boundary_with_small_max() {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            attempt: 1,
            reason: None,
        };
        let result = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunCancelled, 0, &event, 0);
        assert!(
            matches!(result, Err(JournalError::PayloadTooLarge { .. })),
            "non-empty payload with max=0 must yield PayloadTooLarge, got {:?}",
            result
        );
    }

    /// SECURITY: Header decode rejects payload_len > max_payload_len.
    #[test]
    fn header_decode_rejects_oversized_declared_payload() {
        let payload = b"test";
        let mut header = encode_record_header(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            payload,
            1024,
        )
        .expect("encode header should succeed");

        let huge_len: u32 = 0xFFFF_FFFF_u32;
        let len_bytes = huge_len.to_le_bytes();
        header
            .get_mut(12..16)
            .expect("payload_len field")
            .copy_from_slice(&len_bytes);

        let checksum = crc32c::crc32c(&header[..CRC_OFFSET]);
        header
            .get_mut(CRC_OFFSET..CRC_OFFSET + 4)
            .expect("crc field")
            .copy_from_slice(&checksum.to_le_bytes());

        let result = decode_record_header(&header, MAGIC_JOURNAL_EVENT, 1024);
        assert!(
            matches!(result, Err(JournalError::PayloadTooLarge { .. })),
            "huge payload_len must yield PayloadTooLarge, got {:?}",
            result
        );
    }

    /// SECURITY: Header decode rejects header_len != 60.
    #[test]
    fn header_decode_rejects_wrong_header_len() {
        let payload = b"test";
        let mut header = encode_record_header(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            payload,
            1024,
        )
        .expect("encode header should succeed");

        let wrong_len: u32 = 59;
        let len_bytes = wrong_len.to_le_bytes();
        header
            .get_mut(8..12)
            .expect("header_len field")
            .copy_from_slice(&len_bytes);

        let checksum = crc32c::crc32c(&header[..CRC_OFFSET]);
        header
            .get_mut(CRC_OFFSET..CRC_OFFSET + 4)
            .expect("crc field")
            .copy_from_slice(&checksum.to_le_bytes());

        let result = decode_record_header(&header, MAGIC_JOURNAL_EVENT, 1024);
        assert!(
            matches!(result, Err(JournalError::HeaderLengthMismatch { .. })),
            "wrong header_len must yield HeaderLengthMismatch, got {:?}",
            result
        );
    }

    // =========================================================================
    // BH-05: Truncation detection
    // =========================================================================

    /// SECURITY: Exactly RECORD_HEADER_BYTES with declared payload fails.
    #[test]
    fn decode_rejects_header_only_when_payload_declared() {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            attempt: 1,
            reason: None,
        };
        let bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunCancelled,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encode should succeed");

        let truncated = &bytes[..RECORD_HEADER_BYTES];
        let result = decode_record::<JournalEvent>(
            truncated,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "truncated record must yield UnexpectedEof, got {:?}",
            result
        );
    }

    /// SECURITY: One byte shorter than header is rejected.
    #[test]
    fn decode_rejects_one_byte_short_of_header() {
        let short = [0xAAu8; RECORD_HEADER_BYTES - 1];
        let result = decode_record::<JournalEvent>(
            &short,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "one byte short must yield UnexpectedEof, got {:?}",
            result
        );
    }

    // =========================================================================
    // BH-06: Cross-run isolation
    // =========================================================================

    /// SECURITY: Reading events for a different run returns empty.
    #[test]
    fn events_for_run_returns_empty_for_unrelated_run() {
        let (_temp, journal) = temp_journal();
        let run_a = RunId::new(100);
        let run_b = RunId::new(200);

        let event = JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        };
        journal
            .append_strict(&event)
            .expect("append should succeed");

        let events_b = journal
            .events_for_run(run_b)
            .expect("replay should succeed");
        assert!(
            events_b.is_empty(),
            "run B should have zero events from run A"
        );
    }

    /// SECURITY: Different run with same seq succeeds (keys include run ID).
    #[test]
    fn append_succeeds_for_different_run_same_seq() {
        let (_temp, journal) = temp_journal();
        let run_a = RunId::new(100);
        let run_b = RunId::new(200);

        let event_a = JournalEvent::RunAccepted {
            run: run_a,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        };
        journal
            .append_strict(&event_a)
            .expect("first append should succeed");

        let event_b = JournalEvent::RunAccepted {
            run: run_b,
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        };
        let result = journal.append_strict(&event_b);
        assert!(
            result.is_ok(),
            "different run with same seq should succeed (key includes run), got {:?}",
            result
        );
    }

    // =========================================================================
    // BH-07: Schema version validation
    // =========================================================================

    /// SECURITY: Future schema version is rejected.
    #[test]
    fn decode_rejects_future_schema_version_in_full_record() {
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
        .expect("encode should succeed");

        let future_version = CURRENT_SCHEMA_VERSION.saturating_add(1);
        let version_bytes = future_version.to_le_bytes();
        bytes
            .get_mut(4..6)
            .expect("schema version field")
            .copy_from_slice(&version_bytes);

        let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
        let crc_bytes = checksum.to_le_bytes();
        bytes
            .get_mut(CRC_OFFSET..CRC_OFFSET + 4)
            .expect("crc field")
            .copy_from_slice(&crc_bytes);

        let result = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::UnsupportedSchemaVersion { .. })),
            "future schema must yield UnsupportedSchemaVersion, got {:?}",
            result
        );
    }

    // =========================================================================
    // BH-08: Kind-family validation
    // =========================================================================

    /// SECURITY: Cannot encode a workflow source record under journal event magic.
    #[test]
    fn encode_rejects_kind_family_mismatch_workflow_in_journal() {
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
            "kind family mismatch should be rejected, got {result:?}"
        );
    }

    /// SECURITY: Cannot encode a blob kind under snapshot magic.
    #[test]
    fn encode_rejects_kind_family_mismatch_blob_in_snapshot() {
        let event = JournalEvent::RunCancelled {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            attempt: 1,
            reason: None,
        };
        let result = encode_record(MAGIC_SNAPSHOT, RecordKind::Blob, 0, &event, MAX_BLOB_BYTES);
        assert!(
            matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
            "blob kind in snapshot magic must be rejected, got {result:?}"
        );
    }

    // =========================================================================
    // BH-09: CRC tampering detection
    // =========================================================================

    /// SECURITY: Single-bit flip in CRC field is detected.
    #[test]
    fn crc_single_bit_flip_detected() {
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
        .expect("encode should succeed");

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
            "CRC bit flip must yield HeaderChecksumMismatch, got {:?}",
            result
        );
    }

    /// SECURITY: Single-bit flip in magic field is detected via CRC.
    #[test]
    fn magic_tampering_detected_via_crc() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]),
        };
        let mut bytes = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encode should succeed");

        if let Some(byte) = bytes.get_mut(0) {
            *byte ^= 0x01;
        }
        let result = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(
                result,
                Err(JournalError::BadMagic { .. } | JournalError::HeaderChecksumMismatch)
            ),
            "tampered magic must be detected, got {:?}",
            result
        );
    }

    // =========================================================================
    // BH-10: Verify digest match utility correctness
    // =========================================================================

    /// SECURITY: verify_digest_match accepts correct digest.
    #[test]
    fn verify_digest_match_accepts_correct() {
        let payload = b"hello world";
        let digest: [u8; DIGEST_BYTES] = blake3::hash(payload).into();
        let result = verify_digest_match(payload, digest);
        assert!(result.is_ok(), "correct digest should pass verification");
    }

    /// SECURITY: verify_digest_match rejects wrong digest.
    #[test]
    fn verify_digest_match_rejects_wrong() {
        let payload = b"hello world";
        let wrong_digest: [u8; DIGEST_BYTES] = blake3::hash(b"something else").into();
        let result = verify_digest_match(payload, wrong_digest);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "wrong digest must yield PayloadDigestMismatch, got {:?}",
            result
        );
    }

    /// SECURITY: verify_digest_match rejects empty payload with non-zero digest.
    #[test]
    fn verify_digest_match_rejects_empty_payload_with_nonzero_digest() {
        let empty: &[u8] = b"";
        let nonzero_digest: [u8; DIGEST_BYTES] = [0xAA; DIGEST_BYTES];
        let result = verify_digest_match(empty, nonzero_digest);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "empty payload with non-zero digest must be rejected, got {:?}",
            result
        );
    }

    // =========================================================================
    // BH-11: Batch write digest verification (post-fix regression tests)
    // =========================================================================

    /// SECURITY: After fix, batch cannot persist a workflow source with forged digest.
    #[test]
    fn batch_cannot_persist_forged_workflow_source() {
        let (_temp, journal) = temp_journal();
        let forged_digest = WorkflowDigest::from_bytes([0xBA; 32]);
        let record = WorkflowSourceRecord {
            digest: forged_digest,
            source: b"attacker content".to_vec(),
        };
        let mut batch = journal.batch();
        let result = batch.put_workflow_source(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "batch must reject forged workflow source digest, got {:?}",
            result
        );
    }

    /// SECURITY: After fix, batch cannot persist a blob with forged digest.
    #[test]
    fn batch_cannot_persist_forged_blob() {
        let (_temp, journal) = temp_journal();
        let record = BlobRecord {
            digest: [0xCC; 32],
            bytes: b"attacker blob".to_vec(),
        };
        let mut batch = journal.batch();
        let result = batch.put_blob(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "batch must reject forged blob digest, got {:?}",
            result
        );
    }

    /// SECURITY: Batch with correct digests can commit and round-trip.
    #[test]
    fn batch_with_correct_digests_commits_successfully() {
        let (_temp, journal) = temp_journal();
        let source = b"legit source".to_vec();
        let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
        let blob_bytes = b"legit blob".to_vec();
        let blob_digest: [u8; DIGEST_BYTES] = blake3::hash(&blob_bytes).into();

        let mut batch = journal.batch();
        batch
            .put_workflow_source(&WorkflowSourceRecord {
                digest: source_digest,
                source: source.clone(),
            })
            .expect("workflow source should succeed");
        batch
            .put_blob(&BlobRecord {
                digest: blob_digest,
                bytes: blob_bytes.clone(),
            })
            .expect("blob should succeed");
        batch.commit().expect("batch commit should succeed");

        let loaded_source = journal
            .workflow_source(source_digest)
            .expect("read should succeed")
            .expect("record should exist");
        assert_eq!(loaded_source.source, source);

        let loaded_blob = journal
            .blob(blob_digest)
            .expect("read should succeed")
            .expect("record should exist");
        assert_eq!(loaded_blob.bytes, blob_bytes);
    }

    // =========================================================================
    // BH-12: Decode-then-reencode produces identical bytes (tamper evidence)
    // =========================================================================

    /// SECURITY: A valid record round-trips through encode/decode without mutation.
    #[test]
    fn round_trip_preserves_record_integrity() {
        let event = JournalEvent::SlotWrittenEvent {
            run: RunId::new(42),
            seq: EventSeq::new(7),
            slot: SlotIdx::new(3),
            value: Some(vec![0xDE, 0xAD]),
            extra: None,
            attempt: 1,
        };
        let original = encode_record(
            MAGIC_JOURNAL_EVENT,
            RecordKind::SlotWritten,
            7,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("encode should succeed");

        let (envelope, decoded) = decode_record::<JournalEvent>(
            &original,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("decode should succeed");

        let reencoded = encode_record(
            envelope.magic,
            RecordKind::SlotWritten,
            envelope.sequence,
            &decoded,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        )
        .expect("re-encode should succeed");

        assert_eq!(
            original, reencoded,
            "round-trip encode/decode must produce identical bytes"
        );
    }

    // =========================================================================
    // BH-13: Payload declared length vs actual bytes mismatch
    // =========================================================================

    /// SECURITY: Header declaring payload_len=0 fails BLAKE3 digest check.
    #[test]
    fn zero_payload_len_with_bytes_fails_digest_check() {
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
        .expect("encode should succeed");

        let zero_len: [u8; 4] = 0u32.to_le_bytes();
        bytes
            .get_mut(12..16)
            .expect("payload_len field")
            .copy_from_slice(&zero_len);

        let checksum = crc32c::crc32c(&bytes[..CRC_OFFSET]);
        bytes
            .get_mut(CRC_OFFSET..CRC_OFFSET + 4)
            .expect("crc field")
            .copy_from_slice(&checksum.to_le_bytes());

        let result = decode_record::<JournalEvent>(
            &bytes,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "zero payload_len with non-empty digest must fail, got {:?}",
            result
        );
    }

    // =========================================================================
    // BH-14: Empty digest edge case
    // =========================================================================

    /// SECURITY: All-zero digest rejects nonempty content (BLAKE3 never hashes to zeros).
    #[test]
    fn all_zero_digest_rejects_nonempty_content() {
        let content = b"some content that definitely won't hash to zeros";
        let zero_digest: [u8; DIGEST_BYTES] = [0; DIGEST_BYTES];
        let result = verify_digest_match(content, zero_digest);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "all-zero digest must reject nonempty content, got {:?}",
            result
        );
    }

    // =========================================================================
    // BH-15: Payload size limit enforcement
    // =========================================================================

    /// SECURITY: Journal event respects max payload limit.
    #[test]
    fn journal_event_respects_max_payload() {
        let big_value = vec![0xFFu8; MAX_JOURNAL_EVENT_PAYLOAD_BYTES as usize];
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
            "oversized journal event must yield PayloadTooLarge, got {:?}",
            result
        );
    }

    // =========================================================================
    // BH-16: Process lock prevents dual writers
    // =========================================================================

    /// SECURITY: Second journal open on same path fails.
    #[test]
    fn second_journal_open_on_same_path_is_prevented_by_process_lock() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let _journal1 = FjallJournal::open(temp.path(), None).expect("first open should succeed");
        let result = FjallJournal::open(temp.path(), None);
        assert!(
            result.is_err(),
            "second open on same path must fail due to process lock"
        );
    }

    /// SECURITY: Lock file is created and contains holder PID on first open.
    #[test]
    fn process_lock_file_created_with_holder_pid() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let _journal = FjallJournal::open(temp.path(), None).expect("first open should succeed");
        let lock_path = temp.path().join(".process.lock");
        assert!(lock_path.exists(), ".process.lock must exist after open");
        let contents = std::fs::read_to_string(&lock_path).expect("read lock file");
        let pid: u32 = contents
            .trim()
            .parse()
            .expect("lock file should contain valid PID");
        assert_eq!(
            pid,
            std::process::id(),
            "lock file should contain current process PID"
        );
    }

    /// SECURITY: Lock releases on journal drop, allowing re-open.
    #[test]
    fn lock_releases_on_journal_drop() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        {
            let _journal1 =
                FjallJournal::open(temp.path(), None).expect("first open should succeed");
        } // journal1 dropped here, releasing the lock
        let result = FjallJournal::open(temp.path(), None);
        assert!(
            result.is_ok(),
            "re-open after drop must succeed because lock was released"
        );
    }

    /// SECURITY: No Fjall mutation occurs when lock acquisition fails.
    #[test]
    fn no_keyspace_created_when_lock_fails() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let _journal1 = FjallJournal::open(temp.path(), None).expect("first open should succeed");

        // Count files before second attempt
        let before_count = std::fs::read_dir(temp.path()).expect("read_dir").count();

        // Second open fails
        let result = FjallJournal::open(temp.path(), None);
        assert!(result.is_err(), "second open must fail");

        // Count files after second attempt
        let after_count = std::fs::read_dir(temp.path()).expect("read_dir").count();

        assert_eq!(
            before_count, after_count,
            "no new files should be created when lock acquisition fails"
        );
    }

    // =========================================================================
    // BH-17: Magic-family gate validation
    // =========================================================================

    /// SECURITY: Compiled artifact magic rejects workflow source kind.
    #[test]
    fn compiled_artifact_magic_rejects_workflow_source_kind() {
        let record = WorkflowSourceRecord {
            digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
            source: vec![],
        };
        let result = encode_record(
            MAGIC_COMPILED_ARTIFACT,
            RecordKind::WorkflowSource,
            0,
            &record,
            MAX_WORKFLOW_SOURCE_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
            "compiled artifact magic must reject workflow source kind, got {result:?}"
        );
    }

    /// SECURITY: Index record magic rejects event kind.
    #[test]
    fn index_magic_rejects_event_kind() {
        let event = JournalEvent::RunAccepted {
            run: RunId::new(1),
            seq: EventSeq::new(0),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        };
        let result = encode_record(
            MAGIC_INDEX_RECORD,
            RecordKind::RunAccepted,
            0,
            &event,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. })),
            "index magic must reject event kind, got {result:?}"
        );
    }
}
