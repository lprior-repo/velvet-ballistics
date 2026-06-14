// SPDX-License-Identifier: MIT
// check-cold-adapter-isolation: scans the four runtime-core boundary
// crates (vb_core, vb_runtime, vb_storage, vb_ipc) for ACTIVE HTTP /
// JSON / YAML / adapter-only dependencies or `use` / `extern crate`
// imports. The master contract (velvet-ballistics-MASTER.md:62) is:
//
//   "HTTP and JSON are excluded from the v1 runtime core. Any future
//    adapter must be a separate cold-path adapter crate and must not
//    enter vb_core, vb_runtime, vb_storage, or vb_ipc."
//
// Forbidden crate tokens (whole-word, hyphen-/underscore-safe):
//   serde_json, saphyr, saphyr-parser, serde-saphyr, reqwest, hyper,
//   axum, ureq, attohttpc, isahc.
//
// Per-line allowlist: a single line containing
// "# allow-cold-adapter: <reason>" or "// allow-cold-adapter: <reason>"
// suppresses the NEXT non-blank line. The suppressed line is reported
// as "allowlisted:" (still counts in the summary) and never causes a
// failure.
//
// Output (all on stderr):
//   <path>:<lineno>: COLD-ADAPTER: <crate>: <context>: <line>   (active)
//   <path>:<lineno>: allowlisted: <reason>: <line>            (suppressed)
// Final line: "summary: active=N allowlisted=M files_scanned=K"
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

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const BOUNDARY_CRATES: &[&str] = &["vb_core", "vb_runtime", "vb_storage", "vb_ipc"];

const FORBIDDEN_CRATE_NAMES: &[&str] = &[
    "serde_json",
    "saphyr",
    "saphyr-parser",
    "serde-saphyr",
    "reqwest",
    "hyper",
    "axum",
    "ureq",
    "attohttpc",
    "isahc",
];

const ALLOW_MARKER: &str = "allow-cold-adapter:";

const SELF_SKIP_NAMES: &[&str] = &[
    "check-cold-adapter-isolation.rs",
    "check-cold-adapter-isolation.sh",
    "test-check-cold-adapter-isolation.sh",
];

const SKIP_DIRS: &[&str] = &["target", "node_modules", ".bead-progress", ".evidence"];

const CARGO_DEP_TABLES: &[&str] = &["[dependencies]", "[dev-dependencies]"];

