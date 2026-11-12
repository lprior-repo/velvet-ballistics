use super::check_test_integrity_binary::{git_cmd, resolve_real_binary};
use super::check_test_integrity_diff::{resolve_base_blob, resolve_current_blob};
use super::check_test_integrity_roots::resolve_base_hash;
use super::{Finding, Vcs, check, collect_findings};
use std::fs;
use std::io;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Self-test infrastructure
// ---------------------------------------------------------------------------

pub(crate) struct Fixture {
    root: PathBuf,
}

impl Fixture {
    pub(crate) fn new(label: &str) -> Result<Fixture, String> {
        let root =
            std::env::temp_dir().join(format!("test-integrity-{}-{label}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).map_err(|error| format!("{label}: create_dir: {error}"))?;
        let git = resolve_real_binary("git")
            .ok_or_else(|| format!("{label}: could not resolve a real git binary"))?;
        git_cmd(&git, &root, &["init", "-q"])?;
        Ok(Fixture { root })
    }

    pub(crate) fn write(&self, rel: &str, content: &str) -> Result<(), String> {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
        }
        fs::write(&path, content).map_err(|error| format!("write {}: {error}", path.display()))
    }

    pub(crate) fn write_bytes(&self, rel: &str, content: &[u8]) -> Result<(), String> {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
        }
        fs::write(&path, content).map_err(|error| format!("write {}: {error}", path.display()))
    }

    pub(crate) fn remove(&self, rel: &str) -> Result<(), String> {
        let path = self.root.join(rel);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("remove {}: {error}", path.display())),
        }
    }

    pub(crate) fn rename(&self, from: &str, to: &str) -> Result<(), String> {
        let src = self.root.join(from);
        let dst = self.root.join(to);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
        }
        fs::rename(&src, &dst).map_err(|error| format!("rename {from} -> {to}: {error}"))
    }

    pub(crate) fn commit_base(&self) -> Result<(), String> {
        let git = resolve_real_binary("git")
            .ok_or_else(|| "could not resolve a real git binary".to_owned())?;
        git_cmd(&git, &self.root, &["add", "."])?;
        git_cmd(
            &git,
            &self.root,
            &[
                "-c",
                "user.name=guard",
                "-c",
                "user.email=guard@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-q",
                "-m",
                "base",
            ],
        )
    }

    pub(crate) fn stage_all(&self) -> Result<(), String> {
        let git = resolve_real_binary("git")
            .ok_or_else(|| "could not resolve a real git binary".to_owned())?;
        git_cmd(&git, &self.root, &["add", "-A"])
    }

    pub(crate) fn collect(&self) -> Result<Vec<Finding>, String> {
        collect_findings(&self.root, "HEAD", Vcs::Git)
    }

    pub(crate) fn check(&self) -> Result<i32, String> {
        check(&self.root, "HEAD", Vcs::Git)
    }

    pub(crate) fn head_hash(&self) -> Result<String, String> {
        resolve_base_hash(&self.root, "HEAD", Vcs::Git)
    }

    pub(crate) fn base_blob(&self, path: &str) -> Result<Option<String>, String> {
        resolve_base_blob(&self.root, "HEAD", path)
    }

    pub(crate) fn current_blob(&self, path: &str) -> Result<Option<String>, String> {
        resolve_current_blob(&self.root, path)
    }
}

fn sorted_finding_keys(findings: &[Finding]) -> Vec<(String, String)> {
    let mut keys: Vec<(String, String)> = findings
        .iter()
        .map(|finding| (finding.kind.to_owned(), finding.path.clone()))
        .collect();
    keys.sort();
    keys
}

pub(crate) fn expect_findings(
    label: &str,
    findings: &[Finding],
    expected: &[(&str, &str)],
) -> bool {
    let mut exp: Vec<(String, String)> = expected
        .iter()
        .map(|(kind, path)| (kind.to_string(), path.to_string()))
        .collect();
    exp.sort();
    let actual = sorted_finding_keys(findings);
    let passed = actual == exp;
    println!(
        "SelfTest:{label}: {} expected={exp:?} actual={actual:?}",
        if passed { "PASS" } else { "FAIL" }
    );
    passed
}

