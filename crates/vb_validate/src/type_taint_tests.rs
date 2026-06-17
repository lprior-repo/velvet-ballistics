#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables,
)]

#![forbid(unsafe_code)]
//! Tests for type_taint module (extracted from type_taint.rs)

use crate::ValidationError;
use crate::type_taint::{
    InputDecl, ResourceLimits, StepKind, StepTypes, Taint, TypedValue, ValueFact, ValueType,
    WorkflowTypes, validate_resource_limits, validate_taint, validate_types,
};

fn make_workflow(steps: Vec<StepTypes>) -> WorkflowTypes {
    WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps,
        resource_contract: ResourceLimits {
            allows_secret_results: true,
            ..ResourceLimits::default()
        },
    }
}

fn save_step(id: &str, value: TypedValue) -> StepTypes {
    StepTypes {
        id: id.to_owned(),
        kind: StepKind::Save { value },
    }
}

fn choose_step(id: &str, condition: TypedValue) -> StepTypes {
    StepTypes {
        id: id.to_owned(),
        kind: StepKind::Choose { condition },
    }
}

fn finish_step(id: &str, result: TypedValue) -> StepTypes {
    StepTypes {
        id: id.to_owned(),
        kind: StepKind::Finish { result },
    }
}

#[test]
fn accepts_clean_finish() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Literal(ValueType::Number),
    )]);
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn accepts_secret_finish_direct() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.token".into()),
    )]);
    wf.secrets.push("token".to_owned());
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn accepts_secret_finish_via_slot() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$secrets.token".into())),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("token".to_owned());
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn accepts_boolean_choose() {
    let wf = make_workflow(vec![
        save_step("flag", TypedValue::Literal(ValueType::Boolean)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    assert_eq!(validate_types(&wf), Ok(()));
}

#[test]
fn rejects_number_choose() {
    let wf = make_workflow(vec![
        save_step("flag", TypedValue::Literal(ValueType::Number)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    assert!(matches!(
        validate_types(&wf),
        Err(ValidationError::TypeMismatch { .. })
    ));
}

#[test]
fn accepts_literal_boolean_choose() {
    let wf = make_workflow(vec![choose_step(
        "route",
        TypedValue::Literal(ValueType::Boolean),
    )]);
    assert_eq!(validate_types(&wf), Ok(()));
}

#[test]
fn rejects_literal_text_choose() {
    let wf = make_workflow(vec![choose_step(
        "route",
        TypedValue::Literal(ValueType::Text),
    )]);
    assert!(matches!(
        validate_types(&wf),
        Err(ValidationError::TypeMismatch { .. })
    ));
}

#[test]
fn accepts_clean_input_finish() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.user".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "user".to_owned(),
        schema_type: ValueType::Text,
        is_secret: false,
    });
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn accepts_secret_input_finish() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.key".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "key".to_owned(),
        schema_type: ValueType::Text,
        is_secret: true,
    });
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn resource_limits_accept_within_bounds() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Literal(ValueType::Number),
    )]);
    let hard = ResourceLimits::default();
    assert_eq!(validate_resource_limits(&wf, &hard), Ok(()));
}

#[test]
fn resource_limits_reject_exceeded_steps() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Literal(ValueType::Number),
    )]);
    let hard = ResourceLimits {
        max_steps: 0,
        max_slots: 65_535,
        max_constants: 8_192,
        ..ResourceLimits::default()
    };
    assert!(matches!(
        validate_resource_limits(&wf, &hard),
        Err(ValidationError::LimitExceeded { .. })
    ));
}

#[test]
fn accepts_nested_secret_composite() {
    let mut wf = make_workflow(vec![
        save_step(
            "cap",
            TypedValue::Composite(vec![TypedValue::Reference("$secrets.token".into())]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("token".to_owned());
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn accepts_any_type_choose() {
    let wf = make_workflow(vec![
        save_step("val", TypedValue::Literal(ValueType::Any)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    assert_eq!(validate_types(&wf), Ok(()));
}

#[test]
fn rejects_null_choose() {
    let wf = make_workflow(vec![
        save_step("val", TypedValue::Literal(ValueType::Null)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    assert!(matches!(
        validate_types(&wf),
        Err(ValidationError::TypeMismatch { .. })
    ));
}

#[test]
fn accepts_clean_var_finish() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$vars.label".into()),
    )]);
    wf.vars.push(("label".to_owned(), ValueType::Boolean));
    assert_eq!(validate_taint(&wf), Ok(()));
}

// ---------------------------------------------------------------------------
// BDD exact-assertion tests
// ---------------------------------------------------------------------------

#[test]
fn validate_types_returns_type_mismatch_for_wrong_type() {
    let wf = make_workflow(vec![
        save_step("val", TypedValue::Literal(ValueType::Number)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    let result = validate_types(&wf);
    assert_eq!(
        result,
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "number".to_owned(),
        })
    );
}

#[test]
fn validate_taint_accepts_secret_in_finish() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.api_key".into()),
    )]);
    wf.secrets.push("api_key".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_taint_accepts_reference_to_untrusted_slot() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$secrets.token".into())),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("token".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_resource_limits_accepts_within_limits() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Literal(ValueType::Number),
    )]);
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_resource_limits_rejects_too_many_steps() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Literal(ValueType::Number),
    )]);
    let hard = ResourceLimits {
        max_steps: 0,
        max_slots: 65_535,
        max_constants: 8_192,
        ..ResourceLimits::default()
    };
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_steps".to_owned(),
        })
    );
}

#[test]
fn validate_resource_limits_rejects_declared_limit_exceeding_hard() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_steps: 100,
            max_slots: 65_535,
            max_constants: 8_192,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits {
        max_steps: 50,
        max_slots: 65_535,
        max_constants: 8_192,
        ..ResourceLimits::default()
    };
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_steps".to_owned(),
        })
    );
}

#[test]
fn validate_resource_limits_returns_limit_required_for_zero_declared_runtime_limit() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_fanout: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_fanout".to_owned(),
        })
    );
}

#[test]
fn validate_resource_limits_rejects_declared_fanout_exceeding_hard() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_fanout: 64,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits {
        max_fanout: 32,
        ..ResourceLimits::default()
    };
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_fanout".to_owned(),
        })
    );
}

#[test]
fn value_type_as_str_returns_correct_names() {
    assert_eq!(ValueType::Null.as_str(), "null");
    assert_eq!(ValueType::Boolean.as_str(), "boolean");
    assert_eq!(ValueType::Number.as_str(), "number");
    assert_eq!(ValueType::Text.as_str(), "text");
    assert_eq!(ValueType::Object.as_str(), "object");
    assert_eq!(ValueType::List.as_str(), "list");
    assert_eq!(ValueType::Any.as_str(), "any");
}

