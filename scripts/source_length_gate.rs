use crate::source_length_ledger::{check_source_lengths, hot_exceptions, source_exceptions};
use crate::source_length_scan::{hot_violations, is_excluded};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitCode};

const SOURCE_LEDGER: &str = ".config/source-length-exceptions.txt";
const HOT_LEDGER: &str = ".config/hot-function-length-exceptions.txt";
const MUTANTS_RESIDUE_PREFIX: &str = "changed by ";
const MUTANTS_RESIDUE_TOOL: &str = "cargo-mutants";

pub fn main_exit() -> ExitCode {
    match run() {
        Ok(0) => ExitCode::SUCCESS,
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<u8, String> {
    let file_limit = env_limit("SOURCE_LENGTH_FILE_LIMIT", 300)?;
    let fn_limit = env_limit("SOURCE_LENGTH_HOT_FUNCTION_LIMIT", 25)?;
    let source_ledger = env_value("SOURCE_LENGTH_LEDGER", SOURCE_LEDGER);
    let hot_ledger = env_value("SOURCE_LENGTH_HOT_FUNCTION_LEDGER", HOT_LEDGER);
    let files = tracked_rust_files()?;
    let counts = line_counts(&files)?;
    let mut status = 0_u8;
    let source_exceptions = source_exceptions(&source_ledger, &counts, file_limit, &mut status)?;
    check_source_lengths(
        &counts,
        &source_exceptions,
        file_limit,
        &source_ledger,
        &mut status,
    );
    let hot = hot_violations(&files, fn_limit)?;
    let hot_exceptions = hot_exceptions(&hot_ledger, &counts, &hot, &mut status)?;
    check_hot_violations(&hot, &hot_exceptions, fn_limit, &mut status);
    check_mutants_residue(&files, &mut status)?;
    check_compile_split_sources(&mut status)?;
    Ok(status)
}

fn env_value(name: &str, default: &str) -> String {
    match env::var(name) {
        Ok(value) => value,
        Err(_) => default.to_string(),
    }
}

fn env_limit(name: &str, default: usize) -> Result<usize, String> {
    match env::var(name) {
        Ok(value) => match value.parse::<usize>() {
            Ok(parsed) if parsed > 0 => Ok(parsed),
            Ok(_) | Err(_) => Err(format!("{name} must be a positive integer")),
        },
        Err(_) => Ok(default),
    }
}

fn tracked_rust_files() -> Result<Vec<String>, String> {
    let stdout = tracked_file_stdout()?;
    let stdout = String::from_utf8(stdout)
        .map_err(|err| format!("tracked file output is not utf8: {err}"))?;
    Ok(stdout
        .lines()
        .filter(|file| file.ends_with(".rs"))
        .filter(|file| !is_excluded(file))
        .filter(|file| Path::new(file).exists())
        .map(str::to_string)
        .collect())
}

fn tracked_file_stdout() -> Result<Vec<u8>, String> {
    let git = match Command::new("git").args(["ls-files", "*.rs"]).output() {
        Ok(output) if output.status.success() => return Ok(output.stdout),
        Ok(output) => command_failure("git ls-files", &output.stderr),
        Err(err) => format!("git ls-files could not start: {err}"),
    };
    match Command::new("jj").args(["file", "list"]).output() {
        Ok(output) if output.status.success() => Ok(output.stdout),
        Ok(output) => Err(format!(
            "failed to list tracked files; git: {git}; jj: {}",
            command_failure("jj file list", &output.stderr)
        )),
        Err(err) => Err(format!(
            "failed to list tracked files; git: {git}; jj file list could not start: {err}"
        )),
    }
}

fn command_failure(label: &str, stderr: &[u8]) -> String {
    let detail = String::from_utf8_lossy(stderr);
    let trimmed = detail.trim();
    if trimmed.is_empty() {
        format!("{label} exited non-zero")
    } else {
        format!("{label} exited non-zero: {trimmed}")
    }
}

fn line_counts(files: &[String]) -> Result<HashMap<String, usize>, String> {
    let mut counts = HashMap::new();
    for file in files {
        let text =
            fs::read_to_string(file).map_err(|err| format!("failed to read {file}: {err}"))?;
        counts.insert(file.clone(), text.lines().count());
    }
    Ok(counts)
}

fn check_hot_violations(
    violations: &HashMap<String, usize>,
    exceptions: &HashSet<String>,
    limit: usize,
    status: &mut u8,
) {
    let mut keys: Vec<&String> = violations.keys().collect();
    keys.sort();
    for key in keys {
        if exceptions.contains(key) {
            continue;
        }
        if let Some((file, start)) = key.rsplit_once(':') {
            if let Some(count) = violations.get(key) {
                eprintln!("{file}:{start} hot function has {count} logical lines (limit {limit})");
                *status = 1;
            }
        }
    }
}

fn check_mutants_residue(files: &[String], status: &mut u8) -> Result<(), String> {
    let mut found = false;
    for file in files {
        let text = fs::read_to_string(file)
            .map_err(|err| format!("failed to read {file} for cargo-mutants residue: {err}"))?;
        for (index, line) in text.lines().enumerate() {
            if line.contains(MUTANTS_RESIDUE_PREFIX) && line.contains(MUTANTS_RESIDUE_TOOL) {
                if !found {
                    eprintln!("cargo-mutants residue markers found:");
                }
                let line_no = index
                    .checked_add(1)
                    .ok_or_else(|| format!("line number overflowed while scanning {file}"))?;
                eprintln!("{file}:{line_no}:{line}");
                found = true;
            }
        }
    }
    if found {
        *status = 1;
    }
    Ok(())
}

fn check_compile_split_sources(status: &mut u8) -> Result<(), String> {
    let compile_dir = "crates/vb_compile/src";
    let hidden = format!("{compile_dir}/compile_core_impl.rs");
    if std::path::Path::new(&hidden).exists() {
        eprintln!("{hidden} must not remain as a hidden production include body");
        *status = 1;
    }
    for file in [
        "mod_compile_core.rs",
        "mod_compile_errors.rs",
        "mod_compile_validation.rs",
        "mod_compile_lowering.rs",
    ] {
        let path = format!("{compile_dir}/{file}");
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("{path} missing from compile split: {err}"))?;
        if text.contains("include!(") {
            eprintln!("{path} contains monolithic include body");
            *status = 1;
        }
        if text.lines().count() < 50 && !text.lines().any(|line| line.starts_with("mod ")) {
            eprintln!("{path} is doc-only shell, not an owned implementation module");
            *status = 1;
        }
    }
    Ok(())
}
