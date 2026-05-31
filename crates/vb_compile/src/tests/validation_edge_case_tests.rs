//! Validation edge case tests for Together branch count limits (PO-012).

#![forbid(unsafe_code)]

use vb_yaml::ast::{ScalarValue, StepAst, StepPrimitive, TogetherBranch, TriggerAst, WorkflowSourceParts};

fn build_together_workflow(branch_count: usize) -> vb_yaml::ast::WorkflowSource {
    let branches: Vec<TogetherBranch> = (0..branch_count)
        .map(|i| TogetherBranch {
            label: format!("branch_{}", i),
            steps: vec![StepAst {
                id: format!("step_{}", i),
                name: None,
                condition: None,
                primitive: StepPrimitive::Set {
                    output: format!("out_{}", i),
                    value: format!("{}", i),
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            }],
        })
        .collect();

    let parts = WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "test_together_edge".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![
            StepAst {
                id: "fanout".to_string(),
                name: None,
                condition: None,
                primitive: StepPrimitive::Together { branches },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            },
            StepAst {
                id: "finish".to_string(),
                name: None,
                condition: None,
                primitive: StepPrimitive::Finish {
                    result: ScalarValue::String("done".to_string()),
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            },
        ],
        result: None,
        examples: vec![],
    };

    vb_yaml::ast::WorkflowSource::new(parts)
}

#[test]
fn validate_branch_counts_u16_max_minus_one() {
    let source = build_together_workflow(usize::from(u16::MAX) - 1);
    let result = crate::mod_compile_lowering::validate_branch_counts(&source);
    assert!(result.is_ok(), "validate_branch_counts must accept u16::MAX - 1 branches");
}

#[test]
fn validate_branch_counts_u16_max() {
    let source = build_together_workflow(usize::from(u16::MAX));
    let result = crate::mod_compile_lowering::validate_branch_counts(&source);
    assert!(result.is_ok(), "validate_branch_counts must accept u16::MAX branches");
}

#[test]
fn validate_branch_counts_u16_max_plus_one() {
    let source = build_together_workflow(usize::from(u16::MAX).saturating_add(1));
    let result = crate::mod_compile_lowering::validate_branch_counts(&source);
    assert!(result.is_err(), "validate_branch_counts must reject u16::MAX + 1 branches");
}

#[test]
fn validate_branch_counts_zero_branches() {
    let source = build_together_workflow(0);
    let result = crate::mod_compile_lowering::validate_branch_counts(&source);
    assert!(result.is_ok(), "validate_branch_counts accepts 0 (overflow check only)");
}
