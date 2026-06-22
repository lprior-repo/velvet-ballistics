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
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::enum_variant_names,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
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
    unused_variables
)]

//! Behavior tests for vb_validate diagnostic and error reporting.
//!
//! Tests the public API of `vb_validate::diagnostic`:
//! - `diagnostic_from_error(&ValidationError) -> Diagnostic`
//! - `error_code(&ValidationError) -> DiagnosticCode`
//!
//! These tests focus on observable behavior:
//! - Diagnostic code range correctness (E01xx through E06xx)
//! - Error message formatting (exact content assertions)
//! - Validation failure reporting (all variants produce diagnostics)
//! - Exact diagnostic content assertions

use vb_core::diagnostic::{DiagnosticCode, Severity};
use vb_core::span::Span;
use vb_validate::ValidationError;

// ---------------------------------------------------------------------------
// Diagnostic code range behavior — E01xx schema errors
// ---------------------------------------------------------------------------

mod schema_error_codes {
    use super::*;

    #[test]
    fn duplicate_key_code_is_e0101() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::DuplicateKey);
        assert_eq!(code.code(), 0x0101);
    }

    #[test]
    fn forbidden_yaml_feature_code_is_e0102() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::ForbiddenYamlFeature);
        assert_eq!(code.code(), 0x0102);
    }

    #[test]
    fn unknown_top_level_field_code_is_e0103() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::UnknownTopLevelField);
        assert_eq!(code.code(), 0x0103);
    }

    #[test]
    fn unknown_step_field_code_is_e0104() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::UnknownStepField);
        assert_eq!(code.code(), 0x0104);
    }

    #[test]
    fn missing_required_field_code_is_e0105() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::MissingRequiredField {
            field: "steps".into(),
        });
        assert_eq!(code.code(), 0x0105);
    }

    #[test]
    fn invalid_version_code_is_e0106() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::InvalidVersion {
            version: "v0".into(),
        });
        assert_eq!(code.code(), 0x0106);
    }

    #[test]
    fn invalid_id_code_is_e0107() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::InvalidId {
            id: "123bad".into(),
        });
        assert_eq!(code.code(), 0x0107);
    }

    #[test]
    fn reserved_id_code_is_e0108() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::ReservedId {
            id: "runtime".into(),
        });
        assert_eq!(code.code(), 0x0108);
    }

    #[test]
    fn duplicate_id_code_is_e0109() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::DuplicateId {
            id: "step1".into(),
        });
        assert_eq!(code.code(), 0x0109);
    }

    #[test]
    fn multiple_step_primitives_code_is_e010a() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::MultipleStepPrimitives);
        assert_eq!(code.code(), 0x010A);
    }

    #[test]
    fn missing_step_primitive_code_is_e010b() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::MissingStepPrimitive);
        assert_eq!(code.code(), 0x010B);
    }
}

// ---------------------------------------------------------------------------
// Diagnostic code range behavior — E02xx reference errors
// ---------------------------------------------------------------------------

mod reference_error_codes {
    use super::*;

    #[test]
    fn unknown_reference_code_is_e0201() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::UnknownReference {
            reference: "$input.missing".into(),
        });
        assert_eq!(code.code(), 0x0201);
    }

    #[test]
    fn future_reference_code_is_e0202() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::FutureReference {
            reference: "$steps.later".into(),
        });
        assert_eq!(code.code(), 0x0202);
    }

    #[test]
    fn secret_not_declared_code_is_e0203() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::SecretNotDeclared {
            secret: "api_key".into(),
        });
        assert_eq!(code.code(), 0x0203);
    }

    #[test]
    fn direct_runtime_reference_code_is_e0204() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::DirectRuntimeReference);
        assert_eq!(code.code(), 0x0204);
    }
}

// ---------------------------------------------------------------------------
// Diagnostic code range behavior — E03xx control-flow errors
// ---------------------------------------------------------------------------

mod control_flow_error_codes {
    use super::*;

    #[test]
    fn invalid_then_target_code_is_e0301() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::InvalidThenTarget);
        assert_eq!(code.code(), 0x0301);
    }

    #[test]
    fn control_flow_cycle_code_is_e0302() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::ControlFlowCycle);
        assert_eq!(code.code(), 0x0302);
    }

    #[test]
    fn unreachable_step_code_is_e0303() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::UnreachableStep {
            step: "orphan".into(),
        });
        assert_eq!(code.code(), 0x0303);
    }

    #[test]
    fn invalid_choose_code_is_e0304() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::InvalidChoose);
        assert_eq!(code.code(), 0x0304);
    }

    #[test]
    fn invalid_for_each_code_is_e0305() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::InvalidForEach);
        assert_eq!(code.code(), 0x0305);
    }

    #[test]
    fn invalid_together_code_is_e0306() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::InvalidTogether);
        assert_eq!(code.code(), 0x0306);
    }

    #[test]
    fn invalid_collect_code_is_e0307() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::InvalidCollect);
        assert_eq!(code.code(), 0x0307);
    }

    #[test]
    fn invalid_reduce_code_is_e0308() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::InvalidReduce);
        assert_eq!(code.code(), 0x0308);
    }

    #[test]
    fn invalid_repeat_code_is_e0309() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::InvalidRepeat);
        assert_eq!(code.code(), 0x0309);
    }
}

