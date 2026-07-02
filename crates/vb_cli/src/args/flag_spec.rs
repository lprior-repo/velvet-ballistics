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
    match command {
        "validate" | "explain" | "bench-run" | "graph" | "simulate" => output_flag_spec(token),
        "ai-context" => switch_flag_spec(token, "--json")
            .or_else(|| output_flag_spec(token))
            .or_else(|| value_flag_spec(token, "--db")),
        "inspect" | "retry" | "resume" | "incident" => {
            output_flag_spec(token).or_else(|| value_flag_spec(token, "--db"))
        }
        "replay" => output_flag_spec(token).or(match token {
            "--db" => Some(FlagSpec::Value("--db")),
            "--expected-action-abi" => Some(FlagSpec::Value("--expected-action-abi")),
            "--expected-policy-digest" => Some(FlagSpec::Value("--expected-policy-digest")),
            "--allow-empty-expectations" => Some(FlagSpec::Switch),
            _ => None,
        }),
        "verify" => output_flag_spec(token).or_else(|| value_flag_spec(token, "--profile")),
        "compile" => match token {
            "--json" | "--jsonl" => Some(FlagSpec::Switch),
            "--emit" => Some(FlagSpec::Value("--emit")),
            "--out" => Some(FlagSpec::Value("--out")),
            _ => None,
        },
        "run" => output_flag_spec(token).or(match token {
            "--input-bin" => Some(FlagSpec::Value("--input-bin")),
            "--durability" => Some(FlagSpec::Value("--durability")),
            "--db" => Some(FlagSpec::Value("--db")),
            "--step" => Some(FlagSpec::Value("--step")),
            "--step-input" => Some(FlagSpec::Value("--step-input")),
            _ => None,
        }),
        "run-compiled" => output_flag_spec(token).or(match token {
            "--input-bin" => Some(FlagSpec::Value("--input-bin")),
            "--durability" => Some(FlagSpec::Value("--durability")),
            "--db" => Some(FlagSpec::Value("--db")),
            _ => None,
        }),
        "ipc-serve" => match token {
            "--socket" => Some(FlagSpec::Value("--socket")),
            "--db" => Some(FlagSpec::Value("--db")),
            _ => None,
        },
        "events" => output_flag_spec(token).or(match token {
            "--db" => Some(FlagSpec::Value("--db")),
            "--status" => Some(FlagSpec::Value("--status")),
            "--limit" => Some(FlagSpec::Value("--limit")),
            _ => None,
        }),
        "trace" => output_flag_spec(token).or(match token {
            "--db" => Some(FlagSpec::Value("--db")),
            "--step" => Some(FlagSpec::Value("--step")),
            "--action" => Some(FlagSpec::Value("--action")),
            "--status" => Some(FlagSpec::Value("--status")),
            "--since-seq" => Some(FlagSpec::Value("--since-seq")),
            "--until-seq" => Some(FlagSpec::Value("--until-seq")),
            "--limit" => Some(FlagSpec::Value("--limit")),
            _ => None,
        }),
        "cancel" => output_flag_spec(token).or(match token {
            "--db" => Some(FlagSpec::Value("--db")),
            "--reason" => Some(FlagSpec::Value("--reason")),
            _ => None,
        }),
        "doctor" => output_flag_spec(token).or_else(|| value_flag_spec(token, "--db")),
        "answer" => output_flag_spec(token).or(match token {
            "--step" => Some(FlagSpec::Value("--step")),
            "--value-file" => Some(FlagSpec::Value("--value-file")),
            "--db" => Some(FlagSpec::Value("--db")),
            _ => None,
        }),
        "diff" => output_flag_spec(token).or_else(|| value_flag_spec(token, "--db")),
        "submit" => output_flag_spec(token).or(match token {
            "--input-bin" => Some(FlagSpec::Value("--input-bin")),
            "--db" => Some(FlagSpec::Value("--db")),
            "--durability" => Some(FlagSpec::Value("--durability")),
            _ => None,
        }),
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
