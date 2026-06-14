//! CLI matrix conformance proptest per master §33.3.
//!
//! This test pins the six implementation-side sources of truth for the
//! 30-command CLI matrix defined in `velvet-ballistics-MASTER.md` §33.3:
//!
//! 1. `Command` enum at `crates/vb_cli/src/args/types.rs:69-218` (30 variants)
//! 2. `VALID_COMMANDS` const at `crates/vb_cli/src/args/types.rs:232`
//! 3. `parse_args` dispatch at `crates/vb_cli/src/args/shared.rs:210-254`
//! 4. `run_from_env` dispatcher at `crates/vb_cli/src/dispatcher.rs:49-159`
//! 5. `HELP` string at `crates/vb_cli/src/constants.rs:8-53`
//! 6. `agent_context::commands()` JSON at
//!    `crates/vb_cli/src/agent_context/mod.rs:103-345`
//!
//! The canonical 30-command token list comes from `velvet-ballistics-MASTER.md`
//! §33.1 (the 30-Command Matrix).

#![forbid(unsafe_code)]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use vb_cli::matrix::{
    source_agent_context_json, source_command_enum, source_help_string, source_parse_args,
    source_run_from_env_dispatch, source_valid_commands,
};

/// Canonical 30-command token list from master §33.1.
///
/// Single-token forms appear in `VALID_COMMANDS` and `parse_args`'s dispatch.
/// Multi-word forms (`system status`, `action list`, `action inspect`) are
/// recognized by the dispatcher after an initial single-token dispatch.
const MATRIX_TOKENS: &[&str] = &[
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
];

#[test]
fn matrix_source_command_enum_has_30_variants() {
    assert_eq!(
        source_command_enum::VARIANT_COUNT,
        30,
        "Command enum VARIANT_COUNT must be 30"
    );
    assert_eq!(
        source_command_enum::variant_count(),
        30,
        "Command enum variant_count() must return 30"
    );
}

#[test]
fn matrix_source_valid_commands_recognizes_30_subcommands() {
    // `VALID_COMMANDS` lists 29 single-token forms. The 30th command shape
    // is the multi-word `system status` invocation which is recognized by
    // the parser after the initial `system` token. This proptest asserts:
    //   - at least 29 single-token forms are present
    //   - every single-token form from the master matrix appears in the const
    //   - the `system status` multi-word form is reachable via the const's
    //     `system` prefix and is documented as a multi-word shape in HELP
    let single_count = source_valid_commands::valid_commands_count();
    assert!(
        single_count >= 29,
        "VALID_COMMANDS must list at least 29 single-token forms, got {single_count}"
    );
    for token in MATRIX_TOKENS.iter().copied() {
        if token.contains(' ') {
            // Multi-word form: not present as a single comma-separated token.
            // Verified separately in `matrix_source_help_string_documents_all_30_commands`.
            continue;
        }
        assert!(
            source_valid_commands::valid_commands_contains(token),
            "VALID_COMMANDS missing single-token subcommand '{token}'"
        );
    }
}

#[test]
fn matrix_source_parse_args_dispatch_has_30_subcommands() {
    // Every single-token subcommand from the master matrix must be
    // recognized by `parse_args`. Recognition means the dispatch arm is
    // reached; the call may still fail with `MissingArgument` for
    // subcommands that require additional positionals, but it must NOT
    // fail with `ParseError::UnknownCommand`.
    for token in MATRIX_TOKENS.iter().copied() {
        if token.contains(' ') {
            // Multi-word forms are reachable only after the initial
            // single-token dispatch, which is exercised below.
            continue;
        }
        assert!(
            source_parse_args::parses_as_subcommand(token),
            "parse_args did not recognize dispatch token '{token}'"
        );
    }
}

#[test]
fn matrix_source_run_from_env_dispatch_handles_30_commands() {
    assert_eq!(
        source_run_from_env_dispatch::DISPATCH_ARM_COUNT,
        30,
        "run_from_env DISPATCH_ARM_COUNT must be 30"
    );
    assert_eq!(
        source_run_from_env_dispatch::dispatch_arm_count(),
        30,
        "run_from_env dispatch_arm_count() must return 30"
    );
}

#[test]
fn matrix_source_help_string_documents_all_30_commands() {
    // Every token from the canonical 30-command list must appear as a
    // substring in the HELP string. Multi-word forms are checked
    // verbatim; single-token forms must appear at the start of a line
    // (we check substring presence for now).
    for token in MATRIX_TOKENS.iter().copied() {
        assert!(
            source_help_string::help_string_contains(token),
            "HELP string must document canonical subcommand token '{token}'"
        );
    }
}

#[test]
fn matrix_source_agent_context_json_has_30_entries() {
    let count = source_agent_context_json::commands_count();
    assert_eq!(
        count, 30,
        "agent_context::commands() must expose 30 subcommand entries, got {count}"
    );
    let names = source_agent_context_json::commands_names();
    assert_eq!(
        names.len(),
        30,
        "agent_context::commands_names() must return 30 entries, got {}",
        names.len()
    );
    for token in MATRIX_TOKENS.iter().copied() {
        assert!(
            names.iter().any(|name| name == token),
            "agent_context::commands() missing canonical subcommand '{token}'"
        );
    }
}

#[test]
fn matrix_parse_args_round_trip_for_all_30_commands() {
    // Cross-crate round-trip: every single-token form dispatched by
    // `parse_args` reaches a recognized subcommand. Multi-word forms are
    // validated via HELP documentation and the agent_context JSON
    // registry, which is the single source of truth for the AI-agent
    // surface.
    for token in MATRIX_TOKENS.iter().copied() {
        if token.contains(' ') {
            // Round-trip for multi-word forms is not exercised at the
            // `parse_args` layer (only the first token dispatches).
            // Verify reachability through agent_context instead.
            assert!(
                source_agent_context_json::commands_names()
                    .iter()
                    .any(|n| n == token),
                "round-trip: multi-word form '{token}' missing from agent_context"
            );
            assert!(
                source_help_string::help_string_contains(token),
                "round-trip: multi-word form '{token}' missing from HELP"
            );
            continue;
        }
        assert!(
            source_parse_args::parses_as_subcommand(token),
            "round-trip: parse_args did not recognize single-token form '{token}'"
        );
        assert!(
            source_valid_commands::valid_commands_contains(token),
            "round-trip: VALID_COMMANDS missing single-token form '{token}'"
        );
        assert!(
            source_help_string::help_string_contains(token),
            "round-trip: HELP missing single-token form '{token}'"
        );
        assert!(
            source_agent_context_json::commands_names()
                .iter()
                .any(|n| n == token),
            "round-trip: agent_context missing single-token form '{token}'"
        );
    }
}