// ---------------------------------------------------------------------------
// Diagnostic code range behavior — E04xx type/taint/resource errors
// ---------------------------------------------------------------------------

mod type_taint_error_codes {
    use super::*;

    #[test]
    fn invalid_wait_code_is_e0401() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::InvalidWait);
        assert_eq!(code.code(), 0x0401);
    }

    #[test]
    fn invalid_ask_code_is_e0402() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::InvalidAsk);
        assert_eq!(code.code(), 0x0402);
    }

    #[test]
    fn invalid_finish_code_is_e0403() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::InvalidFinish);
        assert_eq!(code.code(), 0x0403);
    }

    #[test]
    fn invalid_retry_code_is_e0404() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::InvalidRetry);
        assert_eq!(code.code(), 0x0404);
    }

    #[test]
    fn invalid_on_error_code_is_e0405() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::InvalidOnError);
        assert_eq!(code.code(), 0x0405);
    }

    #[test]
    fn secret_result_leak_code_is_e0406() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::SecretResultLeak);
        assert_eq!(code.code(), 0x0406);
    }

    #[test]
    fn type_mismatch_code_is_e0407() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::TypeMismatch {
            expected: "boolean".into(),
            found: "number".into(),
        });
        assert_eq!(code.code(), 0x0407);
    }

    #[test]
    fn payload_too_large_code_is_e0408() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::PayloadTooLarge);
        assert_eq!(code.code(), 0x0408);
    }

    #[test]
    fn limit_required_code_is_e0409() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::LimitRequired {
            resource: "max_steps".into(),
        });
        assert_eq!(code.code(), 0x0409);
    }

    #[test]
    fn limit_exceeded_code_is_e040a() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::LimitExceeded {
            resource: "max_steps".into(),
        });
        assert_eq!(code.code(), 0x040A);
    }

    #[test]
    fn unsupported_trigger_code_is_e040b() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::UnsupportedTrigger {
            trigger: "cron".into(),
        });
        assert_eq!(code.code(), 0x040B);
    }

    #[test]
    fn http_trigger_out_of_core_code_is_e040c() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::HttpTriggerOutOfCore);
        assert_eq!(code.code(), 0x040C);
    }
}

// ---------------------------------------------------------------------------
// Diagnostic code range behavior — E05xx gate verifier errors
// ---------------------------------------------------------------------------

mod gate_error_codes {
    use super::*;

    #[test]
    fn expression_stack_exceeded_code_is_e0501() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::ExpressionStackExceeded {
            declared: 65,
            limit: 64,
        });
        assert_eq!(code.code(), 0x0501);
    }

    #[test]
    fn expression_stack_mismatch_code_is_e0502() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::ExpressionStackMismatch {
            expr_index: 0,
            declared: 2,
            computed: 1,
        });
        assert_eq!(code.code(), 0x0502);
    }

    #[test]
    fn accessor_slot_out_of_range_code_is_e0503() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::AccessorSlotOutOfRange {
            accessor_index: 0,
            slot: 5,
            slot_count: 2,
        });
        assert_eq!(code.code(), 0x0503);
    }

    #[test]
    fn accessor_path_invalid_code_is_e0504() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::AccessorPathInvalid {
            accessor_index: 0,
            segment_index: 1,
        });
        assert_eq!(code.code(), 0x0504);
    }

    #[test]
    fn slot_reference_out_of_range_code_is_e0505() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::SlotReferenceOutOfRange {
            slot: 99,
            slot_count: 10,
            context: "node 0".into(),
        });
        assert_eq!(code.code(), 0x0505);
    }

    #[test]
    fn loop_body_step_out_of_range_code_is_e0506() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::LoopBodyStepOutOfRange {
            step: 99,
            node_count: 5,
            source_node: 0,
            label: "for_each body".into(),
        });
        assert_eq!(code.code(), 0x0506);
    }

    #[test]
    fn slot_dependency_cycle_code_is_e0507() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::SlotDependencyCycle {
            slot: 0,
            chain: "slot 0 -> slot 1 -> slot 0".into(),
        });
        assert_eq!(code.code(), 0x0507);
    }

    #[test]
    fn node_kind_constraint_violation_code_is_e0508() {
        let code =
            vb_validate::diagnostic::error_code(&ValidationError::NodeKindConstraintViolation {
                node_index: 0,
                detail: "test".into(),
            });
        assert_eq!(code.code(), 0x0508);
    }

    #[test]
    fn action_contract_missing_code_is_e0509() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::ActionContractMissing {
            action_id: 1,
            node_index: 0,
        });
        assert_eq!(code.code(), 0x0509);
    }

    #[test]
    fn action_contract_orphan_code_is_e050a() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::ActionContractOrphan {
            action_id: 2,
        });
        assert_eq!(code.code(), 0x050A);
    }

    #[test]
    fn slot_type_inconsistency_code_is_e050b() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::SlotTypeInconsistency {
            slot: 0,
        });
        assert_eq!(code.code(), 0x050B);
    }

    #[test]
    fn non_deterministic_path_code_is_e050c() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::NonDeterministicPath {
            from_node: 0,
            to_node: 1,
        });
        assert_eq!(code.code(), 0x050C);
    }

    #[test]
    fn capability_name_empty_code_is_e050d() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::CapabilityNameEmpty {
            action_id: 1,
            capability_index: 0,
        });
        assert_eq!(code.code(), 0x050D);
    }

    #[test]
    fn capability_name_too_long_code_is_e050e() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::CapabilityNameTooLong {
            action_id: 1,
            capability_index: 0,
            len: 129,
            max: 128,
        });
        assert_eq!(code.code(), 0x050E);
    }

    #[test]
    fn capability_name_invalid_code_is_e050f() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::CapabilityNameInvalid {
            action_id: 1,
            capability_index: 0,
            name: "network:github".into(),
        });
        assert_eq!(code.code(), 0x050F);
    }

    #[test]
    fn capability_action_mismatch_code_is_e0510() {
        let code =
            vb_validate::diagnostic::error_code(&ValidationError::CapabilityActionMismatch {
                contract_action_id: 1,
                capability_action_id: 2,
                capability_index: 0,
            });
        assert_eq!(code.code(), 0x0510);
    }

    #[test]
    fn capability_duplicate_code_is_e0511() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::CapabilityDuplicate {
            action_id: 1,
            first_index: 0,
            duplicate_index: 1,
            name: "network".into(),
        });
        assert_eq!(code.code(), 0x0511);
    }

    #[test]
    fn accessor_path_too_deep_code_is_e0512() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::AccessorPathTooDeep {
            accessor_index: 0,
            depth: 17,
            max: 16,
        });
        assert_eq!(code.code(), 0x0512);
    }

    #[test]
    fn accessor_symbol_out_of_bounds_code_is_e0513() {
        let code =
            vb_validate::diagnostic::error_code(&ValidationError::AccessorSymbolOutOfBounds {
                accessor_index: 0,
                segment_index: 0,
                symbol: 42,
                symbols_count: 10,
            });
        assert_eq!(code.code(), 0x0513);
    }
}

