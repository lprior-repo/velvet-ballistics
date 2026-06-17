#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::borrow_deref_ref,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::cloned_ref_to_slice_refs,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_sort_by,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_asref,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables,
)]
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use proc_macro2::Span;
use syn::{
    Item, ItemExternCrate, ItemUse, Stmt, UseTree,
    visit::{self, Visit},
};

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

#[derive(Debug, Default)]
struct ScanTotals {
    active: usize,
    allowlisted: usize,
    files_scanned: usize,
}

#[derive(Debug, Default)]
struct ManifestState {
    raw_hits: Vec<RawHit>,
    aliases: BTreeMap<String, String>,
    in_dependency_table: bool,
}

struct RustScanner<'a> {
    aliases: Option<&'a BTreeMap<String, String>>,
    lines: &'a [&'a str],
    hits: Vec<RawHit>,
}

impl ScanTotals {
    fn note_file(&mut self) {
        self.files_scanned = self.files_scanned.saturating_add(1);
    }

    fn note_finding(&mut self, finding: &Finding) {
        if finding.allowlisted {
            self.allowlisted = self.allowlisted.saturating_add(1);
        } else {
            self.active = self.active.saturating_add(1);
        }
    }

    fn exit_code(&self) -> i32 {
        if self.active == 0 { 0 } else { 1 }
    }
}

impl ManifestState {
    fn process_line(&mut self, line_no: usize, raw_line: &str) {
        let semantic = manifest_semantic(raw_line);
        if semantic.starts_with('[') {
            self.in_dependency_table = is_dependency_table(semantic);
            return;
        }
        if !self.in_dependency_table || semantic.is_empty() {
            return;
        }
        if let Some((local_name, package_name)) = manifest_dependency_entry(semantic) {
            self.record_dependency(line_no, raw_line, local_name, package_name);
        }
    }

    fn record_dependency(
        &mut self,
        line_no: usize,
        raw_line: &str,
        local_name: String,
        package_name: Option<String>,
    ) {
        let effective_package = package_name.unwrap_or_else(|| local_name.clone());
        self.aliases
            .insert(local_name.clone(), effective_package.clone());
        if let Some(forbidden) = forbidden_package_token(&effective_package) {
            self.raw_hits.push(manifest_raw_hit(
                line_no,
                raw_line,
                &local_name,
                &effective_package,
                forbidden,
            ));
        }
    }

    fn finish(self, text: &str) -> CargoScan {
        CargoScan {
            findings: apply_allowlist(text, self.raw_hits),
            aliases: self.aliases,
        }
    }
}

impl<'a> RustScanner<'a> {
    fn new(aliases: Option<&'a BTreeMap<String, String>>, lines: &'a [&'a str]) -> Self {
        Self {
            aliases,
            lines,
            hits: Vec::new(),
        }
    }

    fn into_hits(self) -> Vec<RawHit> {
        self.hits
    }

    fn record_tree(&mut self, tree: &UseTree, line_no: usize) {
        match tree {
            UseTree::Path(path) => self.record_root(&path.ident, line_no),
            UseTree::Name(name) => self.record_root(&name.ident, line_no),
            UseTree::Rename(rename) => self.record_root(&rename.ident, line_no),
            UseTree::Glob(_) => {}
            UseTree::Group(group) => {
                for child in &group.items {
                    self.record_tree(child, line_no);
                }
            }
        }
    }

    fn record_root(&mut self, ident: &syn::Ident, line_no: usize) {
        let root = ident.to_string();
        if is_namespace_root(&root) {
            return;
        }
        let package_name = self
            .aliases
            .and_then(|aliases| aliases.get(&root))
            .map_or(root.as_str(), |value| value.as_str());
        if let Some(forbidden) = forbidden_package_token(package_name) {
            self.hits.push(source_finding(
                line_no,
                self.line_text(line_no),
                &root,
                package_name,
                forbidden,
            ));
        }
    }

    fn line_text(&self, line_no: usize) -> &str {
        self.lines
            .get(line_no.saturating_sub(1))
            .copied()
            .unwrap_or_default()
    }
}

impl<'ast, 'a> Visit<'ast> for RustScanner<'a> {
    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        self.record_tree(&item.tree, span_line(item.use_token.span));
    }

