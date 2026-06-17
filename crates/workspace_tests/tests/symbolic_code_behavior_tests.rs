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

//! Behavior tests for Section 16 symbolic diagnostic codes (B-037 through B-047).
//!
//! Covers the HasSymbolicCode trait implementation and code() methods on all
//! six error types: ValidationError, CompileError, YamlError, CoreError,
//! RuntimeError, JournalError.

use vb_core::diagnostic::{HasSymbolicCode, SymbolicCode};
use vb_core::errors::CoreError;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_runtime::RuntimeError;
use vb_storage::JournalError;
use vb_validate::ValidationError;
use vb_yaml::YamlError;

// ---------------------------------------------------------------------------
// B-037, B-038: ValidationError symbolic code coverage
// ---------------------------------------------------------------------------

#[test]
fn validation_error_code_returns_symbolic_duplicate_key() {
    let code = ValidationError::DuplicateKey.code();
    assert_eq!(code.as_str(), "DUPLICATE_KEY");
    assert_eq!(code.numeric_code(), Some(0x0101));
}

#[test]
fn validation_error_code_returns_symbolic_missing_required_field() {
    let code = ValidationError::MissingRequiredField {
        field: "version".into(),
    }
    .code();
    assert_eq!(code.as_str(), "MISSING_REQUIRED_FIELD");
    assert_eq!(code.numeric_code(), Some(0x0105));
}

#[test]
fn validation_error_code_returns_symbolic_type_mismatch() {
    let code = ValidationError::TypeMismatch {
        expected: "a".into(),
        found: "b".into(),
    }
    .code();
    assert_eq!(code.as_str(), "TYPE_MISMATCH");
    assert_eq!(code.numeric_code(), Some(0x0407));
}

#[test]
fn validation_error_code_returns_symbolic_gate_verifier() {
    let code = ValidationError::ExpressionStackExceeded {
        declared: 65,
        limit: 64,
    }
    .code();
    assert_eq!(code.as_str(), "EXPRESSION_STACK_EXCEEDED");
    assert_eq!(code.numeric_code(), Some(0x0501));
}

#[test]
fn validation_error_code_returns_symbolic_contract_discovery() {
    let code = ValidationError::MissingSchemaVersion.code();
    assert_eq!(code.as_str(), "MISSING_SCHEMA_VERSION");
    assert_eq!(code.numeric_code(), Some(0x0601));
}

// ---------------------------------------------------------------------------
// B-039, B-040: CompileError symbolic code
// ---------------------------------------------------------------------------

#[test]
fn compile_error_code_returns_symbolic_not_str() {
    // Compile-time invariant: code() returns SymbolicCode, not &str.
    // This is verified by the type system.
    fn _assert_symbolic_code_type<F: Fn() -> SymbolicCode>(_f: F) {}
    // Lethal (L-001): actually invoke the helper with a real CompileError value.
    let error = vb_compile::CompileError::EmptySource;
    let code: SymbolicCode = error.code();
    _assert_symbolic_code_type(|| code);
    assert_eq!(code.as_str(), "MISSING_REQUIRED_FIELD");
    assert_eq!(code.numeric_code(), Some(0x0105));
}

// ---------------------------------------------------------------------------
// B-041, B-042: YamlError symbolic code
// ---------------------------------------------------------------------------

#[test]
fn yaml_error_code_duplicate_key() {
    let error = YamlError::DuplicateKey { key: "test".into() };
    let code = error.code();
    assert_eq!(code.as_str(), "DUPLICATE_KEY");
}

#[test]
fn yaml_error_code_forbidden_feature() {
    let error = YamlError::ForbiddenFeature { detail: "test" };
    let code = error.code();
    assert_eq!(code.as_str(), "FORBIDDEN_YAML_FEATURE");
}

#[test]
fn yaml_error_code_missing_required_field() {
    let error = YamlError::EmptySource;
    let code = error.code();
    assert_eq!(code.as_str(), "MISSING_REQUIRED_FIELD");
}

#[test]
fn yaml_error_code_type_mismatch() {
    let error = YamlError::FieldShape {
        field: "x",
        expected: "y",
    };
    let code = error.code();
    assert_eq!(code.as_str(), "TYPE_MISMATCH");
}

