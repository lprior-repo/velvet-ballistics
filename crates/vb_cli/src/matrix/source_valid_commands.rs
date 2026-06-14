//! Source-of-truth #2: `VALID_COMMANDS` const string.
//!
//! The `VALID_COMMANDS` const is defined at
//! `crates/vb_cli/src/args/types.rs:232` as a comma-separated list of CLI
//! subcommand tokens. This module exposes the raw const and a
//! `valid_commands_count` function that splits the const on `,` and counts
//! the resulting tokens.

#![forbid(unsafe_code)]

use crate::args::types::VALID_COMMANDS;

/// The raw, comma-separated list of valid subcommand tokens.
pub const VALID_COMMANDS_RAW: &str = VALID_COMMANDS;

/// Returns the number of comma-separated tokens in `VALID_COMMANDS`.
///
/// Source of truth: `crates/vb_cli/src/args/types.rs:232`.
#[must_use]
pub fn valid_commands_count() -> usize {
    VALID_COMMANDS
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .count()
}

/// Returns true iff `token` appears in `VALID_COMMANDS` as a comma-separated
/// token. Whitespace around the candidate is trimmed during comparison.
#[must_use]
pub fn valid_commands_contains(token: &str) -> bool {
    VALID_COMMANDS
        .split(',')
        .map(str::trim)
        .any(|candidate| candidate == token)
}

/// Returns the canonical single-token forms (no multi-word forms like
/// `system status` or `action list`) recognized by `VALID_COMMANDS`.
#[must_use]
pub fn single_token_commands() -> Vec<&'static str> {
    VALID_COMMANDS
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn valid_commands_count_meets_master_section_33_3_baseline() {
        // Master §33.1 lists 30 command shapes. The single-token const
        // `VALID_COMMANDS` has 29 entries; the 30th is the multi-word
        // `system status` invocation handled by `parse_system` after the
        // initial `system` token. The contract requires at least 29
        // single-token forms so the conftest recognizes all canonical
        // single-word subcommands.
        let count = valid_commands_count();
        assert!(
            count >= 29,
            "VALID_COMMANDS must list at least 29 single-token forms, got {count}"
        );
    }

    #[test]
    fn valid_commands_contains_recognizes_all_canonical_single_tokens() {
        for token in [
            "help",
            "version",
            "agent-context",
            "ai-context",
            "status",
            "system",
            "action",
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
            "bench-run",
            "doctor",
            "answer",
            "graph",
            "diff",
            "incident",
            "submit",
            "simulate",
            "cancel",
        ] {
            assert!(
                valid_commands_contains(token),
                "VALID_COMMANDS missing canonical single-token subcommand '{token}'"
            );
        }
    }
}
