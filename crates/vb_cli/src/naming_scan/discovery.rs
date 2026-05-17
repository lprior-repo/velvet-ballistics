use std::fs;
use std::path::{Path, PathBuf};

use super::types::*;

pub fn discover_scan_inputs(
    root: RepoRoot,
    config: &ScanConfig,
) -> Result<Vec<ScanInput>, NamingScanError> {
    if !root.0.exists() || !root.0.is_dir() {
        return Err(NamingScanError::InvalidRoot { root });
    }
    let mut inputs = Vec::new();
    collect_inputs(&root.0, &root.0, config, &mut inputs)?;
    inputs.sort_by(|left, right| input_path(left).cmp(input_path(right)));
    Ok(inputs)
}

fn collect_inputs(
    root: &Path,
    current: &Path,
    config: &ScanConfig,
    inputs: &mut Vec<ScanInput>,
) -> Result<(), NamingScanError> {
    let Some(entries) = discover_entries(root, current)? else {
        return Ok(());
    };
    for entry_result in entries {
        let Some(entry) = readable_entry(current, entry_result)? else {
            continue;
        };
        collect_entry(root, config, inputs, entry)?;
    }
    Ok(())
}

fn discover_entries(root: &Path, current: &Path) -> Result<Option<fs::ReadDir>, NamingScanError> {
    match fs::read_dir(current) {
        Ok(entries) => Ok(Some(entries)),
        Err(source) if current != root && discovery_permission_denied(&source) => Ok(None),
        Err(source) => Err(discovery_error(current, source)),
    }
}

fn readable_entry(
    current: &Path,
    entry_result: Result<fs::DirEntry, std::io::Error>,
) -> Result<Option<fs::DirEntry>, NamingScanError> {
    match entry_result {
        Ok(entry) => Ok(Some(entry)),
        Err(source) if discovery_permission_denied(&source) => Ok(None),
        Err(source) => Err(discovery_error(current, source)),
    }
}

fn collect_entry(
    root: &Path,
    config: &ScanConfig,
    inputs: &mut Vec<ScanInput>,
    entry: fs::DirEntry,
) -> Result<(), NamingScanError> {
    let path = entry.path();
    let relative = relative_path(root, &path)?;
    if excluded(&relative, config) {
        return Ok(());
    }
    let Some(file_type) = readable_file_type(&entry, &path)? else {
        return Ok(());
    };
    collect_typed_entry(root, config, inputs, path, relative, file_type)
}

fn readable_file_type(
    entry: &fs::DirEntry,
    path: &Path,
) -> Result<Option<fs::FileType>, NamingScanError> {
    match entry.file_type() {
        Ok(file_type) => Ok(Some(file_type)),
        Err(source) if discovery_permission_denied(&source) => Ok(None),
        Err(source) => Err(discovery_error(path, source)),
    }
}

fn collect_typed_entry(
    root: &Path,
    config: &ScanConfig,
    inputs: &mut Vec<ScanInput>,
    path: PathBuf,
    relative: String,
    file_type: fs::FileType,
) -> Result<(), NamingScanError> {
    if file_type.is_dir() {
        collect_inputs(root, &path, config, inputs)
    } else {
        push_eligible_file(inputs, path, &relative);
        Ok(())
    }
}

fn push_eligible_file(inputs: &mut Vec<ScanInput>, path: PathBuf, relative: &str) {
    if eligible(relative) {
        inputs.push(ScanInput::File {
            path: RepoPath::new(relative),
            absolute_path: path,
        });
    }
}

fn discovery_permission_denied(source: &std::io::Error) -> bool {
    source.kind() == std::io::ErrorKind::PermissionDenied
}

fn relative_path(root: &Path, path: &Path) -> Result<String, NamingScanError> {
    path.strip_prefix(root)
        .map(|relative| relative.to_string_lossy().to_string())
        .map_err(|source| NamingScanError::FileDiscoveryFailed {
            path: RepoPath::new(&path.to_string_lossy()),
            source: source.to_string(),
        })
}

fn discovery_error(path: &Path, source: std::io::Error) -> NamingScanError {
    NamingScanError::FileDiscoveryFailed {
        path: RepoPath::new(&path.to_string_lossy()),
        source: source.to_string(),
    }
}

fn excluded(path: &str, config: &ScanConfig) -> bool {
    path.starts_with(".git/")
        || path.starts_with("target/")
        || path.starts_with(".beads/dolt/")
        || path.starts_with(".beads/backup/")
        || path.starts_with(".beads/embeddeddolt/")
        || config
            .excluded_path_rules
            .iter()
            .any(|rule| path_matches_rule(path, rule))
}

fn path_matches_rule(path: &str, rule: &str) -> bool {
    if let Some(prefix) = rule.strip_suffix("/**") {
        path.starts_with(prefix)
    } else {
        path == rule
    }
}

fn eligible(path: &str) -> bool {
    path.ends_with(".rs")
        || path.ends_with(".md")
        || path.ends_with(".toml")
        || path.ends_with(".yml")
        || path.ends_with(".yaml")
        || path.ends_with(".sh")
        || path == "Cargo.toml"
}

fn input_path(input: &ScanInput) -> &RepoPath {
    match input {
        ScanInput::Text { path, .. }
        | ScanInput::Bytes { path, .. }
        | ScanInput::File { path, .. } => path,
    }
}
