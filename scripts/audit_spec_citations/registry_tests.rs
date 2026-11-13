use super::{crate_identity, registry_pin_error};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn fixture(label: &str, lock: Option<&str>) -> Result<(PathBuf, PathBuf, PathBuf), io::Error> {
    let base = std::env::current_dir()?
        .join("target/citation-registry-r112/luna-tests")
        .join(format!("{label}-{}", std::process::id()));
    let root = base.join("repo");
    let cargo_home = base.join("cargo");
    let source = cargo_home.join("registry/src/index-demo/forge-1.2.3/src/lib.rs");
    fs::create_dir_all(
        source
            .parent()
            .ok_or_else(|| io::Error::other("missing source parent"))?,
    )?;
    fs::write(&source, "pub fn literal_pin_canary() {}\n")?;
    if let Some(contents) = lock {
        fs::create_dir_all(&root)?;
        fs::write(root.join("Cargo.lock"), contents)?;
    }
    Ok((root, cargo_home, source))
}

fn pin(root: &Path, cargo_home: &Path, source: &Path) -> Option<String> {
    registry_pin_error(
        root,
        "index-demo/forge-1.2.3/src/lib.rs",
        source,
        cargo_home,
    )
}

#[test]
fn crate_name_and_prerelease_version_parse() {
    assert_eq!(
        crate_identity("forge-tools-1.2.3-beta.4+build"),
        Some(("forge-tools", "1.2.3-beta.4+build"))
    );
}

#[test]
fn malformed_crate_directory_is_rejected() {
    assert_eq!(crate_identity("forge-tools-1.2.x"), None);
    assert_eq!(crate_identity("-1.2.3"), None);
}

#[test]
fn literal_registry_pin_canary_passes() -> Result<(), io::Error> {
    let (root, cargo_home, source) = fixture(
        "literal-positive",
        Some("name = \"forge\"\nversion = \"1.2.3\"\n"),
    )?;
    assert_eq!(pin(&root, &cargo_home, &source), None);
    Ok(())
}

#[test]
fn literal_registry_pin_rejects_wrong_version() -> Result<(), io::Error> {
    let (root, cargo_home, source) = fixture(
        "literal-version",
        Some("name = \"forge\"\nversion = \"1.2.4\"\n"),
    )?;
    assert!(
        pin(&root, &cargo_home, &source)
            .is_some_and(|error| error.contains("Cargo.lock pins 1.2.4"))
    );
    Ok(())
}

#[test]
fn literal_registry_pin_rejects_missing_lock() -> Result<(), io::Error> {
    let (root, cargo_home, source) = fixture("literal-missing-lock", None)?;
    assert!(pin(&root, &cargo_home, &source).is_some_and(|error| error.contains("no Cargo.lock")));
    Ok(())
}

#[test]
fn literal_registry_pin_rejects_unpinned_crate() -> Result<(), io::Error> {
    let lock = "name = \"other\"\nversion = \"1.2.3\"\n";
    let (root, cargo_home, source) = fixture("literal-unpinned", Some(lock))?;
    assert!(
        pin(&root, &cargo_home, &source)
            .is_some_and(|error| error.contains("crate forge not pinned"))
    );
    Ok(())
}

#[test]
fn first_adjacent_lock_pin_wins() -> Result<(), io::Error> {
    let lock = "name = \"forge\"\nversion = \"1.2.4\"\n\nname = \"forge\"\nversion = \"1.2.3\"\n";
    let (root, cargo_home, source) = fixture("first-lock-pin", Some(lock))?;
    assert!(
        pin(&root, &cargo_home, &source)
            .is_some_and(|error| error.contains("Cargo.lock pins 1.2.4"))
    );
    Ok(())
}

#[test]
fn version_line_comment_preserves_registry_pin() -> Result<(), io::Error> {
    let lock = "name = \"forge\"\nversion = \"1.2.3\" # preserved lock comment\n";
    let (root, cargo_home, source) = fixture("version-comment", Some(lock))?;
    assert_eq!(pin(&root, &cargo_home, &source), None);
    Ok(())
}

