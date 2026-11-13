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
    Run(String, String, String),
    Help,
}

#[derive(Default)]
struct CliOptions {
    root: Option<String>,
    extra_root: Option<String>,
    budget: Option<String>,
    evidence: Option<String>,
    selected: Vec<&'static str>,
}

fn execute() -> Result<()> {
    match parse_args()? {
        CliAction::Run(root, budget, evidence) => {
            policy::cmd_benchmark_policy(std::path::Path::new(&root), &budget, &evidence)
        }
        CliAction::Help => write_help(),
    }
}

fn parse_args() -> Result<CliAction> {
    parse_os_cli(std::env::args_os().skip(1))
}

fn parse_os_cli(args: impl Iterator<Item = std::ffi::OsString>) -> Result<CliAction> {
    parse_cli(args)
}

fn parse_cli<A: Into<std::ffi::OsString>>(args: impl Iterator<Item = A>) -> Result<CliAction> {
    let mut args = args.map(|argument| {
        let argument: std::ffi::OsString = argument.into();
        argument
            .into_string()
            .map_err(|_| usage::argument_encoding())
    });
    let mut options = CliOptions::default();
    let mut operands_only = false;
    while let Some(argument) = args.next() {
        let argument = argument?;
        if !operands_only && is_help(&argument) {
            return Ok(CliAction::Help);
        }
        if !operands_only && argument == "--" {
            operands_only = true;
            continue;
        }
        parse_argument(argument, &mut args, &mut options, operands_only)?;
    }
    finish_cli(options)
}

fn finish_cli(options: CliOptions) -> Result<CliAction> {
    if let Some(argument) = options.extra_root {
        return Err(usage::extra_root(&argument));
    }
    Ok(CliAction::Run(
        path_or_default(options.root, "."),
        path_or_default(options.budget, "contracts/perf-budget.yaml"),
        path_or_default(options.evidence, "evidence/benchmark-evidence.jsonl"),
    ))
}

fn parse_argument(
    argument: String,
    args: &mut impl Iterator<Item = Result<String>>,
    options: &mut CliOptions,
    operands_only: bool,
) -> Result<()> {
    if !operands_only && is_option(&argument)? {
        let name = apply_flag(&argument, args, options)?;
        options.selected.push(name);
    } else if options.root.is_none() {
        options.root = Some(argument);
    } else if options.extra_root.is_none() {
        options.extra_root = Some(argument);
    }
    Ok(())
}

fn apply_flag(
    flag: &str,
    args: &mut impl Iterator<Item = Result<String>>,
    options: &mut CliOptions,
) -> Result<&'static str> {
    let (name, inline) = flag
        .split_once('=')
        .map_or((flag, None), |(name, value)| (name, Some(value)));
    let (name, target) = match name {
        "--budget" => ("--budget", &mut options.budget),
        "--evidence" => ("--evidence", &mut options.evidence),
        _ => return Err(usage::unknown(flag, &options.selected)),
    };
    let value = parse_value(name, inline, args)?;
    *target = Some(value);
    Ok(name)
}

fn parse_value(
    flag: &str,
    inline: Option<&str>,
    args: &mut impl Iterator<Item = Result<String>>,
) -> Result<String> {
    if let Some(v) = inline {
        return Ok(v.to_string());
    }
    let value = args
        .next()
        .transpose()?
        .ok_or_else(|| usage::missing(flag))?;
    if is_option(&value)? {
        Err(usage::missing(flag))
    } else {
        Ok(value)
    }
}

fn is_option(value: &str) -> Result<bool> {
    if !value.starts_with('-') || value == "-" {
        return Ok(false);
    }
    // Known options take precedence over the space-bearing operand rule.
    let name = value.split_once('=').map_or(value, |(name, _)| name);
    if matches!(name, "--budget" | "--evidence" | "--help") || value.starts_with("-h") {
        return Ok(true);
    }
    if value.contains(' ') {
        return Ok(false);
    }
    Ok(!has_decimal_negative_prefix(value))
}

