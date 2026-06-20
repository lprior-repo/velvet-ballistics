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
mod header_tests {
    use crate::{DIGEST_BYTES, FjallJournal, RunHeaderRecord};
    use vb_core::{RunId, WorkflowDigest, WorkflowId};

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
        (temp, journal)
    }

    fn make_header(run: RunId) -> RunHeaderRecord {
        RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(1),
            compiled_digest: WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]),
            status: 1,
            accepted_at_ms: 1000,
        }
    }

    #[test]
    fn put_run_header_stores_and_retrieves_by_run_id() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(100);
        let header = make_header(run);

        journal.put_run_header(&header).expect("put_run_header should succeed");

        let loaded = journal.run_header(run).expect("get should succeed");
        let found = loaded.expect("header should exist");
        assert_eq!(found.run, run);
        assert_eq!(found.workflow_id, header.workflow_id);
        assert_eq!(found.compiled_digest, header.compiled_digest);
        assert_eq!(found.status, header.status);
    }

    #[test]
    fn run_header_returns_none_for_missing_run() {
        let (_temp, journal) = temp_journal();
        let missing = RunId::new(9999);
        let result = journal.run_header(missing).expect("get should succeed");
        assert!(result.is_none(), "should return None for missing run header");
    }

    #[test]
    fn run_headers_returns_all_stored_headers() {
        let (_temp, journal) = temp_journal();
        for i in 1u64..=5 {
            let run = RunId::new(i);
            let header = RunHeaderRecord {
                run,
                workflow_id: WorkflowId::new(1),
                compiled_digest: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
                status: 1,
                accepted_at_ms: i * 100,
            };
            journal.put_run_header(&header).expect("put should succeed");
        }

        let all = journal.run_headers().expect("run_headers should succeed");
        assert_eq!(all.len(), 5, "should have 5 run headers");
        assert!(all.iter().any(|h| h.run == RunId::new(1)));
        assert!(all.iter().any(|h| h.run == RunId::new(5)));
    }

    #[test]
    fn run_headers_returns_empty_for_empty_journal() {
        let (_temp, journal) = temp_journal();
        let all = journal.run_headers().expect("run_headers should succeed");
        assert!(all.is_empty(), "should return empty vec for empty journal");
    }

    #[test]
    fn put_run_header_updates_existing_run_header() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(200);

        let h1 = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(2),
            compiled_digest: WorkflowDigest::from_bytes([0x11; DIGEST_BYTES]),
            status: 0,
            accepted_at_ms: 100,
        };
        journal.put_run_header(&h1).expect("first put should succeed");

        let h2 = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(2),
            compiled_digest: WorkflowDigest::from_bytes([0x22; DIGEST_BYTES]),
            status: 1,
            accepted_at_ms: 200,
        };
        journal.put_run_header(&h2).expect("second put should succeed");

        let loaded = journal.run_header(run).expect("get should succeed");
        let found = loaded.expect("header should exist after update");
        assert_eq!(found.status, 1, "status should reflect the update");
        assert_eq!(found.accepted_at_ms, 200);
    }

    #[test]
    fn put_run_header_with_extreme_values() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(u64::MAX);
        let header = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(u32::MAX),
            compiled_digest: WorkflowDigest::from_bytes([0xFF; DIGEST_BYTES]),
            status: 255,
            accepted_at_ms: u64::MAX,
        };
        journal.put_run_header(&header).expect("put should succeed with extreme values");

        let loaded = journal.run_header(run).expect("get should succeed");
        let found = loaded.expect("header should exist");
        assert_eq!(found.run, run);
        assert_eq!(found.status, 255);
        assert_eq!(found.accepted_at_ms, u64::MAX);
    }

    #[test]
    fn put_run_header_convenience_wrapper_works() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(300);
        let header = make_header(run);
        crate::put_run_header(&journal, &header).expect("convenience wrapper should succeed");

        let loaded = journal.run_header(run).expect("get should succeed").expect("should exist");
        assert_eq!(loaded.run, run);
    }
}
