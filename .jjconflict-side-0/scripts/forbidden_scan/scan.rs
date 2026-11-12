use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;

use super::patterns::{FORBIDDEN_PATTERNS, UNSAFE_BLOCK_PATTERN, regex_contains};
use super::types::ScanResult;

pub(crate) fn scan_crate(
    crate_src_path: &Path,
    allowlist: &[String],
) -> anyhow::Result<Vec<ScanResult>> {
    let mut findings = Vec::new();

    // Find all .rs files
    let rs_files = collect_rs_files(crate_src_path)?;

    for file_path in &rs_files {
        // Check if file is in generated/perf directory or is a test file (skip those)
        let rel_path = file_path.strip_prefix(crate_src_path).unwrap_or(file_path);
        let rel_str = rel_path.to_string_lossy();
        if should_skip_rs_file(&rel_str) {
            continue;
        }

        let file_findings = scan_file(file_path, allowlist)?;
        findings.extend(file_findings);
    }

    Ok(findings)
}
fn collect_rs_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !dir.is_dir() {
        return Ok(files);
    }

    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = fs::read_dir(&current)
            .with_context(|| format!("Failed to read directory: {}", current.display()))?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Skip test dirs and target dirs
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default();
                if name != "target" && name != "tests" && !name.starts_with('.') {
                    stack.push(path);
                }
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                files.push(path);
            }
        }
    }
    Ok(files)
}
fn scan_file(file_path: &PathBuf, allowlist: &[String]) -> anyhow::Result<Vec<ScanResult>> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let mut findings = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let ignored_lines = cfg_test_lines(&lines);

    // Check forbidden patterns
    for (pattern, description) in FORBIDDEN_PATTERNS {
        // Skip if pattern is in allowlist
        if allowlist.iter().any(|a| a == *pattern) {
            continue;
        }

        for (line_num, line) in lines.iter().enumerate() {
            if ignored_lines.contains(&line_num) {
                continue;
            }
            if line.contains("dbg!") && pattern == &r"\b\.unwrap\(\)" {
                // Skip dbg! invocations that might contain .unwrap()
                continue;
            }
            if regex_contains(line, pattern) {
                findings.push(ScanResult {
                    pattern: pattern.to_string(),
                    description: description.to_string(),
                    file: file_path.clone(),
                    line: checked_line_number(line_num),
                    line_content: line.to_string(),
                });
            }
        }
    }

    // Check unsafe blocks (excluding allowlisted)
    for (line_num, line) in lines.iter().enumerate() {
        if ignored_lines.contains(&line_num) {
            continue;
        }
        if regex_contains(line, UNSAFE_BLOCK_PATTERN) {
            let is_allowlisted = allowlist.iter().any(|a| regex_contains(line, a));
            if !is_allowlisted {
                findings.push(ScanResult {
                    pattern: UNSAFE_BLOCK_PATTERN.to_string(),
                    description: "unsafe { } block (not allowlisted)".to_string(),
                    file: file_path.clone(),
                    line: checked_line_number(line_num),
                    line_content: line.to_string(),
                });
            }
        }
    }

    Ok(findings)
}
fn should_skip_rs_file(rel_path: &str) -> bool {
    rel_path.contains("generated")
        || rel_path.contains("perf")
        || rel_path.contains("target")
        || rel_path.starts_with("tests/")
        || rel_path.contains("/tests/")
        || rel_path.contains("_tests.")
        || rel_path.ends_with("_tests.rs")
        || rel_path.ends_with("tests.rs")
        || rel_path.contains("/test")
        || rel_path.contains("test_harness")
        || rel_path.contains("_tests")
        || rel_path.contains("/proof")
        || rel_path.contains("_proof")
        || rel_path.contains("proptest")
        || rel_path.contains("property_tests")
        || rel_path.contains("/loom/")
        || rel_path.starts_with("kani/")
        || rel_path.starts_with("kani_")
        || rel_path.contains("/kani/")
        || rel_path.contains("/kani_")
        // Verification extern-body files: Flux `#[extern_spec]` and Verus
        // `#[verifier::external_body]` modules under any `verification/` tree
        // are compiled by their own toolchains (flux-rs, verus), not by rustc.
        // Their module roots (e.g. `crates/vb_runtime/src/verification/mod.rs`)
        // gate the contents behind `#[cfg(flux)]` / `#[cfg(verus)]`, and the
        // bodies intentionally use `unimplemented!()` because the real bodies
        // live in the referenced production source files. Treat them as
        // trusted extern-bodies; they are never reachable from production
        // rustc builds. Sibling kani/loom/proptest trees under verification/
        // are already covered by the preceding rules.
        || rel_path.starts_with("verification/")
        || rel_path.contains("/verification/")
}
fn cfg_test_lines(lines: &[&str]) -> std::collections::HashSet<usize> {
    let mut ignored = std::collections::HashSet::new();
    let mut cfg_test_pending = false;
    let mut attr_pending = false;
    let mut test_depth = 0_i32;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if test_depth > 0 {
            ignored.insert(index);
            test_depth = update_depth(test_depth, line);
            continue;
        }
        if trimmed.starts_with("#[cfg(test)]") {
            ignored.insert(index);
            cfg_test_pending = true;
            continue;
        }
        if trimmed.starts_with("#[test]")
            || trimmed.starts_with("#[kani::proof]")
            || trimmed.starts_with("#[cfg(kani)]")
            || trimmed.starts_with("#[cfg(loom)]")
        {
            ignored.insert(index);
            attr_pending = true;
            continue;
        }
        if attr_pending {
            ignored.insert(index);
            if trimmed.contains("fn ") {
                test_depth = line_depth_delta(line);
                if test_depth <= 0 {
                    test_depth = 1;
                }
            }
            continue;
        }
        if cfg_test_pending {
            ignored.insert(index);
            if trimmed.contains("mod ") || trimmed.contains("fn ") {
                test_depth = line_depth_delta(line);
                if test_depth <= 0 {
                    test_depth = 1;
                }
            }
        }
    }
    ignored
}
fn update_depth(current: i32, line: &str) -> i32 {
    let opened = count_char(line, '{');
    let closed = count_char(line, '}');
    current
        .checked_add(opened)
        .and_then(|value| value.checked_sub(closed))
        .unwrap_or_default()
}
fn line_depth_delta(line: &str) -> i32 {
    count_char(line, '{')
        .checked_sub(count_char(line, '}'))
        .unwrap_or_default()
}
fn count_char(line: &str, needle: char) -> i32 {
    i32::try_from(line.chars().filter(|ch| *ch == needle).count()).unwrap_or(i32::MAX)
}
fn checked_line_number(line_index: usize) -> usize {
    match line_index.checked_add(1) {
        Some(line) => line,
        None => usize::MAX,
    }
}
