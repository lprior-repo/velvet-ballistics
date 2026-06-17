//! Kani harnesses for validate_branch_counts overflow verification (PO-011).

#![cfg(kani)]
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
        name: "test_together_overflow".to_string(),
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

/// KANI-XI2F.15-011: Prove validate_branch_counts rejects branch count > u16::MAX.
#[kani::proof]
#[kani::unwind(6)]
fn branch_count_overflow_harness() {
    let branch_count: usize = kani::any();
    kani::assume(branch_count == usize::from(u16::MAX) || branch_count == usize::from(u16::MAX).saturating_add(1));

    let source = build_together_workflow(branch_count);
    let result = crate::mod_compile_lowering::validate_branch_counts(&source);

    if branch_count > usize::from(u16::MAX) {
        kani::assert(result.is_err(), "validate_branch_counts must reject branches.len() > u16::MAX");
    } else {
        kani::assert(result.is_ok(), "validate_branch_counts must accept branches.len() == u16::MAX");
    }
}
