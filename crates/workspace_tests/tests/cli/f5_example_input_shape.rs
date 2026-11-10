use std::process::Command;
use vb_compile::{CompileError, CompileErrors, YamlError};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn source(examples: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: example_shape\nwhen: {{manual: {{}}}}\ninputs:\n  number: {{is: number, default: 40}}\nsteps:\n  - id: answer\n    set: {{output: answer, value: '$input.number + 2'}}\nresult: {{value: '$steps.answer'}}\nexamples:\n{examples}\n"
    )
}

fn shape_error() -> CompileError {
    CompileError::FieldShape {
        field: "examples",
        expected: "an optional input mapping and a required expected value",
    }
}

#[test]
fn public_compile_rejects_default_matching_nonmapping_examples() -> TestResult {
    for input in ["3", "[3]", "null"] {
        let text = source(&format!("  - input: {input}\n    expected: {{value: 42}}"));
        // The source AST preserves author values; compilation enforces example policy.
        let parsed = vb_compile::parse_workflow_source(&text)?;
        let expected = Err(CompileErrors(vec![shape_error()]));
        assert_eq!(
            vb_compile::compile_workflow(text.as_bytes()).map(|_| ()),
            expected
        );
        assert_eq!(vb_compile::compile_source(&parsed).map(|_| ()), expected);
    }
    let text = source("  - input: {}");
    assert_eq!(
        vb_compile::compile_workflow(text.as_bytes()).map(|_| ()),
        Err(CompileErrors(vec![shape_error()]))
    );
    Ok(())
}

#[test]
fn nonstring_example_mapping_keys_are_rejected_without_dropping_them() {
    for input in ["{7: 3}", "{true: 3}"] {
        let text = source(&format!("  - input: {input}\n    expected: {{value: 42}}"));
        let expected = YamlError::FieldShape {
            field: "mapping key",
            expected: "string",
        };
        assert_eq!(
            vb_compile::parse_workflow_source(&text).map(|_| ()),
            Err(expected.clone())
        );
        assert_eq!(
            vb_compile::compile_workflow(text.as_bytes()).map(|_| ()),
            Err(CompileErrors(vec![CompileError::CanonicalYaml {
                category: "field_shape",
                message: expected.to_string().into(),
            }]))
        );
    }
}

fn verify(text: &str, profile: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("workflow.yaml");
    std::fs::write(&path, text)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .arg("verify")
        .arg(path)
        .args(["--profile", profile, "--emit", "yaml"])
        .output()?)
}

fn assert_compile_rejection(output: std::process::Output, message: &str) -> TestResult {
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout.as_slice(), b"");
    let report: serde_json::Value = serde_saphyr::from_str(std::str::from_utf8(&output.stderr)?)?;
    assert_eq!(
        report,
        serde_json::json!({
            "success": false,
            "error": "ValidationFailed",
            "exit_code": 2,
            "message": format!("compilation failed:\n  {message}\n"),
        })
    );
    Ok(())
}

#[test]
fn malformed_examples_preserve_compile_category_in_all_cli_profiles() -> TestResult {
    for profile in ["quick", "standard", "full"] {
        for input in ["3", "[3]", "null"] {
            let text = source(&format!("  - input: {input}\n    expected: {{value: 42}}"));
            assert_compile_rejection(verify(&text, profile)?, &shape_error().to_string())?;
        }
        assert_compile_rejection(
            verify(&source("  - input: {}"), profile)?,
            &shape_error().to_string(),
        )?;
    }
    Ok(())
}

#[test]
fn valid_defaults_inputs_and_expected_results_work_in_all_cli_profiles() -> TestResult {
    let text = source(
        "  - expected: {value: 42}\n  - input: {}\n    expected: {value: 42}\n  - input: {number: 3}\n    expected: {value: 5}",
    );
    let composite = "version: velvet-ballistics/v1\nname: composite_example\nwhen: {manual: {}}\ninputs:\n  payload:\n    is: object\n    fields:\n      title: {is: text}\n      items: {is: list, of: number, max: 2}\n    default: {title: hi, items: [1, 2]}\nsteps:\n  - id: done\n    finish: {result: '$input.payload'}\nexamples:\n  - expected: {title: hi, items: [1, 2]}\n  - input: {payload: {title: bye, items: [2]}}\n    expected: {title: bye, items: [2]}\n";
    for profile in ["quick", "standard", "full"] {
        for text in [text.as_str(), composite] {
            let output = verify(text, profile)?;
            assert_eq!(
                output.status.code(),
                Some(0),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stderr.as_slice(), b"");
            let report: serde_json::Value =
                serde_saphyr::from_str(std::str::from_utf8(&output.stdout)?)?;
            assert_eq!(report.get("success"), Some(&serde_json::json!(true)));
            assert_eq!(report.get("profile"), Some(&serde_json::json!(profile)));
            assert_eq!(report.get("warnings"), Some(&serde_json::json!([])));
            let checks = report.get("checks").ok_or("missing checks")?;
            let expected = if profile == "quick" {
                serde_json::json!(["yaml_parse", "compilation", "ir_validation", "examples"])
            } else {
                serde_json::json!([
                    "yaml_parse",
                    "compilation",
                    "ir_validation",
                    "budget_computation",
                    "boundedness_policy",
                    "examples"
                ])
            };
            assert_eq!(checks, &expected);
        }
        let wrong = source("  - input: {}\n    expected: {value: 43}");
        assert_compile_rejection(
            verify(&wrong, profile)?,
            "example 0: result differs from expected",
        )?;
    }
    Ok(())
}
