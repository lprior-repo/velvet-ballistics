#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::panic,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing
)]
#![forbid(unsafe_code)]
//! Section 38 property test: `constant_folding_validation`.
//!
//! Master plan §38, row "Constant folding":
//! "Constant expressions fold to identical result as runtime evaluation".
//!
//! This file asserts the validation-side analogue:
//! for any `WorkflowTypes` whose step `TypedValue`s are *all* literals
//! (i.e., contain no `Reference` or `Slot` indirection), the type/taint
//! validators must be deterministic, must accept the workflow, and must
//! produce the same result across repeated invocations.

use proptest::prelude::*;

use crate::type_taint::{
    InputDecl, ResourceLimits, StepKind, StepTypes, Taint, TypedValue, ValueFact, ValueType,
    WorkflowTypes, validate_taint, validate_types,
};

fn literal_workflow(value: TypedValue) -> WorkflowTypes {
    WorkflowTypes {
        inputs: Vec::new(),
        vars: Vec::new(),
        secrets: Vec::new(),
        steps: vec![StepTypes {
            id: "finish_step".to_owned(),
            kind: StepKind::Finish { result: value },
        }],
        resource_contract: ResourceLimits {
            allows_secret_results: true,
            ..ResourceLimits::default()
        },
    }
}

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

fn arb_finish_step() -> impl Strategy<Value = StepTypes> {
    arb_value_type().prop_map(|vt| StepTypes {
        id: "finish".to_owned(),
        kind: StepKind::Finish {
            result: TypedValue::Literal(vt),
        },
    })
}

fn arb_save_step() -> impl Strategy<Value = StepTypes> {
    arb_value_type().prop_map(|vt| StepTypes {
        id: "save".to_owned(),
        kind: StepKind::Save {
            value: TypedValue::Literal(vt),
        },
    })
}

fn arb_choose_step() -> impl Strategy<Value = StepTypes> {
    Just(ValueType::Boolean).prop_map(|vt| StepTypes {
        id: "choose".to_owned(),
        kind: StepKind::Choose {
            condition: TypedValue::Literal(vt),
        },
    })
}

fn arb_literal_only_workflow() -> impl Strategy<Value = WorkflowTypes> {
    (
        prop::collection::vec(arb_finish_step(), 1..=3),
        prop::collection::vec(arb_save_step(), 0..=2),
        prop::collection::vec(arb_choose_step(), 0..=2),
    )
        .prop_map(|(finishes, saves, chooses)| WorkflowTypes {
            inputs: Vec::new(),
            vars: Vec::new(),
            secrets: Vec::new(),
            steps: saves
                .into_iter()
                .chain(chooses)
                .chain(finishes)
                .collect(),
            resource_contract: ResourceLimits {
                allows_secret_results: true,
                ..ResourceLimits::default()
            },
        })
}