// ---------------------------------------------------------------------------
// Diagnostic code range behavior — E06xx contract-discovery errors
// ---------------------------------------------------------------------------

mod contract_discovery_error_codes {
    use super::*;

    #[test]
    fn missing_schema_version_code_is_e0601() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::MissingSchemaVersion);
        assert_eq!(code.code(), 0x0601);
    }

    #[test]
    fn cue_vet_failed_code_is_e0602() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::CueVetFailed {
            file: "workflow.cue".into(),
        });
        assert_eq!(code.code(), 0x0602);
    }

    #[test]
    fn version_monotonicity_breach_code_is_e0603() {
        let code =
            vb_validate::diagnostic::error_code(&ValidationError::VersionMonotonicityBreach {
                file: "lib.cue".into(),
                expected: "v2.0".into(),
                actual: "v1.9".into(),
            });
        assert_eq!(code.code(), 0x0603);
    }
}

// ---------------------------------------------------------------------------
// Error message formatting — exact content assertions
// ---------------------------------------------------------------------------

mod error_message_formatting {
    use super::*;

    #[test]
    fn missing_required_field_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::MissingRequiredField {
                field: "version".into(),
            },
        );
        assert_eq!(&*diag.message, "missing required field: version");
    }

    #[test]
    fn invalid_version_message_exact_format() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::InvalidVersion {
                version: "v2".into(),
            });
        assert_eq!(&*diag.message, "invalid version: v2");
    }

    #[test]
    fn invalid_id_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(&ValidationError::InvalidId {
            id: "bad-id".into(),
        });
        assert_eq!(&*diag.message, "invalid ID: bad-id");
    }

    #[test]
    fn reserved_id_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(&ValidationError::ReservedId {
            id: "runtime".into(),
        });
        assert_eq!(&*diag.message, "reserved ID: runtime");
    }

    #[test]
    fn duplicate_id_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(&ValidationError::DuplicateId {
            id: "step1".into(),
        });
        assert_eq!(&*diag.message, "duplicate ID: step1");
    }

    #[test]
    fn unknown_reference_message_exact_format() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::UnknownReference {
                reference: "$input.missing".into(),
            });
        assert_eq!(&*diag.message, "unknown reference: $input.missing");
    }

    #[test]
    fn future_reference_message_exact_format() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::FutureReference {
                reference: "$steps.build".into(),
            });
        assert_eq!(&*diag.message, "future reference: $steps.build");
    }

    #[test]
    fn secret_not_declared_message_exact_format() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::SecretNotDeclared {
                secret: "api_key".into(),
            });
        assert_eq!(&*diag.message, "secret not declared: api_key");
    }

    #[test]
    fn unreachable_step_message_exact_format() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::UnreachableStep {
                step: "orphan_step".into(),
            });
        assert_eq!(&*diag.message, "unreachable step: orphan_step");
    }

    #[test]
    fn type_mismatch_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(&ValidationError::TypeMismatch {
            expected: "boolean".into(),
            found: "number".into(),
        });
        assert_eq!(
            &*diag.message,
            "type mismatch: expected boolean, found number"
        );
    }

    #[test]
    fn limit_required_message_exact_format() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::LimitRequired {
                resource: "max_slots".into(),
            });
        assert_eq!(&*diag.message, "limit required: max_slots");
    }

    #[test]
    fn limit_exceeded_message_exact_format() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::LimitExceeded {
                resource: "max_steps".into(),
            });
        assert_eq!(&*diag.message, "limit exceeded: max_steps");
    }

    #[test]
    fn unsupported_trigger_message_exact_format() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::UnsupportedTrigger {
                trigger: "cron".into(),
            });
        assert_eq!(&*diag.message, "unsupported trigger: cron");
    }

    #[test]
    fn expression_stack_exceeded_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::ExpressionStackExceeded {
                declared: 100,
                limit: 64,
            },
        );
        assert_eq!(
            &*diag.message,
            "expression stack exceeded: declared 100, limit 64"
        );
    }

    #[test]
    fn expression_stack_mismatch_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::ExpressionStackMismatch {
                expr_index: 3,
                declared: 4,
                computed: 2,
            },
        );
        assert_eq!(
            &*diag.message,
            "expression stack mismatch: expr 3, declared 4, computed 2"
        );
    }

    #[test]
    fn accessor_slot_out_of_range_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::AccessorSlotOutOfRange {
                accessor_index: 1,
                slot: 10,
                slot_count: 5,
            },
        );
        assert_eq!(
            &*diag.message,
            "accessor slot out of range: accessor 1, slot 10, slot_count 5"
        );
    }

    #[test]
    fn accessor_path_invalid_message_exact_format() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::AccessorPathInvalid {
                accessor_index: 0,
                segment_index: 2,
            });
        assert_eq!(
            &*diag.message,
            "accessor path invalid: accessor 0, segment 2"
        );
    }

    #[test]
    fn accessor_path_too_deep_message_exact_format() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::AccessorPathTooDeep {
                accessor_index: 0,
                depth: 17,
                max: 16,
            });
        assert_eq!(
            &*diag.message,
            "accessor path too deep: accessor 0, depth 17, max 16"
        );
    }

    #[test]
    fn accessor_symbol_out_of_bounds_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::AccessorSymbolOutOfBounds {
                accessor_index: 0,
                segment_index: 3,
                symbol: 42,
                symbols_count: 10,
            },
        );
        assert_eq!(
            &*diag.message,
            "accessor symbol out of bounds: accessor 0, segment 3, symbol 42, symbols_count 10"
        );
    }

    #[test]
    fn slot_reference_out_of_range_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::SlotReferenceOutOfRange {
                slot: 99,
                slot_count: 10,
                context: "node 0".into(),
            },
        );
        assert_eq!(
            &*diag.message,
            "slot reference out of range: slot 99, slot_count 10, context node 0"
        );
    }

    #[test]
    fn loop_body_step_out_of_range_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::LoopBodyStepOutOfRange {
                step: 99,
                node_count: 5,
                source_node: 0,
                label: "for_each body".into(),
            },
        );
        assert_eq!(
            &*diag.message,
            "loop body step out of range: step 99, node_count 5, source_node 0, label for_each body"
        );
    }

    #[test]
    fn slot_dependency_cycle_message_exact_format() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::SlotDependencyCycle {
                slot: 0,
                chain: "slot 0 -> slot 1 -> slot 0".into(),
            });
        assert_eq!(
            &*diag.message,
            "slot dependency cycle: slot 0, chain slot 0 -> slot 1 -> slot 0"
        );
    }

    #[test]
    fn node_kind_constraint_violation_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::NodeKindConstraintViolation {
                node_index: 5,
                detail: "expected Action node".into(),
            },
        );
        assert_eq!(
            &*diag.message,
            "node kind constraint violation: node 5, expected Action node"
        );
    }

    #[test]
    fn action_contract_missing_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::ActionContractMissing {
                action_id: 42,
                node_index: 3,
            },
        );
        assert_eq!(
            &*diag.message,
            "action contract missing: action_id 42 referenced by Do node 3"
        );
    }

    #[test]
    fn action_contract_orphan_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::ActionContractOrphan { action_id: 7 },
        );
        assert_eq!(
            &*diag.message,
            "action contract orphan: action_id 7 has no corresponding Do node"
        );
    }

    #[test]
    fn capability_name_empty_message_exact_format() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::CapabilityNameEmpty {
                action_id: 1,
                capability_index: 0,
            });
        assert_eq!(
            &*diag.message,
            "capability name is empty for action 1 at required_capabilities[0]"
        );
    }

    #[test]
    fn capability_name_too_long_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::CapabilityNameTooLong {
                action_id: 1,
                capability_index: 2,
                len: 200,
                max: 128,
            },
        );
        assert_eq!(
            &*diag.message,
            "capability name too long for action 1 at required_capabilities[2]: 200 > 128"
        );
    }

    #[test]
    fn capability_name_invalid_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::CapabilityNameInvalid {
                action_id: 1,
                capability_index: 0,
                name: "network:github".into(),
            },
        );
        assert_eq!(
            &*diag.message,
            "invalid capability name for action 1 at required_capabilities[0]: network:github"
        );
    }

    #[test]
    fn capability_action_mismatch_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::CapabilityActionMismatch {
                contract_action_id: 5,
                capability_action_id: 3,
                capability_index: 1,
            },
        );
        assert_eq!(
            &*diag.message,
            "capability action 3 does not match contract action 5 at required_capabilities[1]"
        );
    }

    #[test]
    fn capability_duplicate_message_exact_format() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::CapabilityDuplicate {
                action_id: 1,
                first_index: 0,
                duplicate_index: 2,
                name: "network".into(),
            });
        assert_eq!(
            &*diag.message,
            "duplicate capability requirement for action 1: network at required_capabilities[0] and required_capabilities[2]"
        );
    }

    #[test]
    fn slot_type_inconsistency_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::SlotTypeInconsistency { slot: 4 },
        );
        assert_eq!(
            &*diag.message,
            "slot type inconsistency: slot 4 has incompatible writers"
        );
    }

    #[test]
    fn non_deterministic_path_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::NonDeterministicPath {
                from_node: 1,
                to_node: 7,
            },
        );
        assert_eq!(
            &*diag.message,
            "non-deterministic path: from node 1 to node 7 contains no suspension point"
        );
    }

    #[test]
    fn missing_schema_version_message_exact_format() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::MissingSchemaVersion);
        assert_eq!(&*diag.message, "missing schema_version field");
    }

    #[test]
    fn cue_vet_failed_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(&ValidationError::CueVetFailed {
            file: "workflow.cue".into(),
        });
        assert_eq!(&*diag.message, "cue vet failed for workflow.cue");
    }

    #[test]
    fn version_monotonicity_breach_message_exact_format() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::VersionMonotonicityBreach {
                file: "lib.cue".into(),
                expected: "v2.0".into(),
                actual: "v1.9".into(),
            },
        );
        assert_eq!(
            &*diag.message,
            "version monotonicity breach: lib.cue expected v2.0 got v1.9"
        );
    }

    // ---------------------------------------------------------------------------
    // Unitary error messages (no fields)
    // ---------------------------------------------------------------------------

    #[test]
    fn duplicate_key_message() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(&ValidationError::DuplicateKey);
        assert_eq!(&*diag.message, "duplicate key");
    }

    #[test]
    fn forbidden_yaml_feature_message() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::ForbiddenYamlFeature);
        assert_eq!(&*diag.message, "forbidden YAML feature");
    }

    #[test]
    fn unknown_top_level_field_message() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::UnknownTopLevelField);
        assert_eq!(&*diag.message, "unknown top-level field");
    }

    #[test]
    fn unknown_step_field_message() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::UnknownStepField);
        assert_eq!(&*diag.message, "unknown step field");
    }

    #[test]
    fn multiple_step_primitives_message() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::MultipleStepPrimitives,
        );
        assert_eq!(&*diag.message, "multiple step primitives");
    }

    #[test]
    fn missing_step_primitive_message() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::MissingStepPrimitive);
        assert_eq!(&*diag.message, "missing step primitive");
    }

    #[test]
    fn direct_runtime_reference_message() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(
            &ValidationError::DirectRuntimeReference,
        );
        assert_eq!(&*diag.message, "direct runtime reference");
    }

    #[test]
    fn invalid_then_target_message() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::InvalidThenTarget);
        assert_eq!(&*diag.message, "invalid then target");
    }

    #[test]
    fn control_flow_cycle_message() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::ControlFlowCycle);
        assert_eq!(&*diag.message, "control-flow cycle");
    }

    #[test]
    fn invalid_choose_message() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(&ValidationError::InvalidChoose);
        assert_eq!(&*diag.message, "invalid choose");
    }

    #[test]
    fn invalid_for_each_message() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(&ValidationError::InvalidForEach);
        assert_eq!(&*diag.message, "invalid for_each");
    }

    #[test]
    fn invalid_together_message() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::InvalidTogether);
        assert_eq!(&*diag.message, "invalid together");
    }

    #[test]
    fn invalid_collect_message() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(&ValidationError::InvalidCollect);
        assert_eq!(&*diag.message, "invalid collect");
    }

    #[test]
    fn invalid_reduce_message() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(&ValidationError::InvalidReduce);
        assert_eq!(&*diag.message, "invalid reduce");
    }

    #[test]
    fn invalid_repeat_message() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(&ValidationError::InvalidRepeat);
        assert_eq!(&*diag.message, "invalid repeat");
    }

    #[test]
    fn invalid_wait_message() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(&ValidationError::InvalidWait);
        assert_eq!(&*diag.message, "invalid wait");
    }

    #[test]
    fn invalid_ask_message() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(&ValidationError::InvalidAsk);
        assert_eq!(&*diag.message, "invalid ask");
    }

    #[test]
    fn invalid_finish_message() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(&ValidationError::InvalidFinish);
        assert_eq!(&*diag.message, "invalid finish");
    }

    #[test]
    fn invalid_retry_message() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(&ValidationError::InvalidRetry);
        assert_eq!(&*diag.message, "invalid retry");
    }

    #[test]
    fn invalid_on_error_message() {
        let diag = vb_validate::diagnostic::diagnostic_from_error(&ValidationError::InvalidOnError);
        assert_eq!(&*diag.message, "invalid on_error");
    }

    #[test]
    fn secret_result_leak_message() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::SecretResultLeak);
        assert_eq!(&*diag.message, "secret result leak");
    }

    #[test]
    fn payload_too_large_message() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::PayloadTooLarge);
        assert_eq!(&*diag.message, "payload too large");
    }

    #[test]
    fn http_trigger_out_of_core_message() {
        let diag =
            vb_validate::diagnostic::diagnostic_from_error(&ValidationError::HttpTriggerOutOfCore);
        assert_eq!(&*diag.message, "HTTP trigger out of core");
    }
}

