//! Source-of-truth #1: `Command` enum variant count.
//!
//! The `Command` enum is defined at `crates/vb_cli/src/args/types.rs:69-218`
//! with exactly 30 variants. This module encodes 30 as a `pub const` and
//! provides a compile-time exhaustiveness guard via a `const fn` `match` over
//! every variant.

#![forbid(unsafe_code)]

use crate::args::types::Command;

/// Number of variants in the `Command` enum.
///
/// Source of truth: `crates/vb_cli/src/args/types.rs:69-218`.
pub const VARIANT_COUNT: usize = 30;

/// Compile-time exhaustive `match` over every `Command` variant.
///
/// If a 31st variant is added to the enum without updating this match,
/// compilation fails with a non-exhaustive match error. The `&Command`
/// reference means we never have to construct a value at the const-eval
/// call site.
const fn classify(c: &Command) -> usize {
    match c {
        Command::Help => 0,
        Command::Version => 1,
        Command::AgentContext { .. } => 2,
        Command::AiContext { .. } => 3,
        Command::Status { .. } => 4,
        Command::SystemStatus { .. } => 5,
        Command::ActionList { .. } => 6,
        Command::ActionInspect { .. } => 7,
        Command::Verify { .. } => 8,
        Command::Validate { .. } => 9,
        Command::Compile { .. } => 10,
        Command::Run { .. } => 11,
        Command::RunCompiled { .. } => 12,
        Command::IpcServe { .. } => 13,
        Command::Inspect { .. } => 14,
        Command::Events { .. } => 15,
        Command::Replay { .. } => 16,
        Command::Trace { .. } => 17,
        Command::Retry { .. } => 18,
        Command::Resume { .. } => 19,
        Command::BenchRun { .. } => 20,
        Command::Doctor { .. } => 21,
        Command::Explain { .. } => 22,
        Command::Answer { .. } => 23,
        Command::Graph { .. } => 24,
        Command::Diff { .. } => 25,
        Command::Incident { .. } => 26,
        Command::Simulate { .. } => 27,
        Command::Submit { .. } => 28,
        Command::Cancel { .. } => 29,
    }
}

/// Returns the constant 30. The `classify` call below is a compile-time
/// guard that forces the exhaustiveness check on the `Command` enum.
pub const fn variant_count() -> usize {
    let _ = classify(&Command::Help);
    VARIANT_COUNT
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::args::types::{
        ActionRegistryMode, DiffMode, DurabilityMode, EmitTarget, OutputFormat, StatusOptions,
        SystemStatusOptions, VerifyProfile,
    };
    use crate::commands_journal::TraceFilters;
    use std::path::PathBuf;

    #[test]
    fn variant_count_const_is_thirty() {
        assert_eq!(VARIANT_COUNT, 30);
        assert_eq!(variant_count(), 30);
    }

    #[test]
    fn classify_assigns_thirty_distinct_indices() {
        let samples: [Command; 30] = [
            Command::Help,
            Command::Version,
            Command::AgentContext { deliver: None },
            Command::AiContext {
                run_id: String::new(),
                db: PathBuf::new(),
                output: OutputFormat::Text,
            },
            Command::Status {
                options: StatusOptions::default(),
                output: OutputFormat::Text,
            },
            Command::SystemStatus {
                options: SystemStatusOptions::default(),
                output: OutputFormat::Text,
            },
            Command::ActionList {
                output: OutputFormat::Text,
                registry: ActionRegistryMode::Registered,
            },
            Command::ActionInspect {
                action_name: vb_core::action::ActionName::new("test")
                    .expect("valid action name"),
                output: OutputFormat::Text,
                registry: ActionRegistryMode::Registered,
            },
            Command::Verify {
                workflow: PathBuf::new(),
                profile: VerifyProfile::Standard,
                output: OutputFormat::Text,
            },
            Command::Validate {
                workflow: PathBuf::new(),
                output: OutputFormat::Text,
            },
            Command::Compile {
                workflow: PathBuf::new(),
                emit: EmitTarget::Ir,
                out: PathBuf::new(),
                output: OutputFormat::Text,
            },
            Command::Run {
                workflow: PathBuf::new(),
                input_bin: PathBuf::new(),
                durability: DurabilityMode::None,
                db: None,
                step: None,
                output: OutputFormat::Text,
            },
            Command::RunCompiled {
                workflow: PathBuf::new(),
                input_bin: PathBuf::new(),
                durability: DurabilityMode::None,
                db: None,
                output: OutputFormat::Text,
            },
            Command::IpcServe {
                socket: PathBuf::new(),
                db: PathBuf::new(),
            },
            Command::Inspect {
                run_id: String::new(),
                db: PathBuf::new(),
                output: OutputFormat::Text,
            },
            Command::Events {
                run_id: String::new(),
                db: PathBuf::new(),
                output: OutputFormat::Text,
                status: None,
                limit: None,
            },
            Command::Replay {
                run_id: String::new(),
                db: PathBuf::new(),
                output: OutputFormat::Text,
            },
            Command::Trace {
                run_id: String::new(),
                db: PathBuf::new(),
                output: OutputFormat::Text,
                filters: TraceFilters::default(),
            },
            Command::Retry {
                run_id: String::new(),
                step: None,
                db: PathBuf::new(),
                output: OutputFormat::Text,
            },
            Command::Resume {
                run_id: String::new(),
                db: PathBuf::new(),
                output: OutputFormat::Text,
            },
            Command::BenchRun {
                workflow: PathBuf::new(),
                output: OutputFormat::Text,
            },
            Command::Doctor {
                db: None,
                output: OutputFormat::Text,
            },
            Command::Explain {
                workflow: PathBuf::new(),
                output: OutputFormat::Text,
            },
            Command::Answer {
                run_id: String::new(),
                slot: 0,
                value: PathBuf::new(),
                db: PathBuf::new(),
                output: OutputFormat::Text,
            },
            Command::Graph {
                workflow: PathBuf::new(),
                output: OutputFormat::Text,
            },
            Command::Diff {
                diff_mode: DiffMode::WorkflowAgainst {
                    workflow: PathBuf::new(),
                    against: PathBuf::new(),
                },
                output: OutputFormat::Text,
            },
            Command::Incident {
                run_id: String::new(),
                db: PathBuf::new(),
                output: OutputFormat::Text,
            },
            Command::Simulate {
                workflow: PathBuf::new(),
                output: OutputFormat::Text,
            },
            Command::Submit {
                workflow: PathBuf::new(),
                input_bin: PathBuf::new(),
                db: PathBuf::new(),
                durability: DurabilityMode::None,
                output: OutputFormat::Text,
            },
            Command::Cancel {
                run_id: String::new(),
                db: PathBuf::new(),
                reason: None,
                output: OutputFormat::Text,
            },
        ];

        let mut seen = std::collections::BTreeSet::new();
        for cmd in &samples {
            let idx = classify(cmd);
            assert!(idx < 30, "classify returned out-of-bounds index {idx}");
            assert!(seen.insert(idx), "classify returned duplicate index {idx}");
        }
        assert_eq!(seen.len(), 30);
    }
}
