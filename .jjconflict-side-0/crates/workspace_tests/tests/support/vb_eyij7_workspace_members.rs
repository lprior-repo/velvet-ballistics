use std::collections::BTreeSet;

pub(super) const EXPECTED: [(&str, &str); 12] = [
    ("vb_core", "crates/vb_core"),
    ("vb_yaml", "crates/vb_yaml"),
    ("vb_validate", "crates/vb_validate"),
    ("vb_expr", "crates/vb_expr"),
    ("vb_compile", "crates/vb_compile"),
    ("vb_storage", "crates/vb_storage"),
    ("vb_runtime", "crates/vb_runtime"),
    ("vb_ipc", "crates/vb_ipc"),
    ("velvet-ballistics", "crates/velvet_ballistics"),
    (
        "velvet-ballistics-workspace-tests",
        "crates/workspace_tests",
    ),
    ("velvet-ballistics-fuzz", "fuzz"),
    ("reference", "reference"),
];

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Mismatch {
    Count {
        expected: usize,
        actual: usize,
    },
    DuplicateMember,
    Members {
        missing: Vec<String>,
        extra: Vec<String>,
    },
    Order {
        index: usize,
        expected: &'static str,
        actual: String,
    },
    DuplicatePackageName,
    DuplicateManifestPath,
    PackageName(String),
    PackagePath {
        name: String,
        expected: String,
        actual: String,
    },
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for Mismatch {}

fn validate_count(actual: usize) -> Result<(), Mismatch> {
    if actual != EXPECTED.len() {
        return Err(Mismatch::Count {
            expected: EXPECTED.len(),
            actual,
        });
    }
    Ok(())
}

pub(super) fn validate_members(actual: &[String]) -> Result<(), Mismatch> {
    validate_count(actual.len())?;
    let members: BTreeSet<&str> = actual.iter().map(String::as_str).collect();
    if members.len() != actual.len() {
        return Err(Mismatch::DuplicateMember);
    }
    let expected: BTreeSet<&str> = EXPECTED.iter().map(|(_, path)| *path).collect();
    if members != expected {
        return Err(Mismatch::Members {
            missing: expected
                .difference(&members)
                .map(|v| (*v).to_owned())
                .collect(),
            extra: members
                .difference(&expected)
                .map(|v| (*v).to_owned())
                .collect(),
        });
    }
    Ok(())
}

pub(super) fn validate_order(actual: &[String]) -> Result<(), Mismatch> {
    validate_count(actual.len())?;
    actual
        .iter()
        .zip(EXPECTED)
        .enumerate()
        .try_for_each(|(index, (actual, (_, expected)))| {
            if actual != expected {
                return Err(Mismatch::Order {
                    index,
                    expected,
                    actual: actual.clone(),
                });
            }
            Ok(())
        })
}

pub(super) fn validate_packages(actual: &[(String, String)]) -> Result<(), Mismatch> {
    validate_count(actual.len())?;
    let names: BTreeSet<&str> = actual.iter().map(|(name, _)| name.as_str()).collect();
    if names.len() != actual.len() {
        return Err(Mismatch::DuplicatePackageName);
    }
    let paths: BTreeSet<&str> = actual.iter().map(|(_, path)| path.as_str()).collect();
    if paths.len() != actual.len() {
        return Err(Mismatch::DuplicateManifestPath);
    }
    actual
        .iter()
        .try_for_each(|(name, path)| validate_package(name, path))
}

fn validate_package(name: &str, path: &str) -> Result<(), Mismatch> {
    let (_, directory) = EXPECTED
        .iter()
        .find(|(expected, _)| *expected == name)
        .ok_or_else(|| Mismatch::PackageName(name.to_owned()))?;
    let expected = format!("{directory}/Cargo.toml");
    if path != expected {
        return Err(Mismatch::PackagePath {
            name: name.to_owned(),
            expected,
            actual: path.to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "vb_eyij7_workspace_controls.rs"]
mod controls;
