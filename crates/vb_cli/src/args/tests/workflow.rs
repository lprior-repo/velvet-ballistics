use std::path::PathBuf;

use super::args;
use crate::args::{Command, EmitTarget, OutputFormat, ParseError, VerifyProfile, parse_args};

#[test]
fn parse_validate_accepts_json_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "validate",
        "workflow.yaml",
        "--json",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Validate { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Validate { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Json);
    }
}

#[test]
fn parse_explain_accepts_jsonl_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "explain",
        "workflow.yaml",
        "--jsonl",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Explain { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Explain { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Jsonl);
    }
}

#[test]
fn parse_compile_includes_output_format() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "compile",
        "workflow.yaml",
        "--emit",
        "ir",
        "--out",
        "output.vbir",
        "--json",
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
        assert_eq!(output, OutputFormat::Json);
    }
}

#[test]
fn parse_compile_rejects_unknown_emit_target_with_exact_variant() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
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
fn parse_verify_defaults_to_standard_profile() {
    let parsed = parse_args(&args(&["velvet-ballastics", "verify", "workflow.yaml"]));
    assert!(
        matches!(parsed, Ok(Command::Verify { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Verify {
        workflow,
        profile,
        output,
    }) = parsed
    {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(profile, VerifyProfile::Standard);
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_verify_accepts_quick_profile() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "verify",
        "workflow.yaml",
        "--profile",
        "quick",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Verify { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Verify { profile, .. }) = parsed {
        assert_eq!(profile, VerifyProfile::Quick);
    }
}

#[test]
fn parse_verify_accepts_full_profile_with_json() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "verify",
        "workflow.yaml",
        "--profile",
        "full",
        "--json",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Verify { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Verify {
        profile, output, ..
    }) = parsed
    {
        assert_eq!(profile, VerifyProfile::Full);
        assert_eq!(output, OutputFormat::Json);
    }
}

#[test]
fn parse_verify_rejects_unknown_profile() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "verify",
        "workflow.yaml",
        "--profile",
        "thorough",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::UnknownProfile(_))),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_graph_defaults_to_text_output() {
    let parsed = parse_args(&args(&["velvet-ballastics", "graph", "workflow.yaml"]));
    assert!(
        matches!(parsed, Ok(Command::Graph { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Graph { workflow, output }) = parsed {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_graph_accepts_json_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "graph",
        "workflow.yaml",
        "--json",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Graph { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Graph { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Json);
    }
}

#[test]
fn parse_simulate_defaults_to_text_output() {
    let parsed = parse_args(&args(&["velvet-ballastics", "simulate", "workflow.yaml"]));
    assert!(
        matches!(parsed, Ok(Command::Simulate { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Simulate { workflow, output }) = parsed {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_simulate_accepts_json_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "simulate",
        "workflow.yaml",
        "--json",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Simulate { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Simulate { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Json);
    }
}

#[test]
fn parse_simulate_accepts_jsonl_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "simulate",
        "workflow.yaml",
        "--jsonl",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Simulate { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Simulate { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Jsonl);
    }
}
