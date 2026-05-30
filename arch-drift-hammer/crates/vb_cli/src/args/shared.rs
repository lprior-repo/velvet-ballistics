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

#[cfg(test)]
mod tests {
    use super::*;

    fn os(val: &str) -> OsString {
        OsString::from(val)
    }

    #[test]
    fn parse_output_format_defaults_to_text() {
        assert_eq!(parse_output_format(&[]), super::OutputFormat::Text);
    }

    #[test]
    fn parse_output_format_returns_yaml_from_emit() {
        let args = [os("--emit"), os("yaml")];
        assert_eq!(parse_output_format(&args), super::OutputFormat::Yaml);
    }

    #[test]
    fn parse_output_format_returns_postcard_from_emit() {
        let args = [os("--emit"), os("postcard")];
        assert_eq!(parse_output_format(&args), super::OutputFormat::Postcard);
    }

    #[test]
    fn parse_output_format_returns_text_for_unknown_emit() {
        let args = [os("--emit"), os("garbage")];
        assert_eq!(parse_output_format(&args), super::OutputFormat::Text);
    }

    #[test]
    fn positional_returns_pathbuf_at_index() {
        let args = [os("prog"), os("file.yaml")];
        let result = positional(&args, 1, "workflow");
        assert_eq!(result.unwrap(), PathBuf::from("file.yaml"));
    }

    #[test]
    fn positional_returns_error_when_missing() {
        let args = [os("prog")];
        let result = positional(&args, 1, "workflow");
        assert!(result.is_err());
        match result.unwrap_err() {
            ParseError::MissingArgument(name) => assert_eq!(name, "workflow"),
            _ => panic!("wrong error"),
        }
    }

    #[test]
    fn positional_str_returns_string_at_index() {
        let args = [os("prog"), os("value")];
        let result = positional_str(&args, 1, "param");
        assert_eq!(result.unwrap(), "value");
    }

    #[test]
    fn positional_str_returns_error_when_missing() {
        let args = [os("prog")];
        let result = positional_str(&args, 1, "param");
        assert!(result.is_err());
    }

    #[test]
    fn named_flag_returns_value_for_matching_flag() {
        let args = [os("--db"), os("/tmp/db")];
        let result = named_flag(&args, "--db");
        assert_eq!(result.unwrap(), "/tmp/db");
    }

    #[test]
    fn named_flag_returns_none_for_missing_flag() {
        let args = [os("--other"), os("val")];
        let result = named_flag(&args, "--db");
        assert!(result.is_none());
    }

    #[test]
    fn named_flag_returns_none_when_flag_is_last() {
        let args = [os("--db")];
        let result = named_flag(&args, "--db");
        assert!(result.is_none());
    }

    #[test]
    fn find_positional_skips_named_flags() {
        let args = [os("prog"), os("--emit"), os("yaml"), os("file.yaml")];
        let result = find_positional(&args, 1);
        assert_eq!(result.unwrap(), PathBuf::from("file.yaml"));
    }

    #[test]
    fn find_positional_returns_none_when_only_flags() {
        let args = [os("prog"), os("--emit"), os("yaml")];
        let result = find_positional(&args, 1);
        assert!(result.is_none());
    }

    #[test]
    fn find_positional_returns_first_non_flag() {
        let args = [os("prog"), os("file.yaml"), os("--emit"), os("yaml")];
        let result = find_positional(&args, 1);
        assert_eq!(result.unwrap(), PathBuf::from("file.yaml"));
    }
}