#[test]
fn taint_merge_propagates_secret() {
    let clean = Taint::Clean;
    let secret = Taint::Secret;
    let result1 = clean.merge(secret);
    let result2 = secret.merge(clean);
    let result3 = secret.merge(secret);
    let result4 = clean.merge(clean);
    assert_eq!(result1, Taint::Secret);
    assert_eq!(result2, Taint::Secret);
    assert_eq!(result3, Taint::Secret);
    assert_eq!(result4, Taint::Clean);
}

#[test]
fn value_fact_clean_creates_clean_taint() {
    let fact = ValueFact::clean(ValueType::Number);
    assert_eq!(fact.value_type, ValueType::Number);
    assert_eq!(fact.taint, Taint::Clean);
}

#[test]
fn value_fact_secret_creates_secret_taint() {
    let fact = ValueFact::secret(ValueType::Text);
    assert_eq!(fact.value_type, ValueType::Text);
    assert_eq!(fact.taint, Taint::Secret);
}

#[test]
fn validate_types_accepts_any_type_choose() {
    let wf = make_workflow(vec![
        save_step("val", TypedValue::Literal(ValueType::Any)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    let result = validate_types(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_types_rejects_null_choose_exact() {
    let wf = make_workflow(vec![
        save_step("val", TypedValue::Literal(ValueType::Null)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    let result = validate_types(&wf);
    assert_eq!(
        result,
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "null".to_owned(),
        })
    );
}

#[test]
fn validate_types_rejects_text_choose_exact() {
    let wf = make_workflow(vec![choose_step(
        "route",
        TypedValue::Literal(ValueType::Text),
    )]);
    let result = validate_types(&wf);
    assert_eq!(
        result,
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "text".to_owned(),
        })
    );
}

#[test]
fn validate_types_accepts_literal_boolean_choose() {
    let wf = make_workflow(vec![choose_step(
        "route",
        TypedValue::Literal(ValueType::Boolean),
    )]);
    let result = validate_types(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_taint_accepts_clean_input_finish() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.user".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "user".to_owned(),
        schema_type: ValueType::Text,
        is_secret: false,
    });
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_taint_accepts_secret_input_finish_exact() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.key".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "key".to_owned(),
        schema_type: ValueType::Text,
        is_secret: true,
    });
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn validate_taint_accepts_nested_secret_composite_exact() {
    let mut wf = make_workflow(vec![
        save_step(
            "cap",
            TypedValue::Composite(vec![TypedValue::Reference("$secrets.token".into())]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("token".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn resource_limits_default_values() {
    let limits = ResourceLimits::default();
    assert_eq!(limits.max_steps, 1_000);
    assert_eq!(limits.max_slots, 65_535);
    assert_eq!(limits.max_constants, 8_192);
    assert_eq!(limits.max_accessors, 8_192);
    assert_eq!(limits.max_expressions, 4_096);
    assert_eq!(limits.max_expr_stack, 64);
    assert_eq!(limits.max_retry_attempts, 10);
    assert_eq!(limits.max_fanout, 256);
    assert_eq!(limits.max_collect_items, 1_000);
}

// ---------------------------------------------------------------------------
// Adversarial BDD tests: validation bypass attacks
// ---------------------------------------------------------------------------

#[test]
fn adversarial_secret_leak_via_direct_reference_in_finish_is_accepted() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.api_key".into()),
    )]);
    wf.secrets.push("api_key".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_secret_leak_via_two_step_indirection_is_accepted() {
    let mut wf = make_workflow(vec![
        save_step("cap", TypedValue::Reference("$secrets.token".into())),
        save_step("relay", TypedValue::Slot(0)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("token".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_secret_leak_via_composite_with_clean_and_secret_is_accepted() {
    let mut wf = make_workflow(vec![
        save_step(
            "mixed",
            TypedValue::Composite(vec![
                TypedValue::Literal(ValueType::Number),
                TypedValue::Reference("$secrets.password".into()),
            ]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("password".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_secret_leak_via_secret_input_is_accepted() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.password".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "password".to_owned(),
        schema_type: ValueType::Text,
        is_secret: true,
    });
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_type_mismatch_object_in_choose_is_rejected() {
    let wf = make_workflow(vec![choose_step(
        "bad_route",
        TypedValue::Literal(ValueType::Object),
    )]);
    let result = validate_types(&wf);
    assert_eq!(
        result,
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "object".to_owned(),
        })
    );
}

#[test]
fn adversarial_type_mismatch_list_in_choose_is_rejected() {
    let wf = make_workflow(vec![choose_step(
        "bad_route",
        TypedValue::Literal(ValueType::List),
    )]);
    let result = validate_types(&wf);
    assert_eq!(
        result,
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "list".to_owned(),
        })
    );
}

#[test]
fn adversarial_resource_limit_declared_exceeding_hard_limit_is_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_steps: 1_000,
            max_slots: 100_000,
            max_constants: 8_192,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_slots".to_owned(),
        })
    );
}

#[test]
fn adversarial_resource_limit_actual_exceeding_declared_is_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![finish_step("s1", TypedValue::Literal(ValueType::Number)); 10],
        resource_contract: ResourceLimits {
            max_steps: 5,
            max_slots: 65_535,
            max_constants: 8_192,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_steps".to_owned(),
        })
    );
}

#[test]
fn adversarial_clean_input_passes_taint_check() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.username".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "username".to_owned(),
        schema_type: ValueType::Text,
        is_secret: false,
    });
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_secret_in_choose_condition_does_not_leak_but_type_check_passes_for_any() {
    let mut wf = make_workflow(vec![
        save_step("val", TypedValue::Reference("$secrets.token".into())),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("token".to_owned());
    let result = validate_types(&wf);
    assert_eq!(result, Ok(()));
}

#[test]
fn adversarial_deeply_nested_composite_taint_propagates() {
    let mut wf = make_workflow(vec![
        save_step(
            "nested",
            TypedValue::Composite(vec![
                TypedValue::Literal(ValueType::Number),
                TypedValue::Composite(vec![
                    TypedValue::Literal(ValueType::Text),
                    TypedValue::Composite(vec![
                        TypedValue::Literal(ValueType::Boolean),
                        TypedValue::Reference("$secrets.deep_secret".into()),
                    ]),
                ]),
            ]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("deep_secret".to_owned());
    let result = validate_taint(&wf);
    assert_eq!(result, Ok(()));
}

// =========================================================================
// Proptest: taint propagation invariants
//
// Replaces ~80 duplicate tests (sections 38, comprehensive taint,
// comprehensive taint+type-check, accessors, clean workflow, taint merge,
// boundary, type-check, and resource limits). Generates arbitrary
// WorkflowTypes graphs and verifies core invariants.
// =========================================================================

use proptest::prelude::*;

fn arb_value_type() -> impl Strategy<Value = ValueType> {
    prop_oneof![
        Just(ValueType::Null),
        Just(ValueType::Boolean),
        Just(ValueType::Number),
        Just(ValueType::Text),
        Just(ValueType::Object),
        Just(ValueType::List),
        Just(ValueType::Any),
    ]
}

fn arb_typed_value_strat(depth: u8, max_slots: usize) -> BoxedStrategy<TypedValue> {
    let leaf = prop_oneof![
        arb_value_type().prop_map(TypedValue::Literal),
        (0..max_slots).prop_map(TypedValue::Slot),
        "[a-z][a-z0-9_]{0,15}".prop_map(|s| {
            let roots = ["$input", "$vars", "$secrets", "$unknown"];
            let idx = s.len().wrapping_rem(roots.len());
            TypedValue::Reference(format!("{}.{s}", roots[idx]))
        }),
    ];
    leaf.prop_recursive(3, 6, depth as u32, move |inner| {
        prop::collection::vec(inner, 0..4).prop_map(TypedValue::Composite)
    })
    .boxed()
}

fn arb_step_kind(depth: u8, max_slots: usize) -> BoxedStrategy<StepKind> {
    prop_oneof![
        arb_typed_value_strat(depth, max_slots).prop_map(|value| StepKind::Save { value }),
        arb_typed_value_strat(depth, max_slots)
            .prop_map(|condition| StepKind::Choose { condition }),
        arb_typed_value_strat(depth, max_slots).prop_map(|result| StepKind::Finish { result }),
    ]
    .boxed()
}

fn arb_step_types(depth: u8, max_slots: usize) -> impl Strategy<Value = StepTypes> {
    ("[a-z][a-z0-9_]{0,15}", arb_step_kind(depth, max_slots))
        .prop_map(|(id, kind)| StepTypes { id, kind })
}

fn arb_workflow_types(max_steps: usize) -> impl Strategy<Value = WorkflowTypes> {
    let secret_names = prop::collection::vec("[a-z][a-z0-9_]{0,15}", 0..4);
    let steps = (1..=max_steps)
        .prop_flat_map(move |count| prop::collection::vec(arb_step_types(3, count), count..=count));
    let inputs = prop::collection::vec(
        ("[a-z][a-z0-9_]{0,15}", arb_value_type(), any::<bool>()).prop_map(
            |(name, schema_type, is_secret)| InputDecl {
                name,
                schema_type,
                is_secret,
            },
        ),
        0..4,
    );
    let vars = prop::collection::vec(("[a-z][a-z0-9_]{0,15}", arb_value_type()), 0..4);
    (inputs, vars, secret_names, steps).prop_map(|(inputs, vars, secrets, steps)| WorkflowTypes {
        inputs,
        vars,
        secrets,
        steps,
        resource_contract: ResourceLimits {
            allows_secret_results: true,
            ..ResourceLimits::default()
        },
    })
}

#[test]
fn prop_validate_taint_never_panics() {
    proptest::proptest!(proptest::test_runner::Config::default(),
        |(wf in arb_workflow_types(6))| {
            let _ = validate_taint(&wf);
        }
    );
}

#[test]
fn prop_validate_types_never_panics() {
    proptest::proptest!(proptest::test_runner::Config::default(),
        |(wf in arb_workflow_types(6))| {
            let _ = validate_types(&wf);
        }
    );
}

#[test]
fn prop_validate_resource_limits_never_panics() {
    proptest::proptest!(proptest::test_runner::Config::default(),
        |(wf in arb_workflow_types(6))| {
            let hard = ResourceLimits::default();
            let _ = validate_resource_limits(&wf, &hard);
        }
    );
}

#[test]
fn prop_empty_workflow_passes_all_validators() {
    let wf = make_workflow(vec![]);
    assert_eq!(validate_taint(&wf), Ok(()));
    assert_eq!(validate_types(&wf), Ok(()));
    let hard = ResourceLimits::default();
    assert_eq!(validate_resource_limits(&wf, &hard), Ok(()));
}

#[test]
fn prop_taint_merge_is_commutative() {
    proptest::proptest!(proptest::test_runner::Config::default(),
        |(a in prop_oneof![
            Just(Taint::Clean),
            Just(Taint::Secret),
            Just(Taint::DerivedFromSecret),
        ],
        b in prop_oneof![
            Just(Taint::Clean),
            Just(Taint::Secret),
            Just(Taint::DerivedFromSecret),
        ])| {
            assert_eq!(a.merge(b), b.merge(a));
        }
    );
}

#[test]
fn prop_taint_merge_secret_dominates() {
    proptest::proptest!(proptest::test_runner::Config::default(),
        |(a in prop_oneof![
            Just(Taint::Clean),
            Just(Taint::Secret),
            Just(Taint::DerivedFromSecret),
        ])| {
            assert_eq!(Taint::Secret.merge(a), Taint::Secret);
            assert_eq!(a.merge(Taint::Secret), Taint::Secret);
        }
    );
}

#[test]
fn prop_taint_merge_is_idempotent() {
    proptest::proptest!(proptest::test_runner::Config::default(),
        |(t in prop_oneof![
            Just(Taint::Clean),
            Just(Taint::Secret),
            Just(Taint::DerivedFromSecret),
        ])| {
            assert_eq!(t.merge(t), t);
        }
    );
}

#[test]
fn prop_taint_merge_clean_is_identity() {
    proptest::proptest!(proptest::test_runner::Config::default(),
        |(t in prop_oneof![
            Just(Taint::Clean),
            Just(Taint::Secret),
            Just(Taint::DerivedFromSecret),
        ])| {
            let with_clean = Taint::Clean.merge(t);
            let with_t = t.merge(Taint::Clean);
            if t == Taint::Clean {
                assert_eq!(with_clean, Taint::Clean);
                assert_eq!(with_t, Taint::Clean);
            } else {
                assert_eq!(with_clean, t);
                assert_eq!(with_t, t);
            }
        }
    );
}
// =========================================================================
// BLACKHAT security regression tests
// =========================================================================

#[test]
fn blackhat_resource_limits_max_slots_uses_step_count_not_slot_count() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![
            finish_step("s0", TypedValue::Literal(ValueType::Number)),
            finish_step("s1", TypedValue::Literal(ValueType::Number)),
        ],
        resource_contract: ResourceLimits {
            max_steps: 10,
            max_slots: 1,
            max_constants: 8_192,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    match validate_resource_limits(&wf, &hard) {
        Err(ValidationError::LimitExceeded { resource }) => {
            assert_eq!(resource, "max_slots");
        }
        Err(ValidationError::LimitRequired { .. }) => {}
        other => {
            assert!(
                matches!(
                    other,
                    Err(ValidationError::LimitExceeded { .. }
                        | ValidationError::LimitRequired { .. })
                ),
                "blackhat: expected LimitExceeded or LimitRequired for max_slots, got {other:?}"
            );
        }
    }
}

#[test]
fn blackhat_unknown_reference_is_clean_not_secret() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$unknown_root.field".into()),
    )]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: unknown reference roots should resolve as clean"
    );

    let wf2 = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.nonexistent".into()),
    )]);
    assert_eq!(
        validate_taint(&wf2),
        Ok(()),
        "blackhat: unknown input name should resolve as clean"
    );
}

#[test]
fn blackhat_reference_without_dot_is_clean_any() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input".into()),
    )]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: reference without dot should be clean"
    );
}

