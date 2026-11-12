use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct Finding {
    kind: &'static str,
    path: String,
    detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vcs {
    Git,
    Jj,
}

#[derive(Debug, Clone)]
struct RootInfo {
    path: PathBuf,
    vcs: Vcs,
}

#[derive(Debug, Clone)]
struct ChangedEntry {
    status: u8,
    old_path: String,
    new_path: String,
}

#[derive(Debug, Clone)]
struct RenamePair {
    old_path: String,
    new_path: String,
}

#[derive(Debug)]
struct JustificationRecord {
    base_commit: String,
    old_path: String,
    old_blob: String,
    new_path: String,
    new_blob: String,
    finding_kind: String,
    finding_detail: String,
    replacement_paths: Vec<(String, String)>,
    review_artifact_path: String,
    review_artifact_blob: String,
    bead_id: String,
}

const JUSTIFICATIONS_REL: &str = "scripts/test-integrity-justifications.tsv";

use check_test_integrity_apply::apply_justifications;
use check_test_integrity_bypass_tests::{
    run_justification_bypass_tests, run_justification_lifecycle_tests,
};
use check_test_integrity_cases::{
    run_delete_case, run_ignore_case, run_relocate_case, run_rename_out_case,
    run_silent_return_case, run_strengthen_case, run_weaken_case,
};
use check_test_integrity_cases_rename::{
    run_binary_with_test_delete_case, run_non_utf8_relevant_case, run_relocate_invalid_utf8_case,
    run_relocate_modified_retained_case, run_relocate_spaces_case, run_relocate_weakened_case,
    run_unrelated_binary_pass_case,
};
use check_test_integrity_diff::{changed_files, rename_pairs};
use check_test_integrity_justification_tests::run_justification_tests;
use check_test_integrity_roots::{
    default_base, resolve_base_hash, root_dir, validate_base_revision,
};
use check_test_integrity_scan::{scan_changed_entry, scan_rename_pair};

mod check_test_integrity_apply;
mod check_test_integrity_binary;
mod check_test_integrity_bypass_tests;
mod check_test_integrity_cases;
mod check_test_integrity_cases_rename;
mod check_test_integrity_commands;
mod check_test_integrity_diff;
mod check_test_integrity_fixture;
mod check_test_integrity_justification_tests;
mod check_test_integrity_justifications;
mod check_test_integrity_paths;
mod check_test_integrity_roots;
mod check_test_integrity_scan;

fn collect_findings(root: &Path, base: &str, vcs: Vcs) -> Result<Vec<Finding>, String> {
    validate_base_revision(root, base, vcs)?;
    let entries = changed_files(root, base, vcs)?;
    let renames = rename_pairs(root, base, vcs)?;
    let rename_old: BTreeSet<String> =
        renames.iter().map(|pair| pair.old_path.clone()).collect();
    let rename_new: BTreeSet<String> =
        renames.iter().map(|pair| pair.new_path.clone()).collect();

    let mut findings = Vec::new();
    for pair in &renames {
        findings.extend(scan_rename_pair(root, base, pair, vcs)?);
    }
    for entry in &entries {
        let participates_in_rename = match entry.status {
            b'D' => rename_old.contains(&entry.old_path),
            b'A' | b'M' => rename_new.contains(&entry.new_path),
            _ => false,
        };
        if participates_in_rename {
            continue;
        }
        findings.extend(scan_changed_entry(root, base, entry, vcs)?);
    }
    Ok(findings)
}

// ---------------------------------------------------------------------------
// Top-level check
// ---------------------------------------------------------------------------

fn check(root: &Path, base: &str, vcs: Vcs) -> Result<i32, String> {
    let resolved = resolve_base_hash(root, base, vcs)?;
    let findings = collect_findings(root, base, vcs)?;
    let (justified, remaining) = apply_justifications(root, &resolved, &findings)?;
    for (finding, bead_id) in &justified {
        println!(
            "JUSTIFIED|{}|{}|{}|bead={bead_id}",
            finding.kind, finding.path, finding.detail
        );
    }
    if remaining.is_empty() {
        println!("test integrity: PASS base={base}");
        Ok(0)
    } else {
        eprintln!("test integrity: FAIL");
        for finding in &remaining {
            eprintln!("{}|{}|{}", finding.kind, finding.path, finding.detail);
        }
        eprintln!("Add equal-or-stronger replacement coverage or bead-linked justification.");
        Ok(1)
    }
}

// ---------------------------------------------------------------------------
// Self-test entry point
// ---------------------------------------------------------------------------

fn self_test() -> i32 {
    let workspace_path = "crates/workspace_tests/tests/behavior.rs";
    let module_rs_path = "crates/vb_runtime/src/action/tests.rs";
    let module_child_path = "crates/vb_runtime/src/action/tests/behavior.rs";

    let mut all = true;
    all &= run_delete_case("delete", workspace_path);
    all &= run_ignore_case("ignore", workspace_path);
    all &= run_weaken_case("weaken", workspace_path);
    all &= run_strengthen_case("strengthen", workspace_path);
    all &= run_silent_return_case("silent-return", workspace_path);
    all &= run_delete_case("module-rs-delete", module_rs_path);
    all &= run_weaken_case("module-dir-weaken", module_child_path);
    all &= run_rename_out_case("rename-out");
    all &= run_relocate_case("relocate-identical");
    all &= run_relocate_modified_retained_case("relocate-modified-retained");
    all &= run_relocate_weakened_case("relocate-weakened");
    all &= run_relocate_spaces_case("relocate-spaces");
    all &= run_relocate_invalid_utf8_case("relocate-invalid-utf8");
    all &= run_delete_case(
        "space-delete",
        "crates/workspace_tests/tests/behavior with spaces.rs",
    );
    all &= run_non_utf8_relevant_case("non-utf8-relevant");
    all &= run_unrelated_binary_pass_case("unrelated-binary-passes");
    all &= run_binary_with_test_delete_case("binary-with-test-delete");
    all &= run_justification_tests();
    all &= run_justification_bypass_tests();
    all &= run_justification_lifecycle_tests();

    if all { 0 } else { 1 }
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

fn argument_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find_map(|window| {
        let first = window.first()?;
        let second = window.get(1)?;
        (first == flag).then(|| second.clone())
    })
}

fn run() -> i32 {
    let args = std::env::args().collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--self-test") {
        return self_test();
    }
    let root = match root_dir() {
        Ok(info) => info,
        Err(error) => {
            eprintln!("{error}");
            return 64;
        }
    };
    let base =
        argument_value(&args, "--base").unwrap_or_else(|| default_base(&root.path, root.vcs));
    match check(&root.path, &base, root.vcs) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("test integrity: ERROR {error}");
            2
        }
    }
}

fn main() {
    std::process::exit(run());
}