pub(crate) fn report_failure(label: &str, message: &str) -> bool {
    eprintln!("SelfTest:{label}: {message}");
    false
}

// ---------------------------------------------------------------------------
// Core diff self-tests
// ---------------------------------------------------------------------------

pub(crate) const BASE_TEXT: &str = "#[test]\nfn behavior_test() {\n    assert_eq!(2 + 2, 4);\n}\n";

// Large-enough test bodies so `git --find-renames` detects modified renames
// (small files fall below git's rename-similarity size floor).
pub(crate) fn large_test_text() -> String {
    let mut out = String::from("#[test]\nfn behavior_test() {\n");
    for i in 0..30 {
        out.push_str(&format!("    let value_{i}: u32 = {i};\n"));
    }
    out.push_str("    assert_eq!(2 + 2, 4);\n}\n");
    out
}

pub(crate) fn large_test_weakened() -> String {
    let mut out = String::from("#[test]\nfn behavior_test() {\n");
    for i in 0..30 {
        out.push_str(&format!("    let value_{i}: u32 = {i};\n"));
    }
    out.push_str("    assert!((2 + 2).checked_sub(4).is_some());\n}\n");
    out
}

pub(crate) fn large_test_retained_modified() -> String {
    let mut out = String::from("#[test]\n// relocated: coverage retained\nfn behavior_test() {\n");
    for i in 0..30 {
        out.push_str(&format!("    let value_{i}: u32 = {i};\n"));
    }
    out.push_str("    assert_eq!(2 + 2, 4);\n}\n");
    out
}

// ---------------------------------------------------------------------------
// Justification mechanism self-tests
// ---------------------------------------------------------------------------

pub(crate) fn justification_fixture(
    label: &str,
) -> Result<(Fixture, String, String, String, String), String> {
    let fixture = Fixture::new(label)?;
    let test_path = "crates/workspace_tests/tests/behavior.rs";
    let replacement_path = "crates/workspace_tests/tests/replacement_coverage.rs";
    let review_path = ".beads/vb-test-fixture/test-review.md";
    fixture.write(test_path, BASE_TEXT)?;
    fixture.commit_base()?;
    fixture.write(
        test_path,
        "#[test]\nfn behavior_test() {\n    assert!((2 + 2).checked_sub(4).is_some());\n}\n",
    )?;
    fixture.write(
        replacement_path,
        "#[test]\nfn replacement() {\n    assert_eq!(2 + 2, 4);\n}\n",
    )?;
    fixture.write(
        review_path,
        "# Fictional review: weakened assert is replaced by replacement_coverage.rs\n",
    )?;

    let base_blob = fixture
        .base_blob(test_path)?
        .ok_or_else(|| "base blob missing".to_owned())?;
    let new_blob = fixture
        .current_blob(test_path)?
        .ok_or_else(|| "current blob missing".to_owned())?;
    let replacement_blob = fixture
        .current_blob(replacement_path)?
        .ok_or_else(|| "replacement blob missing".to_owned())?;

    Ok((
        fixture,
        test_path.to_owned(),
        base_blob,
        new_blob,
        format!("{replacement_path}:{replacement_blob}"),
    ))
}

// The weak finding's exact detail.
pub(crate) const WEAKEN_DETAIL: &str = "removed_exact=1 added_exact=0 added_weak=1";

pub(crate) fn justification_record(
    base_commit: &str,
    test_path: &str,
    base_blob: &str,
    new_blob: &str,
    replacement_entry: &str,
    review_blob: &str,
) -> String {
    let review_path = ".beads/vb-test-fixture/test-review.md";
    format!(
        "{base_commit}\t{test_path}\t{base_blob}\t{test_path}\t{new_blob}\tWeakenedAssertion\t{WEAKEN_DETAIL}\t{replacement_entry}\t{review_path}\t{review_blob}\tvb-test-fixture\n"
    )
}
