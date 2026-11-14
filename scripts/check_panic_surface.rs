#[path = "check_test_ownership.rs"]
mod check_test_ownership;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

fn source_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let crate_entries = fs::read_dir(root.join("crates"))
        .map_err(|error| format!("read crates directory: {error}"))?;
    let mut pending = Vec::new();
    for entry in crate_entries {
        let entry = entry.map_err(|error| format!("crate directory entry: {error}"))?;
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
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
            let entry = entry.map_err(|error| format!("directory entry: {error}"))?;
            let path = entry.path();
            let kind = entry
                .file_type()
                .map_err(|error| format!("file type {}: {error}", path.display()))?;
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

fn excluded(path: &str) -> bool {
    path.contains("/workspace_tests/")
        || path.contains("/test_loop_inventory/")
        || path.contains("/tests/")
        || path.ends_with("/tests.rs")
        || path.ends_with("_tests.rs")
        || Path::new(path)
            .file_name()
            .is_some_and(|name| name.to_string_lossy().contains("_test"))
        || path.contains("/lifecycle_tests/")
        || path.contains("/kani")
        || path.contains("/models/loom/")
        || path.contains("/benches/")
        || path.contains("/examples/")
        || path.contains("/proofs/")
        || path.contains("/fuzz/")
        || path.contains("/target/")
        || path.contains("/.beads/")
        || path.contains("/fixtures/")
        || path.ends_with("/build.rs")
        || path.contains("/test/")
}

fn canonical_edge(root: &Path, source: &Path, edge: &str) -> Option<PathBuf> {
    let parent = source.parent()?;
    let candidate = parent.join(edge).canonicalize().ok()?;
    let canonical_root = root.canonicalize().ok()?;
    candidate.starts_with(canonical_root).then_some(candidate)
}

fn main() -> Result<(), String> {
    if !check_test_ownership::self_test() {
        return Err("ownership lexer self-test failed".to_owned());
    }
    println!("OwnershipLexerSelfTest: PASS");
    if !graph_self_test() {
        return Err("ownership graph self-test failed".to_owned());
    }
    println!("OwnershipGraphSelfTest: PASS");
    let root = std::env::current_dir().map_err(|error| format!("current directory: {error}"))?;
    if !root.join("Cargo.toml").is_file() || !root.join("crates").is_dir() {
        return Err("InvalidInvocation: run from repository root".to_owned());
    }
    let files = source_files(&root)?;
    let mut contents = BTreeMap::new();
    let mut test_scopes = BTreeMap::new();
    let mut test_edges: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    let mut production_edges: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    let mut all_children: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    let mut incoming = BTreeSet::new();
    for file in &files {
        let bytes = fs::read(file).map_err(|error| format!("read {}: {error}", file.display()))?;
        let source = String::from_utf8(bytes)
            .map_err(|_| format!("non-UTF8 Rust source: {}", file.display()))?;
        let scopes = check_test_ownership::test_scopes(&source);
        let mut test_children = Vec::new();
        let mut production_children = Vec::new();
        let mut children = Vec::new();
        for edge in check_test_ownership::all_edges(&source) {
            if let Some(child) = canonical_edge(&root, file, &edge.path) {
                incoming.insert(child.clone());
                children.push(child.clone());
                if scopes.iter().any(|range| range.contains(&edge.offset)) {
                    test_children.push(child);
                } else {
                    production_children.push(child);
                }
            }
        }
        contents.insert(file.clone(), source);
        test_scopes.insert(file.clone(), scopes);
        test_edges.insert(file.clone(), test_children);
        production_edges.insert(file.clone(), production_children);
        all_children.insert(file.clone(), children);
    }
    let roots = files
        .iter()
        .filter(|file| !incoming.contains(*file))
        .cloned()
        .collect::<Vec<_>>();
    let production = reach(&roots, &production_edges);
    let mut test_seeds = Vec::new();
    for children in test_edges.values() {
        test_seeds.extend(children.iter().cloned());
    }
    let test_owned = reach(&test_seeds, &all_children);
    let mut violations = Vec::new();
    for file in files {
        let path_text = file.to_string_lossy().to_string();
        if excluded(&path_text) {
            continue;
        }
        let Some(source) = contents.get(&file) else {
            continue;
        };
        let scopes = test_scopes
            .get(&file)
            .cloned()
            .map_or_else(Vec::new, |owned| owned);
        let test_only_file = test_owned.contains(&file) && !production.contains(&file);
        for (offset, name) in check_test_ownership::panic_hits(source) {
            if exempt_hit(offset, &scopes, test_only_file) {
                continue;
            }
            let line = source.get(..offset).map_or(1usize, |prefix| {
                prefix
                    .bytes()
                    .filter(|byte| *byte == b'\n')
                    .count()
                    .saturating_add(1)
            });
            violations.push(format!("{path_text}:{line}:{}!", name));
        }
    }
    if violations.is_empty() {
        println!("NoViolationFound\nExitCode: 0");
        Ok(())
    } else {
        violations.sort();
        for violation in violations {
            println!("{violation}");
        }
        eprintln!(
            "ViolationFound: production panic/assert macro surface is non-empty\nExitCode: 2"
        );
        std::process::exit(2);
    }
}

fn graph_self_test() -> bool {
    let parent = PathBuf::from("parent.rs");
    let child = PathBuf::from("child.rs");
    let mut test = BTreeMap::new();
    test.insert(parent.clone(), vec![child.clone()]);
    let test_owned = reach(std::slice::from_ref(&child), &test);
    let test_only = test_owned.contains(&child) && !test_owned.contains(&parent);
    let mut production = BTreeMap::new();
    production.insert(parent.clone(), vec![child.clone()]);
    let prod_owned = reach(std::slice::from_ref(&parent), &production);
    let dual_reachable = test_owned.contains(&child) && prod_owned.contains(&child);
    let source = "#[cfg(test)] mod tests { assert_eq!(1, 1); }\nassert_eq!(2, 2);";
    let scopes = check_test_ownership::test_scopes(source);
    let hits = check_test_ownership::panic_hits(source);
    let local_scope_classification = hits.len() == 2
        && hits
            .first()
            .is_some_and(|(offset, _)| exempt_hit(*offset, &scopes, false))
        && hits
            .get(1)
            .is_some_and(|(offset, _)| !exempt_hit(*offset, &scopes, false));
    let passed = test_only
        && dual_reachable
        && local_scope_classification
        && exempt_hit(0, &[], test_only)
        && !exempt_hit(
            0,
            &[],
            test_owned.contains(&child) && !prod_owned.contains(&child),
        );
    if !passed {
        eprintln!(
            "ownership graph details: test_only={test_only} dual={dual_reachable} hits={hits:?} scopes={scopes:?} local={local_scope_classification}"
        );
    }
    passed
}

fn exempt_hit(offset: usize, scopes: &[std::ops::Range<usize>], test_only_file: bool) -> bool {
    test_only_file || scopes.iter().any(|range| range.contains(&offset))
}

fn reach(seeds: &[PathBuf], edges: &BTreeMap<PathBuf, Vec<PathBuf>>) -> BTreeSet<PathBuf> {
    let mut reached = BTreeSet::new();
    let mut pending = VecDeque::new();
    pending.extend(seeds.iter().cloned());
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