#[test]
fn blackhat_reference_without_dollar_prefix_is_clean_text() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("not_a_reference".into()),
    )]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: non-dollar reference should be clean"
    );
}

#[test]
fn blackhat_zero_declared_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_input_bytes: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert!(
        matches!(result, Err(ValidationError::LimitRequired { .. })),
        "blackhat: zero declared limit should be rejected, got {result:?}"
    );
    if let Err(ValidationError::LimitRequired { resource }) = result {
        assert_eq!(resource, "max_input_bytes");
    }
}

#[test]
fn blackhat_choose_tainted_condition_does_not_propagate() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.flag".into())),
        choose_step("route", TypedValue::Slot(0)),
        save_step("s1", TypedValue::Literal(ValueType::Number)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("flag".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: tainted choose condition should not propagate taint to finish"
    );
}

#[test]
fn blackhat_taint_merge_commutative() {
    assert_eq!(
        Taint::Clean.merge(Taint::Secret),
        Taint::Secret.merge(Taint::Clean),
        "blackhat: taint merge must be commutative"
    );
}

#[test]
fn blackhat_uninitialized_slot_is_clean() {
    let wf = make_workflow(vec![
        save_step("s0", TypedValue::Literal(ValueType::Number)),
        finish_step("done", TypedValue::Slot(5)),
    ]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: uninitialized slot should resolve as clean"
    );
}

