use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::Context;

use super::patterns::DEFAULT_FIRST_PARTY_CRATES;
use super::scan::scan_crate;
use super::types::{ScanResult, ScanSummary};

/// Run the forbidden-scan command.
pub fn cmd_forbidden_scan(
    crate_globs: Option<&[String]>,
    allowlist_path: Option<&str>,
) -> anyhow::Result<()> {
    let workspace_root =
        std::env::current_dir().context("Failed to determine current working directory")?;

    let crates_to_scan = resolve_crates(&workspace_root, crate_globs)?;
    let allowlist = load_allowlist(allowlist_path)?;

    let mut all_findings: Vec<ScanResult> = Vec::new();
    let mut summary = ScanSummary::new();

    for crate_name in &crates_to_scan {
        let crate_path = workspace_root.join("crates").join(crate_name);
        let src_path = crate_path.join("src");

        if !src_path.exists() {
            // Try root src for single-crate workspaces
            let root_src = crate_path.join("src");
            if !root_src.exists() {
                write_stderr_line(format_args!(
                    "warning: no src/ found for crate {}, skipping",
                    crate_name
                ))?;
                continue;
            }
        }

        let scan_root = if src_path.exists() {
            &src_path
        } else {
            &crate_path
        };

        write_stderr_line(format_args!("Scanning crate: {}", crate_name))?;

        let crate_findings = scan_crate(scan_root, &allowlist)?;
        for finding in &crate_findings {
            summary.add_finding(&finding.pattern);
        }
        all_findings.extend(crate_findings);
        summary.crates_scanned.push(crate_name.clone());
    }

    // Deduplicate findings by file:line:pattern
    all_findings.sort_by(|a, b| (&a.file, a.line, &a.pattern).cmp(&(&b.file, b.line, &b.pattern)));
    all_findings.dedup_by(|a, b| a.file == b.file && a.line == b.line && a.pattern == b.pattern);

    summary.files_with_findings = all_findings
        .iter()
        .map(|f| f.file.clone())
        .collect::<std::collections::HashSet<_>>()
        .len();

    // Report findings
    if all_findings.is_empty() {
        write_stdout_line(format_args!(
            "forbidden-scan: PASS — no forbidden patterns found"
        ))?;
        write_stdout_line(format_args!(""))?;
        write_stdout_line(format_args!(
            "Crates scanned: {}",
            summary.crates_scanned.join(", ")
        ))?;
        return Ok(());
    }

    write_stdout_line(format_args!(
        "forbidden-scan: FAIL — {} forbidden pattern(s) found",
        all_findings.len()
    ))?;
    write_stdout_line(format_args!(""))?;
    write_stdout_line(format_args!("Summary:"))?;
    write_stdout_line(format_args!("  Total findings: {}", summary.total_findings))?;
    write_stdout_line(format_args!(
        "  Files with findings: {}",
        summary.files_with_findings
    ))?;
    write_stdout_line(format_args!(
        "  Crates scanned: {}",
        summary.crates_scanned.join(", ")
    ))?;
    write_stdout_line(format_args!(""))?;
    write_stdout_line(format_args!("Findings by pattern:"))?;
    for (pattern, count) in &summary.findings_by_pattern {
        write_stdout_line(format_args!("  {}: {}", pattern, count))?;
    }
    write_stdout_line(format_args!(""))?;

    // Print detailed findings
    let mut current_file: Option<PathBuf> = None;
    for finding in &all_findings {
        if current_file.as_ref() != Some(&finding.file) {
            write_stdout_line(format_args!("{}:", finding.file.display()))?;
            current_file = Some(finding.file.clone());
        }
        write_stdout_line(format_args!(
            "  {}:{}: {} [{}]",
            finding.line,
            finding.line_content.trim(),
            finding.pattern,
            finding.description
        ))?;
    }

    anyhow::bail!("forbidden patterns detected");
}
fn resolve_crates(
    _workspace_root: &PathBuf,
    globs: Option<&[String]>,
) -> anyhow::Result<Vec<String>> {
    match globs {
        Some(globs) if !globs.is_empty() => {
            let mut crates = Vec::new();
            for glob in globs {
                let matched: Vec<String> = DEFAULT_FIRST_PARTY_CRATES
                    .iter()
                    .filter(|c| glob_match(glob, c))
                    .map(|s| s.to_string())
                    .collect();
                crates.extend(matched);
            }
            crates.sort();
            crates.dedup();
            Ok(crates)
        }
        _ => Ok(DEFAULT_FIRST_PARTY_CRATES
            .iter()
            .map(|s| (*s).to_string())
            .collect()),
    }
}
fn glob_match(pattern: &str, name: &str) -> bool {
    if pattern == name {
        return true;
    }
    // Simple glob: treat * as wildcard
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        match parts.as_slice() {
            [prefix, suffix] => return name.starts_with(prefix) && name.ends_with(suffix),
            [prefix] => return name.starts_with(prefix),
            _ => {}
        }
    }
    false
}
fn load_allowlist(path: Option<&str>) -> anyhow::Result<Vec<String>> {
    match path {
        Some(p) => {
            let content = fs::read_to_string(p)
                .with_context(|| format!("Failed to read allowlist file: {}", p))?;
            Ok(content
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(String::from)
                .collect())
        }
        None => Ok(Vec::new()),
    }
}
fn write_stdout_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_fmt(args)?;
    handle.write_all(b"\n")
}
fn write_stderr_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    handle.write_fmt(args)?;
    handle.write_all(b"\n")
}
