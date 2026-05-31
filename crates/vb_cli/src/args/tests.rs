
use super::{
    ActionRegistryMode, Command, DurabilityMode, EmitTarget, OutputFormat, ParseError, StepTarget,
    VerifyProfile, parse_args,
};
use crate::commands_journal::TraceStatus;
use std::ffi::OsString;
use std::path::PathBuf;

fn args(parts: &[&str]) -> Vec<OsString> {
    parts.iter().map(|part| OsString::from(*part)).collect()
}

#[test]
fn parse_run_accepts_db_for_journaled_mode() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "journaled",
        "--db",
        "journal-db",
    ]));

    assert!(
        matches!(parsed, Ok(Command::Run { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Run {
        workflow,
        input_bin,
        durability,
        db,
        ..
    }) = parsed
    {
        assert_eq!(workflow, PathBuf::from("workflow.yaml"));
        assert_eq!(input_bin, PathBuf::from("input.bin"));
        assert_eq!(durability, DurabilityMode::Journaled);
        assert_eq!(db, Some(PathBuf::from("journal-db")));
    }
}

#[test]
fn parse_run_compiled_requires_db_for_strict_mode() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run-compiled",
        "workflow.vbir",
        "--input-bin",
        "input.bin",
        "--durability",
        "strict",
    ]));

    assert!(
        matches!(parsed, Err(ParseError::MissingArgument("--db"))),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_run_none_mode_keeps_db_optional() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "none",
    ]));

    assert!(
        matches!(parsed, Ok(Command::Run { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Run { durability, db, .. }) = parsed {
        assert_eq!(durability, DurabilityMode::None);
        assert_eq!(db, None);
    }
}

#[test]
fn parse_run_without_step_flags_produces_none_step() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "none",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Run { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Run { step, .. }) = parsed {
        assert!(step.is_none());
    }
}

#[test]
fn parse_run_step_requires_step_input() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "none",
        "--step",
        "0",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::MissingArgument("--step-input"))),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn step_target_holds_step_id_and_path() {
    let target = StepTarget {
        step_id: 5,
        step_input: PathBuf::from("data.bin"),
    };
    assert_eq!(target.step_id, 5);
    assert_eq!(target.step_input, PathBuf::from("data.bin"));
}

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
fn parse_run_with_step_flags() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "none",
        "--step",
        "3",
        "--step-input",
        "step-data.bin",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Run { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Run {
        step: Some(target), ..
    }) = parsed
    {
        assert_eq!(target.step_id, 3);
        assert_eq!(target.step_input, PathBuf::from("step-data.bin"));
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

#[test]
fn parse_run_rejects_unknown_durability_with_exact_variant() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "run",
        "workflow.yaml",
        "--input-bin",
        "input.bin",
        "--durability",
        "ephemeral",
    ]));

    assert!(
        matches!(parsed, Err(ParseError::UnknownDurability(ref m)) if m == "ephemeral"),
        "expected UnknownDurability(ephemeral), got {parsed:?}"
    );
}

#[test]
fn parse_answer_rejects_invalid_step_with_exact_variant() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "answer",
        "run-1",
        "--step",
        "not-a-step",
        "--value-file",
        "value.bin",
        "--db",
        "test-db",
    ]));

    assert!(
        matches!(parsed, Err(ParseError::InvalidStep(ref s)) if s == "not-a-step"),
        "expected InvalidStep(not-a-step), got {parsed:?}"
    );
}

#[test]
fn parse_inspect_includes_output_format() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "inspect",
        "42",
        "--db",
        "test-db",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Inspect { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Inspect {
        run_id, db, output, ..
    }) = parsed
    {
        assert_eq!(run_id, "42");
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Yaml);
    }
}

#[test]
fn parse_help_command() {
    let parsed = parse_args(&args(&["velvet-ballistics", "help"]));
    assert!(matches!(parsed, Ok(Command::Help)));
}

#[test]
fn parse_version_command() {
    let parsed = parse_args(&args(&["velvet-ballistics", "--version"]));
    assert!(matches!(parsed, Ok(Command::Version)));
}

#[test]
fn parse_agent_context_command() {
    let parsed = parse_args(&args(&["velvet-ballistics", "agent-context"]));
    assert!(matches!(
        parsed,
        Ok(Command::AgentContext { deliver: None })
    ));
}

#[test]
fn parse_agent_context_deliver_target() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "agent-context",
        "--deliver",
        "file:/tmp/out.jsonl",
    ]));
    assert!(
        matches!(parsed, Ok(Command::AgentContext { deliver: Some(ref target) }) if target == "file:/tmp/out.jsonl")
    );
}

#[test]
fn parse_agent_context_rejects_missing_deliver_target() {
    let parsed = parse_args(&args(&["velvet-ballistics", "agent-context", "--deliver"]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidAgentContextArgument(ref reason)) if reason == "--deliver requires stdout or file:<absolute-path>")
    );
}