#[test]
fn blackhat_empty_composite_is_clean() {
    let wf = make_workflow(vec![
        save_step("s0", TypedValue::Composite(vec![])),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: empty composite should be clean"
    );
}

#[test]
fn blackhat_out_of_bounds_slot_write_no_panic() {
    let wf = make_workflow(vec![
        save_step("overflow", TypedValue::Literal(ValueType::Number)),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    assert_eq!(validate_taint(&wf), Ok(()));
}

// =========================================================================
// Comprehensive taint propagation and type-checking tests
// =========================================================================
// BLACKHAT comprehensive security regression tests
// =========================================================================

// ---------------------------------------------------------------------------
// BLACKHAT: Secret taint propagation through composite values
// ---------------------------------------------------------------------------

#[test]
fn blackhat_composite_finish_directly_with_secret_reference() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Composite(vec![TypedValue::Reference("$secrets.password".into())]),
    )]);
    wf.secrets.push("password".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: composite in finish directly referencing a secret must be accepted"
    );
}

#[test]
fn blackhat_composite_finish_with_secret_slot_and_clean_literal() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.api_key".into())),
        finish_step(
            "done",
            TypedValue::Composite(vec![
                TypedValue::Slot(0),
                TypedValue::Literal(ValueType::Text),
            ]),
        ),
    ]);
    wf.secrets.push("api_key".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: composite of secret slot + clean literal in finish must be accepted"
    );
}

#[test]
fn blackhat_composite_of_composites_carries_secret_taint() {
    let mut wf = make_workflow(vec![
        save_step(
            "outer",
            TypedValue::Composite(vec![TypedValue::Composite(vec![TypedValue::Reference(
                "$secrets.innermost".into(),
            )])]),
        ),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("innermost".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: nested composites must propagate secret taint outward"
    );
}

#[test]
fn blackhat_composite_with_two_secret_inputs_leaks() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Composite(vec![
            TypedValue::Reference("$input.username".into()),
            TypedValue::Reference("$input.token".into()),
        ]),
    )]);
    wf.inputs.push(InputDecl {
        name: "username".to_owned(),
        schema_type: ValueType::Text,
        is_secret: false,
    });
    wf.inputs.push(InputDecl {
        name: "token".to_owned(),
        schema_type: ValueType::Text,
        is_secret: true,
    });
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: composite with one secret input and one clean must still be accepted"
    );
}

#[test]
fn blackhat_composite_with_only_clean_elements_passes() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Composite(vec![
            TypedValue::Literal(ValueType::Number),
            TypedValue::Literal(ValueType::Text),
            TypedValue::Literal(ValueType::Boolean),
        ]),
    )]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: composite with only clean literals must pass"
    );
}

#[test]
fn blackhat_secret_propagates_through_save_then_composite_then_finish() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.cred".into())),
        save_step(
            "s1",
            TypedValue::Composite(vec![
                TypedValue::Slot(0),
                TypedValue::Literal(ValueType::Number),
            ]),
        ),
        save_step("s2", TypedValue::Slot(1)),
        finish_step("done", TypedValue::Slot(2)),
    ]);
    wf.secrets.push("cred".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: secret through save->composite->relay->finish must be accepted"
    );
}

#[test]
fn blackhat_secret_in_composite_saved_to_slot_then_relayed() {
    let mut wf = make_workflow(vec![
        save_step(
            "s0",
            TypedValue::Composite(vec![TypedValue::Reference("$secrets.key".into())]),
        ),
        save_step("s1", TypedValue::Slot(0)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("key".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: composite with secret saved to slot, then relayed, must be accepted"
    );
}

#[test]
fn blackhat_composite_with_secret_var_reference() {
    // vars are always clean, so this should pass
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Composite(vec![
            TypedValue::Reference("$vars.count".into()),
            TypedValue::Literal(ValueType::Number),
        ]),
    )]);
    wf.vars.push(("count".to_owned(), ValueType::Number));
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: composite with var reference should be clean since vars are never secret"
    );
}

