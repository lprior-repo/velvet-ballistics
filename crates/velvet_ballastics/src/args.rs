//! Argument parsing for velvet_ballastics.
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;

/// Structured output format for CLI commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum OutputFormat {
    /// Human-readable text output (default).
    #[default]
    Text,
    /// JSON object output.
    Json,
    /// JSON Lines output (one JSON object per line).
    Jsonl,
}

/// Verification profile controlling depth of static analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum VerifyProfile {
    /// Fast surface checks only.
    Quick,
    /// Default verification depth.
    #[default]
    Standard,
    /// Exhaustive verification including budget, capability, taint.
    Full,
}

impl VerifyProfile {
    /// Returns the name used on the command line for this profile.
    pub(crate) const fn as_str(&self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Standard => "standard",
            Self::Full => "full",
        }
    }
}

#[derive(Debug)]
pub(crate) enum Command {
    Help,
    Version,
    Verify {
        workflow: PathBuf,
        profile: VerifyProfile,
        output: OutputFormat,
    },
    Validate {
        workflow: PathBuf,
        #[allow(dead_code)]
        output: OutputFormat,
    },
    Compile {
        workflow: PathBuf,
        emit: EmitTarget,
        out: PathBuf,
        output: OutputFormat,
    },
    Run {
        workflow: PathBuf,
        input_bin: PathBuf,
        durability: DurabilityMode,
        db: Option<PathBuf>,
        step: Option<StepTarget>,
        output: OutputFormat,
    },
    RunCompiled {
        workflow: PathBuf,
        input_bin: PathBuf,
        durability: DurabilityMode,
        db: Option<PathBuf>,
        output: OutputFormat,
    },
    IpcServe {
        socket: PathBuf,
        db: PathBuf,
    },
    Inspect {
        run_id: String,
        db: PathBuf,
        output: OutputFormat,
    },
    Events {
        run_id: String,
        db: PathBuf,
        output: OutputFormat,
    },
    Replay {
        run_id: String,
        db: PathBuf,
        output: OutputFormat,
    },
    Trace {
        run_id: String,
        db: PathBuf,
        output: OutputFormat,
    },
    Retry {
        run_id: String,
        db: PathBuf,
        output: OutputFormat,
    },
    Resume {
        run_id: String,
        db: PathBuf,
        output: OutputFormat,
    },
    BenchRun {
        workflow: PathBuf,
        output: OutputFormat,
    },
    Doctor {
        db: PathBuf,
        output: OutputFormat,
    },
    Explain {
        workflow: PathBuf,
        #[allow(dead_code)]
        output: OutputFormat,
    },
    Answer {
        run_id: String,
        step: u16,
        value_file: PathBuf,
        db: PathBuf,
        output: OutputFormat,
    },
    Graph {
        workflow: PathBuf,
        output: OutputFormat,
    },
    Diff {
        run_a: String,
        run_b: String,
        db: PathBuf,
        output: OutputFormat,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EmitTarget {
    Ir,
    Rust,
    Yaml,
    Postcard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DurabilityMode {
    Strict,
    Journaled,
    None,
}

/// Single-step isolation target for `run --step`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepTarget {
    pub(crate) step_id: u16,
    pub(crate) step_input: PathBuf,
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) enum ParseError {
    MissingArgument(&'static str),
    UnknownEmitTarget(String),
    UnknownDurability(String),
    UnknownProfile(String),
    UnknownCommand(String),
    NoCommand,
    InvalidSlot(String),
}

pub(crate) fn parse_args(args: &[OsString]) -> Result<Command, ParseError> {
    let subcommand = args
        .get(1)
        .and_then(|s| s.to_str())
        .ok_or(ParseError::NoCommand)?;

    match subcommand {
        "help" | "--help" | "-h" => Ok(Command::Help),
        "version" | "--version" | "-V" => Ok(Command::Version),
        "verify" => parse_verify(args),
        "validate" => parse_validate(args),
        "explain" => parse_explain(args),
        "compile" => parse_compile(args),
        "run" => parse_run(args),
        "run-compiled" => parse_run_compiled(args),
        "ipc-serve" => parse_ipc_serve(args),
        "inspect" => parse_inspect(args),
        "events" => parse_events(args),
        "replay" => parse_replay(args),
        "trace" => parse_trace(args),
        "retry" => parse_retry(args),
        "resume" => parse_resume(args),
        "bench-run" => parse_bench_run(args),
        "doctor" => parse_doctor(args),
        "answer" => parse_answer(args),
        "graph" => parse_graph(args),
        "diff" => parse_diff(args),
        other => Err(ParseError::UnknownCommand(other.into())),
    }
}

fn parse_verify(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let profile = match named_flag(args, "--profile") {
        Some(raw) => match raw.as_str() {
            "quick" => VerifyProfile::Quick,
            "standard" => VerifyProfile::Standard,
            "full" => VerifyProfile::Full,
            other => return Err(ParseError::UnknownProfile(other.into())),
        },
        None => VerifyProfile::default(),
    };
    let output = parse_output_format(args);
    Ok(Command::Verify {
        workflow,
        profile,
        output,
    })
}

fn parse_validate(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let output = parse_output_format(args);
    Ok(Command::Validate { workflow, output })
}

fn parse_explain(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let output = parse_output_format(args);
    Ok(Command::Explain { workflow, output })
}

fn parse_compile(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let emit_raw = named_flag(args, "--emit").ok_or(ParseError::MissingArgument("--emit"))?;
    let emit = match emit_raw.as_str() {
        "ir" => EmitTarget::Ir,
        "rust" => EmitTarget::Rust,
        "yaml" => EmitTarget::Yaml,
        "postcard" => EmitTarget::Postcard,
        other => return Err(ParseError::UnknownEmitTarget(other.into())),
    };
    let out = named_flag(args, "--out").ok_or(ParseError::MissingArgument("--out"))?;
    let output = parse_output_format(args);
    Ok(Command::Compile {
        workflow,
        emit,
        out: PathBuf::from(out),
        output,
    })
}

fn parse_run(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let input_bin =
        named_flag(args, "--input-bin").ok_or(ParseError::MissingArgument("--input-bin"))?;
    let durability_raw =
        named_flag(args, "--durability").ok_or(ParseError::MissingArgument("--durability"))?;
    let durability = parse_durability(&durability_raw)?;
    let db = parse_optional_run_db(args, durability)?;
    let step = parse_optional_step(args)?;
    let output = parse_output_format(args);
    Ok(Command::Run {
        workflow,
        input_bin: PathBuf::from(input_bin),
        durability,
        db,
        step,
        output,
    })
}

fn parse_optional_step(args: &[OsString]) -> Result<Option<StepTarget>, ParseError> {
    let step_raw = match named_flag(args, "--step") {
        Some(s) => s,
        None => return Ok(None),
    };
    let step_id = step_raw
        .parse::<u16>()
        .map_err(|_| ParseError::MissingArgument("--step"))?;
    let step_input =
        named_flag(args, "--step-input").ok_or(ParseError::MissingArgument("--step-input"))?;
    Ok(Some(StepTarget {
        step_id,
        step_input: PathBuf::from(step_input),
    }))
}

fn parse_run_compiled(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.vbir")?;
    let input_bin =
        named_flag(args, "--input-bin").ok_or(ParseError::MissingArgument("--input-bin"))?;
    let durability_raw =
        named_flag(args, "--durability").ok_or(ParseError::MissingArgument("--durability"))?;
    let durability = parse_durability(&durability_raw)?;
    let db = parse_optional_run_db(args, durability)?;
    let output = parse_output_format(args);
    Ok(Command::RunCompiled {
        workflow,
        input_bin: PathBuf::from(input_bin),
        durability,
        db,
        output,
    })
}

fn parse_optional_run_db(
    args: &[OsString],
    durability: DurabilityMode,
) -> Result<Option<PathBuf>, ParseError> {
    let db = named_flag(args, "--db").map(PathBuf::from);
    if durability == DurabilityMode::None {
        return Ok(db);
    }
    match db {
        Some(path) => Ok(Some(path)),
        None => Err(ParseError::MissingArgument("--db")),
    }
}

fn parse_ipc_serve(args: &[OsString]) -> Result<Command, ParseError> {
    let socket = named_flag(args, "--socket").ok_or(ParseError::MissingArgument("--socket"))?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    Ok(Command::IpcServe {
        socket: PathBuf::from(socket),
        db: PathBuf::from(db),
    })
}

fn parse_inspect(args: &[OsString]) -> Result<Command, ParseError> {
    let run_id = positional_str(args, 2, "run_id")?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let output = parse_output_format(args);
    Ok(Command::Inspect {
        run_id,
        db: PathBuf::from(db),
        output,
    })
}

fn parse_events(args: &[OsString]) -> Result<Command, ParseError> {
    let run_id = positional_str(args, 2, "run_id")?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let output = parse_output_format(args);
    Ok(Command::Events {
        run_id,
        db: PathBuf::from(db),
        output,
    })
}

fn parse_replay(args: &[OsString]) -> Result<Command, ParseError> {
    let run_id = positional_str(args, 2, "run_id")?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let output = parse_output_format(args);
    Ok(Command::Replay {
        run_id,
        db: PathBuf::from(db),
        output,
    })
}


fn parse_trace(args: &[OsString]) -> Result<Command, ParseError> {
    let run_id = positional_str(args, 2, "run_id")?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let output = parse_output_format(args);
    Ok(Command::Trace {
        run_id,
        db: PathBuf::from(db),
        output,
    })
}

fn parse_retry(args: &[OsString]) -> Result<Command, ParseError> {
    let run_id = positional_str(args, 2, "run_id")?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let output = parse_output_format(args);
    Ok(Command::Retry {
        run_id,
        db: PathBuf::from(db),
        output,
    })
}

fn parse_resume(args: &[OsString]) -> Result<Command, ParseError> {
    let run_id = positional_str(args, 2, "run_id")?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let output = parse_output_format(args);
    Ok(Command::Resume {
        run_id,
        db: PathBuf::from(db),
        output,
    })
}

fn parse_bench_run(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let output = parse_output_format(args);
    Ok(Command::BenchRun { workflow, output })
}

fn parse_doctor(args: &[OsString]) -> Result<Command, ParseError> {
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let output = parse_output_format(args);
    Ok(Command::Doctor {
        db: PathBuf::from(db),
        output,
    })
}

fn parse_answer(args: &[OsString]) -> Result<Command, ParseError> {
    let run_id = positional_str(args, 2, "run_id")?;
    let step_raw = named_flag(args, "--step").ok_or(ParseError::MissingArgument("--step"))?;
    let step = step_raw
        .parse::<u16>()
        .map_err(|_| ParseError::InvalidSlot(step_raw))?;
    let value_file =
        named_flag(args, "--value-file").ok_or(ParseError::MissingArgument("--value-file"))?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let output = parse_output_format(args);
    Ok(Command::Answer {
        run_id,
        step,
        value_file: PathBuf::from(value_file),
        db: PathBuf::from(db),
        output,
    })
}

fn parse_graph(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let output = parse_output_format(args);
    Ok(Command::Graph { workflow, output })
}

fn parse_diff(args: &[OsString]) -> Result<Command, ParseError> {
    let run_a = positional_str(args, 2, "run_a")?;
    let run_b = positional_str(args, 3, "run_b")?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let output = parse_output_format(args);
    Ok(Command::Diff {
        run_a,
        run_b,
        db: PathBuf::from(db),
        output,
    })
}

fn parse_durability(raw: &str) -> Result<DurabilityMode, ParseError> {
    match raw {
        "strict" => Ok(DurabilityMode::Strict),
        "journaled" => Ok(DurabilityMode::Journaled),
        "none" => Ok(DurabilityMode::None),
        other => Err(ParseError::UnknownDurability(other.into())),
    }
}

/// Parse --json or --jsonl output format flags.
/// Returns OutputFormat::Text by default.
fn parse_output_format(args: &[OsString]) -> OutputFormat {
    if contains_flag(args, "--jsonl") {
        OutputFormat::Jsonl
    } else if contains_flag(args, "--json") {
        OutputFormat::Json
    } else {
        OutputFormat::Text
    }
}

/// Check if args contain a specific flag.
fn contains_flag(args: &[OsString], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn positional(args: &[OsString], index: usize, name: &'static str) -> Result<PathBuf, ParseError> {
    args.get(index)
        .and_then(|s| s.to_str())
        .map(PathBuf::from)
        .ok_or(ParseError::MissingArgument(name))
}

fn positional_str(
    args: &[OsString],
    index: usize,
    name: &'static str,
) -> Result<String, ParseError> {
    args.get(index)
        .and_then(|s| s.to_str())
        .map(String::from)
        .ok_or(ParseError::MissingArgument(name))
}

fn named_flag(args: &[OsString], flag: &str) -> Option<String> {
    for (i, arg) in args.iter().enumerate() {
        if arg == flag {
            return args
                .get(i.checked_add(1)?)
                .and_then(|v| v.to_str())
                .map(String::from);
        }
    }
    None
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingArgument(name) => write!(formatter, "missing argument: {name}"),
            Self::UnknownEmitTarget(target) => {
                write!(formatter, "unknown emit target: {target} (expected: ir, rust, yaml, postcard)")
            }
            Self::UnknownDurability(mode) => {
                write!(
                    formatter,
                    "unknown durability mode: {mode} (expected: strict, journaled, none)"
                )
            }
            Self::UnknownProfile(profile) => {
                write!(
                    formatter,
                    "unknown verify profile: {profile} (expected: quick, standard, full)"
                )
            }
            Self::UnknownCommand(cmd) => write!(formatter, "unknown command: {cmd}"),
            Self::NoCommand => write!(formatter, "no command provided"),
            Self::InvalidSlot(slot) => write!(formatter, "invalid slot: {slot}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_args, Command, DurabilityMode, EmitTarget, OutputFormat, ParseError, StepTarget, VerifyProfile};
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn args(parts: &[&str]) -> Vec<OsString> {
        parts.iter().map(|part| OsString::from(*part)).collect()
    }

    #[test]
    fn parse_run_accepts_db_for_journaled_mode() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
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
            "velvet-ballastics",
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
            "velvet-ballastics",
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
            "velvet-ballastics",
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
            "velvet-ballastics",
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
    fn parse_run_with_step_flags() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
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
        if let Ok(Command::Run { step, .. }) = parsed {
            assert!(step.is_some());
            let target = step.expect("step target");
            assert_eq!(target.step_id, 3);
            assert_eq!(target.step_input, PathBuf::from("step-data.bin"));
        }
    }

    #[test]
    fn parse_inspect_includes_output_format() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics",
            "inspect",
            "42",
            "--db",
            "test-db",
            "--json",
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
            assert_eq!(output, OutputFormat::Json);
        }
    }

    #[test]
    fn parse_help_command() {
        let parsed = parse_args(&args(&["velvet-ballastics", "help"]));
        assert!(matches!(parsed, Ok(Command::Help)));
    }

    #[test]
    fn parse_version_command() {
        let parsed = parse_args(&args(&["velvet-ballastics", "--version"]));
        assert!(matches!(parsed, Ok(Command::Version)));
    }

    #[test]
    fn parse_no_command_returns_error() {
        let parsed = parse_args(&args(&["velvet-ballastics"]));
        assert!(matches!(parsed, Err(ParseError::NoCommand)));
    }

    #[test]
    fn parse_unknown_command_returns_error() {
        let parsed = parse_args(&args(&["velvet-ballastics", "foobar"]));
        assert!(matches!(parsed, Err(ParseError::UnknownCommand(_))));
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
        if let Ok(Command::Verify { profile, output, .. }) = parsed {
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
}
