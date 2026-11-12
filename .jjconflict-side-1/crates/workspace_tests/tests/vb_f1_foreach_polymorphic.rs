#![forbid(unsafe_code)]

#[path = "vb_f1_foreach_polymorphic/runtime.rs"]
mod runtime;

use runtime::{TestResult, source, verify};
use vb_compile::{CompileError, compile_workflow};
use vb_core::{SlotValue as V, Taint};

const TAINTS: [Taint; 5] = [
    Taint::Clean,
    Taint::DerivedFromSecret,
    Taint::Secret,
    Taint::Random,
    Taint::TimeDependent,
];

fn matrix(expression: &str, expected: [V; 3], prefix_reads: bool) -> TestResult {
    verify(
        expression,
        expected,
        Taint::Clean,
        [Taint::Clean; 3],
        [Taint::Clean; 3],
    )?;
    for root in TAINTS {
        for item in TAINTS {
            let elements = [item, Taint::Clean, Taint::DerivedFromSecret];
            let mut prefix = Taint::Clean;
            let output_taints = elements.map(|element| {
                prefix = vb_core::value::join_taint(prefix, element);
                vb_core::value::join_taint(root, if prefix_reads { prefix } else { element })
            });
            verify(expression, expected, root, elements, output_taints)?;
        }
    }
    Ok(())
}

#[test]
fn named_action_list_equality_finishes_ordered_with_exact_taints() -> TestResult {
    matrix(
        "$entry == 1",
        [V::Bool(true), V::Bool(false), V::Bool(false)],
        false,
    )
}

#[test]
fn named_action_list_inequality_finishes_ordered_with_exact_taints() -> TestResult {
    matrix(
        "$entry != 1",
        [V::Bool(false), V::Bool(true), V::Bool(true)],
        false,
    )
}

#[test]
fn exists_accepts_direct_unknown_elements() -> TestResult {
    matrix(
        "exists($entry)",
        [V::Bool(true), V::Bool(true), V::Bool(false)],
        false,
    )
}

#[test]
fn contains_accepts_direct_unknown_list_and_elements() -> TestResult {
    matrix("contains($steps.fetch, $entry)", [V::Bool(true); 3], true)
}

#[test]
fn contains_accepts_unknown_needle_with_mixed_membership() -> TestResult {
    matrix(
        "contains(append($vars.empty_list, 1), $entry)",
        [V::Bool(true), V::Bool(false), V::Bool(false)],
        false,
    )
}

#[test]
fn bare_unknown_copy_preserves_values_and_element_taints() -> TestResult {
    matrix("$entry", [V::I64(1), V::I64(2), V::Null], false)
}

fn rejects(expression: &str, expected: CompileError) -> TestResult {
    let errors = compile_workflow(source(expression).as_bytes())
        .err()
        .ok_or("expected compile rejection")?;
    assert_eq!(errors.0, vec![expected]);
    Ok(())
}

#[test]
fn polymorphic_parents_do_not_relax_nested_numeric_or_boolean_operands() -> TestResult {
    for expression in [
        "exists($entry + 1)",
        "($entry + 1) == 1",
        "exists(-$entry)",
        "(not $entry) != true",
        "exists($entry and true)",
    ] {
        rejects(
            expression,
            CompileError::TypeMismatch {
                field: "set.value",
                expected: "known static operand type",
                found: "unknown",
            },
        )?;
    }
    Ok(())
}

#[test]
fn known_invalid_helper_kinds_remain_exact_type_errors() -> TestResult {
    for expression in ["sum(true)", "append(true, 1)"] {
        rejects(
            expression,
            CompileError::TypeMismatch {
                field: "set.value",
                expected: "list",
                found: "boolean",
            },
        )?;
    }
    Ok(())
}

#[test]
fn non_list_action_completion_reports_typed_foreach_error() -> TestResult {
    runtime::non_list()
}
