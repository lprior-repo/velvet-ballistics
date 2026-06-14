// SPDX-License-Identifier: MIT
// check-removed-feature-residue: scans the repository for ACTIVE references to
// the removed release-feature set documented in velvet-ballistics-MASTER.md §41
// ("PGO, target-cpu=native, maxperf, and generated Rust benchmark workflows are
// removed"). Master quote: "generated and maxperf are removed and must not be
// current default or release features".
//
// Banned tokens (precise phrase/substring match, not whole-word):
//   - "target-cpu=native" : exact substring
//   - "pgo"               : restricted to active PGO contexts:
//                           * "pgo = "   (Cargo feature assignment)
//                           * "cargo pgo" (cargo subcommand invocation)
//                           * "pgo-data" (PGO profile data path)
//                           * "RUSTC_PGO" (PGO env var)
//   - "maxperf"           : as a feature identifier:
//                           * "<name> = " inside a [features] block in TOML
//                           * "--features maxperf"  (CLI flag, any file)
//   - "generated"         : as a feature identifier (same contexts as maxperf)
//
// Per-line allowlist: a single line containing the substring
// "# allow-removed-feature: <reason>" or "// allow-removed-feature: <reason>"
// suppresses the NEXT non-blank line. The suppressed line is reported as
// "allowlisted:" (still counts in the summary) and never causes a failure.
//
// Output (all on stderr):
//   <path>:<lineno>: REMOVED-FEATURE: <token>: <line>     (active violation)
//   <path>:<lineno>: allowlisted: <reason>: <line>        (suppressed by marker)
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct Finding {
    line_no: usize,
    line_text: String,
    token: String,
    context: String,
    allowlisted: bool,
    reason: String,
}

const PGO_CONTEXTS: &[&str] = &[
    "pgo = ",
    "cargo pgo",
    "pgo-data",
    "RUSTC_PGO",
];

const PGO_TOKEN: &str = "pgo";
const TARGET_CPU_NATIVE_TOKEN: &str = "target-cpu=native";
const MAXPERF_TOKEN: &str = "maxperf";
const GENERATED_TOKEN: &str = "generated";

const ALLOW_HASH: &str = "# allow-removed-feature:";
const ALLOW_SLASH: &str = "// allow-removed-feature:";

const SCAN_ROOTS: &[&str] = &[
    "Cargo.toml",
    "xtask/Cargo.toml",
    "fuzz/Cargo.toml",
    "crates",
    ".moon",
    "benches",
    "README.md",
    "docs",
    "scripts",
];

fn parse_allow_reason(line: &str) -> Option<String> {
    let idx = line.find(ALLOW_HASH).or_else(|| line.find(ALLOW_SLASH))?;
    let after = if line[idx..].starts_with(ALLOW_HASH) {
        &line[idx + ALLOW_HASH.len()..]
    } else {
        &line[idx + ALLOW_SLASH.len()..]
    };
    let reason = after.trim();
    if reason.is_empty() {
        None
    } else {
        Some(reason.to_owned())
    }
}

fn is_toml(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("toml")
}

fn is_features_header(line: &str) -> bool {
    line.trim() == "[features]"
}

fn is_table_header(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('[') && trimmed.ends_with(']') && !trimmed.starts_with("[[")
}

fn check_line(
    line: &str,
    in_features_block: bool,
    findings: &mut Vec<(String, String)>,
) {
    if line.contains(TARGET_CPU_NATIVE_TOKEN) {
        findings.push((
            TARGET_CPU_NATIVE_TOKEN.to_owned(),
            format!("exact substring '{TARGET_CPU_NATIVE_TOKEN}'"),
        ));
    }
    for pattern in PGO_CONTEXTS {
        if line.contains(pattern) {
            findings.push((
                PGO_TOKEN.to_owned(),
                format!("PGO active context '{pattern}'"),
            ));
        }
    }
    if in_features_block {
        for token in &[MAXPERF_TOKEN, GENERATED_TOKEN] {
            if feature_assignment(line, token) {
                findings.push((
                    (*token).to_owned(),
                    format!("feature identifier '{token} =' inside [features] block"),
                ));
            }
        }
    }
    for token in &[MAXPERF_TOKEN, GENERATED_TOKEN] {
        if cli_features_flag(line, token) {
            findings.push((
                (*token).to_owned(),
                format!("CLI flag '--features {token}'"),
            ));
        }
    }
}

fn feature_assignment(line: &str, token: &str) -> bool {
    let trimmed_start = line.trim_start();
    let rest = trimmed_start.strip_prefix(token).unwrap_or("");
    if rest.is_empty() {
        return false;
    }
    let mut chars = rest.chars();
    match chars.next() {
        Some('=') | Some(' ') => true,
        _ => false,
    }
}

fn cli_features_flag(line: &str, token: &str) -> bool {
    let needle_space = format!("--features {token}");
    if line.contains(&needle_space) {
        return !line_after_token_is_word_char(line, &needle_space, token);
    }
    let needle_eq = format!("--features={token}");
    if line.contains(&needle_eq) {
        return !line_after_token_is_word_char(line, &needle_eq, token);
    }
    false
}

fn line_after_token_is_word_char(line: &str, needle: &str, _token: &str) -> bool {
    let Some(start) = line.find(needle) else {
        return false;
    };
    let after_idx = start + needle.len();
    let Some(next_char) = line[after_idx..].chars().next() else {
        return false;
    };
    next_char.is_alphanumeric() || next_char == '_' || next_char == '-'
}

#[derive(Debug)]
enum ScanOutcome {
    File(Vec<Finding>),
    Unreadable(String),
}

fn scan_text(text: &str, is_toml_file: bool) -> Vec<Finding> {
    let mut findings: Vec<Finding> = Vec::new();
    let mut pending_reason: Option<String> = None;
    let mut in_features_block: bool = false;
    let lines: Vec<&str> = text.lines().collect();
    for (idx, raw_line) in lines.iter().enumerate() {
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
        if is_toml_file && is_table_header(raw_line) {
            in_features_block = is_features_header(raw_line);
            continue;
        }
        let mut line_findings: Vec<(String, String)> = Vec::new();
        check_line(raw_line, in_features_block, &mut line_findings);
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
    let toml = is_toml(path);
    ScanOutcome::File(scan_text(&text, toml))
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
        if name == "target" || name == "node_modules" {
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
    if name == "check-removed-feature-residue.sh"
        || name == "check-removed-feature-residue.rs"
        || name == "test-check-removed-feature-residue.sh"
    {
        return false;
    }
    if name == "Cargo.toml" || name == "README.md" {
        return true;
    }
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    matches!(ext, "toml" | "yml" | "yaml" | "rs" | "sh" | "bash" | "py" | "md")
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
                            "{rel}:{}: REMOVED-FEATURE: {}: {}: {}",
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
            eprintln!("check-removed-feature-residue: cannot read current directory: {error}");
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
        eprintln!("check-removed-feature-residue: no scan targets resolved");
        return ExitCode::from(2);
    }
    match run_scan(&root, &targets) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(_) => ExitCode::from(1),
        Err(e) => {
            eprintln!("check-removed-feature-residue: {e}");
            ExitCode::from(2)
        }
    }
}
