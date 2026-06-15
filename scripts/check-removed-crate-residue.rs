// SPDX-License-Identifier: MIT
// check-removed-crate-residue: scans the repository for ACTIVE references to
// the removed release-crate set documented in velvet-ballistics-MASTER.md
// ("Removed crates: vb_codegen, vb_ui_model, and vb_ui_makepad... must not
// appear as active workspace members or current release gates"). Companion
// UI surface: makepad-widgets, makepad-draw, and the bare `makepad` token.
// Banned tokens:
//   - "vb_codegen"       : exact substring
//   - "vb_ui_model"      : exact substring
//   - "vb_ui_makepad"    : exact substring
//   - "makepad-widgets"  : exact substring
//   - "makepad-draw"     : exact substring
//   - "makepad" (bare)   : case-sensitive, word-boundary, must NOT be
//                          followed by '-', '_', or another word char so
//                          "velvet-ballistics" and "makepad-2.0" do not
//                          false-match. "Makepad" (capitalised) is allowed.
//
// Per-line allowlist: a single true comment-start line containing
// "# allow-removed-crate: <reason>" or a `//`/`//!`/`///` comment that
// starts with `allow-removed-crate: <reason>` suppresses the NEXT non-blank
// line only if that target line is itself comment-like or doc-only/historical
// prose with an explicit historical or negation marker. Active manifest
// entries, workspace members, and source `use` / `extern crate` lines are
// always active even if preceded by an allowlist marker.
//
// Output (all on stderr):
//   <path>:<lineno>: REMOVED-CRATE: <token>: <line>      (active violation)
//   <path>:<lineno>: allowlisted: <reason>: <line>       (suppressed by marker)
// Final line: "summary: active=N allowlisted=M files_scanned=K"
// Exit 0 if active == 0 and the scan completed.
// Exit 1 if active > 0.
// Exit 2 on scan errors, including explicit missing/unreadable targets
// or explicit inputs that yield zero readable files.

#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::dbg_macro
)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const VB_CODEGEN: &str = "vb_codegen";
const VB_UI_MODEL: &str = "vb_ui_model";
const VB_UI_MAKEPAD: &str = "vb_ui_makepad";
const MAKEPAD_WIDGETS: &str = "makepad-widgets";
const MAKEPAD_DRAW: &str = "makepad-draw";
const MAKEPAD_BARE: &str = "makepad";

const EXACT_MATCHES: [(&str, &str); 5] = [
    (VB_CODEGEN, "exact substring 'vb_codegen'"),
    (VB_UI_MODEL, "exact substring 'vb_ui_model'"),
    (VB_UI_MAKEPAD, "exact substring 'vb_ui_makepad'"),
    (MAKEPAD_WIDGETS, "exact substring 'makepad-widgets'"),
    (MAKEPAD_DRAW, "exact substring 'makepad-draw'"),
];

const SCAN_ROOTS: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "xtask/Cargo.toml",
    "fuzz/Cargo.toml",
    "crates",
    ".moon",
    "README.md",
    "docs",
    "verification",
];

const SELF_SKIP_NAMES: &[&str] = &[
    "check-removed-crate-residue.rs",
    "check-removed-crate-residue.sh",
    "test-check-removed-crate-residue.sh",
];

const SKIP_DIRS: &[&str] = &["target", "node_modules", ".bead-progress", ".evidence"];

const ALLOWLIST_PREFIXES: &[&str] = &[
    "# allow-removed-crate:",
    "// allow-removed-crate:",
    "//! allow-removed-crate:",
    "/// allow-removed-crate:",
    "; allow-removed-crate:",
    "<!-- allow-removed-crate:",
];

const HISTORICAL_MARKERS: &[&str] = &[
    "removed",
    "deferred",
    "historical",
    "legacy",
    "retired",
    "obsolete",
    "post-merge",
    "out of scope",
    "not active",
    "no longer",
    "current-scope",
    "release blocker",
    "cleanup debt",
    "fenced out",
    "exclude",
    "excluded",
    "forbid",
    "forbidden",
    "drop",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct Finding {
    line_no: usize,
    line_text: String,
    token: String,
    context: String,
    allowlisted: bool,
    reason: String,
}

fn parse_allow_reason(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    for prefix in ALLOWLIST_PREFIXES {
        if let Some(reason) = trimmed.strip_prefix(prefix) {
            let reason = reason.trim();
            if !reason.is_empty() {
                return Some(reason.to_owned());
            }
        }
    }
    None
}

fn is_comment_line(path: &Path, line: &str) -> bool {
    let trimmed = line.trim_start();
    if matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("sh") | Some("bash")
    ) {
        return trimmed.starts_with('#');
    }
    trimmed.starts_with('#') || trimmed.starts_with("//") || trimmed.starts_with(';') || trimmed.starts_with("<!--")
}

