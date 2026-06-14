// SPDX-License-Identifier: MIT
// check-product-positioning: enforce the velvet-ballistics Product
// Positioning Contract (velvet-ballistics-MASTER.md:29).
//
// Verbatim master quote:
//   "Publicly, velvet-ballistics must not be described as a generic DAG
//    runner, low-code graph editor, YAML-as-programming framework, Airflow
//    replacement, or Temporal clone. Those frames hide the actual wedge and
//    invite false comparisons."
//
// Banned phrases (case-insensitive substring match, not whole-word):
//   - generic dag runner
//   - low-code graph editor
//   - yaml-as-programming
//   - yaml as programming
//   - airflow replacement
//   - airflow alternative
//   - temporal clone
//   - temporal alternative
//
// Per-line allowlist: a single line containing the substring
// "<!-- ALLOW_HISTORICAL: <reason> -->" suppresses the banned phrase(s) on
// that same line. The suppressed line is reported as
// "<rel>:<lineno>: allowlisted: <reason>: <line>" and never causes a failure.
//
// Block allowlist: a "<!-- position-disclaimer -->" line opens a block; the
// matching "<!-- /position-disclaimer -->" line closes it. Every match
// inside the block is reported as
// "<rel>:<lineno>: disclaimered: <phrase>: <line>" and never causes a
// failure.
//
// Self-skip basenames (any directory): velvet-ballistics-MASTER.md,
// CHANGELOG.md, HISTORY.md, MIGRATION.md.
// Self-skip directories (and their descendants): target, node_modules,
// .bead-progress, .evidence.
//
// Default scan surface (relative to repo root):
//   - README.md
//   - docs/**/*.md
//   - crates/**/README.md
//   - crates/vb_cli/**/*.md
//
// Output (all on stderr):
//   <rel>:N: POSITIONING: <phrase>: <line>      (active violation)
//   <rel>:N: allowlisted: <reason>: <line>      (suppressed by per-line marker)
//   <rel>:N: disclaimered: <phrase>: <line>     (suppressed by block marker)
// Final line:
//   "summary: active=N allowlisted=M disclaimered=K files_scanned=J"
//
// Exit 0 if active == 0, exit 1 otherwise.

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

const BANNED_PHRASES: &[&str] = &[
    "generic dag runner",
    "low-code graph editor",
    "yaml-as-programming",
    "yaml as programming",
    "airflow replacement",
    "airflow alternative",
    "temporal clone",
    "temporal alternative",
];

const SKIP_BASENAMES: &[&str] = &[
    "velvet-ballistics-MASTER.md",
    "CHANGELOG.md",
    "HISTORY.md",
    "MIGRATION.md",
];

const SKIP_DIR_NAMES: &[&str] = &[
    "target",
    "node_modules",
    ".bead-progress",
    ".evidence",
];

const ALLOW_HISTORICAL_MARKER: &str = "<!-- ALLOW_HISTORICAL:";
const DISCLAIMER_START: &str = "<!-- position-disclaimer -->";
const DISCLAIMER_END: &str = "<!-- /position-disclaimer -->";
const COMMENT_TERMINATOR: &str = "-->";

#[derive(Debug, Clone, PartialEq, Eq)]
enum FindingKind {
    Active,
    Allowlisted { reason: String },
    Disclaimered,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Finding {
    line_no: usize,
    text: String,
    phrase: String,
    kind: FindingKind,
}

fn is_skipped_basename(name: &str) -> bool {
    SKIP_BASENAMES.iter().any(|s| *s == name)
}

fn is_skipped_dir(name: &str) -> bool {
    SKIP_DIR_NAMES.iter().any(|s| *s == name)
}

fn path_under_skip_dir(path: &Path) -> bool {
    for component in path.components() {
        let s = component.as_os_str().to_string_lossy();
        if is_skipped_dir(s.as_ref()) {
            return true;
        }
    }
    false
}

fn relative_label(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

fn matches_scan_surface(rel: &str) -> bool {
    if rel == "README.md" {
        return true;
    }
    if rel.starts_with("docs/") && rel.ends_with(".md") {
        return true;
    }
    if rel.starts_with("crates/") && rel.ends_with("/README.md") {
        return true;
    }
    if rel.starts_with("crates/vb_cli/") && rel.ends_with(".md") {
        return true;
    }
    false
}

fn parse_allow_reason(line: &str) -> Option<String> {
    let idx = line.find(ALLOW_HISTORICAL_MARKER)?;
    let after = &line[idx + ALLOW_HISTORICAL_MARKER.len()..];
    let trimmed_start = after.trim_start();
    let end_idx = trimmed_start.find(COMMENT_TERMINATOR)?;
    let reason = trimmed_start[..end_idx].trim();
    if reason.is_empty() {
        None
    } else {
        Some(reason.to_owned())
    }
}

fn find_banned_phrases(line: &str) -> Vec<String> {
    let lower = line.to_lowercase();
    BANNED_PHRASES
        .iter()
        .filter(|p| lower.contains(**p))
        .map(|p| (*p).to_owned())
        .collect()
}

fn scan_text(text: &str) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();
    let mut in_disclaimer: bool = false;

    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;

        if raw_line.contains(DISCLAIMER_START) {
            in_disclaimer = true;
        }
        if raw_line.contains(DISCLAIMER_END) {
            in_disclaimer = false;
        }

        let phrases = find_banned_phrases(raw_line);
        if phrases.is_empty() {
            continue;
        }

        for phrase in phrases {
            let kind = if in_disclaimer {
                FindingKind::Disclaimered
            } else if let Some(reason) = parse_allow_reason(raw_line) {
                FindingKind::Allowlisted { reason }
            } else {
                FindingKind::Active
            };
            findings.push(Finding {
                line_no,
                text: raw_line.to_owned(),
                phrase,
                kind,
            });
        }
    }