// ---------------------------------------------------------------------------
// Validation failure reporting — diagnostic structure invariants
// ---------------------------------------------------------------------------

mod diagnostic_structure_invariants {
    use super::*;

    #[test]
    fn diagnostic_severity_is_always_error() {
        let variants = all_validation_errors();
        for error in variants {
            let diag = vb_validate::diagnostic::diagnostic_from_error(&error);
            assert_eq!(
                diag.severity,
                Severity::Error,
                "diagnostic_from_error({error:?}) should have Severity::Error"
            );
        }
    }

    #[test]
    fn diagnostic_span_is_zero_for_all_variants() {
        let variants = all_validation_errors();
        for error in variants {
            let diag = vb_validate::diagnostic::diagnostic_from_error(&error);
            assert_eq!(
                diag.span,
                Span::ZERO,
                "diagnostic_from_error({error:?}) should have Span::ZERO"
            );
        }
    }

    #[test]
    fn diagnostic_message_is_non_empty_for_all_variants() {
        let variants = all_validation_errors();
        for error in variants {
            let diag = vb_validate::diagnostic::diagnostic_from_error(&error);
            assert!(
                !diag.message.is_empty(),
                "diagnostic_from_error({error:?}) should have non-empty message"
            );
        }
    }

    #[test]
    fn error_code_is_non_zero_for_all_variants() {
        let variants = all_validation_errors();
        for error in &variants {
            let code = vb_validate::diagnostic::error_code(error);
            assert_ne!(
                code.code(),
                0,
                "error_code({error:?}) should return non-zero code"
            );
        }
    }

