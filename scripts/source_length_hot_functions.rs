#[path = "check_test_ownership.rs"]
mod ownership;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

fn rust_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let crate_entries = fs::read_dir(root.join("crates"))
        .map_err(|error| format!("read crates directory: {error}"))?;
    let mut pending = Vec::new();
    for entry in crate_entries {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry.file_type().map_err(|error| error.to_string())?.is_dir() {
            let source = entry.path().join("src");
            if source.is_dir() {
                pending.push(source);
            }
        }
    }
    let mut files = Vec::new();
    let mut visited = 0usize;
    while let Some(directory) = pending.pop() {
        visited = visited.saturating_add(1);
        if visited > 20_000 {
            return Err("source traversal exceeded 20000 directories".to_owned());
        }
        let entries = fs::read_dir(&directory)
            .map_err(|error| format!("read {}: {error}", directory.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            let kind = entry.file_type().map_err(|error| error.to_string())?;
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn reach(seeds: &[PathBuf], edges: &BTreeMap<PathBuf, Vec<PathBuf>>) -> BTreeSet<PathBuf> {
    let mut reached = BTreeSet::new();
    let mut pending = VecDeque::from(seeds.to_vec());
    while let Some(path) = pending.pop_front() {
        if reached.len() >= 20_000 || !reached.insert(path.clone()) {
            continue;
        }
        if let Some(children) = edges.get(&path) {
            pending.extend(children.iter().cloned());
        }
    }
    reached
}

fn line_number(source: &str, offset: usize) -> usize {
    source.get(..offset).map_or(1, |prefix| prefix.bytes().filter(|byte| *byte == b'\n').count().saturating_add(1))
}

fn logical_lines(source: &str, start: usize, end: usize) -> usize {
    source.get(start..end).map_or_else(String::new, str::to_owned).lines().filter(|line| {
        let line = line.trim();
        !line.is_empty() && !line.starts_with("//") && line != "{" && line != "}"
    }).count()
}

fn function_body_open<'a>(tokens: impl Iterator<Item = &'a str>, start: usize) -> Option<usize> {
    let mut parentheses = 0usize;
    let mut brackets = 0usize;
    let mut angles = 0usize;
    let mut braces = 0usize;
    let mut previous_dash = false;
    for (index, token) in tokens.enumerate().skip(start) {
        match token {
            "(" => parentheses = parentheses.saturating_add(1),
            ")" => parentheses = parentheses.saturating_sub(1),
            "[" => brackets = brackets.saturating_add(1),
            "]" => brackets = brackets.saturating_sub(1),
            "<" if parentheses == 0 && brackets == 0 && braces == 0 => {
                angles = angles.saturating_add(1);
            }
            ">" if angles > 0 && !previous_dash && parentheses == 0 && brackets == 0 && braces == 0 => {
                angles = angles.saturating_sub(1);
            }
            "{" => {
                if parentheses == 0 && brackets == 0 && angles == 0 && braces == 0 {
                    return Some(index);
                }
                braces = braces.saturating_add(1);
            }
            "}" if braces > 0 => braces = braces.saturating_sub(1),
            ";" if parentheses == 0 && brackets == 0 && angles == 0 && braces == 0 => {
                return None;
            }
            _ => {}
        }
        previous_dash = token == "-";
    }
    None
}

fn main() -> Result<(), String> {
    if !ownership::self_test() {
        return Err("ownership lexer self-test failed".to_owned());
    }
    let root = std::env::current_dir().map_err(|error| error.to_string())?;
    let files = rust_files(&root)?;
    let mut contents = BTreeMap::new();
    let mut scopes = BTreeMap::new();
    let mut production_edges: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    let mut test_edges: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    let mut all_edges: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    let mut incoming = BTreeSet::new();
    for file in &files {
        let source = fs::read_to_string(file).map_err(|error| format!("read {}: {error}", file.display()))?;
        let owned = ownership::test_scopes(&source);
        let mut prod = Vec::new();
        let mut test = Vec::new();
        let mut all = Vec::new();
        for edge in ownership::all_edges(&source) {
            if let Some(parent) = file.parent()
                && let Ok(child) = parent.join(edge.path).canonicalize()
                && child.starts_with(&root)
            {
                incoming.insert(child.clone());
                all.push(child.clone());
                if owned.iter().any(|range| range.contains(&edge.offset)) {
                    test.push(child);
                } else {
                    prod.push(child);
                }
            }
        }
        contents.insert(file.clone(), source);
        scopes.insert(file.clone(), owned);
        production_edges.insert(file.clone(), prod);
        test_edges.insert(file.clone(), test);
        all_edges.insert(file.clone(), all);
    }
    let roots = files.iter().filter(|file| !incoming.contains(*file)).cloned().collect::<Vec<_>>();
    let production_files = reach(&roots, &production_edges);
    let test_seeds = test_edges.values().flatten().cloned().collect::<Vec<_>>();
    let test_files = reach(&test_seeds, &all_edges);
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).map_err(|error| error.to_string())?;
    let mut failed = false;
    for hot_file in input.lines() {
        let path = root.join(hot_file);
        let Some(source) = contents.get(&path) else { continue; };
        let test_only_file = test_files.contains(&path) && !production_files.contains(&path);
        let tokens = ownership::lexer_tokens(source);
        for (index, token) in tokens.iter().enumerate() {
            if token.text != "fn" { continue; }
            let Some(name) = tokens.get(index.saturating_add(1)).map(|item| item.text.as_str()) else { continue; };
            let Some(open) = function_body_open(
                tokens.iter().map(|item| item.text.as_str()),
                index.saturating_add(2),
            ) else { continue; };
            let mut depth = 0usize;
            let mut close = open;
            while let Some(item) = tokens.get(close) {
                match item.text.as_str() {
                    "{" => depth = depth.saturating_add(1),
                    "}" => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 { break; }
                    }
                    _ => {}
                }
                close = close.saturating_add(1);
            }
            let Some(end_token) = tokens.get(close) else { continue; };
            let owned = scopes.get(&path).is_some_and(|ranges| ranges.iter().any(|range| range.contains(&token.start)));
            if test_only_file || owned { continue; }
            let start_line = line_number(source, token.start);
            let end_line = line_number(source, end_token.end);
            let count = logical_lines(source, token.start, end_token.end);
            if count > 25 {
                println!("{hot_file}|{name}|{start_line}|{end_line}|{count}");
            }
            if count > 25 {
                eprintln!("FAIL {hot_file}:{start_line} {name} ends at {end_line}: {count} logical lines (limit 25)");
                failed = true;
            }
        }
    }
    if failed { std::process::exit(1); }
    Ok(())
}
