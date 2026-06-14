//! Source-of-truth #3: `parse_args` dispatch.
//!
//! The `parse_args` function is defined at
//! `crates/vb_cli/src/args/shared.rs:210-254` and dispatches on the
//! subcommand token. This module exposes a static list of the canonical
//! dispatch tokens and a `parses_as_subcommand` helper that invokes
//! `parse_args` and checks the result is not an `UnknownCommand` error.

#![forbid(unsafe_code)]

use std::ffi::OsString;

use crate::args::error::ParseError;
use crate::args::shared::parse_args;

/// Canonical single-token subcommand names dispatched by `parse_args`.
///
/// Source of truth: the match arms in `crates/vb_cli/src/args/shared.rs:216-254`.
pub const DISPATCH_TOKENS: &[&str] = &[
    "help",
    "version",
    "agent-context",
    "ai-context",
    "status",
    "system",
    "action",
    "verify",
    "validate",
    "explain",
    "compile",
    "run",
    "run-compiled",
    "ipc-serve",
    "inspect",
    "events",
    "replay",
    "trace",
    "retry",
    "resume",
    "bench-run",
    "doctor",
    "answer",
    "graph",
    "diff",
    "incident",
    "simulate",
    "submit",
    "cancel",
];

/// Returns true iff `parse_args(["prog", token])` recognizes `token` as a
/// subcommand. Recognition means the dispatch arm was reached — the call
/// may still fail with a `MissingArgument` or similar error for subcommands
/// that require additional positional arguments, but it must NOT fail with
/// `ParseError::UnknownCommand`.
#[must_use]
pub fn parses_as_subcommand(token: &str) -> bool {
    let args: Vec<OsString> = vec![OsString::from("prog"), OsString::from(token)];
    match parse_args(&args) {
        Ok(_) => true,
        Err(ParseError::UnknownCommand(_)) => false,
        Err(_) => true,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_token_count_is_at_least_twenty_nine() {
        assert!(
            DISPATCH_TOKENS.len() >= 29,
            "DISPATCH_TOKENS must list at least 29 tokens, got {}",
            DISPATCH_TOKENS.len()
        );
    }

    #[test]
    fn parse_args_recognizes_every_dispatch_token() {
        for token in DISPATCH_TOKENS {
            assert!(
                parses_as_subcommand(token),
                "parse_args did not recognize dispatch token '{token}'"
            );
        }
    }

    #[test]
    fn parse_args_rejects_unknown_tokens() {
        assert!(!parses_as_subcommand("definitely-not-a-command"));
        assert!(!parses_as_subcommand("xyzzy"));
    }
}