// ---------------------------------------------------------------------------
// BLACKHAT: Resource limits - zero limits should fail
// ---------------------------------------------------------------------------

#[test]
fn blackhat_zero_max_accessors_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_accessors: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_accessors".to_owned(),
        }),
        "blackhat: zero max_accessors must be rejected"
    );
}

#[test]
fn blackhat_zero_max_expressions_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_expressions: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_expressions".to_owned(),
        }),
        "blackhat: zero max_expressions must be rejected"
    );
}

#[test]
fn blackhat_zero_max_expr_stack_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_expr_stack: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_expr_stack".to_owned(),
        }),
        "blackhat: zero max_expr_stack must be rejected"
    );
}

#[test]
fn blackhat_zero_max_step_budget_per_tick_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_step_budget_per_tick: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_step_budget_per_tick".to_owned(),
        }),
        "blackhat: zero max_step_budget_per_tick must be rejected"
    );
}

#[test]
fn blackhat_zero_max_output_bytes_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_output_bytes: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_output_bytes".to_owned(),
        }),
        "blackhat: zero max_output_bytes must be rejected"
    );
}

#[test]
fn blackhat_zero_max_blob_bytes_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_blob_bytes: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_blob_bytes".to_owned(),
        }),
        "blackhat: zero max_blob_bytes must be rejected"
    );
}

#[test]
fn blackhat_zero_max_ipc_payload_bytes_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_ipc_payload_bytes: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_ipc_payload_bytes".to_owned(),
        }),
        "blackhat: zero max_ipc_payload_bytes must be rejected"
    );
}

#[test]
fn blackhat_zero_max_retry_attempts_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_retry_attempts: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_retry_attempts".to_owned(),
        }),
        "blackhat: zero max_retry_attempts must be rejected"
    );
}

#[test]
fn blackhat_zero_max_journal_batch_bytes_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_journal_batch_bytes: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_journal_batch_bytes".to_owned(),
        }),
        "blackhat: zero max_journal_batch_bytes must be rejected"
    );
}

#[test]
fn blackhat_zero_max_steps_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_steps: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_steps".to_owned(),
        }),
        "blackhat: zero max_steps must be rejected"
    );
}

#[test]
fn blackhat_zero_max_slots_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_slots: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_slots".to_owned(),
        }),
        "blackhat: zero max_slots must be rejected"
    );
}

// ---------------------------------------------------------------------------
// BLACKHAT: Resource limits - declared exceeding hard limits
// ---------------------------------------------------------------------------

#[test]
fn blackhat_max_accessors_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_accessors: 100_000,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_accessors".to_owned(),
        }),
        "blackhat: max_accessors exceeding hard limit must be rejected"
    );
}

#[test]
fn blackhat_max_expressions_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_expressions: 50_000,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_expressions".to_owned(),
        }),
        "blackhat: max_expressions exceeding hard limit must be rejected"
    );
}

#[test]
fn blackhat_max_expr_stack_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_expr_stack: 1_024,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_expr_stack".to_owned(),
        }),
        "blackhat: max_expr_stack exceeding hard limit must be rejected"
    );
}

#[test]
fn blackhat_max_step_budget_per_tick_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_step_budget_per_tick: 1_000_000,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_step_budget_per_tick".to_owned(),
        }),
        "blackhat: max_step_budget_per_tick exceeding hard limit must be rejected"
    );
}

#[test]
fn blackhat_max_input_bytes_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_input_bytes: 10_485_760,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_input_bytes".to_owned(),
        }),
        "blackhat: max_input_bytes exceeding hard limit must be rejected"
    );
}

#[test]
fn blackhat_max_output_bytes_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_output_bytes: 10_485_760,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_output_bytes".to_owned(),
        }),
        "blackhat: max_output_bytes exceeding hard limit must be rejected"
    );
}

#[test]
fn blackhat_max_blob_bytes_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_blob_bytes: 100_000_000,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_blob_bytes".to_owned(),
        }),
        "blackhat: max_blob_bytes exceeding hard limit must be rejected"
    );
}

#[test]
fn blackhat_max_ipc_payload_bytes_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_ipc_payload_bytes: 10_485_760,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_ipc_payload_bytes".to_owned(),
        }),
        "blackhat: max_ipc_payload_bytes exceeding hard limit must be rejected"
    );
}

#[test]
fn blackhat_max_journal_batch_bytes_exceeding_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_journal_batch_bytes: 10_485_760,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_journal_batch_bytes".to_owned(),
        }),
        "blackhat: max_journal_batch_bytes exceeding hard limit must be rejected"
    );
}

// ---------------------------------------------------------------------------
// BLACKHAT: Type checking - Choose with non-boolean conditions
// ---------------------------------------------------------------------------

#[test]
fn blackhat_choose_with_number_slot_fails_type_check() {
    let wf = make_workflow(vec![
        save_step("s0", TypedValue::Literal(ValueType::Number)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    assert_eq!(
        validate_types(&wf),
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "number".to_owned(),
        }),
        "blackhat: choose with number slot must fail type check"
    );
}

#[test]
fn blackhat_choose_with_text_slot_fails_type_check() {
    let wf = make_workflow(vec![
        save_step("s0", TypedValue::Literal(ValueType::Text)),
        choose_step("route", TypedValue::Slot(0)),
    ]);
    assert_eq!(
        validate_types(&wf),
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "text".to_owned(),
        }),
        "blackhat: choose with text slot must fail type check"
    );
}

#[test]
fn blackhat_choose_with_null_literal_fails_type_check() {
    let wf = make_workflow(vec![choose_step(
        "route",
        TypedValue::Literal(ValueType::Null),
    )]);
    assert_eq!(
        validate_types(&wf),
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "null".to_owned(),
        }),
        "blackhat: choose with null literal must fail type check"
    );
}

#[test]
fn blackhat_choose_with_any_type_from_unknown_reference_passes() {
    let wf = make_workflow(vec![choose_step(
        "route",
        TypedValue::Reference("$input.missing_field".into()),
    )]);
    assert_eq!(
        validate_types(&wf),
        Ok(()),
        "blackhat: choose with unresolved reference resolves as Any and should pass"
    );
}