fn has_decimal_negative_prefix(value: &str) -> bool {
    let Some(after_minus) = value.strip_prefix('-') else {
        return false;
    };
    let after_dot = after_minus.strip_prefix('.').map_or(after_minus, |s| s);
    after_dot
        .chars()
        .next()
        .is_some_and(unicode16_digits::is_decimal)
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
            format!("error: {message}\n"),
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

#[cfg(test)]
mod space_operand_regression {
    use super::*;

    const BUDGET: &str = "contracts/perf-budget.yaml";
    const EVIDENCE: &str = "evidence/benchmark-evidence.jsonl";
    const MISSING_BUDGET: &str = "error: a value is required for '--budget <BUDGET>' but none was supplied\n\nFor more information, try '--help'.\n";
    const MISSING_EVIDENCE: &str = "error: a value is required for '--evidence <EVIDENCE>' but none was supplied\n\nFor more information, try '--help'.\n";

    const UNKNOWN_ROOT: &str = "error: unexpected argument '-²root' found\n\nUsage: benchmark-policy [OPTIONS] [ROOT]\n\nFor more information, try '--help'.\n";
    const UNKNOWN_HELP: &str = "error: unexpected argument '-h text' found\n\nUsage: benchmark-policy [OPTIONS] [ROOT]\n\nFor more information, try '--help'.\n";

    fn run(args: &[&str], budget: &str, evidence: &str) -> Result<()> {
        let CliAction::Run(r, b, e) = parse_cli(args.iter().map(|s| s.to_string()))? else {
            return Err(PolicyError::Message("expected run".to_string()));
        };
        assert_eq!(
            (r.as_str(), b.as_str(), e.as_str()),
            ("repo", budget, evidence)
        );
        Ok(())
    }
    fn fail(args: &[&str], expected: &str) {
        assert!(matches!(
            parse_cli(args.iter().map(|s| s.to_string())),
            Err(PolicyError::Usage(actual)) if actual == expected
        ));
    }
    fn help(args: &[&str]) {
        assert!(matches!(
            parse_cli(args.iter().map(|s| s.to_string())),
            Ok(CliAction::Help)
        ));
    }

    #[test]
    fn space_paths_reach_run() -> Result<()> {
        run(&["repo", "--budget", "-² budget"], "-² budget", EVIDENCE)?;
        run(
            &["repo", "--evidence", "-² evidence"],
            BUDGET,
            "-² evidence",
        )?;
        run(
            &["repo", "--budget=spaced budget.yaml"],
            "spaced budget.yaml",
            EVIDENCE,
        )?;
        run(
            &["repo", "--evidence=spaced evidence.jsonl"],
            BUDGET,
            "spaced evidence.jsonl",
        )
    }
    #[test]
    fn space_paths_are_help_actions() {
        help(&["--budget", "-² budget", "--help"]);
        help(&["--evidence", "-² evidence", "--help"]);
        help(&["-² root", "--help"]);
    }
    #[test]
    fn no_space_stays_options() {
        fail(&["--budget", "-²budget"], MISSING_BUDGET);
        fail(&["--evidence", "-²evidence"], MISSING_EVIDENCE);
        fail(&["-²root"], UNKNOWN_ROOT);
    }
    #[test]
    fn tab_and_nbsp_are_not_space() {
        fail(&["--budget", "-²\tbudget"], MISSING_BUDGET);
        fail(&["--evidence", "-²\u{00A0}evidence"], MISSING_EVIDENCE);
    }
    #[test]
    fn short_h_space_stays_option() {
        fail(&["-h text"], UNKNOWN_HELP);
        fail(&["--budget", "-h text"], MISSING_BUDGET);
    }
    #[test]
    fn is_option_classifies_space() -> Result<()> {
        assert!(!is_option("-² budget")?);
        assert!(!is_option("-² evidence")?);
        assert!(!is_option("-² root")?);
        assert!(is_option("-²budget")?);
        assert!(is_option("-²evidence")?);
        assert!(is_option("-²root")?);
        Ok(())
    }
}

fn is_help(flag: &str) -> bool {
    matches!(flag, "--help" | "-h")
}

const HELP: &str = "Usage: benchmark-policy [OPTIONS] [ROOT]\n\nCheck the benchmark regression policy gate.\n\nArguments:\n  [ROOT]  Repository root [default: .]\n\nOptions:\n      --budget <BUDGET>      [default: contracts/perf-budget.yaml]\n      --evidence <EVIDENCE>  [default: evidence/benchmark-evidence.jsonl]\n  -h, --help                 Print help\n";

fn write_help() -> Result<()> {
    std::io::stdout().write_all(HELP.as_bytes())?;
    Ok(())
}
