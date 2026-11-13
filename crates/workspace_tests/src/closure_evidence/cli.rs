//! CLI argument parsing for the closure-evidence checker.

use std::io::Write;

pub(crate) const DEFAULT_PARENT: &str = "vb-jpq7";
pub(crate) const DEFAULT_MANIFEST: &str = ".beads/vb-jpq7/closure-evidence-manifest.jsonl";
pub(crate) const DEFAULT_BD_WORKDIR: &str = ".";

pub(crate) struct CliArgs {
    pub(crate) parent: String,
    pub(crate) manifest: String,
    pub(crate) bd_workdir: String,
    pub(crate) self_test: bool,
}

fn collect_args() -> Result<Vec<String>, String> {
    std::env::args_os()
        .map(|os| {
            os.to_str()
                .ok_or("non-UTF-8 argument")
                .map(|s| s.to_string())
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("args_os error: {}", e))
}

fn apply_parsed_arg(parsed: Option<ParsedArg>, args: &mut CliArgs) {
    match parsed {
        Some(ParsedArg::Parent(v)) => args.parent = v,
        Some(ParsedArg::Manifest(v)) => args.manifest = v,
        Some(ParsedArg::BdWorkdir(v)) => args.bd_workdir = v,
        Some(ParsedArg::SelfTest) => args.self_test = true,
        None => {}
    }
}

pub(crate) fn parse() -> Result<CliArgs, String> {
    let mut iter = collect_args()?.into_iter().skip(1);
    let defaults = CliArgs {
        parent: DEFAULT_PARENT.to_string(),
        manifest: DEFAULT_MANIFEST.to_string(),
        bd_workdir: DEFAULT_BD_WORKDIR.to_string(),
        self_test: false,
    };
    std::iter::from_fn(|| iter.next().map(|arg| parse_option(&arg, &mut iter))).try_fold(
        defaults,
        |mut args, parsed| {
            apply_parsed_arg(parsed?, &mut args);
            Ok(args)
        },
    )
}

enum ParsedArg {
    Parent(String),
    Manifest(String),
    BdWorkdir(String),
    SelfTest,
}

fn handle_equals_form(arg: &str) -> Result<Option<ParsedArg>, String> {
    if let Some((name, value)) = parse_option_equals(arg) {
        match name {
            "parent" => Ok(Some(ParsedArg::Parent(value))),
            "manifest" => Ok(Some(ParsedArg::Manifest(value))),
            "bd-workdir" => Ok(Some(ParsedArg::BdWorkdir(value))),
            other => Err(format!("unknown option: --{}", other)),
        }
    } else {
        Ok(None)
    }
}

fn handle_value_option(
    arg: &str,
    iter: &mut impl Iterator<Item = String>,
) -> Result<Option<ParsedArg>, String> {
    let value = separated_value(arg, iter)?;
    match arg {
        "--parent" => Ok(Some(ParsedArg::Parent(value))),
        "--manifest" => Ok(Some(ParsedArg::Manifest(value))),
        "--bd-workdir" => Ok(Some(ParsedArg::BdWorkdir(value))),
        other => Err(format!("unknown option: {}", other)),
    }
}

fn separated_value(
    option: &str,
    iter: &mut impl Iterator<Item = String>,
) -> Result<String, String> {
    let missing = || format!("{option} requires a value");
    let value = iter.next().ok_or_else(missing)?;
    if value.starts_with('-') && value != "-" && !value.contains(' ') && !negative_value(&value)? {
        return Err(missing());
    }
    Ok(value)
}

fn negative_value(value: &str) -> Result<bool, String> {
    regex::Regex::new(r"^-\.?\d")
        .map(|pattern| pattern.is_match(value))
        .map_err(|error| format!("option value matcher failed: {error}"))
}

fn parse_option(
    arg: &str,
    iter: &mut impl Iterator<Item = String>,
) -> Result<Option<ParsedArg>, String> {
    if let Some(result) = handle_equals_form(arg)? {
        return Ok(Some(result));
    }
    match arg {
        "--help" | "-h" => match print_usage() {
            Ok(()) => std::process::exit(0),
            Err(e) => Err(format!("failed to write help: {}", e)),
        },
        "--parent" | "--manifest" | "--bd-workdir" => handle_value_option(arg, iter),
        "--self-test" => Ok(Some(ParsedArg::SelfTest)),
        other => Err(format!("unknown option: {}", other)),
    }
}

fn parse_option_equals(arg: &str) -> Option<(&str, String)> {
    let stripped = arg.strip_prefix("--")?;
    let parts = stripped.split_once('=')?;
    let name = parts.0;
    let value = parts.1.to_string();
    Some((name, value))
}

fn print_usage() -> std::io::Result<()> {
    let msg = "usage: closure-evidence [OPTIONS]\n\
         OPTIONS:\n\
           --parent <ID>       parent bead ID (default: vb-jpq7)\n\
           --manifest <PATH>   path to manifest JSONL file\n\
           --bd-workdir <DIR>  working directory for bd commands\n\
           --self-test         run self-test\n\
           -h, --help          show help\n";
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(msg.as_bytes())?;
    Ok(())
}
