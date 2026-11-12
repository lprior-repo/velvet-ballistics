use super::*;

fn members() -> Vec<String> {
    EXPECTED
        .iter()
        .map(|(_, path)| (*path).to_owned())
        .collect()
}

fn packages() -> Vec<(String, String)> {
    EXPECTED
        .iter()
        .map(|(name, path)| (name.to_string(), format!("{path}/Cargo.toml")))
        .collect()
}

#[test]
fn exact_literal_members_and_packages_are_accepted() {
    assert_eq!(validate_members(&members()), Ok(()));
    assert_eq!(validate_order(&members()), Ok(()));
    assert_eq!(validate_packages(&packages()), Ok(()));
}

#[test]
fn missing_trailing_reference_is_rejected_by_set_and_order() {
    let actual = members().into_iter().take(11).collect::<Vec<_>>();
    assert_eq!(
        validate_members(&actual),
        Err(Mismatch::Count {
            expected: 12,
            actual: 11
        })
    );
    assert_eq!(
        validate_order(&actual),
        Err(Mismatch::Count {
            expected: 12,
            actual: 11
        })
    );
}

#[test]
fn extra_xtask_is_rejected_by_set_and_order() {
    let actual = members()
        .into_iter()
        .chain(std::iter::once("xtask".into()))
        .collect::<Vec<_>>();
    assert_eq!(
        validate_members(&actual),
        Err(Mismatch::Count {
            expected: 12,
            actual: 13
        })
    );
    assert_eq!(
        validate_order(&actual),
        Err(Mismatch::Count {
            expected: 12,
            actual: 13
        })
    );
}

#[test]
fn wrong_order_is_rejected_at_exact_position() {
    let actual = ["crates/vb_yaml".into(), "crates/vb_core".into()]
        .into_iter()
        .chain(members().into_iter().skip(2))
        .collect::<Vec<_>>();
    assert_eq!(
        validate_order(&actual),
        Err(Mismatch::Order {
            index: 0,
            expected: "crates/vb_core",
            actual: "crates/vb_yaml".into(),
        })
    );
}

#[test]
fn same_length_duplicate_member_is_rejected() {
    let actual = members()
        .into_iter()
        .take(11)
        .chain(std::iter::once("fuzz".into()))
        .collect::<Vec<_>>();
    assert_eq!(validate_members(&actual), Err(Mismatch::DuplicateMember));
}

#[test]
fn substituted_member_reports_exact_missing_and_extra() {
    let actual = members()
        .into_iter()
        .take(11)
        .chain(std::iter::once("xtask".into()))
        .collect::<Vec<_>>();
    assert_eq!(
        validate_members(&actual),
        Err(Mismatch::Members {
            missing: vec!["reference".into()],
            extra: vec!["xtask".into()],
        })
    );
}

#[test]
fn duplicate_package_name_is_rejected() {
    let actual = packages()
        .into_iter()
        .map(|(name, path)| {
            (
                if name == "reference" {
                    "vb_core".into()
                } else {
                    name
                },
                path,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        validate_packages(&actual),
        Err(Mismatch::DuplicatePackageName)
    );
}

#[test]
fn duplicate_manifest_path_is_rejected() {
    let actual = packages()
        .into_iter()
        .map(|(name, path)| {
            let path = if name == "reference" {
                "crates/vb_core/Cargo.toml".into()
            } else {
                path
            };
            (name, path)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        validate_packages(&actual),
        Err(Mismatch::DuplicateManifestPath)
    );
}

#[test]
fn wrong_package_path_is_rejected_exactly() {
    let actual = packages()
        .into_iter()
        .map(|(name, path)| {
            let path = if name == "reference" {
                "reference/Other.toml".into()
            } else {
                path
            };
            (name, path)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        validate_packages(&actual),
        Err(Mismatch::PackagePath {
            name: "reference".into(),
            expected: "reference/Cargo.toml".into(),
            actual: "reference/Other.toml".into(),
        })
    );
}

#[test]
fn wrong_package_name_is_rejected_exactly() {
    let actual = packages()
        .into_iter()
        .map(|(name, path)| {
            (
                if name == "reference" {
                    "xtask".into()
                } else {
                    name
                },
                path,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        validate_packages(&actual),
        Err(Mismatch::PackageName("xtask".into()))
    );
}

#[test]
fn wrong_final_member_is_not_ignored_by_order_validation() {
    let actual = members()
        .into_iter()
        .take(11)
        .chain(std::iter::once("xtask".into()))
        .collect::<Vec<_>>();
    assert_eq!(
        validate_order(&actual),
        Err(Mismatch::Order {
            index: 11,
            expected: "reference",
            actual: "xtask".into(),
        })
    );
}

#[test]
fn missing_metadata_package_is_rejected() {
    let actual = packages().into_iter().take(11).collect::<Vec<_>>();
    assert_eq!(
        validate_packages(&actual),
        Err(Mismatch::Count {
            expected: 12,
            actual: 11,
        })
    );
}

#[test]
fn extra_metadata_package_is_rejected() {
    let actual = packages()
        .into_iter()
        .chain(std::iter::once(("xtask".into(), "xtask/Cargo.toml".into())))
        .collect::<Vec<_>>();
    assert_eq!(
        validate_packages(&actual),
        Err(Mismatch::Count {
            expected: 12,
            actual: 13,
        })
    );
}

#[test]
fn correct_manifest_filename_under_wrong_directory_is_rejected() {
    let actual = packages()
        .into_iter()
        .map(|(name, path)| {
            let path = if name == "reference" {
                "other_reference/Cargo.toml".into()
            } else {
                path
            };
            (name, path)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        validate_packages(&actual),
        Err(Mismatch::PackagePath {
            name: "reference".into(),
            expected: "reference/Cargo.toml".into(),
            actual: "other_reference/Cargo.toml".into(),
        })
    );
}
