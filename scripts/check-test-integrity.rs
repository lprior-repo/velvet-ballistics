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
    String::from_utf8(bytes).map_err(|error| {
        format!(
            "{tool} {} returned non-UTF8 stdout: {error}",
            args.join(" ")
        )
    })
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
    tool_output_allow_fail_bytes(tool, args, cwd).and_then(|bytes| String::from_utf8(bytes).ok())
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
        Vcs::Jj => {
            jj_output(&["log", "--no-graph", "-r", base, "-T", "commit_id"], root).map(|_| ())
        }
    }
    .map_err(|error| format!("invalid base revision {base:?}: {error}"))
}

fn resolve_base_hash(root: &Path, base: &str, vcs: Vcs) -> Result<String, String> {
    validate_base_revision(root, base, vcs)?;
    match vcs {
        Vcs::Git => {
            let commit = format!("{base}^{{commit}}");
            let hash = git_output(&["rev-parse", "--verify", &commit], root)?
                .trim()
                .to_owned();
            if hash.is_empty() {
                return Err(format!("could not resolve base {base:?} to a commit hash"));
            }
            Ok(hash)
        }
        Vcs::Jj => {
            let hash = jj_output(&["log", "--no-graph", "-r", base, "-T", "commit_id"], root)?
                .trim()
                .to_owned();
            if hash.is_empty() {
                return Err(format!("could not resolve base {base:?} to a commit hash"));
            }
            Ok(hash)
        }
    }
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
                if trimmed.is_empty() {
                    continue;
                }
                let type_code = trimmed.get(..2).ok_or_else(|| {
                    format!("malformed jj --types line (shorter than 2 bytes): {line:?}")
                })?;
                let path = trimmed
                    .get(2..)
                    .map(str::trim_start)
                    .ok_or_else(|| format!("malformed jj --types line (path offset): {line:?}"))?;
                if path.is_empty() {
                    return Err(format!("malformed jj --types line (empty path): {line:?}"));
                }
                let status = match type_code {
                    "-F" => b'A',
                    "F-" => b'D',
                    "FF" => b'M',
                    other => {
                        return Err(format!(
                            "unsupported jj --types type code {other:?} for path {path:?} (fail-closed)"
                        ));
                    }
                };
                entries.push(ChangedEntry {
                    status,
                    old_path: path.to_owned(),
                    new_path: path.to_owned(),
                });
            }
            Ok(entries)
        }
    }
}

// ---------------------------------------------------------------------------
// Rename-pair enumeration (sound old/new mapping via --find-renames)
// ---------------------------------------------------------------------------

fn git_rename_pairs(root: &Path, base: &str) -> Result<Vec<RenamePair>, String> {
    let bytes = git_output_bytes(
        &["diff", "--name-status", "-z", "--find-renames", base, "--"],
        root,
    )?;
    let mut segments = bytes.split(|&byte| byte == 0);
    let mut pairs = Vec::new();
    while let Some(status_seg) = segments.next() {
        if status_seg.is_empty() {
            continue;
        }
        let status = match status_seg.first() {
            Some(&byte) => byte,
            None => continue,
        };
        match status {
            b'R' => {
                let old_seg = segments.next().ok_or_else(|| {
                    "malformed git rename output: missing old path".to_owned()
                })?;
                let new_seg = segments.next().ok_or_else(|| {
                    "malformed git rename output: missing new path".to_owned()
                })?;
                if old_seg.is_empty() || new_seg.is_empty() {
                    return Err("malformed git rename output: empty path".to_owned());
                }
                let old_path = String::from_utf8(old_seg.to_vec())
                    .map_err(|_| "rename old path is not UTF-8".to_owned())?;
                let new_path = String::from_utf8(new_seg.to_vec())
                    .map_err(|_| "rename new path is not UTF-8".to_owned())?;
                pairs.push(RenamePair {
                    old_path,
                    new_path,
                });
            }
            b'A' | b'M' | b'D' | b'T' | b'U' | b'X' | b'B' => {
                let _ = segments.next().ok_or_else(|| {
                    "malformed git name-status output: missing path".to_owned()
                })?;
            }
            other => {
                return Err(format!(
                    "unsupported git name-status status byte 0x{other:02x} (fail-closed)"
                ));
            }
        }
    }
    // Ambiguity guard: a rename source/destination must be unique.
    let mut seen_old = BTreeSet::new();
    let mut seen_new = BTreeSet::new();
    for pair in &pairs {
        if !seen_old.insert(pair.old_path.clone()) {
            return Err(format!(
                "ambiguous rename: old path {} appears in multiple rename pairs",
                pair.old_path
            ));
        }
        if !seen_new.insert(pair.new_path.clone()) {
            return Err(format!(
                "ambiguous rename: new path {} appears in multiple rename pairs",
                pair.new_path
            ));
        }
    }
    Ok(pairs)
}