fn is_allowlisted_target_line(path: &Path, line: &str) -> bool {
    is_comment_line(path, line) || is_historical_doc_line(path, line)
}

fn is_doc_like(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("md" | "txt" | "rst" | "adoc")
    )
}

fn contains_historical_marker(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    HISTORICAL_MARKERS.iter().any(|marker| lower.contains(marker))
}

fn is_historical_doc_line(path: &Path, line: &str) -> bool {
    if !is_doc_like(path) {
        return false;
    }
    if contains_historical_marker(line) {
        return true;
    }
    let Some(file_stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
        return false;
    };
    let lower = file_stem.to_ascii_lowercase();
    lower.contains("deferred") || lower.contains("historical") || lower.contains("adr")
}

fn is_standalone_makepad(line: &str) -> bool {
    let needle = MAKEPAD_BARE;
    for (idx, _) in line.match_indices(needle) {
        if idx > 0
            && let Some(prev) = line[..idx].chars().next_back()
            && is_word_or_underscore(prev)
        {
            continue;
        }
        let after_idx = idx + needle.len();
        if let Some(next) = line[after_idx..].chars().next()
            && is_word_underscore_or_dash(next)
        {
            continue;
        }
        return true;
    }
    false
}

fn is_word_or_underscore(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_word_underscore_or_dash(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

fn check_line(line: &str, findings: &mut Vec<(String, String)>) {
    push_exact_matches(line, findings);
    if is_standalone_makepad(line) {
        findings.push((
            MAKEPAD_BARE.to_owned(),
            format!("standalone token '{MAKEPAD_BARE}' (word boundary)"),
        ));
    }
}

fn push_exact_matches(line: &str, findings: &mut Vec<(String, String)>) {
    for &(token, context) in &EXACT_MATCHES {
        if line.contains(token) {
            findings.push((token.to_owned(), context.to_owned()));
        }
    }
}

fn push_finding(
    findings: &mut Vec<Finding>,
    line_no: usize,
    line_text: &str,
    token: String,
    context: String,
    allowlisted: bool,
    reason: String,
) {
    findings.push(Finding {
        line_no,
        line_text: line_text.to_owned(),
        token,
        context,
        allowlisted,
        reason,
    });
}

#[derive(Debug, Default)]
struct ScanTotals {
    active_total: usize,
    allowlisted_total: usize,
    files_scanned: usize,
}

fn scan_text(path: &Path, text: &str) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();
    let mut pending_reason: Option<String> = None;
    for (idx, raw_line) in text.lines().enumerate() {
        pending_reason = scan_text_line(path, idx + 1, raw_line, pending_reason, &mut findings);
    }
    findings
}

fn scan_text_line(
    path: &Path,
    line_no: usize,
    raw_line: &str,
    pending_reason: Option<String>,
    findings: &mut Vec<Finding>,
) -> Option<String> {
    if let Some(reason) = parse_allow_reason(raw_line) {
        return Some(reason);
    }
    if raw_line.trim().is_empty() {
        return pending_reason;
    }
    if let Some(reason) = pending_reason && is_allowlisted_target_line(path, raw_line) {
        push_allowlisted_finding(findings, line_no, raw_line, reason);
        return None;
    }
    push_scan_findings(line_no, raw_line, findings);
    None
}

fn push_allowlisted_finding(
    findings: &mut Vec<Finding>,
    line_no: usize,
    line_text: &str,
    reason: String,
) {
    push_finding(
        findings,
        line_no,
        line_text,
        "allowlisted".to_owned(),
        "allowlist consumed".to_owned(),
        true,
        reason,
    );
}

fn push_scan_findings(line_no: usize, raw_line: &str, findings: &mut Vec<Finding>) {
    let mut line_findings: Vec<(String, String)> = Vec::new();
    check_line(raw_line, &mut line_findings);
    for (token, context) in line_findings {
        push_finding(
            findings,
            line_no,
            raw_line,
            token,
            context,
            false,
            String::new(),
        );
    }
}

fn scan_file(path: &Path) -> Result<Vec<Finding>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("{}: unreadable: {error}", path.display()))?;
    Ok(scan_text(path, &text))
}