#[test]
fn lock_name_pattern_keeps_unanchored_prefix_behavior() -> Result<(), io::Error> {
    let lock = "prefix name = \"forge\"\nversion = \"1.2.3\"\n";
    let (root, cargo_home, source) = fixture("name-prefix", Some(lock))?;
    assert_eq!(pin(&root, &cargo_home, &source), None);
    Ok(())
}

#[test]
fn lock_name_line_comment_does_not_match_adjacent_pin() -> Result<(), io::Error> {
    let lock = "name = \"forge\" # trailing name comment\nversion = \"1.2.3\"\n";
    let (root, cargo_home, source) = fixture("name-comment", Some(lock))?;
    assert!(pin(&root, &cargo_home, &source).is_some_and(|error| error.contains("not pinned")));
    Ok(())
}

#[test]
fn version_key_must_start_the_adjacent_line() -> Result<(), io::Error> {
    for (label, version_line) in [
        ("commented-version", "# version = \"1.2.3\""),
        ("prefixed-version", "prefix version = \"1.2.3\""),
        ("indented-version", " version = \"1.2.3\""),
        ("empty-version", "version = \"\" # version = \"1.2.3\""),
    ] {
        let lock = format!("name = \"forge\"\n{version_line}\n");
        let (root, cargo_home, source) = fixture(label, Some(&lock))?;
        assert_eq!(
            pin(&root, &cargo_home, &source),
            Some("crate forge not pinned in Cargo.lock (cited 1.2.3)".to_string()),
            "{label}"
        );
    }
    Ok(())
}

#[test]
fn missing_registry_index_reports_its_path_stage() -> Result<(), io::Error> {
    let lock = "name = \"forge\"\nversion = \"1.2.3\"\n";
    let (root, cargo_home, _) = fixture("missing-registry-index", Some(lock))?;
    let path = cargo_home.join("registry/src");
    assert_eq!(
        registry_pin_error(
            &root,
            "index-demo/forge-1.2.3/src/lib.rs",
            &path,
            &cargo_home
        ),
        Some("registry path missing index directory".to_string())
    );
    Ok(())
}

#[test]
fn outside_registry_path_reports_its_boundary() -> Result<(), io::Error> {
    let lock = "name = \"forge\"\nversion = \"1.2.3\"\n";
    let (root, cargo_home, _) = fixture("outside-registry", Some(lock))?;
    let path = root.join("outside/lib.rs");
    assert_eq!(
        registry_pin_error(
            &root,
            "index-demo/forge-1.2.3/src/lib.rs",
            &path,
            &cargo_home
        ),
        Some(format!(
            "registry path outside $CARGO_HOME registry: {}",
            path.display()
        ))
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn non_utf8_crate_directory_reports_its_encoding_stage() -> Result<(), io::Error> {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let lock = "name = \"forge\"\nversion = \"1.2.3\"\n";
    let (root, cargo_home, _) = fixture("non-utf8-registry", Some(lock))?;
    let mut path = cargo_home.join("registry/src/index-demo");
    path.push(OsString::from_vec(vec![0xff]));
    path.push("src/lib.rs");
    assert_eq!(
        registry_pin_error(
            &root,
            "index-demo/forge-1.2.3/src/lib.rs",
            &path,
            &cargo_home
        ),
        Some("non-UTF-8 registry crate directory".to_string())
    );
    Ok(())
}

#[test]
fn invalid_first_candidate_does_not_hide_later_pin() -> Result<(), io::Error> {
    for (label, first_version) in [
        ("skip-commented-candidate", "# version = \"1.2.3\""),
        ("skip-empty-candidate", "version = \"\""),
    ] {
        let lock = format!(
            "name = \"forge\"\n{first_version}\nname = \"forge\"\nversion = \"1.2.3\"\n"
        );
        let (root, cargo_home, source) = fixture(label, Some(&lock))?;
        assert_eq!(pin(&root, &cargo_home, &source), None, "{label}");
    }
    Ok(())
}
