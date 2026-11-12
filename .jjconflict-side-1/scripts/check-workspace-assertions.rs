use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

// Authoritative master §34: exact 11 members
const EXPECTED_MEMBERS: &[&str] = &[
    "crates/vb_core",
    "crates/vb_yaml",
    "crates/vb_validate",
    "crates/vb_expr",
    "crates/vb_compile",
    "crates/vb_storage",
    "crates/vb_runtime",
    "crates/vb_ipc",
    "crates/velvet_ballistics",
    "crates/workspace_tests",
    "fuzz",
];

// Excluded: xtask and target/miri-tmp
const EXPECTED_EXCLUDES: &[&str] = &["xtask", "target/miri-tmp"];

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

// Product package: velvet-ballistics (binary + lib)
const EXPECTED_PACKAGE_NAMES: &[(&str, &str)] = &[
    ("crates/vb_core", "vb_core"),
    ("crates/vb_yaml", "vb_yaml"),
    ("crates/vb_validate", "vb_validate"),
    ("crates/vb_expr", "vb_expr"),
    ("crates/vb_compile", "vb_compile"),
    ("crates/vb_storage", "vb_storage"),
    ("crates/vb_runtime", "vb_runtime"),
    ("crates/vb_ipc", "vb_ipc"),
    ("crates/velvet_ballistics", "velvet-ballistics"),
    (
        "crates/workspace_tests",
        "velvet-ballistics-workspace-tests",
    ),
];

const EXPECTED_FEATURES: &[(&str, &[&str])] = &[
    (
        "crates/vb_core",
        &[
            "bench",
            "default",
            "kani-diagnostic-codes",
            "test-util",
            "verus-kernels",
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

mod check_workspace_assertions_driver;

fn main() {
    std::process::exit(check_workspace_assertions_driver::run());
}