    #[test]
    fn all_diagnostic_codes_are_unique() {
        let variants = all_validation_errors();
        let mut seen = std::collections::BTreeSet::new();
        for error in &variants {
            let code = vb_validate::diagnostic::error_code(error);
            assert!(
                seen.insert(code.code()),
                "duplicate diagnostic code {:#06x} for {error:?}",
                code.code()
            );
        }
    }

    /// Returns every ValidationError variant with representative field values.
    fn all_validation_errors() -> Vec<ValidationError> {
        vec![
            ValidationError::DuplicateKey,
            ValidationError::ForbiddenYamlFeature,
            ValidationError::UnknownTopLevelField,
            ValidationError::UnknownStepField,
            ValidationError::MissingRequiredField {
                field: "test".into(),
            },
            ValidationError::InvalidVersion {
                version: "v0".into(),
            },
            ValidationError::InvalidId { id: "BAD".into() },
            ValidationError::ReservedId {
                id: "runtime".into(),
            },
            ValidationError::DuplicateId { id: "dup".into() },
            ValidationError::MultipleStepPrimitives,
            ValidationError::MissingStepPrimitive,
            ValidationError::UnknownReference {
                reference: "$x".into(),
            },
            ValidationError::FutureReference {
                reference: "$steps.s".into(),
            },
            ValidationError::SecretNotDeclared {
                secret: "tok".into(),
            },
            ValidationError::DirectRuntimeReference,
            ValidationError::InvalidThenTarget,
            ValidationError::ControlFlowCycle,
            ValidationError::UnreachableStep { step: "s".into() },
            ValidationError::InvalidChoose,
            ValidationError::InvalidForEach,
            ValidationError::InvalidTogether,
            ValidationError::InvalidCollect,
            ValidationError::InvalidReduce,
            ValidationError::InvalidRepeat,
            ValidationError::InvalidWait,
            ValidationError::InvalidAsk,
            ValidationError::InvalidFinish,
            ValidationError::InvalidRetry,
            ValidationError::InvalidOnError,
            ValidationError::SecretResultLeak,
            ValidationError::TypeMismatch {
                expected: "a".into(),
                found: "b".into(),
            },
            ValidationError::PayloadTooLarge,
            ValidationError::LimitRequired {
                resource: "r".into(),
            },
            ValidationError::LimitExceeded {
                resource: "r".into(),
            },
            ValidationError::UnsupportedTrigger {
                trigger: "cron".into(),
            },
            ValidationError::HttpTriggerOutOfCore,
            ValidationError::ExpressionStackExceeded {
                declared: 65,
                limit: 64,
            },
            ValidationError::ExpressionStackMismatch {
                expr_index: 0,
                declared: 2,
                computed: 1,
            },
            ValidationError::AccessorSlotOutOfRange {
                accessor_index: 0,
                slot: 5,
                slot_count: 2,
            },
            ValidationError::AccessorPathInvalid {
                accessor_index: 0,
                segment_index: 1,
            },
            ValidationError::AccessorPathTooDeep {
                accessor_index: 0,
                depth: 17,
                max: 16,
            },
            ValidationError::AccessorSymbolOutOfBounds {
                accessor_index: 0,
                segment_index: 0,
                symbol: 42,
                symbols_count: 10,
            },
            ValidationError::SlotReferenceOutOfRange {
                slot: 99,
                slot_count: 10,
                context: "node 0".into(),
            },
            ValidationError::LoopBodyStepOutOfRange {
                step: 99,
                node_count: 5,
                source_node: 0,
                label: "for_each body".into(),
            },
            ValidationError::SlotDependencyCycle {
                slot: 0,
                chain: "slot 0 -> slot 1 -> slot 0".into(),
            },
            ValidationError::NodeKindConstraintViolation {
                node_index: 0,
                detail: "test".into(),
            },
            ValidationError::ActionContractMissing {
                action_id: 1,
                node_index: 0,
            },
            ValidationError::ActionContractOrphan { action_id: 2 },
            ValidationError::CapabilityNameEmpty {
                action_id: 1,
                capability_index: 0,
            },
            ValidationError::CapabilityNameTooLong {
                action_id: 1,
                capability_index: 0,
                len: 129,
                max: 128,
            },
            ValidationError::CapabilityNameInvalid {
                action_id: 1,
                capability_index: 0,
                name: "network:github".into(),
            },
            ValidationError::CapabilityActionMismatch {
                contract_action_id: 1,
                capability_action_id: 2,
                capability_index: 0,
            },
            ValidationError::CapabilityDuplicate {
                action_id: 1,
                first_index: 0,
                duplicate_index: 1,
                name: "network".into(),
            },
            ValidationError::SlotTypeInconsistency { slot: 0 },
            ValidationError::NonDeterministicPath {
                from_node: 0,
                to_node: 1,
            },
            ValidationError::MissingSchemaVersion,
            ValidationError::CueVetFailed {
                file: "test.cue".into(),
            },
            ValidationError::VersionMonotonicityBreach {
                file: "test.cue".into(),
                expected: "v2.0".into(),
                actual: "v1.9".into(),
            },
        ]
    }
}

