//! Command derivation for proof obligations.
//!
//! `commands_for_obligation` (proof.rs) derives executable commands from the
//! `required:` block of each registry obligation. The registry names proof
//! artifacts (test names, suite names); these pure helpers translate each
//! name into the CLI invocation that executes it.

/// Derive the crash-lab invocation for one `required: crash_lab:` entry.
/// The registry names the crash test (`action_dispatch_crash_points`); the
/// CLI takes the crash point (`action_dispatch`) and the workflow (`minimal`,
/// the only workflow the crash-lab tool defines).
#[must_use]
pub(crate) fn crash_lab_command(entry: &str) -> String {
    let point = match entry.strip_suffix("_crash_points") {
        Some(stem) => stem,
        None => entry,
    };
    format!("cargo xtask crash-lab --workflow minimal --crash-at {point}")
}

/// Derive the diff-test invocation for one `required: diff_test:` entry.
/// The registry names the suite (`replay_suite`); the CLI takes the suite
/// name (`replay`).
#[must_use]
pub(crate) fn diff_test_command(entry: &str) -> String {
    let suite = match entry.strip_suffix("_suite") {
        Some(stem) => stem,
        None => entry,
    };
    format!("cargo xtask diff-test --suite {suite}")
}

/// Derive the nextest invocation for one required test name in a workspace
/// crate. Shared by `tests`, `property_tests`, and `differential_tests`.
#[must_use]
pub(crate) fn nextest_command(crate_name: &str, test: &str) -> String {
    format!("cargo nextest run -p {crate_name} {test}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof::{ProofObligation, RequiredProof, commands_for_obligation};

    #[test]
    fn crash_lab_command_strips_crash_points_suffix() {
        assert_eq!(
            crash_lab_command("action_dispatch_crash_points"),
            "cargo xtask crash-lab --workflow minimal --crash-at action_dispatch"
        );
    }

    #[test]
    fn crash_lab_command_uses_bare_entry_unchanged() {
        assert_eq!(
            crash_lab_command("action_dispatch"),
            "cargo xtask crash-lab --workflow minimal --crash-at action_dispatch"
        );
    }

    #[test]
    fn diff_test_command_strips_suite_suffix() {
        assert_eq!(
            diff_test_command("replay_suite"),
            "cargo xtask diff-test --suite replay"
        );
    }

    #[test]
    fn diff_test_command_uses_bare_entry_unchanged() {
        assert_eq!(
            diff_test_command("replay"),
            "cargo xtask diff-test --suite replay"
        );
    }

    #[test]
    fn nextest_command_uses_package_and_exact_name() {
        assert_eq!(
            nextest_command("vb_core", "whole_workflow_budget_within_policy"),
            "cargo nextest run -p vb_core whole_workflow_budget_within_policy"
        );
    }

    #[test]
    fn commands_for_obligation_emits_all_four_new_legs() {
        let obligation = ProofObligation {
            id: "VB-REPLAY-001".to_string(),
            section: vec![49, 68],
            statement: "Journal-before-dispatch".to_string(),
            proof_level: "L3".to_string(),
            crate_name: "vb_runtime".to_string(),
            files: vec!["crates/vb_runtime/src/journal.rs".to_string()],
            required: RequiredProof {
                verus: None,
                kani: None,
                tests: vec!["journal_before_dispatch".to_string()],
                fuzz: None,
                tla: None,
                loom: None,
                property_tests: vec!["whole_workflow_budget_within_policy".to_string()],
                differential_tests: vec!["bytecode_vs_ast".to_string()],
                crash_lab: vec!["action_dispatch_crash_points".to_string()],
                diff_test: vec!["replay_suite".to_string()],
            },
            commands: vec![],
        };
        assert_eq!(
            commands_for_obligation(&obligation),
            vec![
                "cargo nextest run -p vb_runtime journal_before_dispatch".to_string(),
                "cargo nextest run -p vb_runtime whole_workflow_budget_within_policy".to_string(),
                "cargo nextest run -p vb_runtime bytecode_vs_ast".to_string(),
                "cargo xtask crash-lab --workflow minimal --crash-at action_dispatch".to_string(),
                "cargo xtask diff-test --suite replay".to_string(),
            ]
        );
    }
}
