#[cfg(test)]
use crate::vb_validate::type_sigs::{ResourceLimits, Taint, ValueFact, ValueType, WorkflowTypes};

// -- ValueType::as_str tests --

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

// -- Taint::merge tests --

#[test]
fn taint_merge_clean_clean_is_clean() {
    assert_eq!(Taint::Clean.merge(Taint::Clean), Taint::Clean);
}

#[test]
fn taint_merge_secret_clean_is_secret() {
    assert_eq!(Taint::Secret.merge(Taint::Clean), Taint::Secret);
}

#[test]
fn taint_merge_clean_secret_is_secret() {
    assert_eq!(Taint::Clean.merge(Taint::Secret), Taint::Secret);
}

#[test]
fn taint_merge_secret_secret_is_secret() {
    assert_eq!(Taint::Secret.merge(Taint::Secret), Taint::Secret);
}

// -- ValueFact constructor tests --

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
fn value_fact_clean_all_types() {
    let types = [
        ValueType::Null,
        ValueType::Boolean,
        ValueType::Number,
        ValueType::Text,
        ValueType::Object,
        ValueType::List,
        ValueType::Any,
    ];
    for vt in types {
        let fact = ValueFact::clean(vt);
        assert_eq!(fact.value_type, vt);
        assert_eq!(fact.taint, Taint::Clean);
    }
}

#[test]
fn value_fact_secret_all_types() {
    let types = [
        ValueType::Null,
        ValueType::Boolean,
        ValueType::Number,
        ValueType::Text,
        ValueType::Object,
        ValueType::List,
        ValueType::Any,
    ];
    for vt in types {
        let fact = ValueFact::secret(vt);
        assert_eq!(fact.value_type, vt);
        assert_eq!(fact.taint, Taint::Secret);
    }
}

// -- ResourceLimits default tests --

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

#[test]
fn workflow_types_default_is_empty() {
    let wf = WorkflowTypes::default();
    assert!(wf.inputs.is_empty());
    assert!(wf.vars.is_empty());
    assert!(wf.secrets.is_empty());
    assert!(wf.steps.is_empty());
}

// -- ValueType Copy/PartialEq tests --

#[test]
fn value_type_equality() {
    assert_eq!(ValueType::Number, ValueType::Number);
    assert_ne!(ValueType::Number, ValueType::Text);
}

// -- Taint Copy/PartialEq tests --

#[test]
fn taint_equality() {
    assert_eq!(Taint::Clean, Taint::Clean);
    assert_eq!(Taint::Secret, Taint::Secret);
    assert_ne!(Taint::Clean, Taint::Secret);
}