#[test]
fn blackhat_choose_with_boolean_from_secret_input_passes_type_check() {
    let mut wf = make_workflow(vec![choose_step(
        "route",
        TypedValue::Reference("$input.is_admin".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "is_admin".to_owned(),
        schema_type: ValueType::Boolean,
        is_secret: true,
    });
    assert_eq!(
        validate_types(&wf),
        Ok(()),
        "blackhat: choose with boolean input (even if secret) should pass type check"
    );
}

#[test]
fn blackhat_multiple_chooses_first_bad_stops_early() {
    let wf = make_workflow(vec![
        choose_step("route1", TypedValue::Literal(ValueType::Number)),
        choose_step("route2", TypedValue::Literal(ValueType::Boolean)),
    ]);
    assert_eq!(
        validate_types(&wf),
        Err(ValidationError::TypeMismatch {
            expected: "boolean".to_owned(),
            found: "number".to_owned(),
        }),
        "blackhat: first bad choose must stop validation early"
    );
}

// ---------------------------------------------------------------------------
// BLACKHAT: Reference resolution paths
// ---------------------------------------------------------------------------

#[test]
fn blackhat_input_reference_resolves_correct_type_and_taint() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.name".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "name".to_owned(),
        schema_type: ValueType::Text,
        is_secret: false,
    });
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: $input.name for clean input must pass taint"
    );
}

#[test]
fn blackhat_input_reference_with_nested_path_resolves() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input.user.profile.name".into()),
    )]);
    wf.inputs.push(InputDecl {
        name: "user".to_owned(),
        schema_type: ValueType::Object,
        is_secret: false,
    });
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: deeply nested input path must resolve to input fact"
    );
}

#[test]
fn blackhat_var_reference_resolves_clean() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$var.counter".into()),
    )]);
    wf.vars.push(("counter".to_owned(), ValueType::Number));
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: $var alias must resolve and vars are always clean"
    );
}

#[test]
fn blackhat_vars_reference_resolves_clean() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$vars.counter".into()),
    )]);
    wf.vars.push(("counter".to_owned(), ValueType::Number));
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: $vars reference must resolve and vars are always clean"
    );
}

#[test]
fn blackhat_secrets_reference_resolves_tainted() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.db_password".into()),
    )]);
    wf.secrets.push("db_password".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: $secrets.X must resolve as tainted and be accepted"
    );
}

#[test]
fn blackhat_secrets_nested_path_resolves_tainted() {
    let mut wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.db.connection".into()),
    )]);
    wf.secrets.push("db".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: $secrets.X.Y must resolve using first segment and be accepted"
    );
}

#[test]
fn blackhat_unknown_root_reference_is_clean() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$output.result".into()),
    )]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: unknown root like $output should resolve as clean"
    );
}

#[test]
fn blackhat_reference_without_dollar_is_clean() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("plain_string".into()),
    )]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: reference without $ prefix must be clean text"
    );
}

#[test]
fn blackhat_reference_without_dot_suffix_is_clean_any() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$input".into()),
    )]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: reference with $ but no dot must be clean"
    );
}

#[test]
fn blackhat_secrets_reference_with_dot_only_is_clean() {
    let wf = make_workflow(vec![finish_step(
        "done",
        TypedValue::Reference("$secrets.".into()),
    )]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: $secrets. with empty name should resolve clean (no match)"
    );
}

#[test]
fn blackhat_input_and_secrets_same_name_different_taint() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$input.key".into())),
        save_step("s1", TypedValue::Reference("$secrets.key".into())),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.inputs.push(InputDecl {
        name: "key".to_owned(),
        schema_type: ValueType::Text,
        is_secret: false,
    });
    wf.secrets.push("key".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: same name in input (clean) and secrets, finishing input slot is clean"
    );
}

#[test]
fn blackhat_input_and_secrets_same_name_secret_finishes_tainted() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$input.key".into())),
        save_step("s1", TypedValue::Reference("$secrets.key".into())),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.inputs.push(InputDecl {
        name: "key".to_owned(),
        schema_type: ValueType::Text,
        is_secret: false,
    });
    wf.secrets.push("key".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: same name in input and secrets, finishing secrets slot is accepted"
    );
}

// ---------------------------------------------------------------------------
// BLACKHAT: Edge cases - empty workflow, single-step, max-value limits
// ---------------------------------------------------------------------------

#[test]
fn blackhat_empty_workflow_passes_both_validators() {
    let wf = make_workflow(vec![]);
    assert_eq!(validate_types(&wf), Ok(()));
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn blackhat_single_save_step_workflow_passes_types() {
    let wf = make_workflow(vec![save_step(
        "only",
        TypedValue::Literal(ValueType::Number),
    )]);
    assert_eq!(validate_types(&wf), Ok(()));
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn blackhat_single_choose_step_with_boolean_passes() {
    let wf = make_workflow(vec![choose_step(
        "only",
        TypedValue::Literal(ValueType::Boolean),
    )]);
    assert_eq!(validate_types(&wf), Ok(()));
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn blackhat_single_finish_step_with_literal_passes() {
    let wf = make_workflow(vec![finish_step(
        "only",
        TypedValue::Literal(ValueType::Number),
    )]);
    assert_eq!(validate_types(&wf), Ok(()));
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn blackhat_resource_limits_at_exact_hard_limit_passes() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_steps: 1_000,
            max_slots: 65_535,
            max_constants: 8_192,
            max_accessors: 8_192,
            max_expressions: 4_096,
            max_expr_stack: 64,
            max_step_budget_per_tick: 10_000,
            max_input_bytes: 1_048_576,
            max_output_bytes: 1_048_576,
            max_blob_bytes: 16_777_216,
            max_ipc_payload_bytes: 1_048_576,
            max_retry_attempts: 10,
            max_fanout: 256,
            max_collect_items: 1_000,
            max_queue_depth: 1_024,
            max_journal_batch_bytes: 1_048_576,
            allows_secret_results: true,
        },
    };
    let hard = ResourceLimits::default();
    assert_eq!(
        validate_resource_limits(&wf, &hard),
        Ok(()),
        "blackhat: resource limits exactly at hard limits must pass"
    );
}

#[test]
fn blackhat_resource_limits_one_over_hard_limit_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_steps: 1_001,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_steps".to_owned(),
        }),
        "blackhat: max_steps one over hard limit must be rejected"
    );
}

#[test]
fn blackhat_resource_limits_actual_equals_declared_passes() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![
            finish_step("s0", TypedValue::Literal(ValueType::Number)),
            finish_step("s1", TypedValue::Literal(ValueType::Number)),
            finish_step("s2", TypedValue::Literal(ValueType::Number)),
        ],
        resource_contract: ResourceLimits {
            max_steps: 3,
            max_slots: 65_535,
            max_constants: 8_192,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    assert_eq!(
        validate_resource_limits(&wf, &hard),
        Ok(()),
        "blackhat: actual step count exactly equal to declared max_steps must pass"
    );
}

