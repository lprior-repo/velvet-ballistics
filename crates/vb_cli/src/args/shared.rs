use std::ffi::OsString;
use std::path::PathBuf;

use super::ParseError;

const EMIT_FLAG: &str = "--emit";

/// Parse --json, --jsonl, or --emit text|yaml|postcard output format flags.
/// Returns OutputFormat::Text by default.
pub(super) fn parse_output_format(args: &[OsString]) -> super::OutputFormat {
    // Legacy cold-path flags for backward compatibility
    if args.iter().any(|arg| arg == "--jsonl") {
        return super::OutputFormat::Jsonl;
    }
    if args.iter().any(|arg| arg == "--json") {
        return super::OutputFormat::Json;
    }
    // Canonical v1 flags
    match args.iter().position(|arg| arg == EMIT_FLAG) {
        Some(idx) => {
            if let Some(val) = args.get(idx.saturating_add(1)).and_then(|v| v.to_str()) {
                return match val {
                    "yaml" => super::OutputFormat::Yaml,
                    "postcard" => super::OutputFormat::Postcard,
                    "text" => super::OutputFormat::Text,
                    _ => super::OutputFormat::Text,
                };
            }
            super::OutputFormat::Text
        }
        None => super::OutputFormat::Text,
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