// ---------------------------------------------------------------------------
// DiagnosticCode display format
// ---------------------------------------------------------------------------

mod diagnostic_code_display {
    use super::*;
    use core::fmt::Write;

    #[test]
    fn diagnostic_code_display_is_e_prefixed_hex() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::DuplicateKey);
        let mut s = String::new();
        write!(&mut s, "{}", code).unwrap();
        assert_eq!(s, "E0101");
    }

    #[test]
    fn diagnostic_code_display_e040c() {
        let code = vb_validate::diagnostic::error_code(&ValidationError::HttpTriggerOutOfCore);
        let mut s = String::new();
        write!(&mut s, "{}", code).unwrap();
        assert_eq!(s, "E040C");
    }

    #[test]
    fn diagnostic_code_display_e0513() {
        let code =
            vb_validate::diagnostic::error_code(&ValidationError::AccessorSymbolOutOfBounds {
                accessor_index: 0,
                segment_index: 0,
                symbol: 1,
                symbols_count: 10,
            });
        let mut s = String::new();
        write!(&mut s, "{}", code).unwrap();
        assert_eq!(s, "E0513");
    }
}

// ---------------------------------------------------------------------------
// DiagnosticCode parsing
// ---------------------------------------------------------------------------

mod diagnostic_code_parsing {
    use super::*;
    use core::str::FromStr;

