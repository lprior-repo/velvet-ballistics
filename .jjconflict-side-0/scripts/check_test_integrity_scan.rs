use super::check_test_integrity_commands::git_output_bytes;
use super::check_test_integrity_diff::{base_file_bytes, current_file_bytes, path_diff_bytes};
use super::check_test_integrity_paths::{
    has_compile_only, has_exact_assertion, has_ignore_or_skip, has_test_decl, has_weak_assertion,
    is_behavior_test_path, is_test_path,
};
use super::{ChangedEntry, Finding, RenamePair, Vcs};
use std::path::Path;

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

pub(crate) fn scan_changed_entry(
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

pub(crate) fn scan_rename_pair(
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
