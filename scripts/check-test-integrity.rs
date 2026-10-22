use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

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

// ---------------------------------------------------------------------------
// Command helpers
// ---------------------------------------------------------------------------

fn tool_output_bytes(tool: &str, args: &[&str], cwd: &Path) -> Result<Vec<u8>, String> {
    let output = Command::new(tool)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("{tool} {} failed to start: {error}", args.join(" ")))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("{tool} {} failed: {stderr}", args.join(" ")))
    }
}

fn tool_output(tool: &str, args: &[&str], cwd: &Path) -> Result<String, String> {
    let bytes = tool_output_bytes(tool, args, cwd)?;
    String::from_utf8(bytes)
        .map_err(|error| format!("{tool} {} returned non-UTF8 stdout: {error}", args.join(" ")))
}

fn tool_output_allow_fail_bytes(tool: &str, args: &[&str], cwd: &Path) -> Option<Vec<u8>> {
    Command::new(tool)
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| output.stdout)
}

fn tool_output_allow_fail(tool: &str, args: &[&str], cwd: &Path) -> Option<String> {
    tool_output_allow_fail_bytes(tool, args, cwd)
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

fn git_output(args: &[&str], cwd: &Path) -> Result<String, String> {
    tool_output("git", args, cwd)
}

fn git_output_bytes(args: &[&str], cwd: &Path) -> Result<Vec<u8>, String> {
    tool_output_bytes("git", args, cwd)
}

fn git_allow_fail(args: &[&str], cwd: &Path) -> Option<String> {
    tool_output_allow_fail("git", args, cwd)
}

fn git_allow_fail_bytes(args: &[&str], cwd: &Path) -> Option<Vec<u8>> {
    tool_output_allow_fail_bytes("git", args, cwd)
}

fn jj_output(args: &[&str], cwd: &Path) -> Result<String, String> {
    tool_output("jj", args, cwd)
}

fn jj_output_bytes(args: &[&str], cwd: &Path) -> Result<Vec<u8>, String> {
    tool_output_bytes("jj", args, cwd)
}

fn jj_allow_fail_bytes(args: &[&str], cwd: &Path) -> Option<Vec<u8>> {
    tool_output_allow_fail_bytes("jj", args, cwd)
}

// ---------------------------------------------------------------------------
// Real binary resolution (self-test temp commits bypass the guard wrapper)
// ---------------------------------------------------------------------------

fn is_elf(path: &Path) -> bool {
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut magic = [0u8; 4];
    if file.read_exact(&mut magic).is_err() {
        return false;
    }
    magic == [0x7f, b'E', b'L', b'F']
}

fn resolve_real_binary(name: &str) -> Option<PathBuf> {
    let mut fallback: Option<PathBuf> = None;
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(name);
            if !candidate.is_file() {
                continue;
            }
            let canonical = fs::canonicalize(&candidate).unwrap_or_else(|_| candidate.clone());
            if fallback.is_none() {
                fallback = Some(canonical.clone());
            }
            if is_elf(&canonical) {
                return Some(canonical);
            }
        }
    }
    for dir in ["/usr/bin", "/usr/local/bin", "/opt/homebrew/bin"] {
        let candidate = Path::new(dir).join(name);
        if is_elf(&candidate) {
            return Some(candidate);
        }
    }
    fallback
}

fn git_cmd(git: &Path, cwd: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new(git)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("git {} failed to start: {error}", args.join(" ")))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("git {} failed: {stderr}", args.join(" ")))
    }
}

// ---------------------------------------------------------------------------
// Root / base resolution
// ---------------------------------------------------------------------------

fn root_dir() -> Result<RootInfo, String> {
    if let Ok(text) = git_output(&["rev-parse", "--show-toplevel"], Path::new(".")) {
        return Ok(RootInfo {
            path: PathBuf::from(text.trim()),
            vcs: Vcs::Git,
        });
    }
    jj_output(&["workspace", "root"], Path::new(".")).map(|text| RootInfo {
        path: PathBuf::from(text.trim()),
        vcs: Vcs::Jj,
    })
}

fn default_base(root: &Path, vcs: Vcs) -> String {
    if let Ok(value) = std::env::var("TEST_INTEGRITY_BASE") {
        if !value.trim().is_empty() {
            return value;
        }
    }
    if vcs == Vcs::Jj {
        return "@-".to_owned();
    }
    let dirty = git_output(&["status", "--porcelain"], root)
        .map(|text| !text.trim().is_empty())
        .unwrap_or(true);
    if dirty {
        return "HEAD".to_owned();
    }
    git_allow_fail(&["merge-base", "origin/main", "HEAD"], root)
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "HEAD".to_owned())
}

fn validate_base_revision(root: &Path, base: &str, vcs: Vcs) -> Result<(), String> {
    if base.trim().is_empty() {
        return Err("empty base revision".to_owned());
    }
    match vcs {
        Vcs::Git => {
            let commit = format!("{base}^{{commit}}");
            git_output(&["rev-parse", "--verify", &commit], root).map(|_| ())
        }
        Vcs::Jj => jj_output(&["log", "--no-graph", "-r", base, "-T", "commit_id"], root).map(|_| ()),
    }
    .map_err(|error| format!("invalid base revision {base:?}: {error}"))
}

