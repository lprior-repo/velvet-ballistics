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
        resource_contract: ResourceLimits::default(),
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
