use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const EXPECTED_MEMBERS: &[&str] = &[
    "crates/vb_boundary_inventory",
    "crates/vb_core",
    "crates/vb_yaml",
    "crates/vb_validate",
    "crates/vb_expr",
    "crates/vb_compile",
    "crates/vb_storage",
    "crates/vb_runtime",
    "crates/vb_doc",
    "crates/vb_ipc",
    "crates/vb_proof_kernels",
    "crates/vb_queue_semantics",
    "crates/vb_cli",
    "crates/vb_verification",
    "crates/vb_test_util",
    "crates/workspace_tests/idempotency_suite",
    "crates/workspace_tests",
    "crates/vb_benchmark",
];

const EXPECTED_EXCLUDES: &[&str] = &["target/miri-tmp", "crates/vb_ui", "fuzz", "crates/vb_ajc40_flux"];
const BOUNDARY_CRATES: &[&str] = &["vb_core", "vb_runtime", "vb_storage", "vb_ipc"];
const FORBIDDEN_UI_DEPENDENCIES: &[&str] = &[
    "vb_ui",
    "vb_ui_makepad",
    "vb_ui_model",
    "vb_ui_snapshot",
    "makepad-widgets",
    "makepad-draw",
];
const FORBIDDEN_RUNTIME_FORMAT_DEPENDENCIES: &[&str] =
    &["serde_json", "saphyr", "saphyr-parser", "serde-saphyr"];
const FORBIDDEN_FEATURE_NAMES: &[&str] = &[
    "json",
    "serde-json",
    "generated",
    "maxperf",
    "velvet-ballistics",
    "velvet_ballistics",
];

const EXPECTED_PACKAGE_NAMES: &[(&str, &str)] = &[
    ("crates/vb_boundary_inventory", "vb_boundary_inventory"),
    ("crates/vb_core", "vb_core"),
    ("crates/vb_yaml", "vb_yaml"),
    ("crates/vb_validate", "vb_validate"),
    ("crates/vb_expr", "vb_expr"),
    ("crates/vb_compile", "vb_compile"),
    ("crates/vb_storage", "vb_storage"),
    ("crates/vb_runtime", "vb_runtime"),
    ("crates/vb_doc", "vb_doc"),
    ("crates/vb_ipc", "vb_ipc"),
    ("crates/vb_proof_kernels", "vb_proof_kernels"),
    ("crates/vb_queue_semantics", "vb_queue_semantics"),
    ("crates/vb_cli", "velvet-ballistics"),
    ("crates/vb_verification", "vb_verification"),
    ("crates/vb_test_util", "vb_test_util"),
    (
        "crates/workspace_tests/idempotency_suite",
        "velvet-ballistics-idempotency-workspace-tests",
    ),
    (
        "crates/workspace_tests",
        "velvet-ballistics-workspace-tests",
    ),
    ("crates/vb_benchmark", "vb_benchmark"),
];

const EXPECTED_FEATURES: &[(&str, &[&str])] = &[
    (
        "crates/vb_core",
        &[
            "bench",
            "default",
            "kani-diagnostic-codes",
            "kani-vb-5iebh-check-scope",
            "kani-vb-ajc40",
            "test-util",
            "volatile",
        ],
    ),
    ("crates/vb_validate", &["default", "verus"]),
];

fn quoted_values_in_line(line: &str) -> Vec<String> {
    line.split('"')
        .enumerate()
        .filter_map(|(index, value)| (index % 2 == 1).then(|| value.to_owned()))
        .collect()
}

fn quoted_array_values(text: &str, key: &str) -> BTreeSet<String> {
    let prefix = format!("{key} = [");
    let mut active = false;
    let mut values = BTreeSet::new();

    text.lines().for_each(|line| {
        let trimmed = line.trim();
        let semantic = trimmed
            .split_once('#')
            .map_or(trimmed, |(before_comment, _comment)| before_comment.trim());
        if semantic.is_empty() {
            return;
        }
        if !active && semantic.starts_with(&prefix) {
            active = true;
        }
        if active {
            quoted_values_in_line(semantic)
                .into_iter()
                .for_each(|value| {
                    values.insert(value);
                });
            if semantic.contains(']') {
                active = false;
            }
        }
    });

    values
}

