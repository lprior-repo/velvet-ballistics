//! Workflow command parsers.
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;

use super::error::ParseError;
use super::shared::{find_positional, named_flag, parse_output_format, validate_known_flags};
use super::types::{Command, DurabilityMode, EmitTarget, StepTarget, VerifyProfile};

pub(super) fn parse_verify(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "verify")?;
    let workflow = find_positional(args, 2).ok_or(ParseError::MissingArgument("workflow.yaml"))?;
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

pub(super) fn parse_validate(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "validate")?;
    let workflow = find_positional(args, 2).ok_or(ParseError::MissingArgument("workflow.yaml"))?;
    let output = parse_output_format(args);
    Ok(Command::Validate { workflow, output })
}

pub(super) fn parse_explain(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "explain")?;
    let workflow = find_positional(args, 2).ok_or(ParseError::MissingArgument("workflow.yaml"))?;
    let output = parse_output_format(args);
    Ok(Command::Explain { workflow, output })
}

pub(super) fn parse_compile(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "compile")?;
    let workflow = find_positional(args, 2).ok_or(ParseError::MissingArgument("workflow.yaml"))?;
    let emit_raw = named_flag(args, "--emit").ok_or(ParseError::MissingArgument("--emit"))?;
    let emit = match emit_raw.as_str() {
        "ir" => EmitTarget::Ir,
        "yaml" => EmitTarget::Yaml,
        "postcard" => EmitTarget::Postcard,
        other => return Err(ParseError::UnknownEmitTarget(other.into())),
    };
    let out = named_flag(args, "--out").ok_or(ParseError::MissingArgument("--out"))?;
    let output = super::shared::parse_compile_output_format(args);
    Ok(Command::Compile {
        workflow,
        emit,
        out: PathBuf::from(out),
        output,
    })
}

pub(super) fn parse_run(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "run")?;
    let workflow = find_positional(args, 2).ok_or(ParseError::MissingArgument("workflow.yaml"))?;
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

pub(super) fn parse_run_compiled(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "run-compiled")?;
    let workflow = find_positional(args, 2).ok_or(ParseError::MissingArgument("workflow.vbir"))?;
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

pub(super) fn parse_ipc_serve(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "ipc-serve")?;
    let socket = named_flag(args, "--socket").ok_or(ParseError::MissingArgument("--socket"))?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    Ok(Command::IpcServe {
        socket: PathBuf::from(socket),
        db: PathBuf::from(db),
    })
}

pub(super) fn parse_bench_run(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "bench-run")?;
    let workflow = find_positional(args, 2).ok_or(ParseError::MissingArgument("workflow.yaml"))?;
    let output = parse_output_format(args);
    Ok(Command::BenchRun { workflow, output })
}

pub(super) fn parse_graph(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "graph")?;
    let workflow = find_positional(args, 2).ok_or(ParseError::MissingArgument("workflow.yaml"))?;
    let output = parse_output_format(args);
    Ok(Command::Graph { workflow, output })
}

pub(super) fn parse_simulate(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "simulate")?;
    let workflow = find_positional(args, 2).ok_or(ParseError::MissingArgument("workflow.yaml"))?;
    let output = parse_output_format(args);
    Ok(Command::Simulate { workflow, output })
}

pub(super) fn parse_submit(args: &[OsString]) -> Result<Command, ParseError> {
    validate_known_flags(args, "submit")?;
    let workflow = find_positional(args, 2).ok_or(ParseError::MissingArgument("workflow.yaml"))?;
    let input_bin =
        named_flag(args, "--input-bin").ok_or(ParseError::MissingArgument("--input-bin"))?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    let durability_raw =
        named_flag(args, "--durability").ok_or(ParseError::MissingArgument("--durability"))?;
    let durability = parse_durability(&durability_raw)?;
    let output = parse_output_format(args);
    Ok(Command::Submit {
        workflow,
        input_bin: PathBuf::from(input_bin),
        db: PathBuf::from(db),
        durability,
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
