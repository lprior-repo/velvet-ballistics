use std::ffi::OsString;
use std::path::PathBuf;

use super::ParseError;

const EMIT_FLAG: &str = "--emit";

/// Parse --emit text|yaml|postcard output format flags.
/// Returns OutputFormat::Text by default.
pub(super) fn parse_output_format(args: &[OsString]) -> super::OutputFormat {
    parse_emit_output_format(named_flag(args, EMIT_FLAG).as_deref())
}

fn parse_emit_output_format(raw: Option<&str>) -> super::OutputFormat {
    match raw {
        Some("yaml") => super::OutputFormat::Yaml,
        Some("postcard") => super::OutputFormat::Postcard,
        Some("text") | Some(_) | None => super::OutputFormat::Text,
    }
}

/// Check if args contain a specific flag.
fn contains_flag(args: &[OsString], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

pub(super) fn positional(
    args: &[OsString],
    index: usize,
    name: &'static str,
) -> Result<PathBuf, ParseError> {
    args.get(index)
        .and_then(|s| s.to_str())
        .map(PathBuf::from)
        .ok_or(ParseError::MissingArgument(name))
}

pub(super) fn positional_str(
    args: &[OsString],
    index: usize,
    name: &'static str,
) -> Result<String, ParseError> {
    args.get(index)
        .and_then(|s| s.to_str())
        .map(String::from)
        .ok_or(ParseError::MissingArgument(name))
}

pub(super) fn named_flag(args: &[OsString], flag: &str) -> Option<String> {
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

/// Find the first positional argument (not starting with `--`) starting at `start_idx`.
/// This correctly skips over named flags and their values to locate the workflow path.
pub(super) fn find_positional(args: &[OsString], start_idx: usize) -> Option<PathBuf> {
    let mut i = start_idx;
    while i < args.len() {
        let arg = args.get(i)?.to_str()?;
        if arg.starts_with("--") {
            i = i.saturating_add(2);
        } else {
            return Some(PathBuf::from(arg));
        }
    }
    None
}