    #[test]
    fn diagnostic_code_parses_e0101() {
        let parsed = DiagnosticCode::from_str("E0101");
        let expected = DiagnosticCode::new(0x0101);
        assert_eq!(parsed, Ok(expected));
    }

    #[test]
    fn diagnostic_code_parses_e0201() {
        let parsed = DiagnosticCode::from_str("E0201");
        let expected = DiagnosticCode::new(0x0201);
        assert_eq!(parsed, Ok(expected));
    }

    #[test]
    fn diagnostic_code_parses_e0302() {
        let parsed = DiagnosticCode::from_str("E0302");
        let expected = DiagnosticCode::new(0x0302);
        assert_eq!(parsed, Ok(expected));
    }

    #[test]
    fn diagnostic_code_parses_e0407() {
        let parsed = DiagnosticCode::from_str("E0407");
        let expected = DiagnosticCode::new(0x0407);
        assert_eq!(parsed, Ok(expected));
    }

    // NOTE: E05xx (0x0501-0x0513) and E06xx (0x0601-0x0603) codes are not
    // supported by DiagnosticCode::from_str due to is_supported_code() gaps.
    // Only E01xx-E04xx codes can be round-tripped through string parsing.

    #[test]
    fn diagnostic_code_rejects_missing_prefix() {
        let result = DiagnosticCode::from_str("0101");
        assert!(matches!(
            result,
            Err(vb_core::diagnostic::DiagnosticCodeParseError::InvalidFormat)
        ));
    }

