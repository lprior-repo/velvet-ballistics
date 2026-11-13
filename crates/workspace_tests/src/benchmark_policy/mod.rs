mod benchmark_budget;
mod benchmark_freshness;
#[cfg(test)]
mod benchmark_tests_support;
pub(crate) mod error;
#[path = "benchmark_policy.rs"]
mod policy;
mod suggestions;
mod unicode16_digits;
mod usage;

use self::error::{PolicyError, Result};
use std::io::Write;

pub(crate) fn write_stdout(msg: std::fmt::Arguments<'_>) -> Result<()> {
    let mut out = std::io::stdout();
    out.write_fmt(msg)?;
    out.write_all(b"\n")?;
    Ok(())
}

enum CliAction {
    Run(String, String),
    Help,
}

fn execute() -> Result<()> {
    match parse_args()? {
        CliAction::Run(budget, evidence) => policy::cmd_benchmark_policy(&budget, &evidence),
        CliAction::Help => write_help(),
    }
}

fn parse_args() -> Result<CliAction> {
    parse_cli(std::env::args().skip(1))
}

fn parse_cli(args: impl Iterator<Item = String>) -> Result<CliAction> {
    let mut args = args;
    let mut budget = None;
    let mut evidence = None;
    let mut selected = Vec::new();
    while let Some(flag) = args.next() {
        if is_help(&flag) {
            return Ok(CliAction::Help);
        }
        let name = apply_flag(&flag, &mut args, &mut budget, &mut evidence, &selected)?;
        selected.push(name);
    }
    let budget = path_or_default(budget, "contracts/perf-budget.yaml");
    let evidence = path_or_default(evidence, "evidence/benchmark-evidence.jsonl");
    Ok(CliAction::Run(budget, evidence))
}

fn apply_flag(
    flag: &str,
    args: &mut impl Iterator<Item = String>,
    budget: &mut Option<String>,
    evidence: &mut Option<String>,
    selected: &[&str],
) -> Result<&'static str> {
    let (name, inline) = flag
        .split_once('=')
        .map_or((flag, None), |(name, value)| (name, Some(value)));
    let (name, target) = match name {
        "--budget" => ("--budget", budget),
        "--evidence" => ("--evidence", evidence),
        _ => return Err(usage::unknown(flag, selected)),
    };
    let value = parse_value(name, inline, args, selected)?;
    if target.is_some() {
        return Err(usage::duplicate(name));
    }
    *target = Some(value);
    Ok(name)
}

fn parse_value(
    flag: &str,
    inline: Option<&str>,
    args: &mut impl Iterator<Item = String>,
    selected: &[&str],
) -> Result<String> {
    if let Some(v) = inline {
        return Ok(v.to_string());
    }
    match args.next() {
        None => Err(usage::missing(flag)),
        Some(v) => {
            if is_path_flag(&v) || is_help(&v) {
                Err(usage::missing(flag))
            } else if v.starts_with('-') && v != "-" {
                Err(usage::unknown(&v, selected))
            } else {
                Ok(v)
            }
        }
    }
}

fn path_or_default(value: Option<String>, default: &str) -> String {
    match value {
        Some(v) => v,
        None => String::from(default),
    }
}

pub(crate) fn run_cli() -> std::process::ExitCode {
    match execute() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => report_error(error),
    }
}

fn report_error(error: PolicyError) -> std::process::ExitCode {
    let (message, status) = match error {
        PolicyError::Usage(message) => (message, std::process::ExitCode::from(2)),
        PolicyError::Message(message) => (
            format!("benchmark: {message}\n"),
            std::process::ExitCode::FAILURE,
        ),
    };
    match std::io::stderr().write_all(message.as_bytes()) {
        Ok(()) => status,
        Err(_) => std::process::ExitCode::FAILURE,
    }
}

#[cfg(test)]
#[path = "cli_parse_tests.rs"]
mod cli_parse_tests;

fn is_path_flag(value: &str) -> bool {
    let name = value.split_once('=').map_or(value, |(n, _)| n);
    matches!(name, "--budget" | "--evidence")
}

fn is_help(flag: &str) -> bool {
    matches!(flag, "--help" | "-h")
}

const HELP: &str = "Usage: xtask benchmark-regression-policy [OPTIONS]\n\nOptions:\n      --budget <BUDGET>      [default: contracts/perf-budget.yaml]\n      --evidence <EVIDENCE>  [default: evidence/benchmark-evidence.jsonl]\n  -h, --help                 Print help\n";

fn write_help() -> Result<()> {
    std::io::stdout().write_all(HELP.as_bytes())?;
    Ok(())
}
