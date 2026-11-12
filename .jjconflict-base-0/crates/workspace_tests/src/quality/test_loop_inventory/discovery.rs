use std::fs;
use std::path::Path;

use super::*;

pub fn discover_rust_test_files(
    root: WorkspaceRoot,
    scope: InventoryScope,
) -> Result<Vec<TestFile>, InventoryError> {
    match scope {
        InventoryScope::FirstPartyRustTests => discover_first_party(root.path.as_str()),
        InventoryScope::Roots(roots) => reject_or_discover_roots(root.path.as_str(), roots),
    }
}

fn reject_or_discover_roots(
    root: &str,
    roots: Vec<String>,
) -> Result<Vec<TestFile>, InventoryError> {
    for path in &roots {
        if !is_allowed_root(path) {
            return Err(InventoryError::InputRootOutOfScope { path: path.clone() });
        }
    }
    let root_path = Path::new(root);
    if !root_path.exists() {
        return Err(InventoryError::WorkspaceUnreadable {
            root: root.to_owned(),
        });
    }

    let mut files = Vec::new();
    for path in roots {
        collect_rust_tests(root_path, &root_path.join(&path), &mut files)?;
    }
    files.sort();
    Ok(files)
}

fn is_allowed_root(path: &str) -> bool {
    (path == "tests"
        || path == "crates"
        || path.starts_with("tests/")
        || path.starts_with("crates/"))
        && !path.contains("..")
        && !Path::new(path).is_absolute()
        && !has_excluded_component(path)
}

fn has_excluded_component(path: &str) -> bool {
    path.split('/')
        .any(|part| matches!(part, "vendor" | "target" | "generated" | "external"))
}

fn discover_first_party(root: &str) -> Result<Vec<TestFile>, InventoryError> {
    let root_path = Path::new(root);
    if !root_path.exists() {
        return Err(InventoryError::WorkspaceUnreadable {
            root: root.to_owned(),
        });
    }
    let mut files = Vec::new();
    collect_rust_tests(root_path, root_path, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_rust_tests(
    root: &Path,
    dir: &Path,
    out: &mut Vec<TestFile>,
) -> Result<(), InventoryError> {
    let entries = fs::read_dir(dir).map_err(|_error| InventoryError::WorkspaceUnreadable {
        root: root.display().to_string(),
    })?;
    for entry_result in entries {
        let entry = entry_result.map_err(|_error| InventoryError::WorkspaceUnreadable {
            root: root.display().to_string(),
        })?;
        collect_entry(root, &entry.path(), out)?;
    }
    Ok(())
}

fn collect_entry(root: &Path, path: &Path, out: &mut Vec<TestFile>) -> Result<(), InventoryError> {
    if path.is_dir() && !is_excluded_dir(path) {
        collect_rust_tests(root, path, out)?;
    } else if is_first_party_test_rs(root, path) {
        out.push(TestFile {
            path: DomainPath(relative_string(root, path)?),
        });
    }
    Ok(())
}

fn is_excluded_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("target" | "vendor" | "generated" | "external")
    )
}

fn is_first_party_test_rs(root: &Path, path: &Path) -> bool {
    let Some(rel) = path
        .strip_prefix(root)
        .ok()
        .and_then(|value| value.to_str())
    else {
        return false;
    };
    let is_rust = rel.ends_with(".rs");
    let in_tests = rel.starts_with("tests/");
    let in_crate_tests = rel.starts_with("crates/") && rel.contains("/tests/");
    is_rust && (in_tests || in_crate_tests)
}

fn relative_string(root: &Path, path: &Path) -> Result<String, InventoryError> {
    path.strip_prefix(root)
        .ok()
        .and_then(|rel| rel.to_str())
        .map(str::to_owned)
        .ok_or_else(|| InventoryError::WorkspaceUnreadable {
            root: root.display().to_string(),
        })
}
