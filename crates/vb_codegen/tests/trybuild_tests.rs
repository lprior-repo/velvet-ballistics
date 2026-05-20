#![cfg(not(miri))]
//! Trybuild compile-fail tests for generated Rust workflows.

use std::path::PathBuf;

/// Returns the path to the compile-fail fixtures directory.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("compile-fail")
}

fn compile_fail_fixture_files(fixtures: &std::path::Path) -> Result<Vec<PathBuf>, String> {
    std::fs::read_dir(fixtures)
        .map_err(|e| e.to_string())?
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "rs"))
        .map(|entry| Ok(entry.path()))
        .collect()
}

fn require_non_empty_compile_fail_fixtures(
    fixtures: &std::path::Path,
) -> Result<Vec<PathBuf>, String> {
    let fixture_files = compile_fail_fixture_files(fixtures)?;
    if fixture_files.is_empty() {
        return Err(format!(
            "compile-fail fixture directory is empty: {}",
            fixtures.display()
        ));
    }
    Ok(fixture_files)
}

#[test]
fn trybuild_compile_fail_tests() -> Result<(), String> {
    let t = trybuild::TestCases::new();
    let fixtures = fixtures_dir();

    let fixture_files = require_non_empty_compile_fail_fixtures(&fixtures)?;

    for fixture in fixture_files {
        t.compile_fail(&fixture);
    }
    Ok(())
}

#[test]
fn trybuild_compile_fail_tests_fails_when_compile_fail_fixture_dir_is_empty() -> Result<(), String>
{
    let temp_dir = tempfile::Builder::new()
        .prefix("vb_codegen_empty_compile_fail_")
        .tempdir_in(trybuild_test_temp_root()?)
        .map_err(|e| e.to_string())?;
    let error = require_non_empty_compile_fail_fixtures(temp_dir.path())
        .err()
        .ok_or("empty compile-fail fixture directory unexpectedly succeeded")?;
    assert_eq!(
        error,
        format!(
            "compile-fail fixture directory is empty: {}",
            temp_dir.path().display()
        )
    );
    Ok(())
}

fn trybuild_test_temp_root() -> Result<std::path::PathBuf, String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/vb-codegen-trybuild-tmp");
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    Ok(root)
}

#[test]
fn trybuild_pass_tests() -> Result<(), String> {
    let t = trybuild::TestCases::new();
    let fixtures = fixtures_dir().join("pass");

    if !fixtures.exists() {
        eprintln!(
            "NOTE: No pass fixtures directory found at {}",
            fixtures.display()
        );
        return Ok(());
    }

    let fixture_files: Vec<_> = std::fs::read_dir(&fixtures)
        .map_err(|e| e.to_string())?
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "rs"))
        .map(|entry| entry.path())
        .collect();

    if fixture_files.is_empty() {
        eprintln!("NOTE: No pass fixtures found in {}", fixtures.display());
        return Ok(());
    }

    for fixture in fixture_files {
        t.pass(&fixture);
    }
    Ok(())
}
