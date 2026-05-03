//! Argument parsing for velvet_ballastics.
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Command {
    Help,
    Version,
    Validate { workflow: PathBuf },
    Compile {
        workflow: PathBuf,
        emit: EmitTarget,
        out: PathBuf,
    },
    Run {
        workflow: PathBuf,
        input_bin: PathBuf,
        durability: DurabilityMode,
        db: Option<PathBuf>,
    },
    RunCompiled {
        workflow: PathBuf,
        input_bin: PathBuf,
        durability: DurabilityMode,
        db: Option<PathBuf>,
    },
    IpcServe { socket: PathBuf, db: PathBuf },
    Inspect { run_id: String, db: PathBuf },
    Events { run_id: String, db: PathBuf },
    Replay { run_id: String, db: PathBuf },
    BenchRun { workflow: PathBuf },
    Doctor { db: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmitTarget {
    Ir,
    Rust,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurabilityMode {
    Strict,
    Journaled,
    None,
}

#[derive(Debug)]
pub enum ParseError {
    MissingArgument(&'static str),
    UnknownEmitTarget(String),
    UnknownDurability(String),
    UnknownCommand(String),
    NoCommand,
}

pub fn parse_args(args: &[OsString]) -> Result<Command, ParseError> {
    let subcommand = args.get(1).and_then(|s| s.to_str()).ok_or(ParseError::NoCommand)?;

    match subcommand {
        "help" | "--help" | "-h" => Ok(Command::Help),
        "version" | "--version" | "-V" => Ok(Command::Version),
        "validate" => parse_validate(args),
        "compile" => parse_compile(args),
        "run" => parse_run(args),
        "run-compiled" => parse_run_compiled(args),
        "ipc-serve" => parse_ipc_serve(args),
        "inspect" => parse_inspect(args),
        "events" => parse_events(args),
        "replay" => parse_replay(args),
        "bench-run" => parse_bench_run(args),
        "doctor" => parse_doctor(args),
        other => Err(ParseError::UnknownCommand(other.into())),
    }
}

fn parse_validate(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    Ok(Command::Validate { workflow })
}

fn parse_compile(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let emit_raw = named_flag(args, "--emit").ok_or(ParseError::MissingArgument("--emit"))?;
    let emit = match emit_raw.as_str() {
        "ir" => EmitTarget::Ir,
        "rust" => EmitTarget::Rust,
        other => return Err(ParseError::UnknownEmitTarget(other.into())),
    };
    let out = named_flag(args, "--out").ok_or(ParseError::MissingArgument("--out"))?;
    Ok(Command::Compile {
        workflow,
        emit,
        out: PathBuf::from(out),
    })
}

fn parse_run(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let input_bin = named_flag(args, "--input-bin").ok_or(ParseError::MissingArgument("--input-bin"))?;
    let durability_raw = named_flag(args, "--durability").ok_or(ParseError::MissingArgument("--durability"))?;
    let durability = parse_durability(&durability_raw)?;
    let db = parse_optional_run_db(args, durability)?;
    Ok(Command::Run {
        workflow,
        input_bin: PathBuf::from(input_bin),
        durability,
        db,
    })
}

fn parse_run_compiled(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.vbir")?;
    let input_bin = named_flag(args, "--input-bin").ok_or(ParseError::MissingArgument("--input-bin"))?;
    let durability_raw = named_flag(args, "--durability").ok_or(ParseError::MissingArgument("--durability"))?;
    let durability = parse_durability(&durability_raw)?;
    let db = parse_optional_run_db(args, durability)?;
    Ok(Command::RunCompiled {
        workflow,
        input_bin: PathBuf::from(input_bin),
        durability,
        db,
    })
}

fn parse_optional_run_db(args: &[OsString], durability: DurabilityMode) -> Result<Option<PathBuf>, ParseError> {
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
    Ok(Command::Inspect {
        run_id,
        db: PathBuf::from(db),
    })
}

fn parse_events(args: &[OsString]) -> Result<Command, ParseError> {
    let run_id = positional_str(args, 2, "run_id")?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    Ok(Command::Events {
        run_id,
        db: PathBuf::from(db),
    })
}

fn parse_replay(args: &[OsString]) -> Result<Command, ParseError> {
    let run_id = positional_str(args, 2, "run_id")?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    Ok(Command::Replay {
        run_id,
        db: PathBuf::from(db),
    })
}

fn parse_bench_run(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    Ok(Command::BenchRun { workflow })
}

fn parse_doctor(args: &[OsString]) -> Result<Command, ParseError> {
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    Ok(Command::Doctor { db: PathBuf::from(db) })
}

fn parse_durability(raw: &str) -> Result<DurabilityMode, ParseError> {
    match raw {
        "strict" => Ok(DurabilityMode::Strict),
        "journaled" => Ok(DurabilityMode::Journaled),
        "none" => Ok(DurabilityMode::None),
        other => Err(ParseError::UnknownDurability(other.into())),
    }
}

fn positional(args: &[OsString], index: usize, name: &'static str) -> Result<PathBuf, ParseError> {
    args.get(index)
        .and_then(|s| s.to_str())
        .map(PathBuf::from)
        .ok_or(ParseError::MissingArgument(name))
}

fn positional_str(args: &[OsString], index: usize, name: &'static str) -> Result<String, ParseError> {
    args.get(index)
        .and_then(|s| s.to_str())
        .map(String::from)
        .ok_or(ParseError::MissingArgument(name))
}

fn named_flag(args: &[OsString], flag: &str) -> Option<String> {
    for (i, arg) in args.iter().enumerate() {
        if arg == flag {
            return args.get(i.checked_add(1)?).and_then(|v| v.to_str()).map(String::from);
        }
    }
    None
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingArgument(name) => write!(formatter, "missing argument: {name}"),
            Self::UnknownEmitTarget(target) => write!(formatter, "unknown emit target: {target} (expected: ir, rust)"),
            Self::UnknownDurability(mode) => write!(formatter, "unknown durability mode: {mode} (expected: strict, journaled, none)"),
            Self::UnknownCommand(cmd) => write!(formatter, "unknown command: {cmd}"),
            Self::NoCommand => write!(formatter, "no command provided"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_args, Command, DurabilityMode, ParseError};
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn args(parts: &[&str]) -> Vec<OsString> {
        parts.iter().map(|part| OsString::from(*part)).collect()
    }

    #[test]
    fn parse_run_accepts_db_for_journaled_mode() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics", "run", "workflow.yaml",
            "--input-bin", "input.bin", "--durability", "journaled", "--db", "journal-db",
        ]));
        assert!(matches!(parsed, Ok(Command::Run { .. })), "unexpected parse result: {parsed:?}");
        if let Ok(Command::Run { workflow, input_bin, durability, db }) = parsed {
            assert_eq!(workflow, PathBuf::from("workflow.yaml"));
            assert_eq!(input_bin, PathBuf::from("input.bin"));
            assert_eq!(durability, DurabilityMode::Journaled);
            assert_eq!(db, Some(PathBuf::from("journal-db")));
        }
    }

    #[test]
    fn parse_run_compiled_requires_db_for_strict_mode() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics", "run-compiled", "workflow.vbir",
            "--input-bin", "input.bin", "--durability", "strict",
        ]));
        assert!(matches!(parsed, Err(ParseError::MissingArgument("--db"))), "unexpected parse result: {parsed:?}");
    }

    #[test]
    fn parse_run_none_mode_keeps_db_optional() {
        let parsed = parse_args(&args(&[
            "velvet-ballastics", "run", "workflow.yaml",
            "--input-bin", "input.bin", "--durability", "none",
        ]));
        assert!(matches!(parsed, Ok(Command::Run { .. })), "unexpected parse result: {parsed:?}");
        if let Ok(Command::Run { durability, db, .. }) = parsed {
            assert_eq!(durability, DurabilityMode::None);
            assert_eq!(db, None);
        }
    }
}