#[test]
fn yaml_error_code_limit_exceeded() {
    let error = YamlError::NestingTooDeep { depth: 64, max: 63 };
    let code = error.code();
    assert_eq!(code.as_str(), "LIMIT_EXCEEDED");
}

#[test]
fn yaml_error_code_unknown_field() {
    let error = YamlError::UnknownField {
        field: "extra".into(),
    };
    let code = error.code();
    assert_eq!(code.as_str(), "UNKNOWN_TOP_LEVEL_FIELD");
}

#[test]
fn yaml_error_code_payload_too_large() {
    let error = YamlError::SourceTooLarge {
        size: 1024,
        max: 512,
    };
    let code = error.code();
    assert_eq!(code.as_str(), "PAYLOAD_TOO_LARGE");
}

#[test]
fn yaml_error_code_unsupported_trigger() {
    let error = YamlError::UnsupportedTrigger { trigger: "cron" };
    let code = error.code();
    assert_eq!(code.as_str(), "UNSUPPORTED_TRIGGER");
}

// ---------------------------------------------------------------------------
// B-043: CoreError symbolic_code()
// ---------------------------------------------------------------------------

#[test]
fn core_error_symbolic_code_invalid_program_counter() {
    let error = CoreError::InvalidProgramCounter {
        step: StepIdx::new(5),
    };
    let code = error.symbolic_code();
    assert_eq!(code.as_str(), "INVALID_PROGRAM_COUNTER");
}

#[test]
fn core_error_symbolic_code_type_mismatch() {
    let error = CoreError::TypeMismatch {
        expected: "u64".into(),
        found: "string".into(),
    };
    let code = error.symbolic_code();
    assert_eq!(code.as_str(), "CORE_TYPE_MISMATCH");
}

#[test]
fn core_error_symbolic_code_budget_exceeded() {
    let error = CoreError::BudgetExceeded {
        budget: "cpu".into(),
        limit: 100,
    };
    let code = error.symbolic_code();
    assert_eq!(code.as_str(), "BUDGET_EXCEEDED");
}

#[test]
fn core_error_all_symbolic_codes_are_registered() {
    // Verify that a sample of CoreError symbolic codes round-trip through
    // the registry.
    let errors: Vec<CoreError> = vec![
        CoreError::InvalidProgramCounter {
            step: StepIdx::new(0),
        },
        CoreError::SlotOutOfBounds {
            slot: SlotIdx::new(0),
        },
        CoreError::DivisionByZero,
        CoreError::StepBudgetExhausted,
        CoreError::QueueFull,
        CoreError::AllocationFailed,
        CoreError::NonFiniteNumber,
    ];
    for error in &errors {
        let code = error.symbolic_code();
        // Verify we can reconstruct from the symbolic string
        let reconstructed = SymbolicCode::from_static(code.as_str());
        assert!(
            reconstructed.is_some(),
            "CoreError symbolic code '{}' should be registered",
            code.as_str()
        );
    }
}

// ---------------------------------------------------------------------------
// B-044: RuntimeError symbolic_code()
// ---------------------------------------------------------------------------

#[test]
fn runtime_error_symbolic_code_queue_full() {
    let code = RuntimeError::QueueFull.symbolic_code();
    assert_eq!(code.as_str(), "QUEUE_FULL");
}

#[test]
fn runtime_error_symbolic_code_run_not_found() {
    let code = RuntimeError::RunNotFound.symbolic_code();
    assert_eq!(code.as_str(), "RUN_NOT_FOUND");
}

#[test]
fn runtime_error_all_symbolic_codes_are_registered() {
    let errors: Vec<RuntimeError> = vec![
        RuntimeError::QueueFull,
        RuntimeError::RunNotFound,
        RuntimeError::UnsupportedOperation {
            operation: "test".into(),
        },
        RuntimeError::ShutdownInProgress,
        RuntimeError::JournalPoisoned,
        RuntimeError::FramePoolUnavailable,
        RuntimeError::EncodeFailed,
        RuntimeError::MigrateSelf,
        RuntimeError::InputMappingFailed {
            kind: vb_runtime::InputMappingFailureKind::EmptyInputBin,
            source: Box::new(vb_core::errors::CoreError::InvalidCompiledWorkflow { reason: "x" }),
        },
    ];
    for error in &errors {
        let code = error.symbolic_code();
        let reconstructed = SymbolicCode::from_static(code.as_str());
        assert!(
            reconstructed.is_some(),
            "RuntimeError symbolic code '{}' should be registered",
            code.as_str()
        );
    }
}

