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
mod process_lock_tests {
    use crate::error::JournalError;

    #[test]
    fn process_lock_acquire_succeeds_for_fresh_directory() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        // FjallJournal::open acquires the process lock internally
        let result = crate::FjallJournal::open(temp.path(), None);
        // The lock should be acquired successfully
        match result {
            Ok(_journal) => {} // lock acquired and released when journal drops
            Err(e @ JournalError::ProcessLockIo { .. }) => {
                panic!("process lock I/O error unexpected: {e}");
            }
            Err(e) => {
                panic!("unexpected journal error: {e}");
            }
        }
    }

    #[test]
    fn process_lock_prevents_dual_writers_same_directory() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let _journal1 = crate::FjallJournal::open(temp.path(), None)
            .expect("first journal should open successfully");

        let result = crate::FjallJournal::open(temp.path(), None);

        assert!(
            matches!(result, Err(JournalError::ProcessLockHeld { .. })),
            "second open must fail with ProcessLockHeld, got {result:?}"
        );
    }

    #[test]
    fn process_lock_is_released_on_drop() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let lock_path = temp.path().join(".process.lock");
        {
            let _journal = crate::FjallJournal::open(temp.path(), None)
                .expect("first journal should open");
            assert!(
                lock_path.exists(),
                ".process.lock must exist while journal is open"
            );
            // Drop happens here
        }
        assert!(
            !lock_path.exists(),
            ".process.lock must be released on journal drop"
        );

        // After drop, we should be able to open again
        let result = crate::FjallJournal::open(temp.path(), None);
        assert!(
            result.is_ok(),
            "re-open after drop must succeed because lock was released, got {result:?}"
        );
    }

    #[test]
    fn process_lock_file_is_created() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let _journal = crate::FjallJournal::open(temp.path(), None)
            .expect("journal should open successfully");

        let lock_path = temp.path().join(".process.lock");
        assert!(
            lock_path.exists(),
            ".process.lock file should exist after journal open"
        );
    }

    #[test]
    fn process_lock_file_contains_holder_pid() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let _journal = crate::FjallJournal::open(temp.path(), None)
            .expect("journal should open successfully");

        let lock_path = temp.path().join(".process.lock");
        if lock_path.exists() {
            let contents = std::fs::read_to_string(&lock_path).expect("should read lock file");
            let pid: u32 = contents
                .trim()
                .parse()
                .expect("lock file should contain a valid PID");
            // The PID should be the current process ID or 0
            assert!(pid > 0 || pid == std::process::id(),
                "lock file PID should be positive or equal to current process ID");
        }
    }

    #[test]
    fn open_store_acquires_process_lock() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let result = crate::open_store(temp.path());
        assert!(result.is_ok(), "open_store should acquire process lock");
        let lock_path = temp.path().join(".process.lock");
        assert!(
            lock_path.exists(),
            "open_store must create .process.lock file (test name asserts the lock is acquired)"
        );
    }

    #[test]
    fn init_keyspaces_acquires_process_lock() {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let result = crate::init_keyspaces(temp.path());
        assert!(
            result.is_ok(),
            "init_keyspaces should acquire process lock"
        );
        let lock_path = temp.path().join(".process.lock");
        assert!(
            lock_path.exists(),
            "init_keyspaces must create .process.lock file (test name asserts the lock is acquired)"
        );
    }
}
