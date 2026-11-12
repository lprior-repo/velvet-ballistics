use crate::{
    args::VerifyProfile,
    commands_verify::{VerifyError, run_verification},
};

fn verify(inputs: &str, examples: &str) -> Result<crate::commands_verify::VerifyOk, VerifyError> {
    let source = format!(
        "version: velvet-ballistics/v1\nname: examples\nwhen: {{manual: {{}}}}\n{inputs}\nsteps:\n  - id: answer\n    set: {{output: answer, value: '$input.number + 2'}}\nresult: {{value: '$steps.answer'}}\nexamples:\n{examples}"
    );
    run_verification(&source, source.as_bytes(), VerifyProfile::Quick)
}

#[test]
fn examples_execute_with_explicit_inputs_and_defaults() -> Result<(), String> {
    let result = verify("inputs:\n  number: {is: number, default: 40}",
        "  - input: {number: 3}\n    expected: {value: 5}\n  - input: {}\n    expected: {value: 42}")
        .map_err(|error| match error { VerifyError::Compile(errors) => errors.join("; "), VerifyError::YamlParse(error) | VerifyError::IrValidation(error) => error, _ => "verification failed".into() })?;
    assert!(result.checks.contains(&"examples"));
    Ok(())
}

#[test]
fn mismatched_example_result_fails_verification() {
    assert!(matches!(verify("inputs:\n  number: {is: number}",
        "  - input: {number: 3}\n    expected: {value: 6}"), Err(VerifyError::Compile(errors)) if errors.iter().any(|error| error.contains("result differs"))));
}

#[test]
fn example_input_schema_is_enforced() {
    for input in ["{}", "{number: wrong}", "{unknown: 3}"] {
        assert!(matches!(
            verify(
                "inputs:\n  number: {is: number}",
                &format!("  - input: {input}\n    expected: {{value: 5}}")
            ),
            Err(VerifyError::Compile(_))
        ));
    }
}

#[test]
fn missing_expected_value_is_rejected() {
    assert!(matches!(
        verify(
            "inputs:\n  number: {is: number, default: 40}",
            "  - input: {}"
        ),
        Err(VerifyError::Compile(_))
    ));
}

#[test]
fn examples_resume_across_single_step_slices() -> Result<(), String> {
    let text = "version: velvet-ballistics/v1\nname: sliced_example\nwhen: {manual: {}}\nsteps:\n  - id: a\n    set: {output: a, value: '40'}\n  - id: b\n    set: {output: b, value: '$steps.a + 2'}\nresult: {value: '$steps.b'}\nexamples:\n  - expected: {value: 42}\n";
    let source = vb_compile::parse_workflow_source(text).map_err(|e| e.to_string())?;
    let compiled = vb_compile::compile_workflow(text.as_bytes()).map_err(|e| e.to_string())?;
    let mut parts = compiled.to_parts();
    parts.resource_contract.max_step_budget_per_tick = 1;
    let compiled = vb_core::CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())?;
    assert_eq!(super::verify(&source, &compiled), Ok(()));
    Ok(())
}
