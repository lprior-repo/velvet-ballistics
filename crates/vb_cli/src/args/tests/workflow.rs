use std::path::PathBuf;

use super::args;
use crate::args::{Command, EmitTarget, OutputFormat, ParseError, VerifyProfile, parse_args};

#[test]
fn parse_validate_requires_workflow() {
    let parsed = parse_args(&args(&["velvet-ballistics", "validate", "workflow.yaml"]));
    match parsed {
        Ok(Command::Validate { workflow, output }) => {

            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(output, OutputFormat::Text);

        }
        other => panic!("expected Command::Validate, got {other:?}"),
    }
}

#[test]
fn parse_validate_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "validate",
        "workflow.yaml",
        "--emit",
        "yaml",
    ]));
    match parsed {
        Ok(Command::Validate { output, .. }) => {

            assert_eq!(output, OutputFormat::Yaml);

        }
        other => panic!("expected Command::Validate, got {other:?}"),
    }
}

#[test]
fn parse_validate_accepts_emit_postcard() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "validate",
        "workflow.yaml",
        "--emit",
        "postcard",
    ]));
    match parsed {
        Ok(Command::Validate { output, .. }) => {

            assert_eq!(output, OutputFormat::Postcard);

        }
        other => panic!("expected Command::Validate, got {other:?}"),
    }
}

#[test]
fn parse_validate_rejects_unknown_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "validate",
        "workflow.yaml",
        "--bogus",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::UnknownFlag {
            command: "validate",
            ..
        })
    ));
}

#[test]
fn parse_explain_requires_workflow() {
    let parsed = parse_args(&args(&["velvet-ballistics", "explain", "workflow.yaml"]));
    match parsed {
        Ok(Command::Explain { workflow, output }) => {

            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(output, OutputFormat::Text);

        }
        other => panic!("expected Command::Explain, got {other:?}"),
    }
}

#[test]
fn parse_explain_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "explain",
        "workflow.yaml",
        "--emit",
        "yaml",
    ]));
    match parsed {
        Ok(Command::Explain { output, .. }) => {

            assert_eq!(output, OutputFormat::Yaml);

        }
        other => panic!("expected Command::Explain, got {other:?}"),
    }
}

#[test]
fn parse_explain_legacy_jsonl_flag_keeps_text_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "explain",
        "workflow.yaml",
        "--jsonl",
    ]));
    match parsed {
        Ok(Command::Explain { output, .. }) => {

            assert_eq!(output, OutputFormat::Text);

        }
        other => panic!("expected Command::Explain, got {other:?}"),
    }
}

#[test]
fn parse_compile_requires_emit_and_out() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "compile",
        "workflow.yaml",
        "--emit",
        "ir",
        "--out",
        "output.vbir",
    ]));
    match parsed {
        Ok(Command::Compile {
        workflow,
        emit,
        out,
        output,
    }) => {

            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(emit, EmitTarget::Ir);
            assert_eq!(out, PathBuf::from("output.vbir"));
            assert_eq!(output, OutputFormat::Text);

        }
        other => panic!("expected Command::Compile, got {other:?}"),
    }
}

#[test]
fn parse_compile_accepts_emit_postcard() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "compile",
        "workflow.yaml",
        "--emit",
        "postcard",
        "--out",
        "workflow.vbpc",
    ]));
    match parsed {
        Ok(Command::Compile { emit, .. }) => {

            assert_eq!(emit, EmitTarget::Postcard);

        }
        other => panic!("expected Command::Compile, got {other:?}"),
    }
}

#[test]
fn parse_compile_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "compile",
        "workflow.yaml",
        "--emit",
        "yaml",
        "--out",
        "workflow.out.yaml",
    ]));
    match parsed {
        Ok(Command::Compile { emit, output, .. }) => {

            assert_eq!(emit, EmitTarget::Yaml);
            assert_eq!(output, OutputFormat::Text);

        }
        other => panic!("expected Command::Compile, got {other:?}"),
    }
}

#[test]
fn parse_compile_legacy_json_flag_keeps_text_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "compile",
        "workflow.yaml",
        "--emit",
        "ir",
        "--out",
        "output.vbir",
        "--json",
    ]));
    match parsed {
        Ok(Command::Compile { output, .. }) => {

            assert_eq!(output, OutputFormat::Text);

        }
        other => panic!("expected Command::Compile, got {other:?}"),
    }
}

#[test]
fn parse_compile_legacy_jsonl_flag_keeps_text_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "compile",
        "workflow.yaml",
        "--emit",
        "ir",
        "--out",
        "output.vbir",
        "--jsonl",
    ]));
    match parsed {
        Ok(Command::Compile { output, .. }) => {

            assert_eq!(output, OutputFormat::Text);

        }
        other => panic!("expected Command::Compile, got {other:?}"),
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
fn parse_compile_rejects_missing_emit() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "compile",
        "workflow.yaml",
        "--out",
        "output.vbir",
    ]));
    assert!(matches!(parsed, Err(ParseError::MissingArgument("--emit"))));
}

