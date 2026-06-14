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
// Banned phrases (case-insensitive after normalization, not whole-word):
//   - generic dag runner
//   - low-code graph editor
//   - yaml-as-programming
//   - yaml as programming
//   - airflow replacement
//   - airflow alternative
//   - temporal clone
//   - temporal alternative
//
// Matching normalizes Unicode with NFKC, strips zero-width characters,
// collapses hyphen/underscore/whitespace runs to a single space, and lower-
// cases the result before matching.
//
// Block disclaimer: a "<!-- position-disclaimer -->" line opens a block; the
// matching "<!-- /position-disclaimer -->" line closes it. Only lines inside
// the block that also contain an explicit negation marker such as
// "is not", "isn't", "must not be", "is not a", "isn't a", "not a",
// or "not the" are disclaimered. Any banned phrase on a non-negated line is
// ACTIVE. Unbalanced blocks are hard scan errors.
//
// Self-skip basenames (any directory): velvet-ballistics-MASTER.md,
// CHANGELOG.md, HISTORY.md, MIGRATION.md.
// Self-skip directories (and their descendants): target, node_modules,
// .git, .beads, .dolt, .moon, .jj, .evidence, .bead-progress, and any
// directory starting with '.'
//
// Default scan surface (relative to repo root):
//   - *.md at the repository root
//   - README.md
//   - docs/**/*.md
//   - crates/**/README.md
//   - crates/vb_cli/**/*.md
//   - fuzz/*.md
//
// Output (all on stderr):
//   <rel>:N: POSITIONING: <phrase>: <line>      (active violation)
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
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use unicode_normalization::UnicodeNormalization;

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

const NEGATION_MARKERS: &[&str] = &[
    "is not",
    "isn't",
    "must not be",
    "is not a",
    "isn't a",
    "not a",
    "not the",
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
    ".git",
    ".beads",
    ".dolt",
    ".moon",
    ".jj",
    ".evidence",
    ".bead-progress",
];

const DISCLAIMER_START: &str = "<!-- position-disclaimer -->";
const DISCLAIMER_END: &str = "<!-- /position-disclaimer -->";
const ZERO_WIDTH_CHARS: [char; 4] = ['\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}'];

#[derive(Debug, Clone, PartialEq, Eq)]
enum FindingKind {
    Active,
    Disclaimered,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Finding {
    line_no: usize,
    text: String,
    phrase: String,
    kind: FindingKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScanError {
    UnmatchedDisclaimerEnd { line_no: usize },
    UnclosedDisclaimerBlock { line_no: usize },
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnmatchedDisclaimerEnd { line_no } => {
                write!(f, "unmatched position-disclaimer end at line {line_no}")
            }
            Self::UnclosedDisclaimerBlock { line_no } => {
                write!(f, "unclosed position-disclaimer block opened at line {line_no}")
            }
        }
    }
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
        if s.starts_with('.') || is_skipped_dir(s.as_ref()) {
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
    if rel.ends_with(".md") && !rel.contains('/') {
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
    if let Some(remainder) = rel.strip_prefix("fuzz/") {
        if rel.ends_with(".md") && !remainder.contains('/') {
            return true;
        }
    }
    false
}

fn is_zero_width(ch: char) -> bool {
    ZERO_WIDTH_CHARS.contains(&ch)
}

fn is_separator(ch: char) -> bool {
    ch.is_whitespace() || ch == '-' || ch == '_'
}

fn canonicalize_text(text: &str) -> String {
    let mut canonical = String::with_capacity(text.len());
    let mut needs_space = false;

    for ch in text.nfkc() {
        if is_zero_width(ch) {
            continue;
        }
        if is_separator(ch) {
            needs_space = !canonical.is_empty();
            continue;
        }
        if needs_space && !canonical.ends_with(' ') {
            canonical.push(' ');
        }
        needs_space = false;
        for lower in ch.to_lowercase() {
            canonical.push(lower);
        }
    }

    if canonical.ends_with(' ') {
        canonical.pop();
    }

    canonical
}

fn contains_negation_marker(line: &str) -> bool {
    NEGATION_MARKERS.iter().any(|marker| line.contains(marker))
}

fn find_banned_phrases(normalized_line: &str) -> Vec<&'static str> {
    BANNED_PHRASES
        .iter()
        .copied()
        .filter(|phrase| normalized_line.contains(&canonicalize_text(phrase)))
        .collect()
}

fn scan_text(text: &str) -> Result<Vec<Finding>, ScanError> {
    let mut findings: Vec<Finding> = Vec::new();
    let mut disclaimer_depth: usize = 0;
    let mut disclaimer_open_line: Option<usize> = None;

    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let normalized_line = canonicalize_text(raw_line);
        let starts = normalized_line.contains(DISCLAIMER_START);
        let ends = normalized_line.contains(DISCLAIMER_END);
        let line_in_disclaimer = disclaimer_depth > 0 || starts;

        if ends && !line_in_disclaimer {
            return Err(ScanError::UnmatchedDisclaimerEnd { line_no });
        }

        let phrases = find_banned_phrases(&normalized_line);
        if phrases.is_empty() {
            if starts {
                if disclaimer_depth == 0 {
                    disclaimer_open_line = Some(line_no);
                }
                if disclaimer_depth == usize::MAX {
                    return Err(ScanError::UnclosedDisclaimerBlock { line_no });
                }
                disclaimer_depth += 1;
            }
            if ends {
                if disclaimer_depth == 0 {
                    return Err(ScanError::UnmatchedDisclaimerEnd { line_no });
                }
                disclaimer_depth -= 1;
                if disclaimer_depth == 0 {
                    disclaimer_open_line = None;
                }
            }
            continue;
        }

        let disclaimered = line_in_disclaimer && contains_negation_marker(&normalized_line);
        let kind = if disclaimered {
            FindingKind::Disclaimered
        } else {
            FindingKind::Active
        };

        for phrase in phrases {
            findings.push(Finding {
                line_no,
                text: raw_line.to_owned(),
                phrase,
                kind: kind.clone(),
            });
        }

        if starts {
            if disclaimer_depth == 0 {
                disclaimer_open_line = Some(line_no);
            }
            if disclaimer_depth == usize::MAX {
                return Err(ScanError::UnclosedDisclaimerBlock { line_no });
            }
            disclaimer_depth += 1;
        }
        if ends {
            if disclaimer_depth == 0 {
                return Err(ScanError::UnmatchedDisclaimerEnd { line_no });
            }
            disclaimer_depth -= 1;
            if disclaimer_depth == 0 {
                disclaimer_open_line = None;
            }
        }
    }

    if disclaimer_depth != 0 {
        return Err(ScanError::UnclosedDisclaimerBlock {
            line_no: disclaimer_open_line.unwrap_or_default(),
        });
    }

    Ok(findings)
}

fn scan_file(path: &Path) -> io::Result<Result<Vec<Finding>, ScanError>> {
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
    walk(root, root, out, true)
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
    let allowlisted_total: usize = 0;
    let mut disclaimered_total: usize = 0;
    let mut files_scanned: usize = 0;

    for file in &files {
        if path_under_skip_dir(file) {
            continue;
        }
        match scan_file(file) {
            Ok(Ok(findings)) => {
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
            Ok(Err(error)) => {
                eprintln!(
                    "check-product-positioning: scan error: {}: {error}",
                    file.display()
                );
                return ExitCode::from(2);
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

pub fn main() -> ExitCode {
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