    fn visit_item_extern_crate(&mut self, item: &'ast ItemExternCrate) {
        self.record_root(&item.ident, span_line(item.extern_token.span));
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        if let Stmt::Item(item) = stmt {
            match item {
                Item::Use(item) => self.visit_item_use(item),
                Item::ExternCrate(item) => self.visit_item_extern_crate(item),
                _ => visit::visit_stmt(self, stmt),
            }
            return;
        }
        visit::visit_stmt(self, stmt);
    }
}

pub fn cmd_cold_adapter_isolation(targets: &[PathBuf]) -> Result<i32> {
    let root = current_root()?;
    let totals = if targets.is_empty() {
        scan_workspace(&root)?
    } else {
        scan_targets(&root, targets)?
    };
    print_summary(&totals);
    Ok(totals.exit_code())
}

fn current_root() -> Result<PathBuf> {
    std::env::current_dir().context("cannot read current directory")
}

fn scan_workspace(root: &Path) -> Result<ScanTotals> {
    let mut cache = BTreeMap::new();
    let mut totals = ScanTotals::default();
    for crate_name in BOUNDARY_CRATES {
        scan_boundary_crate(root, crate_name, &mut cache, &mut totals)?;
    }
    Ok(totals)
}

fn scan_targets(root: &Path, targets: &[PathBuf]) -> Result<ScanTotals> {
    let mut cache = BTreeMap::new();
    let mut totals = ScanTotals::default();
    for target in targets {
        scan_target(root, target, &mut cache, &mut totals)?;
    }
    Ok(totals)
}

fn scan_boundary_crate(
    root: &Path,
    crate_name: &str,
    cache: &mut BTreeMap<PathBuf, CargoScan>,
    totals: &mut ScanTotals,
) -> Result<()> {
    let crate_root = boundary_crate_dir(root, crate_name)?;
    scan_manifest_root(root, &crate_root, cache, totals)?;
    let sources = collect_boundary_sources(&crate_root)
        .with_context(|| format!("{}: walk error", crate_root.display()))?;
    for source in sources {
        scan_source_path(root, &source, cache, totals)?;
    }
    Ok(())
}

fn scan_target(
    root: &Path,
    target: &Path,
    cache: &mut BTreeMap<PathBuf, CargoScan>,
    totals: &mut ScanTotals,
) -> Result<()> {
    let resolved = resolve_target_path(root, target);
    if !resolved.exists() {
        bail!("target does not exist: {}", target.display());
    }
    if is_cargo_manifest(&resolved) {
        scan_manifest_target(root, &resolved, cache, totals)
    } else if is_rust_source_file(&resolved) {
        scan_source_target(root, &resolved, cache, totals)
    } else {
        bail!(
            "unsupported target (must be Cargo.toml or .rs file): {}",
            target.display()
        );
    }
}

fn scan_manifest_root(
    root: &Path,
    crate_root: &Path,
    cache: &mut BTreeMap<PathBuf, CargoScan>,
    totals: &mut ScanTotals,
) -> Result<()> {
    let manifest_path = crate_root.join("Cargo.toml");
    let scan = scan_manifest_file(&manifest_path)?;
    emit_findings(
        &relative_label(root, &manifest_path),
        &scan.findings,
        totals,
    );
    totals.note_file();
    cache.insert(crate_root.to_path_buf(), scan);
    Ok(())
}

fn scan_manifest_target(
    root: &Path,
    manifest_path: &Path,
    cache: &mut BTreeMap<PathBuf, CargoScan>,
    totals: &mut ScanTotals,
) -> Result<()> {
    let scan = scan_manifest_file(manifest_path)?;
    emit_findings(&relative_label(root, manifest_path), &scan.findings, totals);
    totals.note_file();
    if let Some(parent) = manifest_path.parent() {
        cache.insert(parent.to_path_buf(), scan);
    }
    Ok(())
}

fn scan_source_path(
    root: &Path,
    source: &Path,
    cache: &mut BTreeMap<PathBuf, CargoScan>,
    totals: &mut ScanTotals,
) -> Result<()> {
    let aliases = alias_map_for_source(root, source, cache)?;
    let findings = scan_rust_source_file(source, Some(&aliases))?;
    emit_findings(&relative_label(root, source), &findings, totals);
    totals.note_file();
    Ok(())
}

