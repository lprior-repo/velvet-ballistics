use super::args;
use crate::args::{Command, EmitTarget, OutputFormat, ParseError, parse_args};
use std::path::PathBuf;

#[test]
fn parse_validate_accepts_json_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "validate",
        "workflow.yaml",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Validate { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Validate { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    }
}

#[test]
fn parse_explain_accepts_yaml_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "explain",
        "workflow.yaml",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Explain { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Explain { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    }
}

#[test]
fn parse_compile_uses_artifact_emit_without_output_format() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "compile",
        "workflow.yaml",
        "--emit",
        "ir",
        "--out",
        "output.vbir",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Compile { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Compile {
        workflow,
        emit,
        out,
        output,
    }) = parsed
    {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(emit, EmitTarget::Ir);
        assert_eq!(out, PathBuf::from("output.vbir"));
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_compile_artifact_yaml_does_not_select_yaml_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "compile",
        "workflow.yaml",
        "--emit",
        "yaml",
        "--out",
        "workflow.out.yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Compile { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Compile { emit, output, .. }) = parsed {
        assert_eq!(emit, EmitTarget::Yaml);
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_compile_rejects_unknown_emit_target_with_exact_variant() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "compile",
        "workflow.yaml",
        "--emit",
        "wasm",
        "--out",
        "output.vbir",
    ]));

    assert!(
        matches!(parsed, Err(ParseError::UnknownEmitTarget(ref t)) if t == "wasm"),
        "expected UnknownEmitTarget(wasm), got {parsed:?}"
    );
}

#[test]
fn parse_compile_rejects_deferred_rust_emit_target() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "compile",
        "workflow.yaml",
        "--emit",
        "rust",
        "--out",
        "output.rs",
    ]));

    assert!(
        matches!(parsed, Err(ParseError::UnknownEmitTarget(ref t)) if t == "rust"),
        "expected UnknownEmitTarget(rust), got {parsed:?}"
    );
}
