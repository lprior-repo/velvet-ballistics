use self::resolve::Identity;
use super::{REGISTRY_PREFIX, STOP};
use std::collections::BTreeSet;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[path = "context.rs"]
mod context;
#[path = "extract.rs"]
mod extract;
#[path = "index.rs"]
mod index;
#[path = "ranges.rs"]
mod ranges;
#[path = "registry.rs"]
mod registry;
#[path = "resolve.rs"]
mod resolve;

struct FileReport {
    name: String,
    checks: usize,
    failures: Vec<String>,
}

struct CitationScan<'a> {
    root: &'a Path,
    cargo_home: &'a Path,
    source_index: &'a [PathBuf],
    citing_lines: &'a [String],
    seen: BTreeSet<(Identity, ranges::CitationNumber, ranges::CitationNumber)>,
}

pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        return report_error(64, "Usage: audit-spec-citations <repo-root> [file ...]");
    }
    match run_inner(args) {
        Ok(code) => code,
        Err(error) => report_error(3, &error),
    }
}

fn run_inner(args: &[String]) -> Result<i32, String> {
    let root = args
        .first()
        .map(PathBuf::from)
        .ok_or_else(|| "Usage: audit-spec-citations <repo-root> [file ...]".to_string())?;
    if !root.is_dir() {
        return Err(format!("not a directory: {}", root.display()));
    }
    let cargo_home = cargo_home()?;
    let files = input_files(args, &root)?;
    if files.is_empty() {
        return Err("no verification/verus Rust specs to audit".to_string());
    }
    let source_index = index::build(&root)?;
    let mut reports = Vec::<FileReport>::new();
    let mut all_failures = Vec::new();
    let mut total_checks = 0usize;
    for (file, name) in files {
        let lines = index::read_lines(&file)?;
        let (checks, failures) = scan_file(&lines, &root, &cargo_home, &source_index)?;
        total_checks = total_checks
            .checked_add(checks)
            .ok_or_else(|| "total citation count overflow".to_string())?;
        all_failures.extend(failures.iter().map(|failure| format!("{name}: {failure}")));
        update_report(&mut reports, name, checks, failures);
    }
    write_report(&reports, total_checks, &all_failures)?;
    Ok(if all_failures.is_empty() { 0 } else { 1 })
}

fn cargo_home() -> Result<PathBuf, String> {
    match std::env::var_os("CARGO_HOME") {
        Some(path) => Ok(PathBuf::from(path)),
        None => std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join(".cargo"))
            .ok_or_else(|| "CARGO_HOME and HOME are both unset".to_string()),
    }
}

fn input_files(args: &[String], root: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    if args.len() > 1 {
        return args
            .iter()
            .skip(1)
            .map(|argument| {
                let path = root.join(argument);
                if !path.is_file() {
                    return Err(format!("not a file: {}", path.display()));
                }
                Ok((path, argument.clone()))
            })
            .collect();
    }
    index::discover_specs(root).map(|paths| {
        paths
            .into_iter()
            .map(|path| {
                let name = path.strip_prefix(root).map_or_else(
                    |_| path.to_string_lossy().into_owned(),
                    |relative| relative.to_string_lossy().into_owned(),
                );
                (path, name)
            })
            .collect()
    })
}

fn scan_file(
    lines: &[String],
    root: &Path,
    cargo_home: &Path,
    source_index: &[PathBuf],
) -> Result<(usize, Vec<String>), String> {
    let mut checks = 0usize;
    let mut failures = Vec::new();
    let mut scanner = CitationScan {
        root,
        cargo_home,
        source_index,
        citing_lines: lines,
        seen: BTreeSet::new(),
    };
    for (line_index, line) in lines.iter().enumerate() {
        if !extract::is_comment_line(line) {
            continue;
        }
        let line_number = line_index
            .checked_add(1)
            .ok_or_else(|| "citation line number overflow".to_string())?;
        for (raw_path, spec) in extract::extract_citations(line) {
            let spec = spec.trim();
            let result = scanner.check(&raw_path, spec, line_number)?;
            checks = checks
                .checked_add(result.0)
                .ok_or_else(|| "per-file citation count overflow".to_string())?;
            failures.extend(result.1);
        }
    }
    Ok((checks, failures))
}