fn quoted_scalar(line: &str) -> Option<String> {
    quoted_values_in_line(line).into_iter().next()
}

fn package_name(manifest: &str) -> Option<String> {
    let mut in_package = false;
    manifest.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_package = trimmed == "[package]";
            return None;
        }
        (in_package && trimmed.starts_with("name ="))
            .then(|| quoted_scalar(trimmed))
            .flatten()
    })
}

fn binary_names(manifest: &str) -> BTreeSet<String> {
    let mut in_bin = false;
    let mut names = BTreeSet::new();
    manifest.lines().for_each(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_bin = trimmed == "[[bin]]";
            return;
        }
        if in_bin && trimmed.starts_with("name =") {
            if let Some(name) = quoted_scalar(trimmed) {
                names.insert(name);
            }
        }
    });
    names
}

fn feature_names(manifest: &str) -> BTreeSet<String> {
    let mut in_features = false;
    let mut names = BTreeSet::new();
    manifest.lines().for_each(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_features = trimmed == "[features]";
            return;
        }
        if in_features && trimmed.contains('=') {
            if let Some((name, _rest)) = trimmed.split_once('=') {
                let cleaned = name.trim();
                if !cleaned.is_empty() {
                    names.insert(cleaned.to_owned());
                }
            }
        }
    });
    names
}

fn dependency_names(manifest: &str) -> BTreeSet<String> {
    let mut in_dependencies = false;
    let mut names = BTreeSet::new();
    manifest.lines().for_each(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_dependencies = matches!(
                trimmed,
                "[dependencies]" | "[dev-dependencies]" | "[build-dependencies]"
            );
            return;
        }
        if !in_dependencies || !trimmed.contains('=') || trimmed.starts_with('#') {
            return;
        }
        if let Some((name, rest)) = trimmed.split_once('=') {
            let dep_name = name.trim();
            if !dep_name.is_empty() {
                names.insert(dep_name.to_owned());
            }
            if rest.contains("package") {
                quoted_values_in_line(rest)
                    .first()
                    .cloned()
                    .into_iter()
                    .for_each(|value| {
                        names.insert(value);
                    });
            }
            if rest.contains("path") {
                quoted_values_in_line(rest)
                    .into_iter()
                    .last()
                    .into_iter()
                    .for_each(|value| {
                        if let Some(alias) = value.trim_end_matches('/').rsplit('/').next() {
                            if !alias.is_empty() {
                                names.insert(alias.to_owned());
                            }
                        }
                    });
            }
        }
    });
    names
}

fn expected_set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn sorted_difference(left: &BTreeSet<String>, right: &BTreeSet<String>) -> Vec<String> {
    left.difference(right).cloned().collect()
}

fn push_set_failure(
    failures: &mut Vec<String>,
    label: &str,
    missing: Vec<String>,
    extra: Vec<String>,
) {
    if !missing.is_empty() {
        failures.push(format!("{label} missing {missing:?}"));
    }
    if !extra.is_empty() {
        failures.push(format!("{label} unexpected {extra:?}"));
    }
}

fn read_manifest(root: &Path, member_path: &str, failures: &mut Vec<String>) -> Option<String> {
    let manifest_path = root.join(member_path).join("Cargo.toml");
    match fs::read_to_string(&manifest_path) {
        Ok(text) => Some(text),
        Err(error) => {
            failures.push(format!(
                "{}/Cargo.toml: unreadable manifest: {error}",
                member_path
            ));
            None
        }
    }
}

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
            if *member_path == "crates/vb_cli" {
                let actual_binaries = binary_names(&manifest);
                let expected_binaries = expected_set(&["velvet-ballistics"]);
                push_set_failure(
                    failures,
                    "crates/vb_cli/Cargo.toml: bin names",
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

fn run() -> i32 {
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
    if failures.is_empty() { 0 } else { 1 }
}

fn main() {
    std::process::exit(run());
}