fn jj_rename_pairs(root: &Path, base: &str) -> Result<Vec<RenamePair>, String> {
    let text = jj_output(&["diff", "--types", "--from", base, "--to", "@"], root)?;
    let mut pairs = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.contains('{') || trimmed.contains('}') || trimmed.contains("=>") {
            // jj renames surface as `{old => new}` (optionally after a type code).
            let Some((old, new)) = trimmed.rsplit_once("=>") else {
                return Err(format!(
                    "ambiguous jj --types rename syntax: {trimmed:?} (fail-closed)"
                ));
            };
            let old = old.trim();
            let new = new.trim();
            let old_path = old
                .strip_prefix('{')
                .map(str::trim)
                .or_else(|| old.split_whitespace().last())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    format!("malformed jj --types rename old path: {trimmed:?}")
                })?;
            let new_path = new
                .strip_suffix('}')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    format!("malformed jj --types rename new path: {trimmed:?}")
                })?;
            pairs.push(RenamePair {
                old_path: old_path.to_owned(),
                new_path: new_path.to_owned(),
            });
        }
    }
    Ok(pairs)
}

fn rename_pairs(root: &Path, base: &str, vcs: Vcs) -> Result<Vec<RenamePair>, String> {
    match vcs {
        Vcs::Git => git_rename_pairs(root, base),
        Vcs::Jj => jj_rename_pairs(root, base),
    }
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
        Vcs::Jj => jj_output_bytes(
            &["diff", "--git", "--from", base, "--to", "@", "--", path],
            root,
        ),
    }
}

