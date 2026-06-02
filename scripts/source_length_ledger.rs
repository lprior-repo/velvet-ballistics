use crate::source_length_scan::{is_excluded, is_hot_source};
use std::collections::{HashMap, HashSet};
use std::fs;

pub fn source_exceptions(
    path: &str,
    counts: &HashMap<String, usize>,
    limit: usize,
    status: &mut u8,
) -> Result<HashSet<String>, String> {
    let mut exceptions = HashSet::new();
    for (line_no, line) in ledger_lines(path)?.iter().enumerate() {
        if skip_ledger_line(line) {
            continue;
        }
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() != 5 || parts.iter().any(|part| part.is_empty()) {
            eprintln!(
                "{}:{} malformed row; expected <file_path>|<owner>|<split_bead>|<removal_plan>|<reason>",
                path,
                line_no + 1
            );
            *status = 1;
            continue;
        }
        let file = parts[0];
        if !valid_source_path(path, line_no + 1, file, counts, status) {
            continue;
        }
        let lines = count_for(path, line_no + 1, file, counts, status);
        if lines <= limit {
            eprintln!(
                "{}:{} stale exception for {file} with {lines} physical lines (limit >{limit})",
                path,
                line_no + 1
            );
            *status = 1;
            continue;
        }
        if !exceptions.insert(file.to_string()) {
            eprintln!("{}:{} duplicate exception for {file}", path, line_no + 1);
            *status = 1;
        }
    }
    Ok(exceptions)
}

pub fn hot_exceptions(
    path: &str,
    counts: &HashMap<String, usize>,
    violations: &HashMap<String, usize>,
    status: &mut u8,
) -> Result<HashSet<String>, String> {
    let mut exceptions = HashSet::new();
    for (line_no, line) in ledger_lines(path)?.iter().enumerate() {
        if skip_ledger_line(line) {
            continue;
        }
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() != 6 || parts.iter().any(|part| part.is_empty()) {
            eprintln!(
                "{}:{} malformed row; expected <file_path>|<start_line>|<owner>|<split_bead>|<removal_plan>|<reason>",
                path,
                line_no + 1
            );
            *status = 1;
            continue;
        }
        let start = match parts[1].parse::<usize>() {
            Ok(value) if value > 0 => value,
            _ => {
                eprintln!(
                    "{}:{} start line is not a positive integer: {}",
                    path,
                    line_no + 1,
                    parts[1]
                );
                *status = 1;
                continue;
            }
        };
        let file = parts[0];
        if !valid_source_path(path, line_no + 1, file, counts, status) || !is_hot_source(file) {
            eprintln!(
                "{}:{} path is not in the hot-function scan scope: {file}",
                path,
                line_no + 1
            );
            *status = 1;
            continue;
        }
        let key = format!("{file}:{start}");
        if !violations.contains_key(&key) {
            eprintln!(
                "{}:{} stale or non-matching hot-function exception for {key}",
                path,
                line_no + 1
            );
            *status = 1;
            continue;
        }
        if !exceptions.insert(key.clone()) {
            eprintln!("{}:{} duplicate exception for {key}", path, line_no + 1);
            *status = 1;
        }
    }
    Ok(exceptions)
}

pub fn check_source_lengths(
    counts: &HashMap<String, usize>,
    exceptions: &HashSet<String>,
    limit: usize,
    ledger: &str,
    status: &mut u8,
) {
    let mut files: Vec<&String> = counts.keys().collect();
    files.sort();
    for file in files {
        let lines = match counts.get(file) {
            Some(value) => *value,
            None => 0,
        };
        if lines > limit && !exceptions.contains(file) {
            eprintln!(
                "{file} has {lines} physical lines (limit <={limit}) and no valid {ledger} row"
            );
            *status = 1;
        }
    }
}

fn ledger_lines(path: &str) -> Result<Vec<String>, String> {
    fs::read_to_string(path)
        .map(|text| text.lines().map(str::to_string).collect())
        .map_err(|err| {
            format!("{path} missing or unreadable; required for source-length checks: {err}")
        })
}

fn skip_ledger_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty() || trimmed.starts_with('#')
}

fn valid_source_path(
    path: &str,
    line_no: usize,
    file: &str,
    counts: &HashMap<String, usize>,
    status: &mut u8,
) -> bool {
    if file.starts_with('/') || file.starts_with("../") || file.contains("/../") {
        eprintln!("{path}:{line_no} invalid path; use a normalized repository-relative path");
        *status = 1;
        return false;
    }
    if !file.ends_with(".rs") || is_excluded(file) || !counts.contains_key(file) {
        eprintln!("{path}:{line_no} path is not a tracked first-party Rust source file: {file}");
        *status = 1;
        return false;
    }
    true
}

fn count_for(
    path: &str,
    line_no: usize,
    file: &str,
    counts: &HashMap<String, usize>,
    status: &mut u8,
) -> usize {
    match counts.get(file) {
        Some(lines) => *lines,
        None => {
            eprintln!(
                "{path}:{line_no} path is not a tracked first-party Rust source file: {file}"
            );
            *status = 1;
            0
        }
    }
}
