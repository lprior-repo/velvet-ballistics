//! Flag specification for known commands.
#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlagSpec {
    Switch,
    Value(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActionListParseState {
    pub(crate) output: super::OutputFormat,
    pub(crate) registry: super::ActionRegistryMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActionInspectParseState {
    pub(crate) output: super::OutputFormat,
    pub(crate) registry: super::ActionRegistryMode,
}

pub(crate) fn known_flag_spec(command: &str, token: &str) -> Option<FlagSpec> {
    source_flags(command, token)
        .or_else(|| execution_flags(command, token))
        .or_else(|| observation_flags(command, token))
        .or_else(|| control_flags(command, token))
}

fn source_flags(command: &str, token: &str) -> Option<FlagSpec> {
    match command {
        "validate" | "explain" | "bench-run" | "graph" | "simulate" => output_flag_spec(token),
        "verify" => output_flag_spec(token).or_else(|| value_flag_spec(token, "--profile")),
        "compile" => match token {
            "--json" | "--jsonl" => Some(FlagSpec::Switch),
            "--emit" => Some(FlagSpec::Value("--emit")),
            "--out" => Some(FlagSpec::Value("--out")),
            _ => None,
        },
        _ => None,
    }
}

fn execution_flags(command: &str, token: &str) -> Option<FlagSpec> {
    match command {
        "run" => output_flag_spec(token)
            .or_else(|| input_flags(token))
            .or_else(|| step_flags(token)),
        "run-compiled" | "submit" => output_flag_spec(token).or_else(|| input_flags(token)),
        "ipc-serve" => match token {
            "--socket" => Some(FlagSpec::Value("--socket")),
            "--db" => Some(FlagSpec::Value("--db")),
            _ => None,
        },
        _ => None,
    }
}

fn input_flags(token: &str) -> Option<FlagSpec> {
    match token {
        "--input-bin" => Some(FlagSpec::Value("--input-bin")),
        "--durability" => Some(FlagSpec::Value("--durability")),
        "--db" => Some(FlagSpec::Value("--db")),
        _ => None,
    }
}

fn step_flags(token: &str) -> Option<FlagSpec> {
    match token {
        "--step" => Some(FlagSpec::Value("--step")),
        "--step-input" => Some(FlagSpec::Value("--step-input")),
        _ => None,
    }
}

fn observation_flags(command: &str, token: &str) -> Option<FlagSpec> {
    match command {
        "ai-context" => switch_flag_spec(token, "--json")
            .or_else(|| output_flag_spec(token))
            .or_else(|| value_flag_spec(token, "--db")),
        "inspect" | "replay" | "resume" | "incident" => {
            output_flag_spec(token).or_else(|| value_flag_spec(token, "--db"))
        }
        "events" => output_flag_spec(token).or_else(|| event_flags(token)),
        "trace" => output_flag_spec(token).or_else(|| trace_flags(token)),
        _ => None,
    }
}

fn event_flags(token: &str) -> Option<FlagSpec> {
    match token {
        "--db" => Some(FlagSpec::Value("--db")),
        "--status" => Some(FlagSpec::Value("--status")),
        "--limit" => Some(FlagSpec::Value("--limit")),
        _ => None,
    }
}

fn trace_flags(token: &str) -> Option<FlagSpec> {
    match token {
        "--db" => Some(FlagSpec::Value("--db")),
        "--step" => Some(FlagSpec::Value("--step")),
        "--action" => Some(FlagSpec::Value("--action")),
        "--status" => Some(FlagSpec::Value("--status")),
        "--since-seq" => Some(FlagSpec::Value("--since-seq")),
        "--until-seq" => Some(FlagSpec::Value("--until-seq")),
        "--limit" => Some(FlagSpec::Value("--limit")),
        _ => None,
    }
}

fn control_flags(command: &str, token: &str) -> Option<FlagSpec> {
    match command {
        "retry" => output_flag_spec(token)
            .or_else(|| value_flag_spec(token, "--db"))
            .or_else(|| value_flag_spec(token, "--step")),
        "cancel" => output_flag_spec(token)
            .or_else(|| value_flag_spec(token, "--db"))
            .or_else(|| value_flag_spec(token, "--reason")),
        "doctor" => output_flag_spec(token).or_else(|| value_flag_spec(token, "--db")),
        "answer" => output_flag_spec(token).or_else(|| answer_flags(token)),
        "diff" => output_flag_spec(token)
            .or_else(|| value_flag_spec(token, "--db"))
            .or_else(|| value_flag_spec(token, "--against")),
        _ => None,
    }
}

fn answer_flags(token: &str) -> Option<FlagSpec> {
    match token {
        "--step" => Some(FlagSpec::Value("--step")),
        "--slot" => Some(FlagSpec::Value("--slot")),
        "--value-file" => Some(FlagSpec::Value("--value-file")),
        "--value" => Some(FlagSpec::Value("--value")),
        "--db" => Some(FlagSpec::Value("--db")),
        _ => None,
    }
}

fn output_flag_spec(token: &str) -> Option<FlagSpec> {
    match token {
        "--json" | "--jsonl" => Some(FlagSpec::Switch),
        "--emit" => Some(FlagSpec::Value("--emit")),
        _ => None,
    }
}

fn value_flag_spec(token: &str, flag: &'static str) -> Option<FlagSpec> {
    if token == flag {
        Some(FlagSpec::Value(flag))
    } else {
        None
    }
}

fn switch_flag_spec(token: &str, flag: &'static str) -> Option<FlagSpec> {
    if token == flag {
        Some(FlagSpec::Switch)
    } else {
        None
    }
}
