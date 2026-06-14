// SPDX-License-Identifier: MIT
// check-cold-adapter-isolation: scans the four runtime-core boundary
// crates (vb_core, vb_runtime, vb_storage, vb_ipc) recursively for
// ACTIVE HTTP / JSON / YAML / adapter-only dependencies and tokenized
// `use` / `extern crate` imports. The master contract
// (velvet-ballistics-MASTER.md:62) is:
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
//   <path>:<lineno>: allowlisted: <reason>: <line>               (suppressed)
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

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const BOUNDARY_CRATES: &[&str] = &[
    "vb_core",
    "vb_runtime",
    "vb_storage",
    "vb_ipc",
];

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawHit {
    line_no: usize,
    line_text: String,
    crate_token: String,
    context: String,
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

#[derive(Debug, Clone, Default)]
struct CargoScan {
    findings: Vec<Finding>,
    aliases: BTreeMap<String, String>,
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

fn find_word_token(line: &str, token: &str) -> Option<usize> {
    let mut search_start: usize = 0;
    while let Some(rel) = line[search_start..].find(token) {
        let abs = search_start + rel;
        if is_word_boundary_at(line, abs, token) {
            return Some(abs);
        }
        search_start = abs + token.len();
    }
    None
}

fn package_name_matches_forbidden(name: &str, forbidden: &str) -> bool {
    if name == forbidden {
        return true;
    }

    let mut name_iter = name.chars();
    let mut forbidden_iter = forbidden.chars().map(|ch| if ch == '-' { '_' } else { ch });
    loop {
        match (name_iter.next(), forbidden_iter.next()) {
            (None, None) => return true,
            (Some(left), Some(right)) if left == right => {}
            _ => return false,
        }
    }
}

fn forbidden_package_token(name: &str) -> Option<&'static str> {
    FORBIDDEN_CRATE_NAMES
        .iter()
        .copied()
        .find(|forbidden| package_name_matches_forbidden(name, forbidden))
}

fn strip_toml_comment(line: &str) -> &str {
    let mut in_double = false;
    let mut in_single = false;
    let mut escaped = false;

    for (idx, ch) in line.char_indices() {
        if in_double {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_double = false,
                _ => {}
            }
            continue;
        }

        if in_single {
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }

        match ch {
            '"' => in_double = true,
            '\'' => in_single = true,
            '#' => return &line[..idx],
            _ => {}
        }
    }

    line
}

fn strip_rust_comment(line: &str) -> &str {
    line.split_once("//").map_or(line, |(code, _)| code)
}

fn parse_toml_string_literal(input: &str) -> Option<String> {
    let trimmed = input.trim_start();
    let mut chars = trimmed.chars();
    let quote = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let mut value = String::new();
    let mut escaped = false;
    for ch in chars {
        if quote == '"' && escaped {
            value.push(ch);
            escaped = false;
            continue;
        }
        if quote == '"' && ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            return Some(value);
        }
        value.push(ch);
    }

    None
}

fn parse_manifest_package_name(segment: &str) -> Option<String> {
    let mut search_start: usize = 0;
    while let Some(rel) = find_word_token(&segment[search_start..], "package") {
        let abs = search_start + rel + "package".len();
        let rhs = segment[abs..].trim_start();
        let rhs = rhs.strip_prefix('=')?.trim_start();
        if let Some(value) = parse_toml_string_literal(rhs) {
            return Some(value);
        }
        search_start = abs;
    }
    None
}

fn parse_manifest_dependency_line(line: &str) -> Option<(String, Option<String>)> {
    let semantic = strip_toml_comment(line).trim();
    if semantic.is_empty() || semantic.starts_with('[') {
        return None;
    }

    let (name_part, rest) = semantic.split_once('=')?;
    let local_name = name_part
        .trim()
        .split('.')
        .next()
        .unwrap_or("")
        .trim();
    if local_name.is_empty() {
        return None;
    }

    let package_name = parse_manifest_package_name(rest);
    Some((local_name.to_owned(), package_name))
}

fn direct_manifest_context() -> &'static str {
    "forbidden dependency in [dependencies]/[dev-dependencies]"
}