fn scan_diff(diff: &str, removed_path: &str, added_path: &str) -> Vec<Finding> {
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
                if is_behavior_test_path(added_path) && has_ignore_or_skip(payload) {
                    findings.push(Finding {
                        kind: "IgnoredOrSkippedTest",
                        path: added_path.to_owned(),
                        detail: payload.trim().to_owned(),
                    });
                }
                if is_behavior_test_path(added_path) && has_compile_only(payload) {
                    findings.push(Finding {
                        kind: "CompileOnlyReplacement",
                        path: added_path.to_owned(),
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
            path: removed_path.to_owned(),
            detail: format!(
                "removed_declarations={removed_test_decl} added_declarations={added_test_decl}"
            ),
        });
    }
    if added_weak > 0 || added_exact < removed_exact {
        findings.push(Finding {
            kind: "WeakenedAssertion",
            path: removed_path.to_owned(),
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
    let is_delete = entry.status == b'D';
    let path = if is_delete {
        entry.old_path.as_str()
    } else {
        entry.new_path.as_str()
    };
    let is_test = is_test_path(path);

    // An added path has no base content; only delete/modify read the base blob.
    let base_bytes = if entry.status == b'A' {
        None
    } else {
        base_file_bytes(root, base, path, vcs)
    };
    let base_has_tests = base_bytes.as_ref().map_or(false, |bytes| {
        std::str::from_utf8(bytes)
            .map_or(false, |text| has_test_decl(text) || has_exact_assertion(text))
    });
    if !is_test && !base_has_tests {
        return Ok(Vec::new());
    }

    let current_bytes = if is_delete {
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
    let mut findings = scan_diff(&text, path, path);
    if is_delete {
        findings.push(Finding {
            kind: "DeletedTestFile",
            path: path.to_owned(),
            detail: "deleted file contained tests or test assertions".to_owned(),
        });
    }
    Ok(findings)
}

fn content_has_tests(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .map_or(false, |text| has_test_decl(text) || has_exact_assertion(text))
}

fn count_signals(text: &str) -> (usize, usize) {
    let mut decls = 0usize;
    let mut exact = 0usize;
    for line in text.lines() {
        if has_test_decl(line) {
            decls += 1;
        }
        if has_exact_assertion(line) {
            exact += 1;
        }
    }
    (decls, exact)
}

fn deletion_findings(path: &str, base_bytes: Option<&[u8]>) -> Vec<Finding> {
    let mut findings = vec![Finding {
        kind: "DeletedTestFile",
        path: path.to_owned(),
        detail: "deleted file contained tests or test assertions".to_owned(),
    }];
    if let Some(bytes) = base_bytes {
        if let Ok(text) = std::str::from_utf8(bytes) {
            let (decls, exact) = count_signals(text);
            if decls > 0 {
                findings.push(Finding {
                    kind: "DeletedTestDeclaration",
                    path: path.to_owned(),
                    detail: format!("removed_declarations={decls} added_declarations=0"),
                });
            }
            if exact > 0 {
                findings.push(Finding {
                    kind: "WeakenedAssertion",
                    path: path.to_owned(),
                    detail: format!("removed_exact={exact} added_exact=0 added_weak=0"),
                });
            }
        }
    }
    findings
}

fn rename_pair_diff_bytes(
    root: &Path,
    base: &str,
    old: &str,
    new: &str,
    vcs: Vcs,
) -> Result<Vec<u8>, String> {
    match vcs {
        Vcs::Git => git_output_bytes(
            &[
                "diff",
                "--find-renames",
                "--text",
                "--no-ext-diff",
                "--no-textconv",
                "--unified=0",
                base,
                "--",
                old,
                new,
            ],
            root,
        ),
        Vcs::Jj => Err(format!(
            "jj per-pair rename diff for {old} -> {new} is unsupported (fail-closed)"
        )),
    }
}

fn scan_rename_pair(
    root: &Path,
    base: &str,
    pair: &RenamePair,
    vcs: Vcs,
) -> Result<Vec<Finding>, String> {
    let old = &pair.old_path;
    let new = &pair.new_path;
    let old_is_test = is_test_path(old);
    let old_bytes = base_file_bytes(root, base, old, vcs);
    let old_has_tests = old_bytes.as_ref().map_or(false, |b| content_has_tests(b));
    if !old_is_test && !old_has_tests {
        // Nothing on the old side to protect: a pure non-test move or a test
        // promotion whose source had no tests.
        return Ok(Vec::new());
    }

    let new_bytes = current_file_bytes(root, new);
    let new_is_test = is_test_path(new);

    // Strict UTF-8/NUL validation BEFORE any relocation skip, so an identical
    // invalid-Rust move still fails rather than being silently dropped.
    let old_invalid = old_bytes
        .as_ref()
        .map_or(false, |b| std::str::from_utf8(b).is_err());
    let new_invalid = new_bytes
        .as_ref()
        .map_or(false, |b| std::str::from_utf8(b).is_err());
    if old_invalid || new_invalid {
        return Ok(vec![Finding {
            kind: "NonUtf8TestFile",
            path: old.clone(),
            detail: "relevant test file contains non-UTF8 bytes".to_owned(),
        }]);
    }

    // A move out of a test path into a non-test path (archive) always loses
    // the test's location, even at 100% similarity or with no body hunks.
    if old_is_test && !new_is_test {
        return Ok(deletion_findings(old, old_bytes.as_deref()));
    }

    // Test-to-test relocation (or inline-test move): an exact byte move is a
    // pure relocation; a modified move is scanned via its actual per-pair
    // hunks so unchanged return/comments/assertions are not re-flagged.
    if old_bytes == new_bytes {
        return Ok(Vec::new());
    }
    let patch = rename_pair_diff_bytes(root, base, old, new, vcs)?;
    let text = String::from_utf8(patch)
        .map_err(|_| format!("non-UTF8 rename diff for {old} -> {new} (fail-closed)"))?;
    if !text.contains("rename from") || !text.contains("rename to") {
        return Err(format!(
            "rename pair {old} -> {new} could not be resolved as a rename (fail-closed)"
        ));
    }
    Ok(scan_diff(&text, old, new))
}

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
        let [
            base_commit,
            old_path,
            old_blob,
            new_path,
            new_blob,
            finding_kind,
            finding_detail,
            replacement_paths_col,
            review_artifact_path,
            review_artifact_blob,
            bead_id,
        ] = match columns.as_slice() {
            [a, b, c, d, e, f, g, h, i, j, k] => [a, b, c, d, e, f, g, h, i, j, k],
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

        let replacement_paths = if replacement_paths_col.is_empty() || replacement_paths_col == "-"
        {
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
    fn matches_identity(&self, finding: &Finding) -> bool {
        self.finding_kind == finding.kind
            && self.finding_detail == finding.detail
            && self.old_path == finding.path
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

        // The record must bind the ACTUAL current state of the finding's
        // location (old_path). An unrelated new_path or ABSENT while the file
        // still exists must not waive the finding.
        let actual_current_old = resolve_current_blob(root, &self.old_path)?;
        if actual_current_old.is_some() {
            // Original still exists: new_path must be the same path and its
            // exact current hash must match.
            if self.new_path != self.old_path || self.new_blob == "ABSENT" {
                return Ok(false);
            }
            if actual_current_old.as_deref() != Some(self.new_blob.as_str()) {
                return Ok(false);
            }
        } else {
            // Original is actually absent: only a real relocation/replacement
            // (different path, exact blob) or an explicit ABSENT is allowed.
            if self.new_path == "ABSENT" {
                if self.new_blob != "ABSENT" {
                    return Ok(false);
                }
            } else if self.new_path == self.old_path || self.new_blob == "ABSENT" {
                return Ok(false);
            } else {
                let actual_new = resolve_current_blob(root, &self.new_path)?;
                if actual_new.as_deref() != Some(self.new_blob.as_str()) {
                    return Ok(false);
                }
            }
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
    resolved_base: &str,
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
            // Inactive records (a different full base commit) can neither waive
            // nor error; only records for the resolved base are applied.
            if record.base_commit != resolved_base {
                continue;
            }
            if !record.matches_identity(finding) {
                continue;
            }
            matched_indices.insert(index);
            if record.hashes_valid(root, resolved_base)? {
                matched_record = Some(record);
            }
            break;
        }
        match matched_record {
            Some(record) => justified.push((finding.clone(), record.bead_id.clone())),
            None => remaining.push(finding.clone()),
        }
    }

    // Only ACTIVE records (matching the resolved base) that reference no
    // finding are stale errors. Inactive other-base records are ignored.
    let unmatched: Vec<&JustificationRecord> = records
        .iter()
        .enumerate()
        .filter(|(index, _)| !matched_indices.contains(index))
        .filter(|(_, record)| record.base_commit == resolved_base)
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
// Self-test infrastructure
// ---------------------------------------------------------------------------

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Result<Fixture, String> {
        let root =
            std::env::temp_dir().join(format!("test-integrity-{}-{label}", std::process::id()));
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
        fs::write(&path, content).map_err(|error| format!("write {}: {error}", path.display()))
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

    fn stage_all(&self) -> Result<(), String> {
        let git = resolve_real_binary("git")
            .ok_or_else(|| "could not resolve a real git binary".to_owned())?;
        git_cmd(&git, &self.root, &["add", "-A"])
    }

    fn collect(&self) -> Result<Vec<Finding>, String> {
        collect_findings(&self.root, "HEAD", Vcs::Git)
    }

    fn check(&self) -> Result<i32, String> {
        check(&self.root, "HEAD", Vcs::Git)
    }

    fn head_hash(&self) -> Result<String, String> {
        resolve_base_hash(&self.root, "HEAD", Vcs::Git)
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

// Large-enough test bodies so `git --find-renames` detects modified renames
// (small files fall below git's rename-similarity size floor).
fn large_test_text() -> String {
    let mut out = String::from("#[test]\nfn behavior_test() {\n");
    for i in 0..30 {
        out.push_str(&format!("    let value_{i}: u32 = {i};\n"));
    }
    out.push_str("    assert_eq!(2 + 2, 4);\n}\n");
    out
}

fn large_test_weakened() -> String {
    let mut out = String::from("#[test]\nfn behavior_test() {\n");
    for i in 0..30 {
        out.push_str(&format!("    let value_{i}: u32 = {i};\n"));
    }
    out.push_str("    assert!((2 + 2).checked_sub(4).is_some());\n}\n");
    out
}

fn large_test_retained_modified() -> String {
    let mut out = String::from("#[test]\n// relocated: coverage retained\nfn behavior_test() {\n");
    for i in 0..30 {
        out.push_str(&format!("    let value_{i}: u32 = {i};\n"));
    }
    out.push_str("    assert_eq!(2 + 2, 4);\n}\n");
    out
}

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
    let to = "archive/behavior.rs";
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(from, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| fixture.rename(from, to))
        .and_then(|()| fixture.stage_all());
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

fn run_relocate_case(label: &str) -> bool {
    let from = "crates/workspace_tests/tests/behavior.rs";
    let to = "crates/workspace_tests/tests/behavior_relocated.rs";
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(from, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| fixture.rename(from, to))
        .and_then(|()| fixture.stage_all());
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

fn run_relocate_modified_retained_case(label: &str) -> bool {
    let from = "crates/workspace_tests/tests/behavior.rs";
    let to = "crates/workspace_tests/tests/behavior_relocated.rs";
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(from, &large_test_text())
        .and_then(|()| fixture.commit_base())
        .and_then(|()| fixture.rename(from, to))
        .and_then(|()| fixture.write(to, &large_test_retained_modified()))
        .and_then(|()| fixture.stage_all());
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

fn run_relocate_weakened_case(label: &str) -> bool {
    let from = "crates/workspace_tests/tests/behavior.rs";
    let to = "crates/workspace_tests/tests/behavior_relocated.rs";
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(from, &large_test_text())
        .and_then(|()| fixture.commit_base())
        .and_then(|()| fixture.rename(from, to))
        .and_then(|()| fixture.write(to, &large_test_weakened()))
        .and_then(|()| fixture.stage_all());
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
    let expected = [("WeakenedAssertion", from)];
    let findings_ok = expect_findings(label, &findings, &expected);
    println!("SelfTest:{label}: exit expected=1 actual={exit}");
    findings_ok && exit == 1
}

fn run_relocate_spaces_case(label: &str) -> bool {
    let from = "crates/workspace_tests/tests/behavior with spaces.rs";
    let to = "crates/workspace_tests/tests/behavior moved.rs";
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write(from, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| fixture.rename(from, to))
        .and_then(|()| fixture.stage_all());
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

fn run_relocate_invalid_utf8_case(label: &str) -> bool {
    let from = "crates/workspace_tests/tests/behavior.rs";
    let to = "crates/workspace_tests/tests/behavior_relocated.rs";
    let mut invalid = BASE_TEXT.as_bytes().to_vec();
    invalid.extend_from_slice(&[0xff, 0x00]);
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let setup = fixture
        .write_bytes(from, &invalid)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| fixture.rename(from, to))
        .and_then(|()| fixture.stage_all());
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
    let expected = [("NonUtf8TestFile", from)];
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
        0x23u8, 0x5b, 0x74, 0x65, 0x73, 0x74, 0x5d, 0x0a, 0x66, 0x6e, 0x20, 0x62, 0x65, 0x68, 0x61,
        0x76, 0x69, 0x6f, 0x72, 0x5f, 0x74, 0x65, 0x73, 0x74, 0x28, 0x29, 0x20, 0x7b, 0x0a, 0x20,
        0x20, 0x20, 0x20, 0x61, 0x73, 0x73, 0x65, 0x72, 0x74, 0x5f, 0x65, 0x71, 0x28, 0x64, 0x75,
        0x6d, 0x6d, 0x79, 0x29, 0x3b, 0x0a, 0x7d, 0x0a, 0xff, 0x00,
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
    let (fixture, test_path, base_blob, new_blob, replacement_entry) =
        match justification_fixture("justify") {
            Ok(values) => values,
            Err(error) => return report_failure("justify", &format!("fixture setup: {error}")),
        };
    let base_commit = match fixture.head_hash() {
        Ok(hash) => hash,
        Err(error) => return report_failure("justify", &format!("head hash: {error}")),
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
    if !expect_findings(
        "justify-pre",
        &pre,
        &[("WeakenedAssertion", test_path.as_str())],
    ) {
        return false;
    }

    // 1. Valid record passes (finding justified).
    let valid_tsv = justification_record(
        &base_commit,
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
    println!(
        "SelfTest:justify-valid: {} expected=0 actual={exit}",
        if exit == 0 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 0;

    // 2. Stale current test blob -> finding stays -> FAIL (exit 1).
    let stale_new = "0".repeat(40);
    let stale_test_tsv = justification_record(
        &base_commit,
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
        &base_commit,
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
        &base_commit,
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
        &base_commit,
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

    // 6. Wrong base commit -> inactive record is ignored, so the real current
    //    finding stays un-justified -> FAIL (exit 1), not a stale error.
    let wrong_base_tsv = justification_record(
        "0000000000000000000000000000000000000000",
        &test_path,
        &base_blob,
        &new_blob,
        &replacement_entry,
        &review_blob,
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &wrong_base_tsv) {
        return report_failure("justify-wrong-base", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:justify-wrong-base: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:justify-wrong-base: {} expected=1 actual={exit}",
        if exit == 1 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 1;

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

fn run_justification_bypass_tests() -> bool {
    let (fixture, test_path, base_blob, _new_blob, _replacement_entry) =
        match justification_fixture("bypass") {
            Ok(values) => values,
            Err(error) => return report_failure("bypass", &format!("fixture setup: {error}")),
        };
    let base_commit = match fixture.head_hash() {
        Ok(hash) => hash,
        Err(error) => return report_failure("bypass", &format!("head hash: {error}")),
    };
    let review_path = ".beads/vb-test-fixture/test-review.md";
    let review_blob = match fixture.current_blob(review_path) {
        Ok(Some(blob)) => blob,
        Ok(None) => return report_failure("bypass", "review blob missing"),
        Err(error) => return report_failure("bypass", &format!("review blob: {error}")),
    };

    let mut ok = true;

    // Bypass #1: old_path still exists (weakened) but new_path binds an
    // unrelated file. The finding must stay un-justified (exit 1).
    if let Err(error) = fixture.write("README.md", "# Unrelated\n") {
        return report_failure("bypass-unrelated", &format!("write README: {error}"));
    }
    let unrelated_blob = match fixture.current_blob("README.md") {
        Ok(Some(blob)) => blob,
        _ => return report_failure("bypass-unrelated", "unrelated blob missing"),
    };
    let unrelated_tsv = format!(
        "{base_commit}\t{test_path}\t{base_blob}\tREADME.md\t{unrelated_blob}\tWeakenedAssertion\t{WEAKEN_DETAIL}\t-\t{review_path}\t{review_blob}\tvb-test-fixture\n"
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &unrelated_tsv) {
        return report_failure("bypass-unrelated", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:bypass-unrelated: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:bypass-unrelated: {} expected=1 actual={exit}",
        if exit == 1 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 1;

    // Bypass #2: new_path=ABSENT while the original still exists. The finding
    // must stay un-justified (exit 1).
    let absent_tsv = format!(
        "{base_commit}\t{test_path}\t{base_blob}\tABSENT\tABSENT\tWeakenedAssertion\t{WEAKEN_DETAIL}\t-\t{review_path}\t{review_blob}\tvb-test-fixture\n"
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &absent_tsv) {
        return report_failure("bypass-absent", &format!("write tsv: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:bypass-absent: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:bypass-absent: {} expected=1 actual={exit}",
        if exit == 1 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 1;

    // A subsequent actual test change is still caught (exit 1) with the bypass
    // record present: the record cannot track or waive the changed file.
    if let Err(error) = fixture.write(
        &test_path,
        "#[test]\nfn behavior_test() {\n    assert!((2 + 2).checked_sub(4).is_some());\n}\n// changed\n",
    ) {
        return report_failure("bypass-changed", &format!("write changed test: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:bypass-changed: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:bypass-changed: {} expected=1 actual={exit}",
        if exit == 1 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 1;

    ok
}

fn run_justification_lifecycle_tests() -> bool {
    let mut ok = true;

    // A stale record for an older base commit must not fail a clean future
    // main: after advancing to a new base with zero findings, the inactive
    // record is ignored and the gate passes (exit 0).
    let fixture = match Fixture::new("lifecycle-clean") {
        Ok(fixture) => fixture,
        Err(error) => return report_failure("lifecycle-clean", &format!("fixture setup: {error}")),
    };
    let test_path = "crates/workspace_tests/tests/behavior.rs";
    let review_path = ".beads/vb-test-fixture/test-review.md";
    let setup = fixture
        .write(test_path, BASE_TEXT)
        .and_then(|()| fixture.commit_base());
    if let Err(error) = setup {
        return report_failure("lifecycle-clean", &format!("setup: {error}"));
    }
    let old_base = match fixture.head_hash() {
        Ok(hash) => hash,
        Err(error) => return report_failure("lifecycle-clean", &format!("head hash: {error}")),
    };
    let base_blob = match fixture.base_blob(test_path) {
        Ok(Some(blob)) => blob,
        _ => return report_failure("lifecycle-clean", "base blob missing"),
    };
    let weakened =
        "#[test]\nfn behavior_test() {\n    assert!((2 + 2).checked_sub(4).is_some());\n}\n";
    let setup = fixture
        .write(test_path, weakened)
        .and_then(|()| fixture.write(review_path, "# review\n"));
    if let Err(error) = setup {
        return report_failure("lifecycle-clean", &format!("weaken: {error}"));
    }
    let new_blob = match fixture.current_blob(test_path) {
        Ok(Some(blob)) => blob,
        _ => return report_failure("lifecycle-clean", "new blob missing"),
    };
    let review_blob = match fixture.current_blob(review_path) {
        Ok(Some(blob)) => blob,
        _ => return report_failure("lifecycle-clean", "review blob missing"),
    };
    let record = format!(
        "{old_base}\t{test_path}\t{base_blob}\t{test_path}\t{new_blob}\tWeakenedAssertion\t{WEAKEN_DETAIL}\t-\t{review_path}\t{review_blob}\tvb-test-fixture\n"
    );
    if let Err(error) = fixture.write(JUSTIFICATIONS_REL, &record) {
        return report_failure("lifecycle-clean", &format!("write tsv: {error}"));
    }
    // Advance to a clean "next main" base: the weakened state is committed and
    // the working tree is clean, so there are no current findings.
    if let Err(error) = fixture.commit_base() {
        return report_failure("lifecycle-clean", &format!("commit clean: {error}"));
    }
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => {
            ok = false;
            eprintln!("SelfTest:lifecycle-clean: check error: {error}");
            2
        }
    };
    println!(
        "SelfTest:lifecycle-clean: {} expected=0 actual={exit}",
        if exit == 0 { "PASS" } else { "FAIL" }
    );
    ok = ok && exit == 0;

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
