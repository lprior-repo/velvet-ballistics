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
// Per-line allowlist: a single line containing the substring
// "# allow-removed-crate: <reason>" or "// allow-removed-crate: <reason>"
// suppresses the NEXT non-blank line. The suppressed line is reported as
// "allowlisted:" (still counts in the summary) and never causes a failure.
//
// Output (all on stderr):
//   <path>:<lineno>: REMOVED-CRATE: <token>: <line>      (active violation)
//   <path>:<lineno>: allowlisted: <reason>: <line>       (suppressed by marker)
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
    let reason_token = "allow-removed-crate:";
    let Some(token_idx) = line.find(reason_token) else {
        return None;
    };
    if !is_valid_marker_prefix(line, token_idx) {
        return None;
    }
    let after = &line[token_idx + reason_token.len()..];
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
    if token_idx == 1 {
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

fn is_standalone_makepad(line: &str) -> bool {
    let needle = MAKEPAD_BARE;
    for (idx, _) in line.match_indices(needle) {
        if idx > 0 {
            if let Some(prev) = line[..idx].chars().next_back() {
                if is_word_or_underscore(prev) {
                    continue;
                }
            }
        }
        let after_idx = idx + needle.len();
        if let Some(next) = line[after_idx..].chars().next() {
            if is_word_underscore_or_dash(next) {
                continue;
            }
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
    if line.contains(VB_CODEGEN) {
        findings.push((
            VB_CODEGEN.to_owned(),
            format!("exact substring '{VB_CODEGEN}'"),
        ));
    }
    if line.contains(VB_UI_MODEL) {
        findings.push((
            VB_UI_MODEL.to_owned(),
            format!("exact substring '{VB_UI_MODEL}'"),
        ));
    }
    if line.contains(VB_UI_MAKEPAD) {
        findings.push((
            VB_UI_MAKEPAD.to_owned(),
            format!("exact substring '{VB_UI_MAKEPAD}'"),
        ));
    }
    if line.contains(MAKEPAD_WIDGETS) {
        findings.push((
            MAKEPAD_WIDGETS.to_owned(),
            format!("exact substring '{MAKEPAD_WIDGETS}'"),
        ));
    }
    if line.contains(MAKEPAD_DRAW) {
        findings.push((
            MAKEPAD_DRAW.to_owned(),
            format!("exact substring '{MAKEPAD_DRAW}'"),
        ));
    }
    if is_standalone_makepad(line) {
        findings.push((
            MAKEPAD_BARE.to_owned(),
            format!("standalone token '{MAKEPAD_BARE}' (word boundary)"),
        ));
    }
}

#[derive(Debug)]
enum ScanOutcome {
    File(Vec<Finding>),
    Unreadable(String),
}

fn scan_text(text: &str) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();
    let mut pending_reason: Option<String> = None;
    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        if let Some(reason) = parse_allow_reason(raw_line) {
            pending_reason = Some(reason);
            continue;
        }
        if raw_line.trim().is_empty() {
            continue;
        }
        if let Some(reason) = pending_reason.take() {
            findings.push(Finding {
                line_no,
                line_text: (*raw_line).to_owned(),
                token: "allowlisted".to_owned(),
                context: "allowlist consumed".to_owned(),
                allowlisted: true,
                reason,
            });
            continue;
        }
        let mut line_findings: Vec<(String, String)> = Vec::new();
        check_line(raw_line, &mut line_findings);
        for (token, context) in line_findings {
            findings.push(Finding {
                line_no,
                line_text: (*raw_line).to_owned(),
                token,
                context,
                allowlisted: false,
                reason: String::new(),
            });
        }
    }
    findings
}

fn scan_file(path: &Path) -> ScanOutcome {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(error) => {
            return ScanOutcome::Unreadable(format!("{}: unreadable: {error}", path.display()));
        }
    };
    ScanOutcome::File(scan_text(&text))
}

fn collect_scan_files(root: &Path, target: &Path) -> io::Result<Vec<PathBuf>> {
    if !target.exists() {
        return Ok(Vec::new());
    }
    let mut out: Vec<PathBuf> = Vec::new();
    if target.is_file() {
        out.push(target.to_path_buf());
        return Ok(out);
    }
    walk(root, target, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
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
        if name.starts_with('.') && name != ".moon" {
            continue;
        }
        if SKIP_DIRS.iter().any(|d| *d == name) {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, out)?;
        } else if should_scan_file(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn should_scan_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if SELF_SKIP_NAMES.iter().any(|n| *n == name) {
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

fn run_scan(root: &Path, targets: &[PathBuf]) -> Result<u8, String> {
    let mut files: Vec<PathBuf> = Vec::new();
    for target in targets {
        let collected = collect_scan_files(root, target)
            .map_err(|e| format!("collect {}: {e}", target.display()))?;
        files.extend(collected);
    }
    files.sort();
    files.dedup();

    let mut active_total: usize = 0;
    let mut allowlisted_total: usize = 0;
    let mut files_scanned: usize = 0;

    for file in &files {
        match scan_file(file) {
            ScanOutcome::File(findings) => {
                files_scanned = files_scanned.saturating_add(1);
                let rel = relative_label(root, file);
                for finding in findings {
                    if finding.allowlisted {
                        allowlisted_total = allowlisted_total.saturating_add(1);
                        eprintln!(
                            "{rel}:{}: allowlisted: {}: {}",
                            finding.line_no, finding.reason, finding.line_text
                        );
                    } else {
                        active_total = active_total.saturating_add(1);
                        eprintln!(
                            "{rel}:{}: REMOVED-CRATE: {}: {}: {}",
                            finding.line_no, finding.token, finding.context, finding.line_text
                        );
                    }
                }
            }
            ScanOutcome::Unreadable(message) => {
                eprintln!("{message}");
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

fn resolve_default_targets(root: &Path) -> Vec<PathBuf> {
    SCAN_ROOTS
        .iter()
        .map(|name| root.join(name))
        .filter(|p| p.exists())
        .collect()
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
    let targets: Vec<PathBuf> = if args.is_empty() {
        resolve_default_targets(&root)
    } else {
        args.iter().map(PathBuf::from).collect()
    };
    if targets.is_empty() {
        eprintln!("check-removed-crate-residue: no scan targets resolved");
        return ExitCode::from(2);
    }
    match run_scan(&root, &targets) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::from(1),
        Err(e) => {
            eprintln!("check-removed-crate-residue: {e}");
            ExitCode::from(2)
        }
    }
}
