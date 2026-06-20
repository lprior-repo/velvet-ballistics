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
mod recover_tests {
    use crate::recovery::{
        RecoveryError, check_action_abi_digest, check_action_abi_digests, check_compiled_ir_digest,
        check_policy_digest, check_policy_digests,
    };
    use vb_core::{ActionId, StepIdx, WorkflowDigest};

    fn digest(byte: u8) -> WorkflowDigest {
        WorkflowDigest::from_bytes([byte; 32])
    }

    #[test]
    fn check_compiled_ir_digest_accepts_match() {
        let d = digest(0x11);
        let result = check_compiled_ir_digest(d, d);
        assert!(
            result.is_ok(),
            "matching digest should succeed, got {:?}",
            result
        );
    }

    #[test]
    fn check_compiled_ir_digest_rejects_mismatch() {
        let expected = digest(0x11);
        let found = digest(0x22);
        let result = check_compiled_ir_digest(expected, found);
        assert!(
            matches!(result, Err(RecoveryError::CompiledIrDigestMismatch { expected: e, found: f })
                if e == expected && f == found),
            "should report mismatch, got {:?}",
            result
        );
    }

    #[test]
    fn check_action_abi_digest_accepts_match() {
        let d = digest(0x33);
        let result = check_action_abi_digest(ActionId::new(1), d, d);
        assert!(
            result.is_ok(),
            "matching ABI digest should succeed, got {:?}",
            result
        );
    }

    #[test]
    fn check_action_abi_digest_rejects_mismatch() {
        let action_id = ActionId::new(7);
        let expected = digest(0xAA);
        let found = digest(0xBB);
        let result = check_action_abi_digest(action_id, expected, found);
        let Err(RecoveryError::ActionAbiMismatch {
            action_id: reported_action,
            expected: reported_expected,
            found: reported_found,
        }) = result
        else {
            panic!("should report ABI mismatch, got {result:?}");
        };
        assert_eq!(reported_action, action_id);
        assert_eq!(reported_expected, expected);
        assert_eq!(reported_found, found);
    }

    #[test]
    fn check_policy_digest_accepts_match() {
        let d = digest(0x44);
        let result = check_policy_digest(StepIdx::new(0), d, d);
        assert!(
            result.is_ok(),
            "matching policy digest should succeed, got {:?}",
            result
        );
    }

    #[test]
    fn check_policy_digest_rejects_mismatch() {
        let step = StepIdx::new(5);
        let expected = digest(0xCC);
        let found = digest(0xDD);
        let result = check_policy_digest(step, expected, found);
        let Err(RecoveryError::PolicyDigestMismatch {
            step: reported_step,
            expected: reported_expected,
            found: reported_found,
        }) = result
        else {
            panic!("should report policy mismatch, got {result:?}");
        };
        assert_eq!(reported_step, step);
        assert_eq!(reported_expected, expected);
        assert_eq!(reported_found, found);
    }

    #[test]
    fn check_action_abi_digests_accepts_all_matching() {
        let entries = vec![
            (ActionId::new(1), digest(0x11), digest(0x11)),
            (ActionId::new(2), digest(0x22), digest(0x22)),
        ];
        let result = check_action_abi_digests(&entries);
        assert!(
            result.is_ok(),
            "all matching should succeed, got {:?}",
            result
        );
    }

    #[test]
    fn check_action_abi_digests_accepts_empty_entries() {
        let result = check_action_abi_digests(&[]);
        assert!(
            result.is_ok(),
            "empty entries should succeed, got {:?}",
            result
        );
    }

    #[test]
    fn check_action_abi_digests_rejects_first_mismatch() {
        let entries = vec![
            (ActionId::new(1), digest(0x11), digest(0x11)),
            (ActionId::new(2), digest(0x22), digest(0x33)),
            (ActionId::new(3), digest(0x44), digest(0x44)),
        ];
        let result = check_action_abi_digests(&entries);
        let Err(RecoveryError::ActionAbiMismatch {
            action_id,
            expected,
            found,
        }) = result
        else {
            panic!("should report first mismatch, got {result:?}");
        };
        assert_eq!(action_id, ActionId::new(2));
        assert_eq!(expected, digest(0x22));
        assert_eq!(found, digest(0x33));
    }

    #[test]
    fn check_policy_digests_accepts_all_matching() {
        let entries = vec![
            (StepIdx::new(0), digest(0x55), digest(0x55)),
            (StepIdx::new(1), digest(0x66), digest(0x66)),
        ];
        let result = check_policy_digests(&entries);
        assert!(
            result.is_ok(),
            "all matching should succeed, got {:?}",
            result
        );
    }

    #[test]
    fn check_policy_digests_accepts_empty_entries() {
        let result = check_policy_digests(&[]);
        assert!(
            result.is_ok(),
            "empty entries should succeed, got {:?}",
            result
        );
    }

    #[test]
    fn check_policy_digests_rejects_first_mismatch() {
        let entries = vec![
            (StepIdx::new(0), digest(0x55), digest(0x55)),
            (StepIdx::new(3), digest(0x77), digest(0x88)),
        ];
        let result = check_policy_digests(&entries);
        let Err(RecoveryError::PolicyDigestMismatch {
            step,
            expected,
            found,
        }) = result
        else {
            panic!("should report first policy mismatch, got {result:?}");
        };
        assert_eq!(step, StepIdx::new(3));
        assert_eq!(expected, digest(0x77));
        assert_eq!(found, digest(0x88));
    }
}