#[test]
fn parse_agent_context_rejects_unknown_flag() {
    let parsed = parse_args(&args(&["velvet-ballistics", "agent-context", "--bogus"]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidAgentContextArgument(ref reason)) if reason == "unknown flag --bogus")
    );
}

#[test]
fn parse_trace_defaults_to_no_filters() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "trace",
        "7",
        "--db",
        "journal-db",
    ]));

    assert!(matches!(parsed, Ok(Command::Trace { .. })));
    if let Ok(Command::Trace {
        run_id,
        db,
        output,
        filters,
    }) = parsed
    {
        assert_eq!(run_id, "7");
        assert_eq!(db, PathBuf::from("journal-db"));
        assert_eq!(output, OutputFormat::Text);
        assert_eq!(filters.step, None);
        assert_eq!(filters.action, None);
        assert_eq!(filters.status, None);
        assert_eq!(filters.since_seq, None);
        assert_eq!(filters.until_seq, None);
        assert_eq!(filters.limit, None);
    }
}

#[test]
fn parse_trace_accepts_all_filters() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "trace",
        "7",
        "--db",
        "journal-db",
        "--step",
        "4",
        "--action",
        "9",
        "--status",
        "active",
        "--since-seq",
        "10",
        "--until-seq",
        "20",
        "--limit",
        "3",
        "--emit",
        "yaml",
    ]));

    assert!(matches!(parsed, Ok(Command::Trace { .. })));
    if let Ok(Command::Trace {
        output, filters, ..
    }) = parsed
    {
        assert_eq!(output, OutputFormat::Yaml);
        assert_eq!(filters.step, Some(4));
        assert_eq!(filters.action, Some(9));
        assert_eq!(filters.status, Some(TraceStatus::Active));
        assert_eq!(filters.since_seq, Some(10));
        assert_eq!(filters.until_seq, Some(20));
        assert_eq!(filters.limit, Some(3));
    }
}

#[test]
fn parse_trace_rejects_invalid_step() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "trace",
        "7",
        "--db",
        "journal-db",
        "--step",
        "not-a-step",
    ]));

    assert!(
        matches!(parsed, Err(ParseError::InvalidTraceArgument(ref reason)) if reason == "--step must be a valid u16"),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_trace_rejects_invalid_since_seq() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "trace",
        "7",
        "--db",
        "journal-db",
        "--since-seq",
        "not-a-seq",
    ]));

    assert!(
        matches!(parsed, Err(ParseError::InvalidTraceArgument(ref reason)) if reason == "--since-seq must be a valid u64"),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_trace_rejects_missing_until_seq_value() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "trace",
        "7",
        "--db",
        "journal-db",
        "--until-seq",
        "--emit",
        "yaml",
    ]));

    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--until-seq"))
    ));
}

#[test]
fn parse_trace_rejects_missing_limit_value() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "trace",
        "7",
        "--db",
        "journal-db",
        "--limit",
        "--emit",
        "yaml",
    ]));

    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--limit"))
    ));
}

#[test]
fn parse_trace_rejects_unknown_filter_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "trace",
        "7",
        "--db",
        "journal-db",
        "--severity",
        "error",
    ]));

    assert!(
        matches!(parsed, Err(ParseError::InvalidTraceArgument(ref reason)) if reason == "unknown trace flag: --severity"),
        "unexpected parse result: {parsed:?}"
    );
}

#[test]
fn parse_status_accepts_no_runtime_defaults() {
    let parsed = parse_args(&args(&["velvet-ballistics", "status", "--emit", "yaml"]));
    assert!(matches!(parsed, Ok(Command::Status { .. })));
    if let Ok(Command::Status { options, output }) = parsed {
        assert_eq!(options.active_runs, None);
        assert_eq!(options.queue_depth, None);
        assert_eq!(options.trace_dropped, None);
        assert_eq!(output, OutputFormat::Yaml);
    }
}

#[test]
fn parse_status_accepts_diagnostic_counters() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "status",
        "--active-runs",
        "5",
        "--queue-depth",
        "3",
        "--trace-dropped",
        "0",
    ]));
    assert!(matches!(parsed, Ok(Command::Status { .. })));
    if let Ok(Command::Status { options, output }) = parsed {
        assert_eq!(options.active_runs, Some(5));
        assert_eq!(options.queue_depth, Some(3));
        assert_eq!(options.trace_dropped, Some(0));
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_status_rejects_invalid_numeric_argument() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "status",
        "--queue-depth",
        "many",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "--queue-depth must be a usize"),
        "expected InvalidStatusArgument(--queue-depth must be a usize), got {parsed:?}"
    );
}