// ---------------------------------------------------------------------------
// Path classification
// ---------------------------------------------------------------------------

fn is_test_path(path: &str) -> bool {
    path.ends_with(".rs")
        && (is_module_test_path(path)
            || [
                "/tests/",
                "/benches/",
                "/examples/",
                "/fuzz/",
                "workspace_tests",
            ]
            .iter()
            .any(|part| format!("/{path}").contains(part)))
}

fn is_behavior_test_path(path: &str) -> bool {
    path.ends_with(".rs")
        && (is_module_test_path(path)
            || ["/tests/", "workspace_tests"]
                .iter()
                .any(|part| format!("/{path}").contains(part)))
}

fn is_module_test_path(path: &str) -> bool {
    if !path.ends_with(".rs") {
        return false;
    }
    let Some(after_src) = path_after_src(path) else {
        return false;
    };
    is_src_tests_rs_path(after_src) || is_src_tests_child_path(after_src)
}

fn path_after_src(path: &str) -> Option<&str> {
    if let Some(after_src) = path.strip_prefix("src/") {
        return Some(after_src);
    }
    path.split_once("/src/")
        .map(|(_prefix, after_src)| after_src)
}

fn is_src_tests_rs_path(after_src: &str) -> bool {
    after_src == "tests.rs" || after_src.ends_with("/tests.rs")
}

fn is_src_tests_child_path(after_src: &str) -> bool {
    let child = if let Some(child) = after_src.strip_prefix("tests/") {
        child
    } else if let Some((_before_tests, child)) = after_src.rsplit_once("/tests/") {
        child
    } else {
        return false;
    };
    !child.is_empty() && child.ends_with(".rs") && !child.contains('/')
}

// ---------------------------------------------------------------------------
// Text signal detection
// ---------------------------------------------------------------------------

