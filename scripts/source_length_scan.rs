use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn hot_violations(files: &[String], limit: usize) -> Result<HashMap<String, usize>, String> {
    let mut violations = HashMap::new();
    for file in files.iter().filter(|file| is_hot_source(file)) {
        let text =
            fs::read_to_string(file).map_err(|err| format!("failed to read {file}: {err}"))?;
        collect_hot_violations(file, &text, limit, &mut violations);
    }
    Ok(violations)
}

fn collect_hot_violations(
    file: &str,
    text: &str,
    limit: usize,
    violations: &mut HashMap<String, usize>,
) {
    let mut in_fn = false;
    let mut start = 0_usize;
    let mut count = 0_usize;
    let mut depth = 0_i32;
    let mut seen_body = false;
    let mut block_comment = false;
    for (idx, raw) in text.lines().enumerate() {
        let line_no = idx + 1;
        if !in_fn && starts_fn(raw.trim_start()) {
            in_fn = true;
            start = line_no;
            count = 0;
            depth = 0;
            seen_body = false;
        }
        if !in_fn {
            continue;
        }
        if logical_line(raw) {
            count += 1;
        }
        let clean = brace_text(raw, &mut block_comment);
        if clean.contains('{') {
            seen_body = true;
        }
        depth += clean.chars().filter(|ch| *ch == '{').count() as i32;
        depth -= clean.chars().filter(|ch| *ch == '}').count() as i32;
        if seen_body && depth <= 0 {
            if count > limit {
                violations.insert(format!("{file}:{start}"), count);
            }
            in_fn = false;
        }
    }
}

fn starts_fn(mut text: &str) -> bool {
    if let Some(rest) = text.strip_prefix("pub ") {
        text = rest;
    } else if let Some(rest) = text.strip_prefix("pub(") {
        let end = match rest.find(')') {
            Some(value) => value,
            None => return false,
        };
        text = match rest.get(end + 1..) {
            Some(value) => value.trim_start(),
            None => return false,
        };
    }
    loop {
        if let Some(rest) = text
            .strip_prefix("const ")
            .or_else(|| text.strip_prefix("async "))
        {
            text = rest;
        } else {
            return text.starts_with("fn ");
        }
    }
}

fn logical_line(line: &str) -> bool {
    let text = line.trim();
    !text.is_empty() && !text.starts_with("//") && text != "{" && text != "}"
}

fn brace_text(line: &str, block_comment: &mut bool) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut idx = 0_usize;
    let mut in_string = false;
    let mut in_char = false;
    while idx < chars.len() {
        let ch = chars[idx];
        let next = chars.get(idx + 1).copied();
        if *block_comment {
            if ch == '*' && next == Some('/') {
                *block_comment = false;
                idx += 2;
            } else {
                idx += 1;
            }
        } else if in_string {
            if ch == '\\' {
                idx += 2;
            } else {
                in_string = ch != '"';
                idx += 1;
            }
        } else if in_char {
            if ch == '\\' {
                idx += 2;
            } else {
                in_char = ch != '\'';
                idx += 1;
            }
        } else if ch == '/' && next == Some('/') {
            break;
        } else if ch == '/' && next == Some('*') {
            *block_comment = true;
            idx += 2;
        } else if ch == '"' {
            in_string = true;
            idx += 1;
        } else if ch == '\'' {
            in_char = true;
            idx += 1;
        } else {
            out.push(ch);
            idx += 1;
        }
    }
    out
}

pub fn is_hot_source(file: &str) -> bool {
    if is_test_like(file) {
        return false;
    }
    let parts: Vec<&str> = file.split('/').collect();
    if parts.len() < 4 || parts.first() != Some(&"crates") || parts.get(2) != Some(&"src") {
        return false;
    }
    let crate_name = match parts.get(1) {
        Some(value) => *value,
        None => return false,
    };
    if crate_name == "vb_runtime" {
        return true;
    }
    if crate_name.starts_with("vb_") {
        let first = match parts.get(3) {
            Some(value) => *value,
            None => return false,
        };
        return matches!(
            first,
            "engine.rs" | "engine" | "runtime" | "generated" | "perf"
        );
    }
    false
}

fn is_test_like(file: &str) -> bool {
    let name = match Path::new(file).file_name().and_then(|value| value.to_str()) {
        Some(value) => value,
        None => "",
    };
    if name == "tests.rs"
        || name.ends_with("_tests.rs")
        || name.contains("tests")
        || file.contains("/tests/")
    {
        return true;
    }
    let tokens = file.replace(['/', '.', '_', '-'], " ");
    tokens.split_whitespace().any(|token| {
        matches!(
            token,
            "diagnostic"
                | "diagnostics"
                | "fixture"
                | "fixtures"
                | "harness"
                | "harnesses"
                | "kani"
                | "loom"
                | "model"
                | "models"
                | "proof"
                | "proofs"
                | "property"
                | "properties"
                | "proptest"
                | "proptests"
                | "support"
                | "test"
                | "tests"
                | "verification"
                | "benches"
        )
    })
}

pub fn is_excluded(file: &str) -> bool {
    file.starts_with("target/")
        || file.starts_with(".jj/")
        || file.starts_with(".beads/")
        || file.starts_with(".evidence/")
        || file.starts_with(".cargo_temp/")
        || file.starts_with("arch-drift-")
        || file.starts_with("cargo-home/")
        || file.starts_with("cargo_home/")
        || file.starts_with(".cargo/registry/")
        || file.contains("/target/")
        || file.contains("/.jj/")
        || file.contains("/.beads/")
        || file.contains("/.evidence/")
        || file.contains("/.cargo_temp/")
}