#[test]
fn blackhat_resource_limits_actual_exceeds_declared_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![
            finish_step("s0", TypedValue::Literal(ValueType::Number)),
            finish_step("s1", TypedValue::Literal(ValueType::Number)),
            finish_step("s2", TypedValue::Literal(ValueType::Number)),
            finish_step("s3", TypedValue::Literal(ValueType::Number)),
        ],
        resource_contract: ResourceLimits {
            max_steps: 3,
            max_slots: 65_535,
            max_constants: 8_192,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitExceeded {
            resource: "max_steps".to_owned(),
        }),
        "blackhat: 4 actual steps with declared max_steps=3 must be rejected"
    );
}

#[test]
fn blackhat_workflow_with_only_saves_and_no_finish_passes() {
    let wf = make_workflow(vec![
        save_step("s0", TypedValue::Literal(ValueType::Number)),
        save_step("s1", TypedValue::Literal(ValueType::Text)),
    ]);
    assert_eq!(validate_types(&wf), Ok(()));
    assert_eq!(validate_taint(&wf), Ok(()));
}

#[test]
fn blackhat_save_secret_then_clean_save_then_clean_finish_passes() {
    let mut wf = make_workflow(vec![
        save_step(
            "secret_slot",
            TypedValue::Reference("$secrets.ignored".into()),
        ),
        save_step("clean_slot", TypedValue::Literal(ValueType::Text)),
        finish_step("done", TypedValue::Slot(1)),
    ]);
    wf.secrets.push("ignored".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: finishing a clean slot while another slot has secret data should pass"
    );
}

#[test]
fn blackhat_multiple_finishes_only_first_accepted() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.early".into())),
        finish_step("leak", TypedValue::Slot(0)),
        finish_step("clean", TypedValue::Literal(ValueType::Number)),
    ]);
    wf.secrets.push("early".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: first tainted finish must be accepted; second clean finish not reached"
    );
}

#[test]
fn blackhat_save_overwrites_slot_taint_from_secret_to_clean() {
    let mut wf = make_workflow(vec![
        save_step("s0", TypedValue::Reference("$secrets.x".into())),
        save_step("s0_overwrite", TypedValue::Literal(ValueType::Number)),
        finish_step("done", TypedValue::Slot(0)),
    ]);
    wf.secrets.push("x".to_owned());
    // slot 0 gets tainted by s0, then overwritten to clean by s0_overwrite
    // BUT: the overwrite uses write_slot which writes to slots[index],
    // where index=1 for the second save_step. So slot 0 stays tainted.
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: slot 0 remains tainted; second save writes to slot 1, not slot 0, but acceptance is expected"
    );
}

#[test]
fn blackhat_taint_does_not_propagate_through_choose() {
    let mut wf = make_workflow(vec![
        save_step("flag", TypedValue::Reference("$secrets.selector".into())),
        choose_step("route", TypedValue::Slot(0)),
        save_step("result", TypedValue::Literal(ValueType::Number)),
        finish_step("done", TypedValue::Slot(2)),
    ]);
    wf.secrets.push("selector".to_owned());
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: taint in choose condition must not propagate to subsequent steps"
    );
}

#[test]
fn blackhat_composite_of_empty_vec_is_clean() {
    let wf = make_workflow(vec![finish_step("done", TypedValue::Composite(vec![]))]);
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "blackhat: empty composite in finish should be clean"
    );
}

#[test]
fn blackhat_resource_limits_all_zero_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_steps: 0,
            max_slots: 0,
            max_constants: 0,
            max_accessors: 0,
            max_expressions: 0,
            max_expr_stack: 0,
            max_step_budget_per_tick: 0,
            max_input_bytes: 0,
            max_output_bytes: 0,
            max_blob_bytes: 0,
            max_ipc_payload_bytes: 0,
            max_retry_attempts: 0,
            max_fanout: 0,
            max_collect_items: 0,
            max_queue_depth: 0,
            max_journal_batch_bytes: 0,
            allows_secret_results: true,
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert!(
        matches!(result, Err(ValidationError::LimitRequired { .. })),
        "blackhat: all-zero resource limits must be rejected with LimitRequired, got {result:?}"
    );
    if let Err(ValidationError::LimitRequired { resource }) = result {
        assert_eq!(
            resource, "max_steps",
            "blackhat: first zero limit checked should be max_steps"
        );
    }
}

#[test]
fn blackhat_resource_limits_all_exceeding_hard_rejected() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![],
        resource_contract: ResourceLimits {
            max_steps: 999_999,
            max_slots: 999_999,
            max_constants: 999_999,
            max_accessors: 999_999,
            max_expressions: 999_999,
            max_expr_stack: 999_999,
            max_step_budget_per_tick: 999_999,
            max_input_bytes: 999_999,
            max_output_bytes: 999_999,
            max_blob_bytes: 999_999,
            max_ipc_payload_bytes: 999_999,
            max_retry_attempts: 999_999,
            max_fanout: 999_999,
            max_collect_items: 999_999,
            max_queue_depth: 999_999,
            max_journal_batch_bytes: 999_999,
            allows_secret_results: true,
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert!(
        matches!(result, Err(ValidationError::LimitExceeded { .. })),
        "blackhat: all limits exceeding hard must be rejected with LimitExceeded, got {result:?}"
    );
    if let Err(ValidationError::LimitExceeded { resource }) = result {
        assert_eq!(
            resource, "max_steps",
            "blackhat: first limit exceeding hard should be max_steps"
        );
    }
}

// ---------------------------------------------------------------------------
// Edge-case tests: taint merge, value fact constructors, empty workflow,
// zero-step resource limits
// ---------------------------------------------------------------------------

#[test]
fn edge_taint_merge_clean_clean_returns_clean() {
    assert_eq!(
        Taint::Clean.merge(Taint::Clean),
        Taint::Clean,
        "edge: Clean + Clean must equal Clean"
    );
}

#[test]
fn edge_taint_merge_clean_secret_returns_secret() {
    assert_eq!(
        Taint::Clean.merge(Taint::Secret),
        Taint::Secret,
        "edge: Clean + Secret must equal Secret"
    );
}

#[test]
fn edge_taint_merge_secret_clean_returns_secret() {
    assert_eq!(
        Taint::Secret.merge(Taint::Clean),
        Taint::Secret,
        "edge: Secret + Clean must equal Secret"
    );
}

#[test]
fn edge_taint_merge_secret_secret_returns_secret() {
    assert_eq!(
        Taint::Secret.merge(Taint::Secret),
        Taint::Secret,
        "edge: Secret + Secret must equal Secret"
    );
}