#[derive(Debug, Clone, PartialEq, Eq)]
enum ViolationKind {
    CargoDep,
    RustImport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawHit {
    line_no: usize,
    line_text: String,
    crate_token: String,
    kind: ViolationKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Finding {
    line_no: usize,
    line_text: String,
    crate_token: String,
    context: String,
    allowlisted: bool,
    reason: String,
}

fn parse_allow_reason(line: &str) -> Option<String> {
    let token_idx = line.find(ALLOW_MARKER)?;
    if !is_valid_marker_prefix(line, token_idx) {
        return None;
    }
    let after = &line[token_idx + ALLOW_MARKER.len()..];
    let reason = after.trim();
    if reason.is_empty() {
        None
    } else {
        Some(reason.to_owned())
    }
}

fn is_valid_marker_prefix(line: &str, token_idx: usize) -> bool {
    let bytes = line.as_bytes();
    if token_idx == 0 {
        return false;
    }
    if bytes[token_idx - 1] != b' ' && bytes[token_idx - 1] != b'\t' {
        return false;
    }
    if token_idx < 2 {
        return false;
    }
    let prev_byte = bytes[token_idx - 2];
    if prev_byte == b'#' {
        return true;
    }
    if prev_byte == b'/' || prev_byte == b'!' {
        return true;
    }
    false
}

fn is_word_boundary_at(line: &str, idx: usize, needle: &str) -> bool {
    if idx > 0 {
        if let Some(prev) = line[..idx].chars().next_back() {
            if prev.is_alphanumeric() || prev == '_' || prev == '-' {
                return false;
            }
        }
    }
    let after_idx = idx + needle.len();
    if let Some(next) = line[after_idx..].chars().next() {
        if next.is_alphanumeric() || next == '_' || next == '-' {
            return false;
        }
    }
    true
}

fn line_contains_token(line: &str, token: &str) -> bool {
    for (idx, _) in line.match_indices(token) {
        if is_word_boundary_at(line, idx, token) {
            return true;
        }
    }
    false
}

fn last_word_of(s: &str) -> &str {
    let trimmed = s.trim_end();
    match trimmed.rfind(char::is_whitespace) {
        Some(pos) => &trimmed[pos + 1..],
        None => trimmed,
    }
}

fn line_is_use_import_of(line: &str, token: &str) -> bool {
    let use_needle = format!("use {token}");
    let extern_needle = format!("extern crate {token}");
    for needle in &[use_needle.as_str(), extern_needle.as_str()] {
        let mut search_start: usize = 0;
        while let Some(rel) = line[search_start..].find(needle) {
            let abs = search_start + rel;
            let after = abs + needle.len();
            let boundary_ok = match line[after..].chars().next() {
                Some(ch) => !(ch.is_alphanumeric() || ch == '_' || ch == '-'),
                None => true,
            };
            if !boundary_ok {
                search_start = abs + needle.len();
                continue;
            }
            let prev = last_word_of(&line[..abs]);
            let valid = (*needle == use_needle.as_str() && prev == "use")
                || (*needle == extern_needle.as_str() && prev == "crate");
            if valid {
                return true;
            }
            search_start = abs + needle.len();
        }
    }
    false
}

fn find_forbidden_dep_in_line(line: &str, forbidden: &[&str]) -> Option<String> {
    let (name_part, _rest) = line.split_once('=')?;
    let dep_name = name_part
        .trim()
        .split('.')
        .next()
        .unwrap_or("")
        .trim();
    if dep_name.is_empty() {
        return None;
    }
    forbidden
        .iter()
        .find(|tok| line_contains_token(dep_name, tok))
        .map(|tok| (*tok).to_owned())
}

fn scan_cargo_manifest(text: &str) -> Vec<RawHit> {
    let mut hits: Vec<RawHit> = Vec::new();
    let allowed_tables: BTreeSet<&str> = CARGO_DEP_TABLES.iter().copied().collect();
    let mut in_dep_table: bool = false;
    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = raw_line.trim();
        if trimmed.starts_with('[') {
            in_dep_table = allowed_tables.contains(trimmed);
            continue;
        }
        if !in_dep_table || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(forbidden) = find_forbidden_dep_in_line(trimmed, FORBIDDEN_CRATE_NAMES) {
            hits.push(RawHit {
                line_no,
                line_text: (*raw_line).to_owned(),
                crate_token: forbidden,
                kind: ViolationKind::CargoDep,
            });
        }
    }
    hits
}

fn scan_rust_file(text: &str) -> Vec<RawHit> {
    let mut hits: Vec<RawHit> = Vec::new();
    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        let code = trimmed.split("//").next().unwrap_or(trimmed);
        for forbidden in FORBIDDEN_CRATE_NAMES {
            if line_is_use_import_of(code, forbidden) {
                hits.push(RawHit {
                    line_no,
                    line_text: (*raw_line).to_owned(),
                    crate_token: (*forbidden).to_owned(),
                    kind: ViolationKind::RustImport,
                });
                break;
            }
        }
    }
    hits
}