fn alias_manifest_context(local_name: &str, package_name: &str) -> String {
    format!(
        "forbidden dependency alias in [dependencies]/[dev-dependencies] via local dep '{local_name}' -> package '{package_name}'"
    )
}

fn direct_source_context() -> &'static str {
    "forbidden `use`/`extern crate` import in source"
}

fn alias_source_context(local_name: &str, package_name: &str) -> String {
    format!(
        "forbidden `use`/`extern crate` import in source via local dep '{local_name}' -> package '{package_name}'"
    )
}

fn resolve_effective_package(local_name: &str, package_name: Option<&str>) -> String {
    package_name.map_or_else(|| local_name.to_owned(), ToOwned::to_owned)
}

fn apply_marker_allowlist(text: &str, raw: Vec<RawHit>) -> Vec<Finding> {
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
            .and_then(|entry| entry.clone());
        match reason {
            Some(reason_text) => findings.push(Finding {
                line_no: hit.line_no,
                line_text: hit.line_text,
                crate_token: "allowlisted".to_owned(),
                context: "allowlist consumed".to_owned(),
                allowlisted: true,
                reason: reason_text,
            }),
            None => findings.push(Finding {
                line_no: hit.line_no,
                line_text: hit.line_text,
                crate_token: hit.crate_token,
                context: hit.context,
                allowlisted: false,
                reason: String::new(),
            }),
        }
    }
    findings
}

fn scan_manifest_text(text: &str) -> CargoScan {
    let mut raw_hits: Vec<RawHit> = Vec::new();
    let mut aliases: BTreeMap<String, String> = BTreeMap::new();
    let mut in_dep_table: bool = false;

    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let semantic = strip_toml_comment(raw_line).trim();
        if semantic.starts_with('[') {
            in_dep_table = semantic == "[dependencies]" || semantic == "[dev-dependencies]";
            continue;
        }

        if !in_dep_table || semantic.is_empty() {
            continue;
        }

        let Some((local_name, package_name)) = parse_manifest_dependency_line(raw_line) else {
            continue;
        };

        let effective_package = resolve_effective_package(&local_name, package_name.as_deref());
        aliases.insert(local_name.clone(), effective_package.clone());

        if let Some(forbidden) = forbidden_package_token(&effective_package) {
            let context = if effective_package == local_name {
                direct_manifest_context().to_owned()
            } else {
                alias_manifest_context(&local_name, &effective_package)
            };
            raw_hits.push(RawHit {
                line_no,
                line_text: raw_line.to_owned(),
                crate_token: forbidden.to_owned(),
                context,
            });
        }
    }

    CargoScan {
        findings: apply_marker_allowlist(text, raw_hits),
        aliases,
    }
}

fn scan_cargo_file(path: &Path) -> Result<CargoScan, String> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(error) => {
            return Err(format!("{}: unreadable: {error}", path.display()));
        }
    };
    Ok(scan_manifest_text(&text))
}

fn take_identifier(input: &str) -> Option<(&str, &str)> {
    let mut end: usize = 0;
    for (idx, ch) in input.char_indices() {
        if idx == 0 {
            if !(ch == '_' || ch.is_alphabetic()) {
                return None;
            }
            end = ch.len_utf8();
            continue;
        }

        if ch == '_' || ch.is_alphanumeric() {
            end = idx + ch.len_utf8();
        } else {
            break;
        }
    }

    if end == 0 {
        None
    } else {
        Some((&input[..end], &input[end..]))
    }
}

fn parse_import_root(input: &str) -> Option<(String, &str)> {
    let mut rest = input.trim_start();
    if let Some(stripped) = rest.strip_prefix("::") {
        rest = stripped.trim_start();
    }
    let (root, remainder) = take_identifier(rest)?;
    Some((root.to_owned(), remainder))
}

fn is_namespace_root(root: &str) -> bool {
    matches!(root, "crate" | "self" | "super")
}

