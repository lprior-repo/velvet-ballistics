use std::ffi::OsString;
use std::path::PathBuf;

use super::shared::{find_positional, named_flag, parse_output_format, positional};
use super::{Command, DurabilityMode, EmitTarget, ParseError, StepTarget, VerifyProfile};

pub(super) fn parse_verify(args: &[OsString]) -> Result<Command, ParseError> {
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
    let workflow = positional(args, 2, "workflow.yaml")?;
    let output = parse_output_format(args);
    Ok(Command::Validate { workflow, output })
}

pub(super) fn parse_explain(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let output = parse_output_format(args);
    Ok(Command::Explain { workflow, output })
}

pub(super) fn parse_compile(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let emit_raw = named_flag(args, "--emit").ok_or(ParseError::MissingArgument("--emit"))?;
    let emit = match emit_raw.as_str() {
        "ir" => EmitTarget::Ir,
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

pub(super) fn parse_run(args: &[OsString]) -> Result<Command, ParseError> {
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

pub(super) fn parse_run_compiled(args: &[OsString]) -> Result<Command, ParseError> {
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

pub(super) fn parse_ipc_serve(args: &[OsString]) -> Result<Command, ParseError> {
    let socket = named_flag(args, "--socket").ok_or(ParseError::MissingArgument("--socket"))?;
    let db = named_flag(args, "--db").ok_or(ParseError::MissingArgument("--db"))?;
    Ok(Command::IpcServe {
        socket: PathBuf::from(socket),
        db: PathBuf::from(db),
    })
}

pub(super) fn parse_bench_run(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let output = parse_output_format(args);
    Ok(Command::BenchRun { workflow, output })
}

pub(super) fn parse_graph(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let output = parse_output_format(args);
    Ok(Command::Graph { workflow, output })
}

pub(super) fn parse_simulate(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
    let output = parse_output_format(args);
    Ok(Command::Simulate { workflow, output })
}

pub(super) fn parse_submit(args: &[OsString]) -> Result<Command, ParseError> {
    let workflow = positional(args, 2, "workflow.yaml")?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn os(val: &str) -> OsString {
        OsString::from(val)
    }

    #[test]
    fn parse_validate_returns_ok() {
        let args = [os("vb"), os("validate"), os("wf.yaml")];
        let result = parse_validate(&args).unwrap();
        match result {
            Command::Validate { workflow, .. } => assert_eq!(workflow, PathBuf::from("wf.yaml")),
            _ => panic!("wrong command"),
        }
    }

    #[test]
    fn parse_validate_errors_when_missing_workflow() {
        let args = [os("vb"), os("validate")];
        let result = parse_validate(&args);
        assert!(matches!(result.unwrap_err(), ParseError::MissingArgument(_)));
    }

    #[test]
    fn parse_verify_returns_ok_with_default_profile() {
        let args = [os("vb"), os("verify"), os("wf.yaml")];
        let result = parse_verify(&args).unwrap();
        match result {
            Command::Verify { workflow, profile, .. } => {
                assert_eq!(workflow, PathBuf::from("wf.yaml"));
                assert_eq!(profile, VerifyProfile::Quick);
            }
            _ => panic!("wrong command"),
        }
    }

    #[test]
    fn parse_verify_parses_quick_profile() {
        let args = [os("vb"), os("verify"), os("wf.yaml"), os("--profile"), os("quick")];
        let result = parse_verify(&args).unwrap();
        match result {
            Command::Verify { profile, .. } => assert_eq!(profile, VerifyProfile::Quick),
            _ => panic!("wrong command"),
        }
    }

    #[test]
    fn parse_verify_parses_standard_profile() {
        let args = [os("vb"), os("verify"), os("wf.yaml"), os("--profile"), os("standard")];
        let result = parse_verify(&args).unwrap();
        match result {
            Command::Verify { profile, .. } => assert_eq!(profile, VerifyProfile::Standard),
            _ => panic!("wrong command"),
        }
    }

    #[test]
    fn parse_verify_parses_full_profile() {
        let args = [os("vb"), os("verify"), os("wf.yaml"), os("--profile"), os("full")];
        let result = parse_verify(&args).unwrap();
        match result {
            Command::Verify { profile, .. } => assert_eq!(profile, VerifyProfile::Full),
            _ => panic!("wrong command"),
        }
    }

    #[test]
    fn parse_verify_rejects_unknown_profile() {
        let args = [os("vb"), os("verify"), os("wf.yaml"), os("--profile"), os("deep")];
        let result = parse_verify(&args);
        assert!(matches!(result.unwrap_err(), ParseError::UnknownProfile(_)));
    }

    #[test]
    fn parse_compile_parses_emit_ir() {
        let args = [
            os("vb"), os("compile"), os("wf.yaml"),
            os("--emit"), os("ir"), os("--out"), os("out.vbir"),
        ];
        let result = parse_compile(&args).unwrap();
        match result {
            Command::Compile { emit, out, .. } => {
                assert_eq!(emit, EmitTarget::Ir);
                assert_eq!(out, PathBuf::from("out.vbir"));
            }
            _ => panic!("wrong command"),
        }
    }

    #[test]
    fn parse_compile_rejects_unknown_emit() {
        let args = [
            os("vb"), os("compile"), os("wf.yaml"),
            os("--emit"), os("json"), os("--out"), os("out"),
        ];
        let result = parse_compile(&args);
        assert!(matches!(result.unwrap_err(), ParseError::UnknownEmitTarget(_)));
    }

    #[test]
    fn parse_graph_returns_ok() {
        let args = [os("vb"), os("graph"), os("wf.yaml")];
        let result = parse_graph(&args).unwrap();
        match result {
            Command::Graph { workflow, .. } => assert_eq!(workflow, PathBuf::from("wf.yaml")),
            _ => panic!("wrong command"),
        }
    }

    #[test]
    fn parse_simulate_returns_ok() {
        let args = [os("vb"), os("simulate"), os("wf.yaml")];
        let result = parse_simulate(&args).unwrap();
        match result {
            Command::Simulate { workflow, .. } => assert_eq!(workflow, PathBuf::from("wf.yaml")),
            _ => panic!("wrong command"),
        }
    }

    #[test]
    fn parse_bench_run_returns_ok() {
        let args = [os("vb"), os("bench-run"), os("wf.yaml")];
        let result = parse_bench_run(&args).unwrap();
        match result {
            Command::BenchRun { workflow, .. } => assert_eq!(workflow, PathBuf::from("wf.yaml")),
            _ => panic!("wrong command"),
        }
    }

    #[test]
    fn parse_run_rejects_missing_durability() {
        let args = [
            os("vb"), os("run"), os("wf.yaml"),
            os("--input-bin"), os("in.bin"),
        ];
        let result = parse_run(&args);
        assert!(matches!(result.unwrap_err(), ParseError::MissingArgument(_)));
    }

    #[test]
    fn parse_run_rejects_unknown_durability() {
        let args = [
            os("vb"), os("run"), os("wf.yaml"),
            os("--input-bin"), os("in.bin"),
            os("--durability"), os("superfast"),
        ];
        let result = parse_run(&args);
        assert!(matches!(result.unwrap_err(), ParseError::UnknownDurability(_)));
    }

    #[test]
    fn parse_ipc_serve_parses_socket_and_db() {
        let args = [
            os("vb"), os("ipc-serve"),
            os("--socket"), os("/tmp/sock"),
            os("--db"), os("/tmp/db"),
        ];
        let result = parse_ipc_serve(&args).unwrap();
        match result {
            Command::IpcServe { socket, db } => {
                assert_eq!(socket, PathBuf::from("/tmp/sock"));
                assert_eq!(db, PathBuf::from("/tmp/db"));
            }
            _ => panic!("wrong command"),
        }
    }

    #[test]
    fn parse_submit_parses_all_args() {
        let args = [
            os("vb"), os("submit"), os("wf.yaml"),
            os("--input-bin"), os("in.bin"),
            os("--db"), os("/tmp/db"),
            os("--durability"), os("journaled"),
        ];
        let result = parse_submit(&args).unwrap();
        match result {
            Command::Submit { durability, .. } => {
                assert_eq!(durability, DurabilityMode::Journaled);
            }
            _ => panic!("wrong command"),
        }
    }

    #[test]
    fn parse_durability_returns_strict() {
        assert_eq!(parse_durability("strict").unwrap(), DurabilityMode::Strict);
    }

    #[test]
    fn parse_durability_returns_journaled() {
        assert_eq!(parse_durability("journaled").unwrap(), DurabilityMode::Journaled);
    }

    #[test]
    fn parse_durability_returns_none() {
        assert_eq!(parse_durability("none").unwrap(), DurabilityMode::None);
    }

    #[test]
    fn parse_durability_rejects_unknown() {
        assert!(matches!(parse_durability("bad").unwrap_err(), ParseError::UnknownDurability(_)));
    }
}