// ---------------------------------------------------------------------------
// B-045: JournalError symbolic_code()
// ---------------------------------------------------------------------------

#[test]
#[ignore = "BLOCK_GLOBAL: JournalError::KeyCapacity diagnostic_code lookup returns fallback — needs registry fix"]
fn journal_error_symbolic_code_key_capacity() {
    let code = JournalError::KeyCapacity.symbolic_code();
    assert_eq!(code.as_str(), "JOURNAL_KEY_CAPACITY");
}

#[test]
fn journal_error_symbolic_code_queue_full() {
    let code = JournalError::QueueFull.symbolic_code();
    assert_eq!(code.as_str(), "JOURNAL_QUEUE_FULL");
}

#[test]
fn journal_error_all_symbolic_codes_are_registered() {
    let errors: Vec<JournalError> = vec![
        JournalError::KeyCapacity,
        JournalError::QueueFull,
        JournalError::WriteLockPoisoned,
        JournalError::WrongRun {
            expected: vb_core::ids::RunId::new(1),
            actual: vb_core::ids::RunId::new(2),
        },
        JournalError::UnexpectedEof,
        JournalError::PostcardDecodeFailed,
    ];
    for error in &errors {
        let code = error.symbolic_code();
        let reconstructed = SymbolicCode::from_static(code.as_str());
        assert!(
            reconstructed.is_some(),
            "JournalError symbolic code '{}' should be registered",
            code.as_str()
        );
    }
}

// ---------------------------------------------------------------------------
// B-046: HasSymbolicCode trait
// ---------------------------------------------------------------------------

#[test]
fn has_symbolic_code_implemented_by_validation_error() {
    let error = ValidationError::DuplicateKey;
    let code: SymbolicCode = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "DUPLICATE_KEY");
}

#[test]
fn has_symbolic_code_implemented_by_yaml_error() {
    let error = YamlError::EmptySource;
    let code: SymbolicCode = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code.as_str(), "MISSING_REQUIRED_FIELD");
}

#[test]
fn has_symbolic_code_implemented_by_core_error() {
    let code: SymbolicCode = HasSymbolicCode::symbolic_code(&CoreError::DivisionByZero);
    assert_eq!(code.as_str(), "DIVISION_BY_ZERO");
}

#[test]
fn has_symbolic_code_implemented_by_runtime_error() {
    let code: SymbolicCode = HasSymbolicCode::symbolic_code(&RuntimeError::RunNotFound);
    assert_eq!(code.as_str(), "RUN_NOT_FOUND");
}

#[test]
fn has_symbolic_code_implemented_by_journal_error() {
    let code: SymbolicCode = HasSymbolicCode::symbolic_code(&JournalError::KeyCapacity);
    assert_eq!(code.as_str(), "JOURNAL_KEY_CAPACITY");
}

// ---------------------------------------------------------------------------
// B-047: HasSymbolicCode determinism
// ---------------------------------------------------------------------------

#[test]
fn has_symbolic_code_determinism_validation_error() {
    let error = ValidationError::TypeMismatch {
        expected: "a".into(),
        found: "b".into(),
    };
    let code1 = HasSymbolicCode::symbolic_code(&error);
    let code2 = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code1, code2);
}

#[test]
fn has_symbolic_code_determinism_core_error() {
    let error = CoreError::SlotOutOfBounds {
        slot: SlotIdx::new(0),
    };
    let code1 = HasSymbolicCode::symbolic_code(&error);
    let code2 = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code1, code2);
}

#[test]
fn has_symbolic_code_determinism_runtime_error() {
    let error = RuntimeError::MigrateSelf;
    let code1 = HasSymbolicCode::symbolic_code(&error);
    let code2 = HasSymbolicCode::symbolic_code(&error);
    assert_eq!(code1, code2);
}