fn source_import_hit(
    root: &str,
    aliases: Option<&BTreeMap<String, String>>,
    line_no: usize,
    line_text: &str,
) -> Option<RawHit> {
    if is_namespace_root(root) {
        return None;
    }

    if let Some(alias_map) = aliases {
        if let Some(package_name) = alias_map.get(root) {
            if let Some(forbidden) = forbidden_package_token(package_name) {
                let context = if package_name == root {
                    direct_source_context().to_owned()
                } else {
                    alias_source_context(root, package_name)
                };
                return Some(RawHit {
                    line_no,
                    line_text: line_text.to_owned(),
                    crate_token: forbidden.to_owned(),
                    context,
                });
            }
            return None;
        }
    }

    forbidden_package_token(root).map(|forbidden| RawHit {
        line_no,
        line_text: line_text.to_owned(),
        crate_token: forbidden.to_owned(),
        context: direct_source_context().to_owned(),
    })
}

fn scan_use_statement(
    segment: &str,
    aliases: Option<&BTreeMap<String, String>>,
    line_no: usize,
    line_text: &str,
) -> Option<RawHit> {
    let use_idx = find_word_token(segment, "use")?;
    let after_use = segment[use_idx + "use".len()..].trim_start();
    let (root, _) = parse_import_root(after_use)?;
    source_import_hit(&root, aliases, line_no, line_text)
}

fn consume_keyword<'a>(input: &'a str, keyword: &str) -> Option<&'a str> {
    let trimmed = input.trim_start();
    let rest = trimmed.strip_prefix(keyword)?;
    if let Some(ch) = rest.chars().next() {
        if ch.is_alphanumeric() || ch == '_' || ch == '-' {
            return None;
        }
    }
    Some(rest)
}

fn scan_extern_statement(
    segment: &str,
    aliases: Option<&BTreeMap<String, String>>,
    line_no: usize,
    line_text: &str,
) -> Option<RawHit> {
    let extern_idx = find_word_token(segment, "extern")?;
    let after_extern = segment[extern_idx + "extern".len()..].trim_start();
    let after_crate = consume_keyword(after_extern, "crate")?;
    let (root, _) = parse_import_root(after_crate)?;
    source_import_hit(&root, aliases, line_no, line_text)
}

fn scan_rust_segment(
    segment: &str,
    aliases: Option<&BTreeMap<String, String>>,
    line_no: usize,
    line_text: &str,
) -> Option<RawHit> {
    scan_extern_statement(segment, aliases, line_no, line_text)
        .or_else(|| scan_use_statement(segment, aliases, line_no, line_text))
}

fn scan_rust_text(text: &str, aliases: Option<&BTreeMap<String, String>>) -> Vec<RawHit> {
    let mut hits: Vec<RawHit> = Vec::new();

    for (idx, raw_line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let code = strip_rust_comment(raw_line).trim();
        if code.is_empty() {
            continue;
        }

        for segment in code.split(';') {
            let segment = segment.trim();
            if segment.is_empty() {
                continue;
            }
            if let Some(hit) = scan_rust_segment(segment, aliases, line_no, raw_line) {
                hits.push(hit);
            }
        }
    }

    hits
}

fn scan_rust_source_file(
    path: &Path,
    aliases: Option<&BTreeMap<String, String>>,
) -> Result<Vec<Finding>, String> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(error) => {
            return Err(format!("{}: unreadable: {error}", path.display()));
        }
    };
    Ok(apply_marker_allowlist(&text, scan_rust_text(&text, aliases)))
}

fn resolve_target_path(root: &Path, target: &Path) -> PathBuf {
    if target.is_absolute() {
        target.to_path_buf()
    } else {
        root.join(target)
    }
}

fn manifest_parent_dir(manifest_path: &Path) -> Option<PathBuf> {
    manifest_path.parent().map(Path::to_path_buf)
}

fn boundary_crate_root(root: &Path, source: &Path) -> Option<PathBuf> {
    for crate_name in BOUNDARY_CRATES {
        let crate_root = root.join("crates").join(crate_name);
        if source.starts_with(&crate_root) {
            return Some(crate_root);
        }
    }
    None
}

fn alias_map_for_source(
    root: &Path,
    source: &Path,
    cache: &mut BTreeMap<PathBuf, CargoScan>,
) -> BTreeMap<String, String> {
    let mut cursor = source.parent();
    while let Some(dir) = cursor {
        if let Some(scan) = cache.get(dir) {
            return scan.aliases.clone();
        }
        cursor = dir.parent();
    }

    let Some(crate_root) = boundary_crate_root(root, source) else {
        return BTreeMap::new();
    };

    let manifest_path = crate_root.join("Cargo.toml");
    if !manifest_path.is_file() {
        return BTreeMap::new();
    }

    match scan_cargo_file(&manifest_path) {
        Ok(scan) => {
            let aliases = scan.aliases.clone();
            cache.insert(crate_root, scan);
            aliases
        }
        Err(error) => {
            eprintln!("{error}");
            BTreeMap::new()
        }
    }
}