fn scan_source_target(
    root: &Path,
    source: &Path,
    cache: &mut BTreeMap<PathBuf, CargoScan>,
    totals: &mut ScanTotals,
) -> Result<()> {
    scan_source_path(root, source, cache, totals)
}

fn scan_manifest_file(path: &Path) -> Result<CargoScan> {
    let text =
        fs::read_to_string(path).with_context(|| format!("{}: unreadable", path.display()))?;
    Ok(scan_manifest_text(&text))
}

fn scan_manifest_text(text: &str) -> CargoScan {
    let mut state = ManifestState::default();
    for (index, raw_line) in text.lines().enumerate() {
        state.process_line(index.saturating_add(1), raw_line);
    }
    state.finish(text)
}

fn scan_rust_source_file(
    path: &Path,
    aliases: Option<&BTreeMap<String, String>>,
) -> Result<Vec<Finding>> {
    let text =
        fs::read_to_string(path).with_context(|| format!("{}: unreadable", path.display()))?;
    let raw_hits = scan_rust_source_text(&text, aliases)
        .with_context(|| format!("{}: parse error", path.display()))?;
    Ok(apply_allowlist(&text, raw_hits))
}

fn scan_rust_source_text(
    text: &str,
    aliases: Option<&BTreeMap<String, String>>,
) -> Result<Vec<RawHit>> {
    let lines: Vec<&str> = text.lines().collect();
    let normalized_text = normalize_allowlist_markers(text);
    let file = syn::parse_file(&normalized_text).context("invalid Rust source")?;
    let mut scanner = RustScanner::new(aliases, &lines);
    scanner.visit_file(&file);
    Ok(scanner.into_hits())
}

fn normalize_allowlist_markers(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    for raw_line in text.lines() {
        normalized.push_str(&normalize_allowlist_line(raw_line));
        normalized.push('\n');
    }
    if !text.ends_with('\n') && normalized.ends_with('\n') {
        normalized.pop();
    }
    normalized
}

fn normalize_allowlist_line(raw_line: &str) -> String {
    let trimmed = raw_line.trim_start();
    if !trimmed.starts_with("# ") && !trimmed.starts_with("#allow-cold-adapter:") {
        return raw_line.to_owned();
    }
    if !trimmed.contains(ALLOW_MARKER) {
        return raw_line.to_owned();
    }
    let prefix_len = raw_line.len().saturating_sub(trimmed.len());
    let indent = raw_line.get(..prefix_len).unwrap_or("");
    let suffix = trimmed.strip_prefix('#').unwrap_or(trimmed);
    format!("{indent}//{suffix}")
}

fn apply_allowlist(text: &str, raw_hits: Vec<RawHit>) -> Vec<Finding> {
    let reasons = allowlist_reasons(text);
    raw_hits
        .into_iter()
        .map(|hit| map_hit_with_reason(hit, &reasons))
        .collect()
}

fn allowlist_reasons(text: &str) -> Vec<Option<String>> {
    let mut reasons = vec![None; text.lines().count()];
    let mut pending: Option<String> = None;
    for (index, raw_line) in text.lines().enumerate() {
        if let Some(reason) = parse_allow_reason(raw_line) {
            pending = Some(reason);
            continue;
        }
        if raw_line.trim().is_empty() {
            continue;
        }
        if let Some(reason) = pending.take() {
            #[allow(clippy::option_map_unit_fn)]
            reasons.get_mut(index).map(|r| *r = Some(reason));
        }
    }
    reasons
}

fn map_hit_with_reason(hit: RawHit, reasons: &[Option<String>]) -> Finding {
    let reason = reasons
        .get(hit.line_no.saturating_sub(1))
        .and_then(|entry| entry.clone());
    match reason {
        Some(reason_text) => Finding {
            line_no: hit.line_no,
            line_text: hit.line_text,
            crate_token: "allowlisted".to_owned(),
            context: "allowlist consumed".to_owned(),
            allowlisted: true,
            reason: reason_text,
        },
        None => Finding {
            line_no: hit.line_no,
            line_text: hit.line_text,
            crate_token: hit.crate_token,
            context: hit.context,
            allowlisted: false,
            reason: String::new(),
        },
    }
}

