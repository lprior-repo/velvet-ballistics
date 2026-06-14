//! Source-of-truth #5: `HELP` string.
//!
//! The `HELP` const is defined at `crates/vb_cli/src/constants.rs:8-53` and
//! documents the canonical CLI surface. This module exposes the raw string
//! and a `help_string_contains` helper that performs a substring check.

#![forbid(unsafe_code)]

use crate::constants::HELP;

/// The raw `HELP` string from `crates/vb_cli/src/constants.rs:8-53`.
pub const HELP_STRING: &str = HELP;

/// Returns true iff `token` appears as a substring in the `HELP` string.
#[must_use]
pub fn help_string_contains(token: &str) -> bool {
    HELP.contains(token)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn help_string_documents_every_canonical_token() {
        for token in [
            "validate",
            "verify",
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
            "cancel",
            "bench-run",
            "doctor",
            "answer",
            "graph",
            "diff",
            "incident",
            "submit",
            "simulate",
            "ai-context",
            "help",
            "version",
            "agent-context",
            "status",
            "system status",
            "action list",
            "action inspect",
        ] {
            assert!(
                help_string_contains(token),
                "HELP string must document canonical subcommand token '{token}'"
            );
        }
    }

    #[test]
    fn help_string_documents_agent_context_and_filter_shapes() {
        for token in [
            "agent-context [--deliver stdout|file:<absolute-path>|webhook:<url>]",
            "events     <run_id> --db <path> [--status <status>] [--limit <N>] [--emit text|yaml|postcard]",
            "trace      <run_id> --db <path> [--step <N>] [--action <N>] [--status <status>]",
            "retry      <run_id> --db <path> [--step <N>] [--emit text|yaml|postcard]",
            "doctor     [--db <path>] [--emit text|yaml|postcard]",
        ] {
            assert!(
                help_string_contains(token),
                "HELP string must document CLI shape '{token}'"
            );
        }
    }
}