fn collect_scan_files(targets: &[PathBuf], explicit_targets: bool) -> Result<Vec<PathBuf>, String> {
    let mut files: Vec<PathBuf> = Vec::new();
    for target in targets {
        collect_target_files(target, explicit_targets, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn collect_target_files(target: &Path, explicit_targets: bool, files: &mut Vec<PathBuf>) -> Result<(), String> {
    ensure_target_resolved(target, explicit_targets)?;
    if target.is_file() {
        files.push(target.to_path_buf());
        return Ok(());
    }
    collect_target_directory(target, explicit_targets, files)
}

fn ensure_target_resolved(target: &Path, explicit_targets: bool) -> Result<(), String> {
    match target.try_exists() {
        Ok(true) => Ok(()),
        Ok(false) if explicit_targets => Err(format!("explicit target missing: {}", target.display())),
        Ok(false) => Ok(()),
        Err(error) if explicit_targets => Err(format!("explicit target unreadable: {}: {error}", target.display())),
        Err(_) => Ok(()),
    }
}

fn collect_target_directory(target: &Path, explicit_targets: bool, files: &mut Vec<PathBuf>) -> Result<(), String> {
    walk(target, files, explicit_targets).map_err(|error| format!("scan {}: {error}", target.display()))
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>, explicit_targets: bool) -> io::Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            if explicit_targets {
                return Err(error);
            }
            return Ok(());
        }
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if should_skip_walk_entry(&path) {
            continue;
        }
        if path.is_dir() {
            walk(&path, out, explicit_targets)?;
        } else if should_scan_file(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn should_skip_walk_entry(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    (name.starts_with('.') && name != ".moon") || SKIP_DIRS.contains(&name)
}

fn should_scan_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if SELF_SKIP_NAMES.contains(&name) {
        return false;
    }
    if name == "Cargo.toml"
        || name == "Cargo.lock"
        || name == "README.md"
        || name == "Makefile"
    {
        return true;
    }
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    matches!(
        ext,
        "toml" | "lock" | "yml" | "yaml" | "rs" | "sh" | "bash" | "py" | "md" | "txt" | "tla" | "cfg"
    )
}

fn relative_label(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

fn emit_summary(totals: &ScanTotals) {
    eprintln!(
        "summary: active={} allowlisted={} files_scanned={}",
        totals.active_total, totals.allowlisted_total, totals.files_scanned
    );
}

fn emit_finding(rel: &str, finding: Finding, totals: &mut ScanTotals) {
    if finding.allowlisted {
        totals.allowlisted_total = totals.allowlisted_total.saturating_add(1);
        eprintln!(
            "{rel}:{}: allowlisted: {}: {}",
            finding.line_no, finding.reason, finding.line_text
        );
        return;
    }
    totals.active_total = totals.active_total.saturating_add(1);
    eprintln!(
        "{rel}:{}: REMOVED-CRATE: {}: {}: {}",
        finding.line_no, finding.token, finding.context, finding.line_text
    );
}

fn emit_file_findings(root: &Path, file: &Path, findings: Vec<Finding>, totals: &mut ScanTotals) {
    totals.files_scanned = totals.files_scanned.saturating_add(1);
    let rel = relative_label(root, file);
    for finding in findings {
        emit_finding(&rel, finding, totals);
    }
}

fn run_scan(root: &Path, targets: &[PathBuf], explicit_targets: bool) -> Result<u8, String> {
    let files = collect_scan_files(targets, explicit_targets)?;
    let mut totals = ScanTotals::default();
    for file in &files {
        let findings = match scan_file(file) {
            Ok(findings) => findings,
            Err(message) => {
                if explicit_targets {
                    return Err(message);
                }
                eprintln!("{message}");
                continue;
            }
        };
        emit_file_findings(root, file, findings, &mut totals);
    }
    if explicit_targets && totals.files_scanned == 0 {
        return Err("no files successfully scanned from explicit targets".to_owned());
    }
    emit_summary(&totals);
    Ok(if totals.active_total == 0 { 0 } else { 1 })
}

fn resolve_default_targets(root: &Path) -> Vec<PathBuf> {
    SCAN_ROOTS
        .iter()
        .map(|name| root.join(name))
        .filter(|p| p.exists())
        .collect()
}

fn resolve_invocation_targets(root: &Path, args: &[String]) -> (Vec<PathBuf>, bool) {
    if args.is_empty() {
        return (resolve_default_targets(root), false);
    }
    (args.iter().map(PathBuf::from).collect(), true)
}

fn main() -> ExitCode {
    let root = match std::env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("check-removed-crate-residue: cannot read current directory: {error}");
            return ExitCode::from(2);
        }
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (targets, explicit_targets) = resolve_invocation_targets(&root, &args);
    if targets.is_empty() {
        eprintln!("check-removed-crate-residue: no scan targets resolved");
        return ExitCode::from(2);
    }
    match run_scan(&root, &targets, explicit_targets) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::from(1),
        Err(e) => {
            eprintln!("check-removed-crate-residue: {e}");
            ExitCode::from(2)
        }
    }
}