fn parse_allow_reason(line: &str) -> Option<String> {
    let token_idx = line.find(ALLOW_MARKER)?;
    if !is_valid_marker_prefix(line, token_idx) {
        return None;
    }
    let reason = line
        .get(token_idx.saturating_add(ALLOW_MARKER.len())..)
        .unwrap_or("")
        .trim();
    if reason.is_empty() {
        None
    } else {
        Some(reason.to_owned())
    }
}

fn is_valid_marker_prefix(line: &str, token_idx: usize) -> bool {
    let bytes = line.as_bytes();
    if token_idx < 2 {
        return false;
    }
    let prev = bytes.get(token_idx.saturating_sub(1)).copied();
    if prev != Some(b' ') && prev != Some(b'\t') {
        return false;
    }
    let prev_byte = bytes
        .get(token_idx.saturating_sub(2))
        .copied()
        .unwrap_or(b' ');
    prev_byte == b'#' || prev_byte == b'/' || prev_byte == b'!'
}

fn scan_targeted_manifest_cache(
    root: &Path,
    source: &Path,
    cache: &mut BTreeMap<PathBuf, CargoScan>,
) -> Result<BTreeMap<String, String>> {
    let Some(crate_root) = boundary_crate_root(root, source) else {
        return Ok(BTreeMap::new());
    };
    if let Some(scan) = cache.get(&crate_root) {
        return Ok(scan.aliases.clone());
    }
    let manifest_path = crate_root.join("Cargo.toml");
    let scan = scan_manifest_file(&manifest_path)?;
    cache.insert(crate_root, scan.clone());
    Ok(scan.aliases)
}

fn alias_map_for_source(
    root: &Path,
    source: &Path,
    cache: &mut BTreeMap<PathBuf, CargoScan>,
) -> Result<BTreeMap<String, String>> {
    scan_targeted_manifest_cache(root, source, cache)
}

fn boundary_crate_dir(root: &Path, crate_name: &str) -> Result<PathBuf> {
    let crate_dir = root.join("crates").join(crate_name);
    if crate_dir.is_dir() {
        Ok(crate_dir)
    } else {
        bail!("missing boundary crate directory: {}", crate_dir.display())
    }
}

fn boundary_crate_root(root: &Path, source: &Path) -> Option<PathBuf> {
    BOUNDARY_CRATES.iter().find_map(|crate_name| {
        let crate_root = root.join("crates").join(crate_name);
        source.starts_with(&crate_root).then_some(crate_root)
    })
}

fn collect_boundary_sources(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut sources = Vec::new();
    walk_dir(root, &mut sources)?;
    sources.sort();
    Ok(sources)
}

fn walk_dir(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if should_skip_dir(&path) {
            continue;
        }
        if path.is_dir() {
            walk_dir(&path, out)?;
        } else if is_rust_source_file(&path) && !should_skip_source_file(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn should_skip_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.starts_with('.')
                || name == "target"
                || name == "verification"
                || name == "proof"
                || name == "kani"
        })
}

fn should_skip_source_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.starts_with("kani_")
                || name.ends_with("_kani.rs")
                || name.ends_with("_proof.rs")
                || name.contains("_proof_")
                || name.contains("_verif")
        })
}

fn is_rust_source_file(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()) == Some("rs")
}

fn is_cargo_manifest(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some("Cargo.toml")
}

fn resolve_target_path(root: &Path, target: &Path) -> PathBuf {
    if target.is_absolute() {
        target.to_path_buf()
    } else {
        root.join(target)
    }
}

fn relative_label(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|relative| relative.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string_lossy().to_string())
}

fn emit_findings(rel: &str, findings: &[Finding], totals: &mut ScanTotals) {
    for finding in findings {
        totals.note_finding(finding);
        if finding.allowlisted {
            eprintln!(
                "{rel}:{}: allowlisted: {}: {}",
                finding.line_no, finding.reason, finding.line_text
            );
        } else {
            eprintln!(
                "{rel}:{}: COLD-ADAPTER: {}: {}: {}",
                finding.line_no, finding.crate_token, finding.context, finding.line_text
            );
        }
    }
}

fn print_summary(totals: &ScanTotals) {
    eprintln!(
        "summary: active={} allowlisted={} files_scanned={}",
        totals.active, totals.allowlisted, totals.files_scanned
    );
}

fn span_line(span: Span) -> usize {
    span.start().line
}

fn manifest_semantic(raw_line: &str) -> &str {
    strip_toml_comment(raw_line).trim()
}