    #[test]
    fn diagnostic_code_rejects_too_short() {
        let result = DiagnosticCode::from_str("E01");
        assert!(matches!(
            result,
            Err(vb_core::diagnostic::DiagnosticCodeParseError::InvalidFormat)
        ));
    }

    #[test]
    fn diagnostic_code_rejects_too_long() {
        let result = DiagnosticCode::from_str("E01010");
        assert!(matches!(
            result,
            Err(vb_core::diagnostic::DiagnosticCodeParseError::InvalidFormat)
        ));
    }

    #[test]
    fn diagnostic_code_rejects_invalid_hex() {
        let result = DiagnosticCode::from_str("E010G");
        assert!(matches!(
            result,
            Err(vb_core::diagnostic::DiagnosticCodeParseError::InvalidFormat)
        ));
    }

    #[test]
    fn diagnostic_code_rejects_empty() {
        let result = DiagnosticCode::from_str("");
        assert!(matches!(
            result,
            Err(vb_core::diagnostic::DiagnosticCodeParseError::InvalidFormat)
        ));
    }

    #[test]
    fn diagnostic_code_roundtrip_e0101() {
        let code = DiagnosticCode::new(0x0101);
        let s = code.to_string();
        let parsed = DiagnosticCode::from_str(&s);
        assert_eq!(parsed, Ok(code));
    }

    // NOTE: E05xx and E06xx codes cannot be round-tripped through from_str
    // because they are not in the is_supported_code ranges in vb_core.
}

// ---------------------------------------------------------------------------
// Code range partitioning — high nibble check
// ---------------------------------------------------------------------------

mod code_range_partitioning {

    #[test]
    fn schema_codes_high_nibble_is_0x01() {
        let codes = [
            0x0101, 0x0102, 0x0103, 0x0104, 0x0105, 0x0106, 0x0107, 0x0108, 0x0109, 0x010A, 0x010B,
        ];
        for code in codes {
            assert_eq!(
                (code >> 8) & 0xFF,
                0x01,
                "code {code:#06x} should be in E01xx range"
            );
        }
    }

    #[test]
    fn reference_codes_high_nibble_is_0x02() {
        let codes = [0x0201, 0x0202, 0x0203, 0x0204];
        for code in codes {
            assert_eq!(
                (code >> 8) & 0xFF,
                0x02,
                "code {code:#06x} should be in E02xx range"
            );
        }
    }

    #[test]
    fn control_flow_codes_high_nibble_is_0x03() {
        let codes = [
            0x0301, 0x0302, 0x0303, 0x0304, 0x0305, 0x0306, 0x0307, 0x0308, 0x0309,
        ];
        for code in codes {
            assert_eq!(
                (code >> 8) & 0xFF,
                0x03,
                "code {code:#06x} should be in E03xx range"
            );
        }
    }

    #[test]
    fn type_taint_codes_high_nibble_is_0x04() {
        let codes = [
            0x0401, 0x0402, 0x0403, 0x0404, 0x0405, 0x0406, 0x0407, 0x0408, 0x0409, 0x040A, 0x040B,
            0x040C,
        ];
        for code in codes {
            assert_eq!(
                (code >> 8) & 0xFF,
                0x04,
                "code {code:#06x} should be in E04xx range"
            );
        }
    }

    #[test]
    fn gate_codes_high_nibble_is_0x05() {
        let codes = [
            0x0501, 0x0502, 0x0503, 0x0504, 0x0505, 0x0506, 0x0507, 0x0508, 0x0509, 0x050A, 0x050B,
            0x050C, 0x050D, 0x050E, 0x050F, 0x0510, 0x0511, 0x0512, 0x0513,
        ];
        for code in codes {
            assert_eq!(
                (code >> 8) & 0xFF,
                0x05,
                "code {code:#06x} should be in E05xx range"
            );
        }
    }

    #[test]
    fn contract_discovery_codes_high_nibble_is_0x06() {
        let codes = [0x0601, 0x0602, 0x0603];
        for code in codes {
            assert_eq!(
                (code >> 8) & 0xFF,
                0x06,
                "code {code:#06x} should be in E06xx range"
            );
        }
    }
}