impl CitationScan<'_> {
    fn check(
        &mut self,
        raw_path: &str,
        spec: &str,
        line_number: usize,
    ) -> Result<(usize, Vec<String>), String> {
        let resolved = match resolve::resolve_path(
            self.root,
            raw_path,
            self.cargo_home,
            self.source_index,
        ) {
            Ok(Some(path)) => path,
            Ok(None) => {
                let error = if raw_path.starts_with(REGISTRY_PREFIX) {
                    format!(
                        "line {line_number}: cargo-registry source not present locally: {raw_path}"
                    )
                } else {
                    format!("line {line_number}: ambiguous/unresolvable short path {raw_path}")
                };
                return Ok((0, vec![error]));
            }
            Err(error) => return Ok((0, vec![format!("line {line_number}: {error}")])),
        };
        if raw_path.starts_with(REGISTRY_PREFIX)
            && let Some(error) =
                registry::registry_pin_error(self.root, raw_path, &resolved, self.cargo_home)
        {
            return Ok((0, vec![format!("line {line_number}: {error}")]));
        }
        if !resolved.is_file() {
            return Ok((
                0,
                vec![format!(
                    "line {line_number}: cited file missing: {}",
                    display_path(self.root, raw_path, &resolved)
                )],
            ));
        }
        let target_lines = index::read_lines(&resolved)?;
        let parsed = match ranges::parse_ranges(spec) {
            Ok(parsed) => parsed,
            Err(error) => {
                return Ok((
                    0,
                    vec![format!("line {line_number}: {raw_path}:{spec} {error}")],
                ));
            }
        };
        let context = context::get_context(self.citing_lines, line_number)
            .map_err(|error| format!("citation context failed: {error}"))?;
        let identity = resolve::citation_identity(raw_path, &resolved);
        let mut checks = 0usize;
        let mut failures = Vec::new();
        for (start, end) in parsed {
            let key = (identity.clone(), start.clone(), end.clone());
            if !self.seen.insert(key) {
                continue;
            }
            checks = checks
                .checked_add(1)
                .ok_or_else(|| "citation count overflow".to_string())?;
            let shown_path = display_path(self.root, raw_path, &resolved);
            let failure = |reason: &str| {
                ranges::range_failure(line_number, &shown_path, &start, &end, reason)
            };
            if let Some(error) = ranges::validate_range(&start, &end, target_lines.len()) {
                failures.push(failure(&error));
                continue;
            }
            let (start_line, end_line) = match ranges::bounded_range(&start, &end) {
                Ok(bounds) => bounds,
                Err(error) => {
                    failures.push(failure(&error));
                    continue;
                }
            };
            let span = match context::get_span(&target_lines, start_line, end_line) {
                Ok(span) => span,
                Err(error) => {
                    failures.push(failure(&error));
                    continue;
                }
            };
            if !context::match_identifiers(&span, &context, STOP) {
                let excerpt = span.lines().next().map_or("unknown", str::trim);
                let error =
                    format!("span contains none of context idents; span starts: {excerpt:?}");
                failures.push(failure(&error));
            }
        }
        Ok((checks, failures))
    }
}
fn display_path(root: &Path, raw_path: &str, resolved: &Path) -> String {
    let path = if raw_path.starts_with(REGISTRY_PREFIX) {
        resolved
    } else {
        resolved.strip_prefix(root).map_or(resolved, |path| path)
    };
    path.display().to_string()
}

fn update_report(
    reports: &mut Vec<FileReport>,
    name: String,
    checks: usize,
    failures: Vec<String>,
) {
    if let Some(report) = reports.iter_mut().find(|report| report.name == name) {
        report.checks = checks;
        report.failures = failures;
    } else {
        reports.push(FileReport {
            name,
            checks,
            failures,
        });
    }
}

fn write_report(reports: &[FileReport], checks: usize, failures: &[String]) -> Result<(), String> {
    let mut stdout = io::stdout().lock();
    for report in reports {
        writeln!(
            stdout,
            "{}: {} checks, {} failures",
            report.name,
            report.checks,
            report.failures.len()
        )
        .map_err(|error| format!("cannot write report: {error}"))?;
    }
    writeln!(
        stdout,
        "TOTAL: {checks} checks, {} failures",
        failures.len()
    )
    .map_err(|error| format!("cannot write report: {error}"))?;
    for failure in failures {
        writeln!(stdout, "FAIL {failure}")
            .map_err(|error| format!("cannot write report: {error}"))?;
    }
    Ok(())
}

fn report_error(code: i32, message: &str) -> i32 {
    let mut stderr = io::stderr().lock();
    match writeln!(stderr, "{message}") {
        Ok(()) => code,
        Err(_) => 3,
    }
}