fn is_dependency_table(semantic: &str) -> bool {
    matches!(
        semantic,
        "[dependencies]" | "[dev-dependencies]" | "[build-dependencies]"
    )
}

fn manifest_dependency_entry(semantic: &str) -> Option<(String, Option<String>)> {
    let (name_part, rest) = semantic.split_once('=')?;
    let local_name = name_part.split('.').next()?.trim();
    if local_name.is_empty() {
        return None;
    }
    Some((local_name.to_owned(), manifest_package_name(rest)))
}

fn manifest_package_name(rest: &str) -> Option<String> {
    let idx = find_word_token(rest, "package")?;
    let after = rest
        .get(idx.saturating_add("package".len())..)
        .unwrap_or("")
        .trim_start();
    let after = after.strip_prefix('=')?.trim_start();
    quoted_string(after)
}

fn find_word_token(input: &str, token: &str) -> Option<usize> {
    let mut search_start = 0;
    while let Some(rel) = input.get(search_start..).and_then(|s| s.find(token)) {
        let idx = search_start.saturating_add(rel);
        if is_word_token(input, idx, token) {
            return Some(idx);
        }
        search_start = idx.saturating_add(token.len());
    }
    None
}

fn is_word_token(input: &str, idx: usize, token: &str) -> bool {
    if idx > 0 {
        if let Some(prev) = input.get(..idx).and_then(|s| s.chars().next_back()) {
            if prev.is_alphanumeric() || prev == '_' || prev == '-' {
                return false;
            }
        }
    }
    let after_idx = idx.saturating_add(token.len());
    if let Some(next) = input.get(after_idx..).and_then(|s| s.chars().next()) {
        if next.is_alphanumeric() || next == '_' || next == '-' {
            return false;
        }
    }
    true
}

fn quoted_string(input: &str) -> Option<String> {
    let trimmed = input.trim_start();
    let mut chars = trimmed.chars();
    let quote = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    quoted_string_body(&mut chars, quote)
}

fn strip_toml_comment(line: &str) -> &str {
    comment_start(line)
        .and_then(|idx| line.get(..idx))
        .unwrap_or(line)
}

fn comment_start(line: &str) -> Option<usize> {
    let mut state = TomlCommentState::default();
    for (idx, ch) in line.char_indices() {
        if state.feed(ch) {
            return Some(idx);
        }
    }
    None
}

#[derive(Default)]
struct TomlCommentState {
    in_double: bool,
    in_single: bool,
    escaped: bool,
}

impl TomlCommentState {
    fn feed(&mut self, ch: char) -> bool {
        if self.in_double {
            return self.feed_double(ch);
        }
        if self.in_single {
            return self.feed_single(ch);
        }
        self.feed_plain(ch)
    }

    fn feed_double(&mut self, ch: char) -> bool {
        if self.escaped {
            self.escaped = false;
            return false;
        }
        match ch {
            '\\' => self.escaped = true,
            '"' => self.in_double = false,
            _ => {}
        }
        false
    }

    fn feed_single(&mut self, ch: char) -> bool {
        if ch == '\'' {
            self.in_single = false;
        }
        false
    }

    fn feed_plain(&mut self, ch: char) -> bool {
        match ch {
            '"' => self.in_double = true,
            '\'' => self.in_single = true,
            '#' => return true,
            _ => {}
        }
        false
    }
}

