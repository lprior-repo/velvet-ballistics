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