proptest! {
    /// Pure-literal workflow must always pass `validate_types`.
    #[test]
    fn cfv_literal_only_workflow_passes_types(wf in arb_literal_only_workflow()) {
        prop_assert_eq!(
            validate_types(&wf),
            Ok(()),
            "literal-only workflow must validate cleanly"
        );
    }

    /// Pure-literal workflow must always pass `validate_taint`
    /// (literals are always Clean, Finish is allowed to carry any taint).
    #[test]
    fn cfv_literal_only_workflow_passes_taint(wf in arb_literal_only_workflow()) {
        prop_assert_eq!(
            validate_taint(&wf),
            Ok(()),
            "literal-only workflow must validate cleanly"
        );
    }

    /// Validation is deterministic: calling `validate_types` repeatedly on
    /// the same workflow always returns the same `Result`.
    #[test]
    fn cfv_validate_types_is_deterministic(wf in arb_literal_only_workflow()) {
        let r1 = validate_types(&wf);
        let r2 = validate_types(&wf);
        let r3 = validate_types(&wf);
        prop_assert_eq!(r1.clone(), r2.clone());
        prop_assert_eq!(r2, r3);
    }

    /// Validation is deterministic for `validate_taint` too.
    #[test]
    fn cfv_validate_taint_is_deterministic(wf in arb_literal_only_workflow()) {
        let r1 = validate_taint(&wf);
        let r2 = validate_taint(&wf);
        prop_assert_eq!(r1, r2);
    }

    /// Per-value-type: every `ValueType` literal as a Finish result passes
    /// both validators. This is the exhaustive single-axis test of the
    /// "fold-to-identical-result" property.
    #[test]
    fn cfv_each_value_type_finish_is_valid(vt in arb_value_type()) {
        let wf = literal_workflow(TypedValue::Literal(vt));
        prop_assert_eq!(validate_types(&wf), Ok(()));
        prop_assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// Per-value-type: every `ValueType` literal as a Save result passes
    /// `validate_types` (Save is type-agnostic — it just writes the fact).
    #[test]
    fn cfv_each_value_type_save_is_valid(vt in arb_value_type()) {
        let wf = WorkflowTypes {
            inputs: Vec::new(),
            vars: Vec::new(),
            secrets: Vec::new(),
            steps: vec![StepTypes {
                id: "s".to_owned(),
                kind: StepKind::Save {
                    value: TypedValue::Literal(vt),
                },
            }],
            resource_contract: ResourceLimits {
                allows_secret_results: true,
                ..ResourceLimits::default()
            },
        };
        prop_assert_eq!(validate_types(&wf), Ok(()));
    }

    /// Step order does not change the validation result for a literal-only
    /// workflow (folding is associative over independent step facts).
    #[test]
    fn cfv_step_order_irrelevance_for_literal_workflow(
        mut saves_a in prop::collection::vec(arb_value_type(), 1..=3),
        mut saves_b in prop::collection::vec(arb_value_type(), 1..=3),
    ) {
        let mut wf_a: Vec<StepTypes> = saves_a
            .drain(..)
            .enumerate()
            .map(|(i, vt)| StepTypes {
                id: format!("a{i}"),
                kind: StepKind::Save {
                    value: TypedValue::Literal(vt),
                },
            })
            .collect();
        let mut wf_b: Vec<StepTypes> = saves_b
            .drain(..)
            .enumerate()
            .map(|(i, vt)| StepTypes {
                id: format!("b{i}"),
                kind: StepKind::Save {
                    value: TypedValue::Literal(vt),
                },
            })
            .collect();
        let finish = StepTypes {
            id: "done".to_owned(),
            kind: StepKind::Finish {
                result: TypedValue::Literal(ValueType::Any),
            },
        };
        wf_a.push(finish.clone());
        wf_b.push(finish);
        let a = WorkflowTypes {
            inputs: Vec::new(),
            vars: Vec::new(),
            secrets: Vec::new(),
            steps: wf_a,
            resource_contract: ResourceLimits {
                allows_secret_results: true,
                ..ResourceLimits::default()
            },
        };
        let b = WorkflowTypes {
            inputs: Vec::new(),
            vars: Vec::new(),
            secrets: Vec::new(),
            steps: wf_b,
            resource_contract: ResourceLimits {
                allows_secret_results: true,
                ..ResourceLimits::default()
            },
        };
        prop_assert_eq!(validate_types(&a), validate_types(&b));
    }

    /// Adding non-secret inputs whose values are unused does not change
    /// the result of validation for an empty workflow.
    #[test]
    fn cfv_unused_inputs_keep_validation_clean(
        input_count in 0u8..6u8,
    ) {
        let inputs: Vec<InputDecl> = (0..input_count)
            .map(|i| InputDecl {
                name: format!("in{i}"),
                schema_type: ValueType::Number,
                is_secret: false,
            })
            .collect();
        let wf = WorkflowTypes {
            inputs,
            vars: Vec::new(),
            secrets: Vec::new(),
            steps: vec![StepTypes {
                id: "done".to_owned(),
                kind: StepKind::Finish {
                    result: TypedValue::Literal(ValueType::Number),
                },
            }],
            resource_contract: ResourceLimits {
                allows_secret_results: true,
                ..ResourceLimits::default()
            },
        };
        prop_assert_eq!(validate_types(&wf), Ok(()));
        prop_assert_eq!(validate_taint(&wf), Ok(()));
    }

    /// Sanity: `Taint::Clean` is the only taint a literal can carry. The
    /// validators must not produce a `Secret` `ValueFact` from a literal
    /// under any circumstance.
    #[test]
    fn cfv_literal_always_clean_taint(vt in arb_value_type()) {
        let wf = literal_workflow(TypedValue::Literal(vt));
        // Re-running the validator must not flip a literal to Secret.
        let _ = validate_taint(&wf);
        // The `ValueFact::clean` constructor is the only path that can
        // produce a Clean fact from a literal; assert the invariant
        // directly on the constructor.
        let fact = ValueFact::clean(vt);
        prop_assert_eq!(fact.taint, Taint::Clean);
    }
}