fn quoted_string_body(chars: &mut std::str::Chars<'_>, quote: char) -> Option<String> {
    let mut value = String::new();
    let mut escaped = false;
    for ch in chars.by_ref() {
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

fn forbidden_package_token(name: &str) -> Option<&'static str> {
    FORBIDDEN_CRATE_NAMES
        .iter()
        .copied()
        .find(|forbidden| package_name_matches_forbidden(name, forbidden))
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

fn is_namespace_root(root: &str) -> bool {
    matches!(root, "crate" | "self" | "super")
}

fn manifest_raw_hit(
    line_no: usize,
    raw_line: &str,
    local_name: &str,
    package_name: &str,
    forbidden: &str,
) -> RawHit {
    let context = if local_name == package_name {
        direct_manifest_context().to_owned()
    } else {
        alias_manifest_context(local_name, package_name)
    };
    RawHit {
        line_no,
        line_text: raw_line.to_owned(),
        crate_token: forbidden.to_owned(),
        context,
    }
}

fn source_finding(
    line_no: usize,
    line_text: &str,
    root: &str,
    package_name: &str,
    forbidden: &str,
) -> RawHit {
    let context = if root == package_name {
        direct_source_context().to_owned()
    } else {
        alias_source_context(root, package_name)
    };
    RawHit {
        line_no,
        line_text: line_text.to_owned(),
        crate_token: forbidden.to_owned(),
        context,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    fn temp_dir() -> TempDir {
        let base = Path::new("target").join("cold-adapter-isolation-tests");
        fs::create_dir_all(&base).expect("base dir");
        tempfile::Builder::new()
            .prefix("scan-")
            .tempdir_in(&base)
            .expect("tempdir")
    }

    #[test]
    fn source_items_cover_leading_colon_extern_and_aliases() {
        let mut aliases = BTreeMap::new();
        aliases.insert("serde_http".to_owned(), "serde_json".to_owned());
        let text = concat!(
            "use ::serde_json::Value;\n",
            "use serde_http;\n",
            "extern crate serde_json as sj;\n",
            "mod nested { use serde_json::Value; }\n",
        );
        let hits = scan_rust_source_text(text, Some(&aliases)).expect("scan");
        let tokens: Vec<String> = hits.into_iter().map(|hit| hit.crate_token).collect();
        assert_eq!(
            tokens,
            vec!["serde_json", "serde_json", "serde_json", "serde_json"]
        );
    }

    #[test]
    fn string_literals_do_not_trigger_source_hits() {
        let text = r#"const S: &str = "use serde_json::Value; use reqwest::Client; use hyper::body::Body;";"#;
        let hits = scan_rust_source_text(text, None).expect("scan");
        assert!(hits.is_empty());
    }

    #[test]
    fn manifest_package_alias_is_recorded_and_used_for_source_aliases() {
        let root = temp_dir();
        let crate_root = root.path().join("crates").join("vb_core");
        let source = crate_root.join("src").join("lib.rs");
        fs::create_dir_all(source.parent().expect("source parent")).expect("create tree");
        fs::write(
            crate_root.join("Cargo.toml"),
            "[dependencies]\nserde_http = { package = \"serde_json\", path = \"../serde_http\" }\n",
        )
        .expect("write manifest");
        fs::write(&source, "use serde_http;\n").expect("write source");

        let mut cache = BTreeMap::new();
        let aliases = alias_map_for_source(root.path(), &source, &mut cache).expect("aliases");
        assert_eq!(
            aliases.get("serde_http").map(String::as_str),
            Some("serde_json")
        );

        let hits = scan_rust_source_text("use serde_http;\n", Some(&aliases)).expect("scan");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].crate_token, "serde_json");
    }

    #[test]
    fn collect_boundary_sources_keeps_tests_benches_and_examples() {
        let root = temp_dir();
        let crate_root = root.path().join("crate");
        fs::create_dir_all(crate_root.join("src")).expect("src");
        fs::create_dir_all(crate_root.join("tests")).expect("tests");
        fs::create_dir_all(crate_root.join("benches")).expect("benches");
        fs::create_dir_all(crate_root.join("examples")).expect("examples");
        fs::create_dir_all(crate_root.join("target").join("ignored")).expect("target");
        fs::write(crate_root.join("src").join("lib.rs"), "\n").expect("src file");
        fs::write(crate_root.join("tests").join("integration.rs"), "\n").expect("tests file");
        fs::write(crate_root.join("benches").join("bench.rs"), "\n").expect("bench file");
        fs::write(crate_root.join("examples").join("demo.rs"), "\n").expect("example file");
        fs::write(
            crate_root.join("target").join("ignored").join("skip.rs"),
            "\n",
        )
        .expect("skip file");

        let files = collect_boundary_sources(&crate_root).expect("collect");
        let rels: Vec<String> = files
            .iter()
            .map(|path| {
                path.strip_prefix(&crate_root)
                    .expect("rel")
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        assert_eq!(
            rels,
            vec![
                "benches/bench.rs",
                "examples/demo.rs",
                "src/lib.rs",
                "tests/integration.rs",
            ]
        );
    }

    #[test]
    fn comment_bypasses_are_detected_via_parser() {
        let text = concat!(
            "use /* hidden */ serde_json::Value;\n",
            "use /* hidden */ reqwest::Client;\n",
            "use /* hidden */ hyper::body::Body;\n",
        );
        let hits = scan_rust_source_text(text, None).expect("scan");
        let tokens: Vec<String> = hits.into_iter().map(|hit| hit.crate_token).collect();
        assert_eq!(tokens, vec!["serde_json", "reqwest", "hyper"]);
    }

    #[test]
    fn manifest_allowlist_is_consumed_for_narrow_historical_entries() {
        let manifest = concat!(
            "[dependencies]\n",
            "# allow-cold-adapter: historical example\n",
            "serde_json = \"1\"\n",
        );
        let scan = scan_manifest_text(manifest);
        assert_eq!(scan.findings.len(), 1);
        assert!(scan.findings[0].allowlisted);
    }

    #[test]
    fn all_ten_forbidden_crate_names_are_detected_in_source() {
        let text = concat!(
            "use serde_json::Value;\n",
            "use saphyr::Node;\n",
            "use saphyr_parser::parse;\n",
            "use serde_saphyr::from_str;\n",
            "use reqwest::Client;\n",
            "use hyper::body::Body;\n",
            "use axum::Router;\n",
            "use ureq::get;\n",
            "use attohttpc::get;\n",
            "use isahc::get;\n",
        );
        let hits = scan_rust_source_text(text, None).expect("scan");
        let tokens: Vec<String> = hits.into_iter().map(|hit| hit.crate_token).collect();
        let expected: Vec<&str> = vec![
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
        assert_eq!(
            tokens.len(),
            expected.len(),
            "all 10 forbidden crate names should be detected; got {tokens:?}"
        );
        for (got, want) in tokens.iter().zip(expected.iter()) {
            assert_eq!(got, want, "forbidden crate name mismatch");
        }
    }

    #[test]
    fn dash_underscore_normalization_detects_forbidden_names() {
        let text = concat!("use serde_json::Value;\n", "use reqwest;\n",);
        let hits = scan_rust_source_text(text, None).expect("scan");
        assert_eq!(hits.len(), 2, "both serde_json and reqwest detected");
        // serde_json → serde_json (exact match)
        assert_eq!(hits[0].crate_token, "serde_json");
        // reqwest → reqwest (exact match)
        assert_eq!(hits[1].crate_token, "reqwest");
    }

    #[test]
    fn manifest_dev_dependencies_are_scanned() {
        let manifest = concat!("[dev-dependencies]\n", "serde_json = \"1\"\n",);
        let scan = scan_manifest_text(manifest);
        assert_eq!(scan.findings.len(), 1);
        assert_eq!(scan.findings[0].crate_token, "serde_json");
    }

    #[test]
    fn manifest_build_dependencies_are_scanned() {
        let manifest = concat!("[build-dependencies]\n", "serde_json = \"1\"\n",);
        let scan = scan_manifest_text(manifest);
        assert_eq!(scan.findings.len(), 1);
        assert_eq!(scan.findings[0].crate_token, "serde_json");
    }

    #[test]
    fn nonexistent_target_returns_error() {
        let result = scan_rust_source_text("use serde_json::Value;\n", None);
        assert!(
            result.is_ok(),
            "scan_rust_source_text should not error for valid input"
        );
        let hits = result.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].crate_token, "serde_json");
    }

    #[test]
    fn namespace_roots_are_not_detected_as_forbidden() {
        let text = concat!(
            "use crate::some::module;\n",
            "use self::inner;\n",
            "use super::parent;\n",
        );
        let hits = scan_rust_source_text(text, None).expect("scan");
        assert!(
            hits.is_empty(),
            "crate/self/super roots must not trigger hits"
        );
    }

    #[test]
    fn function_level_use_statements_are_detected() {
        // use inside a function body (inside a block) must still be detected
        let text = concat!(
            "fn test() {\n",
            "    use serde_json::Value;\n",
            "    let _ = Value::Null;\n",
            "}\n",
        );
        let hits = scan_rust_source_text(text, None).expect("scan");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].crate_token, "serde_json");
    }

    #[test]
    fn extern_crate_without_alias_is_detected() {
        let text = "extern crate serde_json;\n";
        let hits = scan_rust_source_text(text, None).expect("scan");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].crate_token, "serde_json");
    }
}