#[test]
fn parse_compile_rejects_missing_out() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "compile",
        "workflow.yaml",
        "--emit",
        "ir",
    ]));
    assert!(matches!(parsed, Err(ParseError::MissingArgument("--out"))));
}

#[test]
fn parse_verify_defaults_to_standard_profile() {
    let parsed = parse_args(&args(&["velvet-ballistics", "verify", "workflow.yaml"]));
    match parsed {
        Ok(Command::Verify {
        workflow,
        profile,
        output,
    }) => {

            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(profile, VerifyProfile::Standard);
            assert_eq!(output, OutputFormat::Text);

        }
        other => panic!("expected Command::Verify, got {other:?}"),
    }
}

#[test]
fn parse_verify_accepts_quick_profile() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "verify",
        "workflow.yaml",
        "--profile",
        "quick",
    ]));
    match parsed {
        Ok(Command::Verify { profile, .. }) => {

            assert_eq!(profile, VerifyProfile::Quick);

        }
        other => panic!("expected Command::Verify, got {other:?}"),
    }
}

#[test]
fn parse_verify_accepts_full_profile_with_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "verify",
        "workflow.yaml",
        "--profile",
        "full",
        "--emit",
        "yaml",
    ]));
    match parsed {
        Ok(Command::Verify {
        profile, output, ..
    }) => {

            assert_eq!(profile, VerifyProfile::Full);
            assert_eq!(output, OutputFormat::Yaml);

        }
        other => panic!("expected Command::Verify, got {other:?}"),
    }
}

#[test]
fn parse_verify_rejects_unknown_profile() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
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
    let parsed = parse_args(&args(&["velvet-ballistics", "graph", "workflow.yaml"]));
    match parsed {
        Ok(Command::Graph { workflow, output }) => {

            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(output, OutputFormat::Text);

        }
        other => panic!("expected Command::Graph, got {other:?}"),
    }
}

#[test]
fn parse_graph_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "graph",
        "workflow.yaml",
        "--emit",
        "yaml",
    ]));
    match parsed {
        Ok(Command::Graph { output, .. }) => {

            assert_eq!(output, OutputFormat::Yaml);

        }
        other => panic!("expected Command::Graph, got {other:?}"),
    }
}

#[test]
fn parse_graph_legacy_json_flag_keeps_text_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "graph",
        "workflow.yaml",
        "--json",
    ]));
    match parsed {
        Ok(Command::Graph { output, .. }) => {

            assert_eq!(output, OutputFormat::Text);

        }
        other => panic!("expected Command::Graph, got {other:?}"),
    }
}

#[test]
fn parse_simulate_defaults_to_text_output() {
    let parsed = parse_args(&args(&["velvet-ballistics", "simulate", "workflow.yaml"]));
    match parsed {
        Ok(Command::Simulate { workflow, output }) => {

            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(output, OutputFormat::Text);

        }
        other => panic!("expected Command::Simulate, got {other:?}"),
    }
}

#[test]
fn parse_simulate_legacy_json_flag_keeps_text_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "simulate",
        "workflow.yaml",
        "--json",
    ]));
    match parsed {
        Ok(Command::Simulate { output, .. }) => {

            assert_eq!(output, OutputFormat::Text);

        }
        other => panic!("expected Command::Simulate, got {other:?}"),
    }
}

#[test]
fn parse_simulate_legacy_jsonl_flag_keeps_text_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "simulate",
        "workflow.yaml",
        "--jsonl",
    ]));
    match parsed {
        Ok(Command::Simulate { output, .. }) => {

            assert_eq!(output, OutputFormat::Text);

        }
        other => panic!("expected Command::Simulate, got {other:?}"),
    }
}

#[test]
fn parse_simulate_accepts_emit_postcard() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "simulate",
        "workflow.yaml",
        "--emit",
        "postcard",
    ]));
    match parsed {
        Ok(Command::Simulate { output, .. }) => {

            assert_eq!(output, OutputFormat::Postcard);

        }
        other => panic!("expected Command::Simulate, got {other:?}"),
    }
}

#[test]
fn parse_bench_run_requires_workflow() {
    let parsed = parse_args(&args(&["velvet-ballistics", "bench-run", "workflow.yaml"]));
    match parsed {
        Ok(Command::BenchRun {
        workflow, output, ..
    }) => {

            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(output, OutputFormat::Text);

        }
        other => panic!("expected Command::BenchRun, got {other:?}"),
    }
}

#[test]
fn parse_bench_run_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "bench-run",
        "workflow.yaml",
        "--emit",
        "yaml",
    ]));
    match parsed {
        Ok(Command::BenchRun { output, .. }) => {

            assert_eq!(output, OutputFormat::Yaml);

        }
        other => panic!("expected Command::BenchRun, got {other:?}"),
    }
}

#[test]
fn parse_verify_handles_profile_before_workflow() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "verify",
        "--profile",
        "quick",
        "workflow.yaml",
    ]));
    match parsed {
        Ok(Command::Verify {
        workflow, profile, ..
    }) => {

            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(profile, VerifyProfile::Quick);

        }
        other => panic!("expected Command::Verify, got {other:?}"),
    }
}