#[test]
fn parse_status_rejects_missing_queue_depth_value() {
    let parsed = parse_args(&args(&["velvet-ballistics", "status", "--queue-depth"]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--queue-depth"))
    ));
}

#[test]
fn parse_status_rejects_missing_active_runs_value() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "status",
        "--active-runs",
        "--emit",
        "yaml",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--active-runs"))
    ));
}

#[test]
fn parse_status_rejects_missing_trace_dropped_value() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "status",
        "--trace-dropped",
        "--queue-depth",
        "1",
    ]));
    assert!(matches!(
        parsed,
        Err(ParseError::MissingArgument("--trace-dropped"))
    ));
}

#[test]
fn parse_status_rejects_unknown_flag() {
    let parsed = parse_args(&args(&["velvet-ballistics", "status", "--bogus"]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "unknown flag --bogus"),
        "expected InvalidStatusArgument(unknown flag --bogus), got {parsed:?}"
    );
}

#[test]
fn parse_status_rejects_extra_positional_argument() {
    let parsed = parse_args(&args(&["velvet-ballistics", "status", "extra"]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "unexpected positional argument extra"),
        "expected InvalidStatusArgument(unexpected positional argument extra), got {parsed:?}"
    );
}

#[test]
fn parse_status_rejects_out_of_range_queue_depth() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "status",
        "--queue-depth",
        "1025",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "--queue-depth must be <= 1024"),
        "expected InvalidStatusArgument(--queue-depth must be <= 1024), got {parsed:?}"
    );
}

#[test]
fn parse_status_rejects_out_of_range_active_runs() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "status",
        "--active-runs",
        "1025",
    ]));
    assert!(
        matches!(parsed, Err(ParseError::InvalidStatusArgument(ref s)) if s == "--active-runs must be <= 1024"),
        "expected InvalidStatusArgument(--active-runs must be <= 1024), got {parsed:?}"
    );
}

#[test]
fn parse_system_status_defaults_to_standard_none_text() {
    let parsed = parse_args(&args(&["velvet-ballistics", "system", "status"]));
    assert!(matches!(parsed, Ok(Command::SystemStatus { .. })));
    if let Ok(Command::SystemStatus { options, output }) = parsed {
        assert_eq!(options.profile, VerifyProfile::Standard);
        assert_eq!(options.server, DurabilityMode::None);
        assert!(!options.emit_yaml);
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_system_status_accepts_profile_server_and_emit_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "system",
        "status",
        "--profile",
        "full",
        "--server",
        "none",
        "--emit",
        "yaml",
    ]));
    assert!(matches!(parsed, Ok(Command::SystemStatus { .. })));
    if let Ok(Command::SystemStatus { options, output }) = parsed {
        assert_eq!(options.profile, VerifyProfile::Full);
        assert_eq!(options.server, DurabilityMode::None);
        assert!(options.emit_yaml);
        assert_eq!(output, OutputFormat::Yaml);
    }
}

#[test]
fn parse_system_status_rejects_unknown_profile() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "system",
        "status",
        "--profile",
        "deep",
    ]));
    assert!(matches!(parsed, Err(ParseError::UnknownProfile(ref p)) if p == "deep"));
}

#[test]
fn parse_system_status_rejects_unknown_server_mode() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "system",
        "status",
        "--server",
        "remote",
    ]));
    assert!(matches!(parsed, Err(ParseError::UnknownServerMode(ref m)) if m == "remote"));
}

#[test]
fn parse_system_status_rejects_unprobed_server_mode() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "system",
        "status",
        "--server",
        "strict",
    ]));
    assert!(matches!(parsed, Err(ParseError::UnknownServerMode(ref m)) if m == "strict"));
}

#[test]
fn parse_no_command_returns_error() {
    let parsed = parse_args(&args(&["velvet-ballistics"]));
    assert!(matches!(parsed, Err(ParseError::NoCommand)));
}

#[test]
fn parse_unknown_command_returns_error() {
    let parsed = parse_args(&args(&["velvet-ballistics", "foobar"]));
    assert!(matches!(parsed, Err(ParseError::UnknownCommand(_))));
}

#[test]
fn unknown_command_error_enumerates_valid_commands() {
    let err = ParseError::UnknownCommand(String::from("foobar"));
    let rendered = err.to_string();

    assert!(rendered.contains("expected one of"));
    assert!(rendered.contains("agent-context"));
}