fn has_exact_assertion(text: &str) -> bool {
    [
        "assert_eq!(",
        "assert_ne!(",
        "assert_matches!(",
        "assert_json_",
        "insta::assert_",
        "snapshot!(",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn has_weak_assertion(text: &str) -> bool {
    text.contains("assert!(")
        && [
            ".is_ok(",
            ".is_err(",
            ".is_some(",
            ".is_none(",
            ".is_empty(",
        ]
        .iter()
        .any(|needle| text.contains(needle))
}

fn has_test_decl(text: &str) -> bool {
    text.contains("#[test")
        || text.contains("#[tokio::test")
        || text.contains("fn test_")
        || text.contains("_test(")
}

fn has_ignore_or_skip(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("#[ignore")
        || lower.contains("cfg_attr") && lower.contains("ignore")
        || lower.contains("return;")
        || lower.contains(" skipped")
        || lower.contains(" skip")
        || lower.contains("ignored")
}

fn has_compile_only(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "no_run",
        "compile_only",
        "compile-only",
        "smoke only",
        "compile smoke",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

// ---------------------------------------------------------------------------
// Changed-file enumeration
// ---------------------------------------------------------------------------

fn changed_files(root: &Path, base: &str, vcs: Vcs) -> Result<Vec<ChangedEntry>, String> {
    match vcs {
        Vcs::Git => {
            let bytes = git_output_bytes(
                &["diff", "--name-status", "-z", "--no-renames", base, "--"],
                root,
            )?;
            let mut entries = Vec::new();
            let mut segments = bytes.split(|&byte| byte == 0);
            while let Some(status_seg) = segments.next() {
                if status_seg.is_empty() {
                    continue;
                }
                let status = match status_seg.first() {
                    Some(&byte) => byte,
                    None => continue,
                };
                let path_seg = segments.next().ok_or_else(|| {
                    "malformed git name-status output: dangling status byte".to_owned()
                })?;
                if path_seg.is_empty() {
                    continue;
                }
                let path = String::from_utf8(path_seg.to_vec())
                    .map_err(|_| "changed path is not UTF-8".to_owned())?;
                entries.push(ChangedEntry {
                    status,
                    old_path: path.clone(),
                    new_path: path,
                });
            }
            Ok(entries)
        }
        Vcs::Jj => {
            let text = jj_output(&["diff", "--types", "--from", base, "--to", "@"], root)?;
            let mut entries = Vec::new();
            for line in text.lines() {
                let trimmed = line.trim();
                let type_code = match trimmed.get(..2) {
                    Some(code) => code,
                    None => continue,
                };
                let path = match trimmed.get(2..).map(str::trim_start) {
                    Some(p) => p,
                    None => continue,
                };
                if path.is_empty() {
                    continue;
                }
                if type_code == "-F" {
                    entries.push(ChangedEntry {
                        status: b'A',
                        old_path: path.to_owned(),
                        new_path: path.to_owned(),
                    });
                } else if type_code == "F-" {
                    entries.push(ChangedEntry {
                        status: b'D',
                        old_path: path.to_owned(),
                        new_path: path.to_owned(),
                    });
                } else if let Some((old, new)) = expand_rename_path(path) {
                    entries.push(ChangedEntry {
                        status: b'D',
                        old_path: old.clone(),
                        new_path: new.clone(),
                    });
                    entries.push(ChangedEntry {
                        status: b'A',
                        old_path: old,
                        new_path: new,
                    });
                } else {
                    entries.push(ChangedEntry {
                        status: b'M',
                        old_path: path.to_owned(),
                        new_path: path.to_owned(),
                    });
                }
            }
            Ok(entries)
        }
    }
}

fn expand_rename_path(path: &str) -> Option<(String, String)> {
    let open = path.find('{')?;
    let close = path.find('}')?;
    if close <= open {
        return None;
    }
    let inner = path.get(open.saturating_add(1)..close)?;
    let (old_part, new_part) = inner.split_once(" => ")?;
    let prefix = path.get(..open)?;
    let suffix = path.get(close.saturating_add(1)..)?;
    Some((
        format!("{prefix}{old_part}{suffix}"),
        format!("{prefix}{new_part}{suffix}"),
    ))
}

// ---------------------------------------------------------------------------
// File content access (raw bytes) and blob resolution
// ---------------------------------------------------------------------------

fn base_file_bytes(root: &Path, base: &str, path: &str, vcs: Vcs) -> Option<Vec<u8>> {
    match vcs {
        Vcs::Git => git_allow_fail_bytes(&["show", &format!("{base}:{path}")], root),
        Vcs::Jj => jj_allow_fail_bytes(&["file", "show", "-r", base, path], root),
    }
}

fn current_file_bytes(root: &Path, path: &str) -> Option<Vec<u8>> {
    fs::read(root.join(path)).ok()
}

fn base_file_has_tests(root: &Path, base: &str, path: &str, vcs: Vcs) -> bool {
    let Some(bytes) = base_file_bytes(root, base, path, vcs) else {
        return false;
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return false;
    };
    has_test_decl(&text) || has_exact_assertion(&text)
}

fn resolve_base_blob(root: &Path, base: &str, path: &str) -> Result<Option<String>, String> {
    let spec = format!("{base}:{path}");
    match git_output(&["rev-parse", "--verify", &spec], root) {
        Ok(text) => {
            let hash = text.trim().to_owned();
            if hash.is_empty() {
                Ok(None)
            } else {
                Ok(Some(hash))
            }
        }
        Err(_) => Ok(None),
    }
}

fn resolve_current_blob(root: &Path, path: &str) -> Result<Option<String>, String> {
    let full = root.join(path);
    if !full.is_file() {
        return Ok(None);
    }
    let full_str = full
        .to_str()
        .ok_or_else(|| format!("non-UTF8 path {}", full.display()))?;
    let text = git_output(&["hash-object", full_str], root)?;
    let hash = text.trim().to_owned();
    if hash.is_empty() {
        return Err("git hash-object returned empty output".to_owned());
    }
    Ok(Some(hash))
}

fn is_hex40(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

// ---------------------------------------------------------------------------
// Diff scanning (per-path, strict UTF-8)
// ---------------------------------------------------------------------------

fn path_diff_bytes(root: &Path, base: &str, path: &str, vcs: Vcs) -> Result<Vec<u8>, String> {
    match vcs {
        Vcs::Git => git_output_bytes(
            &[
                "diff",
                "--no-renames",
                "--no-textconv",
                "--no-ext-diff",
                "--text",
                "--unified=0",
                base,
                "--",
                path,
            ],
            root,
        ),
        Vcs::Jj => jj_output_bytes(&["diff", "--git", "--from", base, "--to", "@", "--", path], root),
    }
}

fn scan_diff(diff: &str, path: &str) -> Vec<Finding> {
    let mut removed_test_decl = 0usize;
    let mut added_test_decl = 0usize;
    let mut removed_exact = 0usize;
    let mut added_exact = 0usize;
    let mut added_weak = 0usize;
    let mut findings = Vec::new();

    for line in diff.lines() {
        if line.starts_with('-') && !line.starts_with("---") {
            if let Some(payload) = line.strip_prefix('-') {
                if has_test_decl(payload) {
                    removed_test_decl += 1;
                }
                if has_exact_assertion(payload) {
                    removed_exact += 1;
                }
            }
        } else if line.starts_with('+') && !line.starts_with("+++") {
            if let Some(payload) = line.strip_prefix('+') {
                if has_test_decl(payload) {
                    added_test_decl += 1;
                }
                if is_behavior_test_path(path) && has_ignore_or_skip(payload) {
                    findings.push(Finding {
                        kind: "IgnoredOrSkippedTest",
                        path: path.to_owned(),
                        detail: payload.trim().to_owned(),
                    });
                }
                if is_behavior_test_path(path) && has_compile_only(payload) {
                    findings.push(Finding {
                        kind: "CompileOnlyReplacement",
                        path: path.to_owned(),
                        detail: payload.trim().to_owned(),
                    });
                }
                if has_exact_assertion(payload) {
                    added_exact += 1;
                }
                if has_weak_assertion(payload) {
                    added_weak += 1;
                }
            }
        }
    }

    if added_test_decl < removed_test_decl {
        findings.push(Finding {
            kind: "DeletedTestDeclaration",
            path: path.to_owned(),
            detail: format!(
                "removed_declarations={removed_test_decl} added_declarations={added_test_decl}"
            ),
        });
    }
    if added_weak > 0 || added_exact < removed_exact {
        findings.push(Finding {
            kind: "WeakenedAssertion",
            path: path.to_owned(),
            detail: format!(
                "removed_exact={removed_exact} added_exact={added_exact} added_weak={added_weak}"
            ),
        });
    }

    findings
}

fn scan_changed_entry(
    root: &Path,
    base: &str,
    entry: &ChangedEntry,
    vcs: Vcs,
) -> Result<Vec<Finding>, String> {
    let path = if entry.status == b'D' {
        entry.old_path.as_str()
    } else {
        entry.new_path.as_str()
    };
    let is_test = is_test_path(path);
    let base_has_tests = base_file_has_tests(root, base, path, vcs);
    if !is_test && !base_has_tests {
        return Ok(Vec::new());
    }

    let base_bytes = base_file_bytes(root, base, path, vcs);
    let current_bytes = if entry.status == b'D' {
        None
    } else {
        current_file_bytes(root, path)
    };
    let base_invalid = base_bytes
        .as_ref()
        .map_or(false, |bytes| std::str::from_utf8(bytes).is_err());
    let current_invalid = current_bytes
        .as_ref()
        .map_or(false, |bytes| std::str::from_utf8(bytes).is_err());
    if base_invalid || current_invalid {
        return Ok(vec![Finding {
            kind: "NonUtf8TestFile",
            path: path.to_owned(),
            detail: "relevant test file contains non-UTF8 bytes".to_owned(),
        }]);
    }

    let diff = path_diff_bytes(root, base, path, vcs)?;
    let text = match String::from_utf8(diff) {
        Ok(text) => text,
        Err(_) => {
            return Ok(vec![Finding {
                kind: "NonUtf8TestFile",
                path: path.to_owned(),
                detail: "relevant test file contains non-UTF8 bytes".to_owned(),
            }]);
        }
    };
    Ok(scan_diff(&text, path))
}

fn scan_deleted_files(
    root: &Path,
    base: &str,
    entries: &[ChangedEntry],
    vcs: Vcs,
) -> Vec<Finding> {
    entries
        .iter()
        .filter(|entry| entry.status == b'D')
        .filter(|entry| {
            is_test_path(&entry.old_path) || base_file_has_tests(root, base, &entry.old_path, vcs)
        })
        .map(|entry| Finding {
            kind: "DeletedTestFile",
            path: entry.old_path.clone(),
            detail: "deleted file contained tests or test assertions".to_owned(),
        })
        .collect()
}

fn collect_findings(root: &Path, base: &str, vcs: Vcs) -> Result<Vec<Finding>, String> {
    validate_base_revision(root, base, vcs)?;
    let entries = changed_files(root, base, vcs)?;
    let mut findings = scan_deleted_files(root, base, &entries, vcs);
    for entry in &entries {
        findings.extend(scan_changed_entry(root, base, entry, vcs)?);
    }
    Ok(findings)
}

// ---------------------------------------------------------------------------
// Justifications (hash-bound, exact)
// ---------------------------------------------------------------------------

fn is_code_level_skip_detail(detail: &str) -> bool {
    let d = detail.trim();
    d.starts_with("#[ignore")
        || d.starts_with("#[cfg_attr")
        || d == "return"
        || d == "return;"
        || d == "skip"
        || d == "skip;"
        || d.starts_with("skip(")
}

fn parse_justifications(text: &str) -> Result<Vec<JustificationRecord>, String> {
    let mut records = Vec::new();
    let mut seen_identities: BTreeSet<(String, String, String, String)> = BTreeSet::new();
    for (line_no, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let columns: Vec<&str> = line.split('\t').collect();
        let [base_commit, old_path, old_blob, new_path, new_blob, finding_kind, finding_detail, replacement_paths_col, review_artifact_path, review_artifact_blob, bead_id] =
            match columns.as_slice() {
                [a, b, c, d, e, f, g, h, i, j, k] => {
                    [a, b, c, d, e, f, g, h, i, j, k]
                }
                _ => {
                    return Err(format!(
                        "justification record {}: expected 11 tab-separated columns, got {}",
                        line_no.saturating_add(1),
                        columns.len()
                    ));
                }
            };
        let base_commit = base_commit.trim().to_owned();
        let old_path = old_path.trim().to_owned();
        let old_blob = old_blob.trim().to_owned();
        let new_path = new_path.trim().to_owned();
        let new_blob = new_blob.trim().to_owned();
        let finding_kind = finding_kind.trim().to_owned();
        let finding_detail = finding_detail.trim().to_owned();
        let replacement_paths_col = replacement_paths_col.trim();
        let review_artifact_path = review_artifact_path.trim().to_owned();
        let review_artifact_blob = review_artifact_blob.trim().to_owned();
        let bead_id = bead_id.trim().to_owned();

        if base_commit.is_empty()
            || old_path.is_empty()
            || finding_kind.is_empty()
            || finding_detail.is_empty()
            || review_artifact_path.is_empty()
            || bead_id.is_empty()
        {
            return Err(format!(
                "justification record {}: empty required field",
                line_no.saturating_add(1)
            ));
        }
        if old_blob != "ABSENT" && !is_hex40(&old_blob) {
            return Err(format!(
                "justification record {}: old_blob must be a 40-hex id or ABSENT",
                line_no.saturating_add(1)
            ));
        }
        if new_blob != "ABSENT" && !is_hex40(&new_blob) {
            return Err(format!(
                "justification record {}: new_blob must be a 40-hex id or ABSENT",
                line_no.saturating_add(1)
            ));
        }
        if !is_hex40(&review_artifact_blob) {
            return Err(format!(
                "justification record {}: review_artifact_blob must be a 40-hex id",
                line_no.saturating_add(1)
            ));
        }

        let replacement_paths = if replacement_paths_col.is_empty() || replacement_paths_col == "-" {
            Vec::new()
        } else {
            let mut paths = Vec::new();
            for pair in replacement_paths_col.split(';') {
                let pair = pair.trim();
                if pair.is_empty() {
                    continue;
                }
                let (path, blob) = pair.rsplit_once(':').ok_or_else(|| {
                    format!(
                        "justification record {}: replacement entry {pair:?} lacks ':' blob separator",
                        line_no.saturating_add(1)
                    )
                })?;
                let path = path.trim();
                let blob = blob.trim();
                if path.is_empty() || !is_hex40(blob) {
                    return Err(format!(
                        "justification record {}: replacement entry {pair:?} is malformed",
                        line_no.saturating_add(1)
                    ));
                }
                paths.push((path.to_owned(), blob.to_owned()));
            }
            paths
        };

        if finding_kind == "NonUtf8TestFile" {
            return Err(format!(
                "justification record {}: NonUtf8TestFile findings cannot be justified",
                line_no.saturating_add(1)
            ));
        }
        if finding_kind == "IgnoredOrSkippedTest"
            && is_code_level_skip_detail(&finding_detail)
            && replacement_paths.is_empty()
        {
            return Err(format!(
                "justification record {}: code-level IgnoredOrSkippedTest requires replacement coverage binding",
                line_no.saturating_add(1)
            ));
        }

        let identity = (
            base_commit.clone(),
            old_path.clone(),
            finding_kind.clone(),
            finding_detail.clone(),
        );
        if !seen_identities.insert(identity) {
            return Err(format!(
                "justification record {}: duplicate record for {finding_kind}|{old_path}",
                line_no.saturating_add(1)
            ));
        }

        records.push(JustificationRecord {
            base_commit,
            old_path,
            old_blob,
            new_path,
            new_blob,
            finding_kind,
            finding_detail,
            replacement_paths,
            review_artifact_path,
            review_artifact_blob,
            bead_id,
        });
    }
    Ok(records)
}

impl JustificationRecord {
    fn matches_identity(&self, base: &str, finding: &Finding) -> bool {
        self.finding_kind == finding.kind
            && self.finding_detail == finding.detail
            && self.old_path == finding.path
            && self.base_commit == base
    }

    fn hashes_valid(&self, root: &Path, base: &str) -> Result<bool, String> {
        let actual_base = resolve_base_blob(root, base, &self.old_path)?;
        if self.old_blob == "ABSENT" {
            if actual_base.is_some() {
                return Ok(false);
            }
        } else if actual_base.as_deref() != Some(self.old_blob.as_str()) {
            return Ok(false);
        }

        let actual_new = if self.new_path == "ABSENT" {
            None
        } else {
            resolve_current_blob(root, &self.new_path)?
        };
        if self.new_blob == "ABSENT" {
            if actual_new.is_some() {
                return Ok(false);
            }
        } else if actual_new.as_deref() != Some(self.new_blob.as_str()) {
            return Ok(false);
        }

        let actual_review = resolve_current_blob(root, &self.review_artifact_path)?;
        if actual_review.as_deref() != Some(self.review_artifact_blob.as_str()) {
            return Ok(false);
        }

        for (path, blob) in &self.replacement_paths {
            let actual = resolve_current_blob(root, path)?;
            if actual.as_deref() != Some(blob.as_str()) {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

fn load_justifications(root: &Path) -> Result<Vec<JustificationRecord>, String> {
    let tsv_path = root.join(JUSTIFICATIONS_REL);
    let text = match fs::read_to_string(&tsv_path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!("cannot read {}: {error}", tsv_path.display()));
        }
    };
    parse_justifications(&text)
}

fn apply_justifications(
    root: &Path,
    base: &str,
    findings: &[Finding],
) -> Result<(Vec<(Finding, String)>, Vec<Finding>), String> {
    let records = load_justifications(root)?;
    if records.is_empty() {
        return Ok((Vec::new(), findings.to_vec()));
    }

    let mut justified: Vec<(Finding, String)> = Vec::new();
    let mut remaining: Vec<Finding> = Vec::new();
    let mut matched_indices: BTreeSet<usize> = BTreeSet::new();

    for finding in findings {
        let mut matched_record: Option<&JustificationRecord> = None;
        for (index, record) in records.iter().enumerate() {
            if !record.matches_identity(base, finding) {
                continue;
            }
            matched_indices.insert(index);
            if record.hashes_valid(root, base)? {
                matched_record = Some(record);
            }
            break;
        }
        match matched_record {
            Some(record) => justified.push((finding.clone(), record.bead_id.clone())),
            None => remaining.push(finding.clone()),
        }
    }

    let unmatched: Vec<&JustificationRecord> = records
        .iter()
        .enumerate()
        .filter(|(index, _)| !matched_indices.contains(index))
        .map(|(_, record)| record)
        .collect();
    if !unmatched.is_empty() {
        let names = unmatched
            .iter()
            .map(|record| {
                format!(
                    "{}|{}|{}",
                    record.finding_kind, record.old_path, record.finding_detail
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "stale justification records reference no matching finding: {names}"
        ));
    }

    Ok((justified, remaining))
}

// ---------------------------------------------------------------------------
// Top-level check
// ---------------------------------------------------------------------------

fn check(root: &Path, base: &str, vcs: Vcs) -> Result<i32, String> {
    let findings = collect_findings(root, base, vcs)?;
    let (justified, remaining) = apply_justifications(root, base, &findings)?;
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
// Self-test infrastructure
// ---------------------------------------------------------------------------

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Result<Fixture, String> {
        let root = std::env::temp_dir().join(format!(
            "test-integrity-{}-{label}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).map_err(|error| format!("{label}: create_dir: {error}"))?;
        let git = resolve_real_binary("git")
            .ok_or_else(|| format!("{label}: could not resolve a real git binary"))?;
        git_cmd(&git, &root, &["init", "-q"])?;
        Ok(Fixture { root })
    }

    fn write(&self, rel: &str, content: &str) -> Result<(), String> {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
        }
        fs::write(&path, content).map_err(|error| format!("write {}: {error}", path.display()))
    }

    fn write_bytes(&self, rel: &str, content: &[u8]) -> Result<(), String> {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
        }
        fs::write(&path, content)
            .map_err(|error| format!("write {}: {error}", path.display()))
    }

    fn remove(&self, rel: &str) -> Result<(), String> {
        let path = self.root.join(rel);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("remove {}: {error}", path.display())),
        }
    }

    fn rename(&self, from: &str, to: &str) -> Result<(), String> {
        let src = self.root.join(from);
        let dst = self.root.join(to);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
        }
        fs::rename(&src, &dst).map_err(|error| format!("rename {from} -> {to}: {error}"))
    }

    fn commit_base(&self) -> Result<(), String> {
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

    fn collect(&self) -> Result<Vec<Finding>, String> {
        collect_findings(&self.root, "HEAD", Vcs::Git)
    }

    fn check(&self) -> Result<i32, String> {
        check(&self.root, "HEAD", Vcs::Git)
    }

    fn base_blob(&self, path: &str) -> Result<Option<String>, String> {
        resolve_base_blob(&self.root, "HEAD", path)
    }

    fn current_blob(&self, path: &str) -> Result<Option<String>, String> {
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

fn expect_findings(label: &str, findings: &[Finding], expected: &[(&str, &str)]) -> bool {
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

fn report_failure(label: &str, message: &str) -> bool {
    eprintln!("SelfTest:{label}: {message}");
    false
}

// ---------------------------------------------------------------------------
// Core diff self-tests
// ---------------------------------------------------------------------------

const BASE_TEXT: &str = "#[test]\nfn behavior_test() {\n    assert_eq!(2 + 2, 4);\n}\n";

fn run_delete_case(label: &str, path: &str) -> bool {
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(path, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| fixture.remove(path));
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected = [
        ("DeletedTestFile", path),
        ("DeletedTestDeclaration", path),
        ("WeakenedAssertion", path),
    ];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=1 actual={exit}");
    findings_ok && exit == 1
}

fn run_ignore_case(label: &str, path: &str) -> bool {
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(path, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| {
            fixture.write(
                path,
                "#[test]\n#[ignore]\nfn behavior_test() {\n    assert_eq!(2 + 2, 4);\n}\n",
            )
        });
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected = [("IgnoredOrSkippedTest", path)];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=1 actual={exit}");
    findings_ok && exit == 1
}

fn run_weaken_case(label: &str, path: &str) -> bool {
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(path, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| {
            fixture.write(
                path,
                "#[test]\nfn behavior_test() {\n    assert!((2 + 2).checked_sub(4).is_some());\n}\n",
            )
        });
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected = [("WeakenedAssertion", path)];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=1 actual={exit}");
    findings_ok && exit == 1
}

fn run_strengthen_case(label: &str, path: &str) -> bool {
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(path, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| {
            fixture.write(
                path,
                "#[test]\nfn behavior_test() {\n    assert_eq!(2 + 2, 4);\n    assert_ne!(2 + 2, 5);\n}\n",
            )
        });
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected: [(&str, &str); 0] = [];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=0 actual={exit}");
    findings_ok && exit == 0
}

fn run_silent_return_case(label: &str, path: &str) -> bool {
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(path, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| {
            fixture.write(
                path,
                "#[test]\nfn behavior_test() {\n    let Ok(value) = Result::<u8, ()>::Ok(4) else {\n        return;\n    };\n    assert_eq!(value, 4);\n}\n",
            )
        });
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected = [("IgnoredOrSkippedTest", path)];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=1 actual={exit}");
    findings_ok && exit == 1
}

fn run_rename_out_case(label: &str) -> bool {
    let from = "crates/workspace_tests/tests/behavior.rs";
    let to = "crates/workspace_tests/archive/behavior.rs";
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(from, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| fixture.rename(from, to));
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected = [
        ("DeletedTestFile", from),
        ("DeletedTestDeclaration", from),
        ("WeakenedAssertion", from),
    ];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=1 actual={exit}");
    findings_ok && exit == 1
}

fn run_non_utf8_relevant_case(label: &str) -> bool {
    let path = "crates/workspace_tests/tests/behavior.rs";
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(path, BASE_TEXT)
        .and_then(|()| fixture.commit_base());
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    // Relevant .rs file with a NUL byte and trailing invalid UTF-8 (0xff 0x00).
    let mutated = [
        0x23u8, 0x5b, 0x74, 0x65, 0x73, 0x74, 0x5d, 0x0a, 0x66, 0x6e, 0x20, 0x62, 0x65, 0x68,
        0x61, 0x76, 0x69, 0x6f, 0x72, 0x5f, 0x74, 0x65, 0x73, 0x74, 0x28, 0x29, 0x20, 0x7b, 0x0a,
        0x20, 0x20, 0x20, 0x20, 0x61, 0x73, 0x73, 0x65, 0x72, 0x74, 0x5f, 0x65, 0x71, 0x28, 0x64,
        0x75, 0x6d, 0x6d, 0x79, 0x29, 0x3b, 0x0a, 0x7d, 0x0a, 0xff, 0x00,
    ];
    if let Err(error) = fixture.write_bytes(path, &mutated) {
        return report_failure(label, &format!("mutate: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected = [("NonUtf8TestFile", path)];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=1 actual={exit}");
    findings_ok && exit == 1
}

fn run_unrelated_binary_pass_case(label: &str) -> bool {
    let path = "crates/workspace_tests/tests/behavior.rs";
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(path, BASE_TEXT)
        .and_then(|()| fixture.commit_base());
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    let binary = [0xffu8, 0xfe, 0x00, 0x01, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85];
    if let Err(error) = fixture.write_bytes(".beads/evidence/proof.dat", &binary) {
        return report_failure(label, &format!("write evidence: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected: [(&str, &str); 0] = [];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=0 actual={exit}");
    findings_ok && exit == 0
}

fn run_binary_with_test_delete_case(label: &str) -> bool {
    let path = "crates/workspace_tests/tests/behavior.rs";
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(path, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| fixture.remove(path));
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    let binary = [0xffu8, 0xfe, 0x00, 0x01, 0x80, 0x81, 0x82, 0x83, 0x84, 0x85];
    if let Err(error) = fixture.write_bytes(".beads/evidence/proof.dat", &binary) {
        return report_failure(label, &format!("write evidence: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected = [
        ("DeletedTestFile", path),
        ("DeletedTestDeclaration", path),
        ("WeakenedAssertion", path),
    ];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=1 actual={exit}");
    findings_ok && exit == 1
}

// ---------------------------------------------------------------------------
// Justification mechanism self-tests
// ---------------------------------------------------------------------------

fn justification_fixture(label: &str) -> Result<(Fixture, String, String, String, String), String> {
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
const WEAKEN_DETAIL: &str = "removed_exact=1 added_exact=0 added_weak=1";

fn justification_record(
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

fn run_justification_tests() -> bool {
    let base_commit = "HEAD";
    let (fixture, test_path, base_blob, new_blob, replacement_entry) =
        match justification_fixture("justify") {
            Ok(values) => values,
            Err(error) => return report_failure("justify", &format!("fixture setup: {error}")),
        };

    let review_path = ".beads/vb-test-fixture/test-review.md";
    let review_blob = match fixture.current_blob(review_path) {
        Ok(Some(blob)) => blob,
        Ok(None) => return report_failure("justify", "review blob missing"),
        Err(error) => return report_failure("justify", &format!("review blob: {error}")),
    };

    // Sanity: the fixture yields exactly the WeakenedAssertion finding.
    let pre = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure("justify", &format!("pre-collect: {error}")),
    };
    if !expect_findings("justify-pre", &pre, &[("WeakenedAssertion", test_path.as_str())]) {
        return false;
    }

    // 1. Valid record passes (finding justified).
    let valid_tsv = justification_record(
        base_commit,
        &test_path,
        &base_blob,
        &new_blob,
        &replacement_entry,
        &review_blob,
    );
    let mut ok = true;
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &valid_tsv) {
        return report_failure("justify-valid", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:justify-valid: check error: {error}");
            2
        }
    };
    println!("SelfTest:justify-valid: {} expected=0 actual={exit}", if exit == 0 { "PASS" } else { "FAIL" });
    ok = ok && exit == 0;

    // 2. Stale current test blob -> finding stays -> FAIL (exit 1).
    let stale_new = "0".repeat(40);
    let stale_test_tsv = justification_record(
        base_commit,
        &test_path,
        &base_blob,
        &stale_new,
        &replacement_entry,
        &review_blob,
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &stale_test_tsv) {
        return report_failure("justify-stale-test", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:justify-stale-test: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-stale-test: {} expected=1 actual={exit}",
        if exit == 1 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 1;

    // 3. Stale base blob -> finding stays -> FAIL (exit 1).
    let stale_base = "0".repeat(40);
    let stale_base_tsv = justification_record(
        base_commit,
        &test_path,
        &stale_base,
        &new_blob,
        &replacement_entry,
        &review_blob,
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &stale_base_tsv) {
        return report_failure("justify-stale-base", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:justify-stale-base: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-stale-base: {} expected=1 actual={exit}",
        if exit == 1 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 1;

    // 4. Stale replacement blob -> finding stays -> FAIL (exit 1).
    let replacement_path = "crates/workspace_tests/tests/replacement_coverage.rs";
    let stale_replacement_entry = format!("{replacement_path}:{}", "0".repeat(40));
    let stale_replacement_tsv = justification_record(
        base_commit,
        &test_path,
        &base_blob,
        &new_blob,
        &stale_replacement_entry,
        &review_blob,
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &stale_replacement_tsv) {
        return report_failure("justify-stale-replacement", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:justify-stale-replacement: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-stale-replacement: {} expected=1 actual={exit}",
        if exit == 1 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 1;

    // 5. Stale review artifact blob -> finding stays -> FAIL (exit 1).
    let stale_review = "0".repeat(40);
    let stale_review_tsv = justification_record(
        base_commit,
        &test_path,
        &base_blob,
        &new_blob,
        &replacement_entry,
        &stale_review,
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &stale_review_tsv) {
        return report_failure("justify-stale-review", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:justify-stale-review: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-stale-review: {} expected=1 actual={exit}",
        if exit == 1 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 1;

    // 6. Wrong base commit -> record matches no finding -> error (exit 2).
    let wrong_base_tsv = justification_record(
        "0000000000000000000000000000000000000000",
        &test_path,
        &base_blob,
        &new_blob,
        &replacement_entry,
        &review_blob,
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &wrong_base_tsv) {
        return report_failure("justify-unknown-base", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("SelfTest:justify-unknown-base: rejected with: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-unknown-base: {} expected=2 actual={exit}",
        if exit == 2 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 2;

    // 7. Unknown path -> no matching finding -> error (exit 2).
    let unknown_path_tsv = format!(
        "{base_commit}\tcrates/workspace_tests/tests/other.rs\t{base_blob}\tcrates/workspace_tests/tests/other.rs\t{new_blob}\tWeakenedAssertion\t{WEAKEN_DETAIL}\t{replacement_entry}\t{review_path}\t{review_blob}\tvb-test-fixture\n"
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &unknown_path_tsv) {
        return report_failure("justify-unknown-path", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("SelfTest:justify-unknown-path: rejected with: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-unknown-path: {} expected=2 actual={exit}",
        if exit == 2 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 2;

    // 8. Forbidden kind (NonUtf8TestFile) -> rejected at load (exit 2).
    let forbidden_tsv = format!(
        "{base_commit}\t{test_path}\t{base_blob}\t{test_path}\t{new_blob}\tNonUtf8TestFile\trelevant test file contains non-UTF8 bytes\t{replacement_entry}\t{review_path}\t{review_blob}\tvb-test-fixture\n"
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &forbidden_tsv) {
        return report_failure("justify-forbidden-nonutf8", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("SelfTest:justify-forbidden-nonutf8: rejected with: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-forbidden-nonutf8: {} expected=2 actual={exit}",
        if exit == 2 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 2;

    // 9. Malformed record (wrong column count) -> error (exit 2).
    let malformed_tsv = "HEAD\tpath\tABSENT\n";
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, malformed_tsv) {
        return report_failure("justify-malformed", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("SelfTest:justify-malformed: rejected with: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-malformed: {} expected=2 actual={exit}",
        if exit == 2 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 2;

    // 10. Duplicate identity -> error (exit 2).
    let duplicate_tsv = format!("{valid_tsv}{valid_tsv}");
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &duplicate_tsv) {
        return report_failure("justify-duplicate", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("SelfTest:justify-duplicate: rejected with: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-duplicate: {} expected=2 actual={exit}",
        if exit == 2 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 2;

    // 11. Code-level skip (silent return) without replacement -> rejected (exit 2).
    let skip_tsv = format!(
        "{base_commit}\t{test_path}\t{base_blob}\t{test_path}\t{new_blob}\tIgnoredOrSkippedTest\treturn;\t-\t{review_path}\t{review_blob}\tvb-test-fixture\n"
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &skip_tsv) {
        return report_failure("justify-forbidden-skip", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("SelfTest:justify-forbidden-skip: rejected with: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-forbidden-skip: {} expected=2 actual={exit}",
        if exit == 2 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 2;

    ok
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
    all &= run_non_utf8_relevant_case("non-utf8-relevant");
    all &= run_unrelated_binary_pass_case("unrelated-binary-passes");
    all &= run_binary_with_test_delete_case("binary-with-test-delete");
    all &= run_justification_tests();

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