fn apply_marker_allowlist(text: &str, raw: Vec<RawHit>) -> Vec<Finding> {
    // Build a per-line allowlist reason: line_reasons[i] = Some(reason) if a
    // marker on a previous non-blank line suppresses line i+1.
    let mut line_reasons: Vec<Option<String>> = vec![None; text.lines().count()];
    let mut pending: Option<String> = None;
    for (idx, raw_line) in text.lines().enumerate() {
        if let Some(reason) = parse_allow_reason(raw_line) {
            pending = Some(reason);
            continue;
        }
        if raw_line.trim().is_empty() {
            continue;
        }
        if let Some(reason) = pending.take() {
            line_reasons[idx] = Some(reason);
        }
    }
    let mut findings: Vec<Finding> = Vec::new();
    for hit in raw {
        let reason = line_reasons
            .get(hit.line_no.saturating_sub(1))
            .and_then(|r| r.clone());
        match reason {
            Some(reason_text) => {
                findings.push(Finding {
                    line_no: hit.line_no,
                    line_text: hit.line_text,
                    crate_token: "allowlisted".to_owned(),
                    context: "allowlist consumed".to_owned(),
                    allowlisted: true,
                    reason: reason_text,
                });
            }
            None => {
                let context = match hit.kind {
                    ViolationKind::CargoDep => {
                        "forbidden dependency in [dependencies]/[dev-dependencies]".to_owned()
                    }
                    ViolationKind::RustImport => {
                        "forbidden `use`/`extern crate` import in source".to_owned()
                    }
                };
                findings.push(Finding {
                    line_no: hit.line_no,
                    line_text: hit.line_text,
                    crate_token: hit.crate_token,
                    context,
                    allowlisted: false,
                    reason: String::new(),
                });
            }
        }
    }
    findings
}

#[derive(Debug)]
enum ScanOutcome {
    Findings(Vec<Finding>),
    Unreadable(String),
}

fn scan_cargo_text(text: &str) -> Vec<Finding> {
    let raw = scan_cargo_manifest(text);
    apply_marker_allowlist(text, raw)
}

fn scan_rust_text(text: &str) -> Vec<Finding> {
    let raw = scan_rust_file(text);
    apply_marker_allowlist(text, raw)
}

fn scan_cargo_file(path: &Path) -> ScanOutcome {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(error) => {
            return ScanOutcome::Unreadable(format!(
                "{}: unreadable: {error}",
                path.display()
            ));
        }
    };
    ScanOutcome::Findings(scan_cargo_text(&text))
}

fn scan_rust_source_file(path: &Path) -> ScanOutcome {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(error) => {
            return ScanOutcome::Unreadable(format!(
                "{}: unreadable: {error}",
                path.display()
            ));
        }
    };
    ScanOutcome::Findings(scan_rust_text(&text))
}

fn is_self_skip_path(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    SELF_SKIP_NAMES.iter().any(|n| *n == name)
}