#[test]
fn parse_verify_defaults_to_standard_profile() {
    let parsed = parse_args(&args(&["velvet-ballistics", "verify", "workflow.yaml"]));
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
        "velvet-ballistics",
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
fn parse_verify_accepts_full_profile_with_yaml() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "verify",
        "workflow.yaml",
        "--profile",
        "full",
        "--emit",
        "yaml",
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
        assert_eq!(output, OutputFormat::Yaml);
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
fn parse_graph_accepts_yaml_emit() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "graph",
        "workflow.yaml",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Graph { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Graph { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    }
}

#[test]
fn parse_diff_requires_both_run_ids_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "diff",
        "1",
        "2",
        "--db",
        "test-db",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Diff { .. })),
        "unexpected: {parsed:?}"
    );
    if let Ok(Command::Diff {
        run_a,
        run_b,
        db,
        output,
    }) = parsed
    {
        assert_eq!(run_a, "1");
        assert_eq!(run_b, "2");
        assert_eq!(db, PathBuf::from("test-db"));
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_diff_accepts_json_flag() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "diff",
        "10",
        "20",
        "--db",
        "test-db",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Diff { .. })),
        "unexpected: {parsed:?}"
    );
    if let Ok(Command::Diff { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    }
}

#[test]
fn parse_diff_requires_db_flag() {
    let parsed = parse_args(&args(&["velvet-ballistics", "diff", "1", "2"]));
    assert!(
        matches!(parsed, Err(ParseError::MissingArgument("--db"))),
        "unexpected: {parsed:?}"
    );
}

#[test]
fn parse_simulate_defaults_to_text_output() {
    let parsed = parse_args(&args(&["velvet-ballistics", "simulate", "workflow.yaml"]));
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
fn parse_simulate_accepts_yaml_emit() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "simulate",
        "workflow.yaml",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Simulate { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Simulate { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    }
}

#[test]
fn parse_simulate_accepts_postcard_emit() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "simulate",
        "workflow.yaml",
        "--emit",
        "postcard",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Simulate { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Simulate { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Postcard);
    }
}

#[test]
fn parse_doctor_without_db_is_stateless_text_mode() {
    let parsed = parse_args(&args(&["velvet-ballistics", "doctor"]));
    assert!(
        matches!(parsed, Ok(Command::Doctor { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Doctor { db, output }) = parsed {
        assert_eq!(db, None);
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_doctor_accepts_optional_db_and_yaml_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "doctor",
        "--db",
        "journal-db",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Doctor { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Doctor { db, output }) = parsed {
        assert_eq!(db, Some(PathBuf::from("journal-db")));
        assert_eq!(output, OutputFormat::Yaml);
    }
}

#[test]
fn parse_action_list_accepts_yaml_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "action",
        "list",
        "--emit",
        "yaml",
    ]));
    assert!(
        matches!(parsed, Ok(Command::ActionList { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::ActionList { output, registry }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
        assert_eq!(registry, ActionRegistryMode::Registered);
    }
}

// --- Cancel command parsing tests ---

#[test]
fn parse_cancel_accepts_run_id_and_db() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "cancel",
        "42",
        "--db",
        "journal-db",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Cancel { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Cancel {
        run_id,
        db,
        reason,
        output,
    }) = parsed
    {
        assert_eq!(run_id, "42");
        assert_eq!(db, PathBuf::from("journal-db"));
        assert_eq!(reason, None);
        assert_eq!(output, OutputFormat::Text);
    }
}

#[test]
fn parse_cancel_accepts_reason() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--reason",
        "user request",
    ]));
    assert!(
        matches!(parsed, Ok(Command::Cancel { .. })),
        "unexpected parse result: {parsed:?}"
    );
    if let Ok(Command::Cancel { reason, .. }) = parsed {
        assert_eq!(reason, Some("user request".to_string()));
    }
}

#[test]
fn parse_cancel_accepts_yaml_output() {
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--emit",
        "yaml",
    ]));
    if let Ok(Command::Cancel { output, .. }) = parsed {
        assert_eq!(output, OutputFormat::Yaml);
    } else {
        assert!(parsed.is_ok(), "expected Ok, got {parsed:?}");
    }
}

#[test]
fn parse_cancel_rejects_missing_db() {
    let parsed = parse_args(&args(&["velvet-ballistics", "cancel", "42"]));
    assert!(
        matches!(parsed, Err(ParseError::MissingArgument("--db"))),
        "unexpected: {parsed:?}"
    );
}

#[test]
fn parse_cancel_rejects_reason_longer_than_256_bytes() {
    let long_reason = "a".repeat(257);
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--reason",
        &long_reason,
    ]));
    assert!(
        matches!(parsed, Err(ParseError::ReasonTooLong)),
        "unexpected: {parsed:?}"
    );
}

#[test]
fn parse_cancel_accepts_reason_exactly_256_bytes() {
    let reason = "a".repeat(256);
    let parsed = parse_args(&args(&[
        "velvet-ballistics",
        "cancel",
        "42",
        "--db",
        "journal-db",
        "--reason",
        &reason,
    ]));
    assert!(
        matches!(parsed, Ok(Command::Cancel { .. })),
        "unexpected: {parsed:?}"
    );
}
