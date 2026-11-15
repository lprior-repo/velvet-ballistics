#![forbid(unsafe_code)]

use vb_compile::{CompileError, compile_workflow};
use vb_core::WorkflowError;
use vb_core::workflow::ContractResource;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn flat_ir_lowering_enforces_node_limit_before_slot_analysis() -> TestResult {
    for count in [1_000_u16, 1_001] {
        let nodes = (0..count)
            .map(|index| {
                let next = index
                    .checked_add(1)
                    .filter(|next| *next < count)
                    .map(vb_core::StepIdx::new);
                vb_core::workflow::node(
                    vb_core::StepIdx::new(index),
                    vb_core::CompiledNodeKind::Nop,
                    None,
                    next,
                )
            })
            .collect();
        let result = vb_compile::lower_steps_to_ir(
            nodes,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            0,
            0,
            "flat_node_limit",
            vb_core::WorkflowDigest::from_bytes([0; 32]),
        );
        match (count, result) {
            (1_000, Ok(workflow)) => assert_eq!(workflow.node_count(), 1_000),
            (1_001, Err(errors)) => {
                assert_eq!(errors.0, vec![CompileError::NodeLimit { limit: 1_000 }]);
            }
            (_, result) => return Err(format!("unexpected result for {count}: {result:?}").into()),
        }
    }
    Ok(())
}

fn workflow_with_steps(step_count: usize) -> String {
    let mut source = String::from(
        "version: velvet-ballistics/v1\nname: hard_limit\nwhen:\n  manual: {}\nsteps:\n",
    );
    for index in 0..step_count.saturating_sub(1) {
        source.push_str(&format!(
            "  - id: step_{index}\n    save:\n      output: value_{index}\n      value: '1'\n"
        ));
    }
    source.push_str("  - id: done\n    finish:\n      result: value_0\n");
    source
}

fn workflow_with_constants(constant_count: usize) -> TestResult<String> {
    let mut source = String::from(
        "version: velvet-ballistics/v1\nname: constant_limit\nwhen:\n  manual: {}\nsteps:\n  - id: seed\n    set: {output: seed, value: '1'}\n",
    );
    append_constant_steps(&mut source, constant_count.saturating_sub(1))?;
    source.push_str("  - id: done\n    finish: {result: '$step.seed'}\n");
    Ok(source)
}

fn append_constant_steps(source: &mut String, count: usize) -> TestResult {
    let mut remaining = count;
    let mut step = 0_usize;
    while remaining > 0 {
        let literal_count = remaining.min(9);
        let expression = constant_expression(step, literal_count)?;
        source.push_str(&format!(
            "  - id: value_{step}\n    set: {{output: result_{step}, value: '$step.seed + {expression}'}}\n"
        ));
        remaining = remaining
            .checked_sub(literal_count)
            .ok_or("constant count underflow")?;
        step = step.checked_add(1).ok_or("step index overflow")?;
    }
    Ok(())
}

fn constant_expression(step: usize, count: usize) -> TestResult<String> {
    let mut literals = Vec::with_capacity(count);
    for index in 0..count {
        let offset = step
            .checked_mul(9)
            .and_then(|base| base.checked_add(index))
            .ok_or("constant offset overflow")?;
        let value = 100_000_usize
            .checked_add(offset)
            .ok_or("constant value overflow")?;
        literals.push(value.to_string());
    }
    Ok(literals.join(" + "))
}

#[test]
fn compiler_accepts_exact_compiled_step_limit() {
    let workflow = workflow_with_steps(1_000);
    let compiled = compile_workflow(workflow.as_bytes());
    assert!(
        compiled.is_ok(),
        "exact hard-limit source should compile: {compiled:?}"
    );
    assert_eq!(compiled.map(|workflow| workflow.node_count()), Ok(1_000));
}

#[test]
fn compiler_rejects_one_over_compiled_step_limit_with_current_error() {
    let workflow = workflow_with_steps(1_001);
    let errors = compile_workflow(workflow.as_bytes());
    assert!(matches!(
        errors,
        Err(errors) if errors.0 == vec![CompileError::NodeLimit { limit: 1_000 }]
    ));
}

#[test]
fn compiler_accepts_exact_compiled_constant_limit() -> TestResult {
    let source = workflow_with_constants(8_192)?;
    let compiled = compile_workflow(source.as_bytes())?;
    assert_eq!(compiled.to_parts().constants.len(), 8_192);
    assert!(usize::from(compiled.node_count()) < 1_000);
    Ok(())
}

#[test]
fn compiler_rejects_one_over_compiled_constant_limit_with_current_error() -> TestResult {
    let source = workflow_with_constants(8_193)?;
    let errors = compile_workflow(source.as_bytes()).expect_err("constant limit must be enforced");
    assert_eq!(
        errors.0,
        vec![CompileError::Workflow(
            WorkflowError::ResourceContractExceeded {
                resource: ContractResource::Constants
            }
        )]
    );
    Ok(())
}