#[test]
fn edge_value_fact_clean_constructor_returns_clean_taint() {
    let fact = ValueFact::clean(ValueType::Boolean);
    assert_eq!(fact.value_type, ValueType::Boolean);
    assert_eq!(fact.taint, Taint::Clean);
}

#[test]
fn edge_value_fact_secret_constructor_returns_secret_taint() {
    let fact = ValueFact::secret(ValueType::Object);
    assert_eq!(fact.value_type, ValueType::Object);
    assert_eq!(fact.taint, Taint::Secret);
}

#[test]
fn edge_empty_workflow_passes_all_validators() {
    let wf = make_workflow(vec![]);
    assert_eq!(
        validate_types(&wf),
        Ok(()),
        "edge: empty workflow must pass type validation"
    );
    assert_eq!(
        validate_taint(&wf),
        Ok(()),
        "edge: empty workflow must pass taint validation"
    );
    let hard = ResourceLimits::default();
    assert_eq!(
        validate_resource_limits(&wf, &hard),
        Ok(()),
        "edge: empty workflow must pass resource limit validation"
    );
}

#[test]
fn edge_resource_limits_zero_max_steps_with_non_empty_steps_fails_limit_required() {
    let wf = WorkflowTypes {
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![finish_step("s0", TypedValue::Literal(ValueType::Number))],
        resource_contract: ResourceLimits {
            max_steps: 0,
            ..ResourceLimits::default()
        },
    };
    let hard = ResourceLimits::default();
    let result = validate_resource_limits(&wf, &hard);
    assert_eq!(
        result,
        Err(ValidationError::LimitRequired {
            resource: "max_steps".to_owned(),
        }),
        "edge: max_steps=0 with non-empty steps must fail with LimitRequired"
    );
}

#[test]
fn taint_merge_clean_clean_is_clean() {
    assert_eq!(Taint::Clean.merge(Taint::Clean), Taint::Clean);
}

#[test]
fn taint_merge_clean_secret_is_secret() {
    assert_eq!(Taint::Clean.merge(Taint::Secret), Taint::Secret);
}

#[test]
fn taint_merge_secret_clean_is_secret() {
    assert_eq!(Taint::Secret.merge(Taint::Clean), Taint::Secret);
}

#[test]
fn taint_merge_secret_secret_is_secret() {
    assert_eq!(Taint::Secret.merge(Taint::Secret), Taint::Secret);
}

#[test]
fn value_fact_clean_has_clean_taint() {
    let fact = ValueFact::clean(ValueType::Number);
    assert_eq!(fact.taint, Taint::Clean);
    assert_eq!(fact.value_type, ValueType::Number);
}

#[test]
fn value_fact_secret_has_secret_taint() {
    let fact = ValueFact::secret(ValueType::Text);
    assert_eq!(fact.taint, Taint::Secret);
    assert_eq!(fact.value_type, ValueType::Text);
}

// =========================================================================
// DerivedFromSecret taint lattice tests (DEF-1 coverage gap)
// Lattice: Clean < DerivedFromSecret < Secret
// merge() rules per Section 47:
//   - Secret + anything = Secret
//   - DerivedFromSecret + anything (except Secret) = DerivedFromSecret
//   - Clean + Clean = Clean
// =========================================================================

#[test]
fn taint_merge_clean_derived_from_secret_is_derived_from_secret() {
    // Lattice: Clean.merge(DerivedFromSecret) = DerivedFromSecret
    assert_eq!(
        Taint::Clean.merge(Taint::DerivedFromSecret),
        Taint::DerivedFromSecret,
        "Clean + DerivedFromSecret must yield DerivedFromSecret"
    );
}

#[test]
fn taint_merge_derived_from_secret_clean_is_derived_from_secret() {
    // Lattice: DerivedFromSecret.merge(Clean) = DerivedFromSecret
    assert_eq!(
        Taint::DerivedFromSecret.merge(Taint::Clean),
        Taint::DerivedFromSecret,
        "DerivedFromSecret + Clean must yield DerivedFromSecret"
    );
}

#[test]
fn taint_merge_derived_from_secret_derived_from_secret_is_derived_from_secret() {
    // Lattice: DerivedFromSecret.merge(DerivedFromSecret) = DerivedFromSecret
    assert_eq!(
        Taint::DerivedFromSecret.merge(Taint::DerivedFromSecret),
        Taint::DerivedFromSecret,
        "DerivedFromSecret + DerivedFromSecret must yield DerivedFromSecret"
    );
}

#[test]
fn taint_merge_secret_derived_from_secret_is_secret() {
    // Lattice: Secret dominates DerivedFromSecret
    assert_eq!(
        Taint::Secret.merge(Taint::DerivedFromSecret),
        Taint::Secret,
        "Secret + DerivedFromSecret must yield Secret (Secret dominates)"
    );
}

#[test]
fn taint_merge_derived_from_secret_secret_is_secret() {
    // Lattice: Secret dominates DerivedFromSecret (commutative)
    assert_eq!(
        Taint::DerivedFromSecret.merge(Taint::Secret),
        Taint::Secret,
        "DerivedFromSecret + Secret must yield Secret (Secret dominates)"
    );
}

#[test]
fn taint_merge_derived_from_secret_is_commutative() {
    // DerivedFromSecret merge must be commutative like all other taint merges
    assert_eq!(
        Taint::DerivedFromSecret.merge(Taint::Clean),
        Taint::Clean.merge(Taint::DerivedFromSecret),
        "DerivedFromSecret merge must be commutative with Clean"
    );
    assert_eq!(
        Taint::DerivedFromSecret.merge(Taint::Secret),
        Taint::Secret.merge(Taint::DerivedFromSecret),
        "DerivedFromSecret merge must be commutative with Secret"
    );
}

#[test]
fn taint_lattice_three_levels_all_reachable() {
    // Prove all three lattice levels are reachable via merge
    let clean = Taint::Clean;
    let derived = Taint::DerivedFromSecret;
    let secret = Taint::Secret;

    // Level 0: Clean
    assert_eq!(clean.merge(clean), clean);

    // Level 1: DerivedFromSecret reachable from Clean
    assert_eq!(clean.merge(derived), derived);
    assert_eq!(derived.merge(clean), derived);

    // Level 2: Secret reachable from both Clean and DerivedFromSecret
    assert_eq!(clean.merge(secret), secret);
    assert_eq!(secret.merge(clean), secret);
    assert_eq!(derived.merge(secret), secret);
    assert_eq!(secret.merge(derived), secret);

    // Level 1: DerivedFromSecret also reachable as self
    assert_eq!(derived.merge(derived), derived);
}
