use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{
    BOUNDARY_CRATES, EXPECTED_EXCLUDES, EXPECTED_FEATURES, EXPECTED_MEMBERS,
    EXPECTED_PACKAGE_NAMES, FORBIDDEN_FEATURE_NAMES, FORBIDDEN_RUNTIME_FORMAT_DEPENDENCIES,
    FORBIDDEN_UI_DEPENDENCIES, binary_names, dependency_names, expected_set, feature_names,
    package_name, push_set_failure, quoted_array_values, read_manifest, sorted_difference,
};

fn check_workspace_members(root: &Path, failures: &mut Vec<String>) {
    let cargo_path = root.join("Cargo.toml");
    let manifest = match fs::read_to_string(&cargo_path) {
        Ok(text) => text,
        Err(error) => {
            failures.push(format!("Cargo.toml: unreadable: {error}"));
            return;
        }
    };
    let actual_members = quoted_array_values(&manifest, "members");
    let expected_members = expected_set(EXPECTED_MEMBERS);
    push_set_failure(
        failures,
        "Cargo.toml: workspace.members",
        sorted_difference(&expected_members, &actual_members),
        sorted_difference(&actual_members, &expected_members),
    );

    let actual_excludes = quoted_array_values(&manifest, "exclude");
    let expected_excludes = expected_set(EXPECTED_EXCLUDES);
    push_set_failure(
        failures,
        "Cargo.toml: workspace.exclude",
        sorted_difference(&expected_excludes, &actual_excludes),
        sorted_difference(&actual_excludes, &expected_excludes),
    );
}

fn check_crate_names(root: &Path, failures: &mut Vec<String>) {
    EXPECTED_PACKAGE_NAMES.iter().for_each(|(member_path, expected_name)| {
        if let Some(manifest) = read_manifest(root, member_path, failures) {
            let actual_name = package_name(&manifest);
            if actual_name.as_deref() != Some(*expected_name) {
                failures.push(format!(
                    "{member_path}/Cargo.toml: package.name expected {expected_name:?}, got {actual_name:?}"
                ));
            }
            // Product binary name check for velvet_ballistics crate
            if *member_path == "crates/velvet_ballistics" {
                let actual_binaries = binary_names(&manifest);
                let expected_binaries = expected_set(&["velvet-ballistics"]);
                push_set_failure(
                    failures,
                    "crates/velvet_ballistics/Cargo.toml: bin names",
                    sorted_difference(&expected_binaries, &actual_binaries),
                    sorted_difference(&actual_binaries, &expected_binaries),
                );
            }
            EXPECTED_FEATURES.iter().for_each(|(feature_path, expected)| {
                if member_path == feature_path {
                    let actual_features = feature_names(&manifest);
                    let expected_features = expected_set(expected);
                    push_set_failure(
                        failures,
                        &format!("{member_path}/Cargo.toml: features"),
                        sorted_difference(&expected_features, &actual_features),
                        sorted_difference(&actual_features, &expected_features),
                    );
                }
            });
            let forbidden_features = feature_names(&manifest)
                .intersection(&expected_set(FORBIDDEN_FEATURE_NAMES))
                .cloned()
                .collect::<Vec<_>>();
            if !forbidden_features.is_empty() {
                failures.push(format!(
                    "{member_path}/Cargo.toml: forbidden feature names {forbidden_features:?}"
                ));
            }
        }
    });
}

fn check_forbidden_dependencies(root: &Path, failures: &mut Vec<String>) {
    BOUNDARY_CRATES.iter().for_each(|crate_name| {
        let member_path = format!("crates/{crate_name}");
        if let Some(manifest) = read_manifest(root, &member_path, failures) {
            let names = dependency_names(&manifest);
            let ui_hits = names
                .intersection(&expected_set(FORBIDDEN_UI_DEPENDENCIES))
                .cloned()
                .collect::<Vec<_>>();
            if !ui_hits.is_empty() {
                failures.push(format!(
                    "{member_path}/Cargo.toml: forbidden UI dependency in boundary crate {crate_name}: {ui_hits:?}"
                ));
            }
            let format_hits = names
                .intersection(&expected_set(FORBIDDEN_RUNTIME_FORMAT_DEPENDENCIES))
                .cloned()
                .collect::<Vec<_>>();
            if !format_hits.is_empty() {
                failures.push(format!(
                    "{member_path}/Cargo.toml: forbidden runtime format dependency in {crate_name}: {format_hits:?}"
                ));
            }
        }
    });
}

fn collect_generated_dirs(root: &Path) -> io::Result<Vec<PathBuf>> {
    let crates_dir = root.join("crates");
    if !crates_dir.exists() {
        return Ok(Vec::new());
    }
    fs::read_dir(crates_dir)?.try_fold(Vec::new(), |mut acc, entry| {
        let path = entry?.path().join("src").join("generated");
        if path.exists() {
            acc.push(path);
        }
        Ok(acc)
    })
}

fn rust_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    fs::read_dir(root)?.try_fold(Vec::new(), |mut acc, entry| {
        let path = entry?.path();
        if path.is_dir() {
            acc.extend(rust_files(&path)?);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            acc.push(path);
        }
        Ok(acc)
    })
}

fn check_generated_boundaries(root: &Path, failures: &mut Vec<String>) {
    match collect_generated_dirs(root) {
        Ok(dirs) => dirs.into_iter().for_each(|dir| match rust_files(&dir) {
            Ok(files) => files
                .into_iter()
                .for_each(|source| match fs::read_to_string(&source) {
                    Ok(text) => FORBIDDEN_UI_DEPENDENCIES
                        .iter()
                        .chain(FORBIDDEN_RUNTIME_FORMAT_DEPENDENCIES.iter())
                        .for_each(|forbidden| {
                            if text.contains(forbidden) {
                                let rel = source
                                    .strip_prefix(root)
                                    .map_or(source.as_path(), |path| path);
                                failures.push(format!(
                                    "{}: forbidden generated boundary token {forbidden}",
                                    rel.display()
                                ));
                            }
                        }),
                    Err(error) => {
                        failures.push(format!("{}: unreadable: {error}", source.display()))
                    }
                }),
            Err(error) => failures.push(format!(
                "{}: unreadable generated dir: {error}",
                dir.display()
            )),
        }),
        Err(error) => failures.push(format!("crates: unreadable: {error}")),
    }
}

pub(crate) fn run() -> i32 {
    let root = match std::env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("InvalidInvocation: cannot read current directory: {error}");
            return 64;
        }
    };
    let mut failures = Vec::new();
    check_workspace_members(&root, &mut failures);
    check_crate_names(&root, &mut failures);
    check_forbidden_dependencies(&root, &mut failures);
    check_generated_boundaries(&root, &mut failures);

    failures.iter().for_each(|failure| eprintln!("{failure}"));
    if failures.is_empty() {
        0
    } else {
        1
    }
}
