use std::fs;
use std::path::{Path, PathBuf};

const REGISTRY_PREFIX: &str = "/.cargo/registry/src/";

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
pub enum CitationDomain {
    Repository,
    Registry,
}

pub type Identity = (CitationDomain, std::ffi::OsString);

pub fn citation_identity(raw_path: &str, resolved: &Path) -> Identity {
    if raw_path.starts_with(REGISTRY_PREFIX) {
        (
            CitationDomain::Registry,
            resolved.components().collect::<PathBuf>().into_os_string(),
        )
    } else {
        (CitationDomain::Repository, resolved.as_os_str().to_owned())
    }
}

pub fn resolve_path(
    root: &Path,
    raw_path: &str,
    cargo_home: &Path,
    index: &[PathBuf],
) -> Result<Option<PathBuf>, String> {
    if raw_path.starts_with(REGISTRY_PREFIX) {
        return resolve_registry(raw_path, cargo_home);
    }
    if has_explicit_prefix(raw_path) {
        return Ok(Some(root.join(raw_path)));
    }
    if raw_path.contains('/') {
        let candidates: Vec<PathBuf> = suffix_candidates(raw_path, index)
            .into_iter()
            .map(|path| root.join(path))
            .collect();
        if candidates.len() == 1 {
            return Ok(candidates.into_iter().next());
        }
        let verus_path = root.join(format!("verification/verus/{raw_path}"));
        if verus_path.exists() {
            return Ok(Some(verus_path));
        }
        return Ok(candidates.into_iter().next());
    }
    let basename_matches: Vec<PathBuf> = index
        .iter()
        .filter(|path| path.file_name().is_some_and(|name| name == raw_path))
        .map(|path| root.join(path))
        .collect();
    if basename_matches.len() == 1 {
        return Ok(basename_matches.into_iter().next());
    }
    Ok(None)
}

fn has_explicit_prefix(raw_path: &str) -> bool {
    ["crates/", "verification/", "contracts/", "specs/"]
        .iter()
        .any(|prefix| raw_path.starts_with(prefix))
}

fn suffix_candidates<'a>(raw_path: &str, index: &'a [PathBuf]) -> Vec<&'a PathBuf> {
    let suffix = format!("/{raw_path}");
    index
        .iter()
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name == basename(raw_path))
                && path
                    .as_os_str()
                    .as_encoded_bytes()
                    .ends_with(suffix.as_bytes())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::suffix_candidates;
    use std::path::PathBuf;

    #[test]
    fn suffix_candidates_keep_input_traversal_order() {
        let index = vec![
            PathBuf::from("crates/z/src/main.rs"),
            PathBuf::from("crates/a/src/main.rs"),
        ];
        let actual: Vec<_> = suffix_candidates("src/main.rs", &index)
            .into_iter()
            .cloned()
            .collect();
        assert_eq!(actual, index);
    }

    #[test]
    fn suffix_candidates_require_original_literal_slash_boundary() {
        let index = vec![PathBuf::from("src/main.rs")];
        assert_eq!(
            suffix_candidates("src/main.rs", &index),
            Vec::<&PathBuf>::new()
        );
        assert_eq!(
            suffix_candidates("src//main.rs", &index),
            Vec::<&PathBuf>::new()
        );
        assert_eq!(
            suffix_candidates("src/./main.rs", &index),
            Vec::<&PathBuf>::new()
        );
    }
}

fn basename(raw_path: &str) -> &str {
    match raw_path.rsplit('/').next() {
        Some(name) => name,
        None => raw_path,
    }
}

fn resolve_registry(raw_path: &str, cargo_home: &Path) -> Result<Option<PathBuf>, String> {
    let relative = raw_path
        .strip_prefix("/.cargo/")
        .ok_or_else(|| format!("invalid cargo-registry path: {raw_path}"))?;
    let literal = cargo_home.join(relative);
    if literal.is_file() {
        return Ok(Some(literal));
    }
    let parts: Vec<&str> = relative.split('/').collect();
    if parts.len() < 5 || parts.first() != Some(&"registry") || parts.get(1) != Some(&"src") {
        return Err(format!("malformed cargo-registry path: {raw_path}"));
    }
    let crate_dir = parts
        .get(3)
        .ok_or_else(|| format!("malformed cargo-registry path: {raw_path}"))?;
    let source_parts = parts
        .get(4..)
        .ok_or_else(|| format!("malformed cargo-registry path: {raw_path}"))?;
    let source_root = cargo_home.join("registry/src");
    let entries = fs::read_dir(&source_root).map_err(|error| {
        format!(
            "cannot read registry source {}: {error}",
            source_root.display()
        )
    })?;
    let mut matches = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "cannot read registry source entry in {}: {error}",
                source_root.display()
            )
        })?;
        let mut candidate = entry.path().join(crate_dir);
        for part in source_parts {
            candidate.push(part);
        }
        if candidate.is_file() {
            matches.push(candidate);
        }
    }
    matches.sort();
    match matches.as_slice() {
        [only] => Ok(Some(only.clone())),
        [] => Ok(None),
        _ => Err(format!(
            "registry path ambiguous across {} index-hash dirs",
            matches.len()
        )),
    }
}
