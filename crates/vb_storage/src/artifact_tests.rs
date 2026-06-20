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
mod artifact_tests {
    use crate::{DIGEST_BYTES, FjallJournal, JournalError};
    use vb_core::WorkflowDigest;

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
        (temp, journal)
    }

    fn put_test_artifact(journal: &FjallJournal, seed: &[u8]) -> WorkflowDigest {
        let record = crate::try_accepted_compiled_ir_record_for_test(seed.to_vec()).expect("test fixture should encode");
        let digest = record.digest;
        journal
            .put_compiled_ir(&record)
            .expect("put should succeed");
        digest
    }

    #[test]
    fn list_artifacts_returns_empty_for_empty_journal() {
        let (_temp, journal) = temp_journal();
        let artifacts = journal.list_artifacts().expect("list should succeed");
        assert!(
            artifacts.is_empty(),
            "should return empty list for empty journal"
        );
    }

    #[test]
    fn list_artifacts_returns_all_stored_digests() {
        let (_temp, journal) = temp_journal();
        let d1 = put_test_artifact(&journal, &[0x11]);
        let d2 = put_test_artifact(&journal, &[0x22]);
        let d3 = put_test_artifact(&journal, &[0x33]);

        let artifacts = journal.list_artifacts().expect("list should succeed");
        assert_eq!(artifacts.len(), 3, "should list 3 artifacts");
        assert!(artifacts.contains(&d1));
        assert!(artifacts.contains(&d2));
        assert!(artifacts.contains(&d3));
    }

    #[test]
    fn artifact_exists_returns_true_for_stored_digest() {
        let (_temp, journal) = temp_journal();
        let digest = put_test_artifact(&journal, &[0x44]);

        let exists = journal
            .artifact_exists(digest)
            .expect("check should succeed");
        assert!(exists, "artifact should exist after put");
    }

    #[test]
    fn artifact_exists_returns_false_for_missing_digest() {
        let (_temp, journal) = temp_journal();
        let missing = WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]);
        let exists = journal
            .artifact_exists(missing)
            .expect("check should succeed");
        assert!(!exists, "artifact should not exist for unknown digest");
    }

    #[test]
    fn remove_artifact_deletes_existing_artifact() {
        let (_temp, journal) = temp_journal();
        let digest = put_test_artifact(&journal, &[0x55]);

        assert!(
            journal
                .artifact_exists(digest)
                .expect("check before remove")
        );
        journal
            .remove_artifact(digest)
            .expect("remove should succeed");
        assert!(
            !journal.artifact_exists(digest).expect("check after remove"),
            "artifact should not exist after removal"
        );
    }

    #[test]
    fn remove_artifact_returns_error_for_missing_digest() {
        let (_temp, journal) = temp_journal();
        let missing = WorkflowDigest::from_bytes([0xEE; DIGEST_BYTES]);
        let result = journal.remove_artifact(missing);
        assert!(
            matches!(result, Err(JournalError::ArtifactNotFound { digest }) if digest == missing),
            "must return ArtifactNotFound for missing digest, got {:?}",
            result
        );
    }

    #[test]
    fn list_artifacts_reflects_removal() {
        let (_temp, journal) = temp_journal();
        let d1 = put_test_artifact(&journal, &[0x66]);
        let d2 = put_test_artifact(&journal, &[0x77]);

        journal
            .remove_artifact(d1)
            .expect("remove d1 should succeed");

        let artifacts = journal.list_artifacts().expect("list should succeed");
        assert_eq!(artifacts.len(), 1);
        assert!(artifacts.contains(&d2));
        assert!(!artifacts.contains(&d1));
    }

    #[test]
    fn artifact_exists_idempotent_after_removal() {
        let (_temp, journal) = temp_journal();
        let digest = put_test_artifact(&journal, &[0x88]);
        journal
            .remove_artifact(digest)
            .expect("first remove should succeed");

        let result = journal.remove_artifact(digest);
        assert!(
            matches!(result, Err(JournalError::ArtifactNotFound { .. })),
            "second remove should return ArtifactNotFound, got {:?}",
            result
        );
    }
}