fn preload_manifest_targets(
    root: &Path,
    targets: &[PathBuf],
    cache: &mut BTreeMap<PathBuf, CargoScan>,
) -> Result<(), String> {
    for target in targets {
        let resolved = resolve_target_path(root, target);
        let is_cargo = resolved.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml");
        if !is_cargo {
            continue;
        }

        let scan = scan_cargo_file(&resolved)?;
        let Some(parent) = manifest_parent_dir(&resolved) else {
            return Err(format!(
                "manifest missing parent directory: {}",
                resolved.display()
            ));
        };
        cache.insert(parent, scan);
    }
    Ok(())
}

fn is_rust_source_target(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("rs")
}

fn is_cargo_target(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml")
}

fn scan_cargo_target_from_cache(
    root: &Path,
    target: &Path,
    cache: &BTreeMap<PathBuf, CargoScan>,
) -> Result<Vec<Finding>, String> {
    let resolved = resolve_target_path(root, target);
    let Some(parent) = manifest_parent_dir(&resolved) else {
        return Err(format!(
            "manifest missing parent directory: {}",
            resolved.display()
        ));
    };
    let Some(scan) = cache.get(&parent) else {
        return Err(format!(
            "missing cached manifest scan for {}",
            resolved.display()
        ));
    };
    Ok(scan.findings.clone())
}

fn emit_findings(
    rel: &str,
    findings: &[Finding],
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
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if SKIP_DIRS.iter().any(|skip| *skip == name) {
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

fn run_default_scan(root: &Path) -> Result<u8, String> {
    let mut active_total: usize = 0;
    let mut allowlisted_total: usize = 0;
    let mut files_scanned: usize = 0;
    let mut manifest_cache: BTreeMap<PathBuf, CargoScan> = BTreeMap::new();

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
        let cargo_scan = scan_cargo_file(&manifest_path)?;
        manifest_cache.insert(crate_dir.clone(), cargo_scan.clone());
        files_scanned = files_scanned.saturating_add(1);
        emit_findings(
            &manifest_rel,
            &cargo_scan.findings,
            &mut active_total,
            &mut allowlisted_total,
        );

        let sources = collect_rust_sources(&crate_dir)
            .map_err(|error| format!("{}: walk error: {error}", crate_dir.display()))?;
        for source_path in sources {
            let rel = relative_label(root, &source_path);
            let aliases = alias_map_for_source(root, &source_path, &mut manifest_cache);
            let findings = scan_rust_source_file(&source_path, Some(&aliases))?;
            files_scanned = files_scanned.saturating_add(1);
            emit_findings(&rel, &findings, &mut active_total, &mut allowlisted_total);
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
    let mut manifest_cache: BTreeMap<PathBuf, CargoScan> = BTreeMap::new();

    preload_manifest_targets(root, targets, &mut manifest_cache)?;

    for target in targets {
        let resolved = resolve_target_path(root, target);
        if !resolved.exists() {
            return Err(format!("target does not exist: {}", target.display()));
        }

        let rel = relative_label(root, target);
        if is_cargo_target(&resolved) {
            let findings = scan_cargo_target_from_cache(root, target, &manifest_cache)?;
            files_scanned = files_scanned.saturating_add(1);
            emit_findings(&rel, &findings, &mut active_total, &mut allowlisted_total);
        } else if is_rust_source_target(&resolved) {
            let aliases = alias_map_for_source(root, &resolved, &mut manifest_cache);
            let findings = scan_rust_source_file(&resolved, Some(&aliases))?;
            files_scanned = files_scanned.saturating_add(1);
            emit_findings(&rel, &findings, &mut active_total, &mut allowlisted_total);
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
        Err(error) => {
            eprintln!("check-cold-adapter-isolation: {error}");
            ExitCode::from(2)
        }
    }
}
