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
// matching "<!-- /position-disclaimer -->" line closes it. The block is
// structural; explicit negation markers such as "is not", "isn't",
// "must not be", "is not a", "isn't a", "not a", and "not the" are what
// suppress banned phrases, and each banned phrase occurrence must be negated
// independently (or fall outside the negation clause). Unbalanced blocks are
// hard scan errors.
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
//   "summary: active=N disclaimered=K files_scanned=J"
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

use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
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

const CLAUSE_BREAK_MARKERS: &[&str] = &[
    " but ",
    " however ",
    " though ",
    " yet ",
    " instead ",
    " whereas ",
    " while ",
    " nevertheless ",
    " nonetheless ",
    " although ",
    ".",
    ";",
    ":",
    "!",
    "?",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FindingKind {
    Active,
    Disclaimered,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Finding {
    line_no: usize,
    text: String,
    phrase: &'static str,
    kind: FindingKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScanError {
    UnmatchedDisclaimerEnd { line_no: usize },
    UnclosedDisclaimerBlock { line_no: usize },
    NoScannableExplicitTargets,
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnmatchedDisclaimerEnd { line_no } => {
                write!(f, "unmatched position-disclaimer end at line {line_no}")
            }
            Self::UnclosedDisclaimerBlock { line_no } => {
                write!(
                    f,
                    "unclosed position-disclaimer block opened at line {line_no}"
                )
            }
            Self::NoScannableExplicitTargets => {
                write!(f, "no explicit scan targets were scanned")
            }
        }
    }
}

#[derive(Debug, Default)]
struct ScanSummary {
    active_total: usize,
    disclaimered_total: usize,
    files_scanned: usize,
}

impl ScanSummary {
    fn record_finding(&mut self, kind: FindingKind) {
        match kind {
            FindingKind::Active => {
                self.active_total = self.active_total.saturating_add(1);
            }
            FindingKind::Disclaimered => {
                self.disclaimered_total = self.disclaimered_total.saturating_add(1);
            }
        }
    }

    fn note_scanned_file(&mut self) {
        self.files_scanned = self.files_scanned.saturating_add(1);
    }

    fn has_scanned(&self) -> bool {
        self.files_scanned != 0
    }

    fn emit(&self) {
        eprintln!(
            "summary: active={} disclaimered={} files_scanned={}",
            self.active_total, self.disclaimered_total, self.files_scanned
        );
    }

    fn exit_code(&self) -> ExitCode {
        if self.active_total == 0 {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        }
    }
}

#[derive(Debug, Default)]
struct DisclaimerState {
    depth: usize,
    open_line: Option<usize>,
}

impl DisclaimerState {
    fn apply_markers(&mut self, line_no: usize, starts: bool, ends: bool) -> Result<(), ScanError> {
        if starts {
            if self.depth == 0 {
                self.open_line = Some(line_no);
            }
            self.depth = self.depth.saturating_add(1);
        }
        if ends {
            if self.depth == 0 {
                return Err(ScanError::UnmatchedDisclaimerEnd { line_no });
            }
            self.depth -= 1;
            if self.depth == 0 {
                self.open_line = None;
            }
        }
        Ok(())
    }

    fn finish(&self) -> Result<(), ScanError> {
        if self.depth == 0 {
            Ok(())
        } else {
            Err(ScanError::UnclosedDisclaimerBlock {
                line_no: self.open_line.unwrap_or_default(),
            })
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
        let Component::Normal(name) = component else {
            continue;
        };
        let s = name.to_string_lossy();
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

fn push_lowercase(target: &mut String, ch: char) {
    for lower in ch.to_lowercase() {
        target.push(lower);
    }
}

fn canonicalize_text(text: &str) -> String {
    let mut canonical = String::with_capacity(text.len());
    let mut pending_space = false;
    for ch in text.nfkc() {
        if is_zero_width(ch) {
            continue;
        }
        if is_separator(ch) {
            pending_space = !canonical.is_empty();
            continue;
        }
        if pending_space && !canonical.ends_with(' ') {
            canonical.push(' ');
        }
        pending_space = false;
        push_lowercase(&mut canonical, ch);
    }
    if canonical.ends_with(' ') {
        canonical.pop();
    }
    canonical
}

fn normalize_marker_text(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    for ch in text.nfkc() {
        if is_zero_width(ch) {
            continue;
        }
        push_lowercase(&mut normalized, ch);
    }
    normalized
}

fn line_has_negation_marker(line: &str) -> bool {
    NEGATION_MARKERS.iter().any(|marker| line.contains(marker))
}

fn latest_negation_scope(line: &str, phrase_start: usize) -> Option<(usize, usize)> {
    let prefix = &line[..phrase_start];
    let mut latest: Option<(usize, usize)> = None;

    for marker in NEGATION_MARKERS {
        if let Some(start) = prefix.rfind(marker) {
            let end = start + marker.len();
            if latest.map_or(true, |(_, latest_end)| end > latest_end) {
                latest = Some((start, end));
            }
        }
    }

    latest
}

fn clause_scope_broken(segment: &str) -> bool {
    CLAUSE_BREAK_MARKERS.iter().any(|marker| segment.contains(marker))
}

fn phrase_is_disclaimered(line: &str, phrase_start: usize) -> bool {
    let Some((_, marker_end)) = latest_negation_scope(line, phrase_start) else {
        return false;
    };
    let segment = &line[marker_end..phrase_start];
    !clause_scope_broken(segment)
}

fn finding_kind(line: &str, phrase_start: usize, line_has_negation: bool) -> FindingKind {
    if line_has_negation && phrase_is_disclaimered(line, phrase_start) {
        FindingKind::Disclaimered
    } else {
        FindingKind::Active
    }
}

fn emit_phrase_findings(
    line_no: usize,
    raw_line: &str,
    normalized_line: &str,
    line_has_negation: bool,
    findings: &mut Vec<Finding>,
) {
    for phrase in BANNED_PHRASES {
        let pattern = canonicalize_text(phrase);
        for (phrase_start, _) in normalized_line.match_indices(&pattern) {
            findings.push(Finding {
                line_no,
                text: raw_line.to_owned(),
                phrase,
                kind: finding_kind(normalized_line, phrase_start, line_has_negation),
            });
        }
    }
}

fn scan_line(
    line_no: usize,
    raw_line: &str,
    state: &mut DisclaimerState,
    findings: &mut Vec<Finding>,
) -> Result<(), ScanError> {
    let marker_line = normalize_marker_text(raw_line);
    let normalized_line = canonicalize_text(raw_line);
    let starts = marker_line.contains(DISCLAIMER_START);
    let ends = marker_line.contains(DISCLAIMER_END);
    let block_context = state.depth > 0 || starts;

    if ends && !block_context {
        return Err(ScanError::UnmatchedDisclaimerEnd { line_no });
    }

    let line_has_negation = line_has_negation_marker(&normalized_line);
    emit_phrase_findings(
        line_no,
        raw_line,
        &normalized_line,
        line_has_negation,
        findings,
    );
    state.apply_markers(line_no, starts, ends)
}

fn scan_text(text: &str) -> Result<Vec<Finding>, ScanError> {
    let mut state = DisclaimerState::default();
    let mut findings = Vec::new();

    for (idx, raw_line) in text.lines().enumerate() {
        scan_line(idx + 1, raw_line, &mut state, &mut findings)?;
    }

    state.finish()?;
    Ok(findings)
}

fn scan_file(path: &Path) -> io::Result<Result<Vec<Finding>, ScanError>> {
    let text = fs::read_to_string(path)?;
    Ok(scan_text(&text))
}

fn emit_finding(rel: &str, finding: &Finding) {
    match finding.kind {
        FindingKind::Active => {
            eprintln!(
                "{}:{}: POSITIONING: {}: {}",
                rel, finding.line_no, finding.phrase, finding.text
            );
        }
        FindingKind::Disclaimered => {
            eprintln!(
                "{}:{}: disclaimered: {}: {}",
                rel, finding.line_no, finding.phrase, finding.text
            );
        }
    }
}

fn emit_findings(root: &Path, file: &Path, findings: Vec<Finding>, summary: &mut ScanSummary) {
    let rel = relative_label(root, file);
    for finding in findings {
        summary.record_finding(finding.kind);
        emit_finding(&rel, &finding);
    }
}

fn scan_target(root: &Path, file: &Path, summary: &mut ScanSummary) -> Result<(), ScanError> {
    if path_under_skip_dir(file) {
        return Ok(());
    }

    match scan_file(file) {
        Ok(Ok(findings)) => {
            summary.note_scanned_file();
            emit_findings(root, file, findings, summary);
            Ok(())
        }
        Ok(Err(error)) => Err(error),
        Err(error) => {
            eprintln!("check-product-positioning: unreadable: {}: {error}", file.display());
            Ok(())
        }
    }
}

fn scan_files(root: &Path, files: &[PathBuf], explicit_inputs: bool) -> Result<ScanSummary, ScanError> {
    let mut summary = ScanSummary::default();

    for file in files {
        scan_target(root, file, &mut summary)?;
    }

    if explicit_inputs && !summary.has_scanned() {
        return Err(ScanError::NoScannableExplicitTargets);
    }

    Ok(summary)
}

fn should_scan_file(root: &Path, path: &Path, enforce_surface: bool) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if is_skipped_basename(name) {
        return false;
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
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

fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>, enforce_surface: bool) -> io::Result<()> {
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

fn collect_explicit_files(root: &Path, args: &[String], out: &mut Vec<PathBuf>) -> io::Result<()> {
    for raw in args {
        collect_arg_files(root, Path::new(raw), out)?;
    }
    Ok(())
}

fn collect_scan_targets(root: &Path, args: &[String], using_defaults: bool) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if using_defaults {
        collect_default_files(root, &mut files)?;
    } else {
        collect_explicit_files(root, args, &mut files)?;
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn process(root: &Path, files: &[PathBuf], explicit_inputs: bool) -> ExitCode {
    match scan_files(root, files, explicit_inputs) {
        Ok(summary) => {
            summary.emit();
            summary.exit_code()
        }
        Err(error) => {
            eprintln!("check-product-positioning: scan error: {error}");
            ExitCode::from(2)
        }
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

    match collect_scan_targets(&root, &args, using_defaults) {
        Ok(files) => process(&root, &files, !using_defaults),
        Err(error) => {
            eprintln!("check-product-positioning: collect targets: {error}");
            ExitCode::from(2)
        }
    }
}