fn collect_rust_sources(src_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    walk(src_dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if SKIP_DIRS.iter().any(|d| *d == name) {
            continue;
        }
        if name.starts_with('.') {
            continue;
        }
        if is_self_skip_path(&path) {
            continue;
        }
        if path.is_dir() {
            walk(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn relative_label(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

fn emit_findings(
    rel: &str,
    findings: Vec<Finding>,
    active_total: &mut usize,
    allowlisted_total: &mut usize,
) {
    for finding in findings {
        if finding.allowlisted {
            *allowlisted_total = allowlisted_total.saturating_add(1);
            eprintln!(
                "{rel}:{}: allowlisted: {}: {}",
                finding.line_no, finding.reason, finding.line_text
            );
        } else {
            *active_total = active_total.saturating_add(1);
            eprintln!(
                "{rel}:{}: COLD-ADAPTER: {}: {}: {}",
                finding.line_no,
                finding.crate_token,
                finding.context,
                finding.line_text
            );
        }
    }
}

fn run_default_scan(root: &Path) -> Result<u8, String> {
    let mut active_total: usize = 0;
    let mut allowlisted_total: usize = 0;
    let mut files_scanned: usize = 0;

    for crate_name in BOUNDARY_CRATES {
        let crate_dir = root.join("crates").join(crate_name);
        if !crate_dir.is_dir() {
            return Err(format!("missing boundary crate directory: {}", crate_dir.display()));
        }
        let manifest_path = crate_dir.join("Cargo.toml");
        if !manifest_path.is_file() {
            return Err(format!(
                "missing manifest for {crate_name}: {}",
                manifest_path.display()
            ));
        }
        let manifest_rel = relative_label(root, &manifest_path);
        match scan_cargo_file(&manifest_path) {
            ScanOutcome::Findings(findings) => {
                files_scanned = files_scanned.saturating_add(1);
                emit_findings(
                    &manifest_rel,
                    findings,
                    &mut active_total,
                    &mut allowlisted_total,
                );
            }
            ScanOutcome::Unreadable(message) => {
                eprintln!("{message}");
            }
        }

        let src_dir = crate_dir.join("src");
        if !src_dir.is_dir() {
            continue;
        }
        let sources = match collect_rust_sources(&src_dir) {
            Ok(s) => s,
            Err(error) => {
                eprintln!("{}: walk error: {error}", src_dir.display());
                continue;
            }
        };
        for source_path in sources {
            let rel = relative_label(root, &source_path);
            match scan_rust_source_file(&source_path) {
                ScanOutcome::Findings(findings) => {
                    files_scanned = files_scanned.saturating_add(1);
                    emit_findings(&rel, findings, &mut active_total, &mut allowlisted_total);
                }
                ScanOutcome::Unreadable(message) => {
                    eprintln!("{message}");
                }
            }
        }
    }

    eprintln!(
        "summary: active={active_total} allowlisted={allowlisted_total} files_scanned={files_scanned}"
    );

    if active_total == 0 {
        Ok(0)
    } else {
        Ok(1)
    }
}

fn run_targeted_scan(root: &Path, targets: &[PathBuf]) -> Result<u8, String> {
    let mut active_total: usize = 0;
    let mut allowlisted_total: usize = 0;
    let mut files_scanned: usize = 0;
    for target in targets {
        if !target.exists() {
            return Err(format!("target does not exist: {}", target.display()));
        }
        let file_name = target
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let is_cargo = file_name == "Cargo.toml";
        let is_rs = target
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e == "rs")
            .unwrap_or(false);
        let rel = relative_label(root, target);
        if is_cargo {
            match scan_cargo_file(target) {
                ScanOutcome::Findings(findings) => {
                    files_scanned = files_scanned.saturating_add(1);
                    emit_findings(&rel, findings, &mut active_total, &mut allowlisted_total);
                }
                ScanOutcome::Unreadable(message) => {
                    eprintln!("{message}");
                }
            }
        } else if is_rs {
            match scan_rust_source_file(target) {
                ScanOutcome::Findings(findings) => {
                    files_scanned = files_scanned.saturating_add(1);
                    emit_findings(&rel, findings, &mut active_total, &mut allowlisted_total);
                }
                ScanOutcome::Unreadable(message) => {
                    eprintln!("{message}");
                }
            }
        } else {
            return Err(format!(
                "unsupported target (must be Cargo.toml or .rs file): {}",
                target.display()
            ));
        }
    }
    eprintln!(
        "summary: active={active_total} allowlisted={allowlisted_total} files_scanned={files_scanned}"
    );
    if active_total == 0 {
        Ok(0)
    } else {
        Ok(1)
    }
}

fn main() -> ExitCode {
    let root = match std::env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("check-cold-adapter-isolation: cannot read current directory: {error}");
            return ExitCode::from(2);
        }
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = if args.is_empty() {
        run_default_scan(&root)
    } else {
        let targets: Vec<PathBuf> = args.iter().map(PathBuf::from).collect();
        run_targeted_scan(&root, &targets)
    };
    match outcome {
        Ok(0) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::from(1),
        Err(e) => {
            eprintln!("check-cold-adapter-isolation: {e}");
            ExitCode::from(2)
        }
    }
}
