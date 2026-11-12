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
        resource_contract: ResourceLimits::default(),
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

include!("type_taint_tests/basic_acceptance.rs");
include!("type_taint_tests/bdd_exact_assertions.rs");
include!("type_taint_tests/adversarial_bdd.rs");
include!("type_taint_tests/proptest_invariants.rs");
include!("type_taint_tests/blackhat_security_regression.rs");
include!("type_taint_tests/blackhat_comprehensive_composite.rs");
include!("type_taint_tests/blackhat_resource_limits_zero.rs");
include!("type_taint_tests/blackhat_resource_limits_declared.rs");
include!("type_taint_tests/blackhat_type_checking_choose.rs");
include!("type_taint_tests/blackhat_reference_resolution.rs");
include!("type_taint_tests/blackhat_edge_cases.rs");
include!("type_taint_tests/edge_case_taint_merge.rs");
