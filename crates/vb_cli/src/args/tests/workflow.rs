use std::path::PathBuf;

use super::args;
use crate::args::{Command, EmitTarget, OutputFormat, ParseError, VerifyProfile, parse_args};

#[test]
fn parse_validate_requires_workflow() {
    let parsed = parse_args(&args(&["velvet-ballastics", "validate", "workflow.yaml"]));
    if let Ok(Command::Validate { workflow, output }) = parsed {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_validate_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "validate",
        "workflow.yaml",
        "--emit",
        "yaml",
    ]));
    if let Ok(Command::Validate { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_validate_accepts_emit_postcard() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "validate",
        "workflow.yaml",
        "--emit",
        "postcard",
    ]));
    if let Ok(Command::Validate { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Postcard);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_validate_rejects_unknown_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "validate",
        "workflow.yaml",
        "--bogus",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::UnknownFlag { command: "validate", .. })
    ));
}

#[test]
fn parse_explain_requires_workflow() {
    let parsed = parse_args(&args(&["velvet-ballastics", "explain", "workflow.yaml"]));
    if let Ok(Command::Explain { workflow, output }) = parsed {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_explain_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "explain",
        "workflow.yaml",
        "--emit",
        "yaml",
    ]));
    if let Ok(Command::Explain { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
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
    if let Ok(Command::Explain { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Jsonl);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_compile_requires_emit_and_out() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "compile",
        "workflow.yaml",
        "--emit",
        "ir",
        "--out",
        "output.vbir",
    ]));
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
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_compile_accepts_emit_postcard() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "compile",
        "workflow.yaml",
        "--emit",
        "postcard",
        "--out",
        "workflow.vbpc",
    ]));
    if let Ok(Command::Compile { emit, .. }) = parsed {
        assert_eq!(emit, EmitTarget::Postcard);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_compile_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "compile",
        "workflow.yaml",
        "--emit",
        "yaml",
        "--out",
        "workflow.out.yaml",
    ]));
    if let Ok(Command::Compile { emit, output, .. }) = parsed {
        assert_eq!(emit, EmitTarget::Yaml);
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_compile_accepts_legacy_json_flag() {
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
    if let Ok(Command::Compile { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Json);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_compile_accepts_legacy_jsonl_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "compile",
        "workflow.yaml",
        "--emit",
        "ir",
        "--out",
        "output.vbir",
        "--jsonl",
    ]));
    if let Ok(Command::Compile { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Jsonl);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
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
fn parse_compile_rejects_missing_emit() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "compile",
        "workflow.yaml",
        "--out",
        "output.vbir",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--emit"))
    ));
}

#[test]
fn parse_compile_rejects_missing_out() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "compile",
        "workflow.yaml",
        "--emit",
        "ir",
    ]));
    assert!(matches!(parsed, Err(ParseError::MissingArgument("--out"))));
}

#[test]
fn parse_verify_defaults_to_standard_profile() {
    let parsed = parse_args(&args(&["velvet-ballastics", "verify", "workflow.yaml"]));
    if let Ok(Command::Verify {
        workflow,
        profile,
        output,
    }) = parsed
    {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(profile, VerifyProfile::Standard);
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
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
    if let Ok(Command::Verify { profile, .. }) = parsed {
        assert_eq!(profile, VerifyProfile::Quick);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_verify_accepts_full_profile_with_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "verify",
        "workflow.yaml",
        "--profile",
        "full",
        "--emit",
        "yaml",
    ]));
    if let Ok(Command::Verify {
        profile, output, ..
    }) = parsed
    {
        assert_eq!(profile, VerifyProfile::Full);
        assert_eq!(output, OutputFormat::Yaml);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
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
    if let Ok(Command::Graph { workflow, output }) = parsed {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_graph_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "graph",
        "workflow.yaml",
        "--emit",
        "yaml",
    ]));
    if let Ok(Command::Graph { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
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
    if let Ok(Command::Graph { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Json);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_simulate_defaults_to_text_output() {
    let parsed = parse_args(&args(&["velvet-ballastics", "simulate", "workflow.yaml"]));
    if let Ok(Command::Simulate { workflow, output }) = parsed {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
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
    if let Ok(Command::Simulate { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Json);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
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
    if let Ok(Command::Simulate { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Jsonl);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_simulate_accepts_emit_postcard() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "simulate",
        "workflow.yaml",
        "--emit",
        "postcard",
    ]));
    if let Ok(Command::Simulate { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Postcard);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_bench_run_requires_workflow() {
    let parsed = parse_args(&args(&["velvet-ballastics", "bench-run", "workflow.yaml"]));
    if let Ok(Command::BenchRun {
        workflow, output, ..
    }) = parsed
    {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(output, OutputFormat::Text);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_bench_run_accepts_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "bench-run",
        "workflow.yaml",
        "--emit",
        "yaml",
    ]));
    if let Ok(Command::BenchRun { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_verify_handles_profile_before_workflow() {
    let parsed = parse_args(&args(&[
        "velvet-ballastics",
        "verify",
        "--profile",
        "quick",
        "workflow.yaml",
    ]));
    if let Ok(Command::Verify {
        workflow,
        profile,
        ..
    }) = parsed
    {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(profile, VerifyProfile::Quick);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}
