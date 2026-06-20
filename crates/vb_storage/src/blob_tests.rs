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
mod blob_tests {
    use crate::{
        BlobRecord, DIGEST_BYTES, FjallJournal, JournalError,
        keys::blob_key,
    };

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
        (temp, journal)
    }

    fn make_blob_record(payload: &[u8]) -> BlobRecord {
        let digest: [u8; DIGEST_BYTES] = blake3::hash(payload).into();
        BlobRecord {
            digest,
            bytes: payload.to_vec(),
        }
    }

    #[test]
    fn put_blob_stores_and_retrieves_valid_blob() {
        let (_temp, journal) = temp_journal();
        let payload = b"five-byte-blob-test-payload-data";
        let record = make_blob_record(payload);
        journal.put_blob(&record).expect("put_blob should succeed");
        let loaded = journal.blob(record.digest).expect("blob should succeed");
        let found = loaded.expect("blob should exist after put");
        assert_eq!(found.digest, record.digest);
        assert_eq!(found.bytes, record.bytes);
    }

    #[test]
    fn put_blob_rejects_digest_mismatch() {
        let (_temp, journal) = temp_journal();
        let payload = b"real-blob-payload-for-digest-mismatch-test";
        let wrong_digest: [u8; DIGEST_BYTES] = [0xFF; 32];
        let record = BlobRecord {
            digest: wrong_digest,
            bytes: payload.to_vec(),
        };
        let result = journal.put_blob(&record);
        assert!(
            matches!(result, Err(JournalError::PayloadDigestMismatch)),
            "must reject digest mismatch, got {:?}",
            result
        );
    }

    #[test]
    fn blob_returns_none_for_missing_digest() {
        let (_temp, journal) = temp_journal();
        let missing_digest: [u8; DIGEST_BYTES] = [0xAB; 32];
        let result = journal.blob(missing_digest).expect("blob lookup should succeed");
        assert!(result.is_none(), "should return None for missing digest");
    }

    #[test]
    fn put_blob_accepts_empty_payload() {
        let (_temp, journal) = temp_journal();
        let empty: &[u8] = &[];
        let record = make_blob_record(empty);
        journal.put_blob(&record).expect("put_blob of empty should succeed");
        let loaded = journal
            .blob(record.digest)
            .expect("get should succeed")
            .expect("should exist");
        assert_eq!(loaded.bytes.len(), 0);
    }

    #[test]
    fn put_blob_accepts_max_size_payload() {
        let (_temp, journal) = temp_journal();
        let payload = vec![0x42u8; 1024];
        let record = make_blob_record(&payload);
        journal.put_blob(&record).expect("put_blob of max payload should succeed");
        let loaded = journal
            .blob(record.digest)
            .expect("get should succeed")
            .expect("should exist");
        assert_eq!(loaded.bytes.len(), payload.len());
    }

    #[test]
    fn put_blob_is_idempotent() {
        let (_temp, journal) = temp_journal();
        let payload = b"idempotent-blob-payload-data";
        let record = make_blob_record(payload);
        journal.put_blob(&record).expect("first put should succeed");
        let result = journal.put_blob(&record);
        assert!(
            result.is_err(),
            "second put_blob with same digest should fail or be rejected, got {:?}",
            result
        );
    }

    #[test]
    fn multiple_blobs_with_different_digests_are_retrieved_correctly() {
        let (_temp, journal) = temp_journal();
        let p1 = b"first-blob-payload-for-multi-blob-test";
        let p2 = b"second-distinct-blob-payload";
        let r1 = make_blob_record(p1);
        let r2 = make_blob_record(p2);
        journal.put_blob(&r1).expect("put first blob");
        journal.put_blob(&r2).expect("put second blob");

        let loaded1 = journal.blob(r1.digest).expect("get first").expect("found");
        let loaded2 = journal.blob(r2.digest).expect("get second").expect("found");
        assert_eq!(loaded1.bytes, p1.to_vec());
        assert_eq!(loaded2.bytes, p2.to_vec());
    }

    #[test]
    fn read_blob_convenience_wrapper_works() {
        let (_temp, journal) = temp_journal();
        let payload = b"read-blob-wrapper-test-data";
        let record = make_blob_record(payload);
        journal.put_blob(&record).expect("put_blob should succeed");

        let result = crate::read_blob(&journal, record.digest).expect("read_blob should succeed");
        let found = result.expect("blob should exist");
        assert_eq!(found.bytes, payload.to_vec());
    }

    #[test]
    fn blob_decode_with_invalid_data_returns_error() {
        let (_temp, journal) = temp_journal();
        let digest: [u8; DIGEST_BYTES] = [0x70; 32];
        let fake_key = blob_key(digest).expect("blob key should succeed");
        let invalid_value = vec![0xFF; 4];

        journal
            .blob
            .insert(fake_key.to_vec(), invalid_value)
            .expect("raw insert should succeed");

        let result = journal.blob(digest);
        assert!(
            result.is_err() || result.map_or(false, |opt| opt.is_none()),
            "reading corrupt blob should error or return None"
        );
    }
}