    findings
}

fn scan_file(path: &Path) -> io::Result<Vec<Finding>> {
    let text = fs::read_to_string(path)?;
    Ok(scan_text(&text))
}

fn should_scan_file(root: &Path, path: &Path, enforce_surface: bool) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if is_skipped_basename(name) {
        return false;
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();
    if ext != "md" {
        return false;
    }
    if enforce_surface {
        let rel = relative_label(root, path);
        if !matches_scan_surface(&rel) {
            return false;
        }
    }
    true
}

fn walk(
    root: &Path,
    dir: &Path,
    out: &mut Vec<PathBuf>,
    enforce_surface: bool,
) -> io::Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_owned(),
            None => continue,
        };
        if is_skipped_dir(&name) {
            continue;
        }
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, out, enforce_surface)?;
        } else if should_scan_file(root, &path, enforce_surface) {
            out.push(path);
        }
    }
    Ok(())
}

fn collect_default_files(root: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    let readme = root.join("README.md");
    if readme.is_file() && should_scan_file(root, &readme, true) {
        out.push(readme);
    }
    let docs = root.join("docs");
    if docs.exists() {
        walk(root, &docs, out, true)?;
    }
    let crates = root.join("crates");
    if crates.exists() {
        walk(root, &crates, out, true)?;
    }
    Ok(())
}

fn collect_arg_files(root: &Path, arg: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    if !arg.exists() {
        return Ok(());
    }
    if arg.is_file() {
        if should_scan_file(root, arg, false) {
            out.push(arg.to_path_buf());
        }
        return Ok(());
    }
    walk(root, arg, out, false)
}

fn process(root: &Path, files: Vec<PathBuf>) -> ExitCode {
    let mut active_total: usize = 0;
    let mut allowlisted_total: usize = 0;
    let mut disclaimered_total: usize = 0;
    let mut files_scanned: usize = 0;

    for file in &files {
        if path_under_skip_dir(file) {
            continue;
        }
        match scan_file(file) {
            Ok(findings) => {
                files_scanned = files_scanned.saturating_add(1);
                let rel = relative_label(root, file);
                for finding in findings {
                    match &finding.kind {
                        FindingKind::Active => {
                            active_total = active_total.saturating_add(1);
                            eprintln!(
                                "{}:{}: POSITIONING: {}: {}",
                                rel, finding.line_no, finding.phrase, finding.text
                            );
                        }
                        FindingKind::Allowlisted { reason } => {
                            allowlisted_total = allowlisted_total.saturating_add(1);
                            eprintln!(
                                "{}:{}: allowlisted: {}: {}",
                                rel, finding.line_no, reason, finding.text
                            );
                        }
                        FindingKind::Disclaimered => {
                            disclaimered_total = disclaimered_total.saturating_add(1);
                            eprintln!(
                                "{}:{}: disclaimered: {}: {}",
                                rel, finding.line_no, finding.phrase, finding.text
                            );
                        }
                    }
                }
            }
            Err(error) => {
                eprintln!(
                    "check-product-positioning: unreadable: {}: {error}",
                    file.display()
                );
            }
        }
    }

    eprintln!(
        "summary: active={} allowlisted={} disclaimered={} files_scanned={}",
        active_total, allowlisted_total, disclaimered_total, files_scanned
    );

    if active_total == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn main() -> ExitCode {
    let root = match std::env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("check-product-positioning: cannot read current directory: {error}");
            return ExitCode::from(2);
        }
    };

    let args: Vec<String> = std::env::args().skip(1).collect();
    let using_defaults = args.is_empty();

    if using_defaults {
        let mut files: Vec<PathBuf> = Vec::new();
        if let Err(error) = collect_default_files(&root, &mut files) {
            eprintln!("check-product-positioning: collect defaults: {error}");
            return ExitCode::from(2);
        }
        files.sort();
        files.dedup();
        process(&root, files)
    } else {
        let mut files: Vec<PathBuf> = Vec::new();
        for raw in &args {
            let arg = PathBuf::from(raw);
            if let Err(error) = collect_arg_files(&root, &arg, &mut files) {
                eprintln!("check-product-positioning: collect {}: {error}", arg.display());
                return ExitCode::from(2);
            }
        }
        files.sort();
        files.dedup();
        process(&root, files)
    }
}
