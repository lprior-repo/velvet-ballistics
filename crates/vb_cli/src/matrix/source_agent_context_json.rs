//! Source-of-truth #6: `agent_context::commands()` JSON.
//!
//! The `commands` function is defined at
//! `crates/vb_cli/src/agent_context/mod.rs:103-345` and returns a JSON
//! `Value::Object` whose top-level keys are the 30 canonical subcommand
//! names. This module calls the public `agent_context::build` entry point
//! and counts the `commands` object keys.

#![forbid(unsafe_code)]

use crate::agent_context;

/// Returns the number of subcommand entries in `agent_context::commands()`.
///
/// Source of truth: `crates/vb_cli/src/agent_context/mod.rs:103-345`.
/// The JSON has 30 top-level keys (one per `Command` variant including
/// multi-word forms like `system status`, `action list`, `action inspect`).
#[must_use]
pub fn commands_count() -> usize {
    let value = build("velvet-ballistics/v1");
    let commands = value.get("commands");
    match commands {
        Some(serde_json::Value::Object(map)) => map.len(),
        _ => 0,
    }
}

/// Returns the canonical subcommand names exposed in `agent_context::commands()`.
#[must_use]
pub fn commands_names() -> Vec<String> {
    let value = build("velvet-ballistics/v1");
    match value.get("commands") {
        Some(serde_json::Value::Object(map)) => {
            let mut names: Vec<String> = map.keys().cloned().collect();
            names.sort();
            names
        }
        _ => Vec::new(),
    }
}

fn build(version: &str) -> serde_json::Value {
    agent_context::build(version)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn commands_count_is_thirty() {
        let count = commands_count();
        assert_eq!(count, 30, "agent_context::commands() must have 30 entries, got {count}");
    }

    #[test]
    fn commands_names_contain_every_canonical_token() {
        let names = commands_names();
        for token in [
            "agent-context",
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
            "help",
            "version",
            "status",
            "system status",
            "action list",
            "action inspect",
            "ai-context",
        ] {
            assert!(
                names.iter().any(|name| name == token),
                "agent_context::commands() missing canonical subcommand '{token}'"
            );
        }
    }
}
