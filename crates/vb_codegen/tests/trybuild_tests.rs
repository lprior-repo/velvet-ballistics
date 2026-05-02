//! Trybuild compile-fail tests for generated Rust workflows.

use std::path::PathBuf;

/// Returns the path to the compile-fail fixtures directory.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("compile-fail")
}

#[test]
fn trybuild_compile_fail_tests() {
    let t = trybuild::TestCases::new();
    let fixtures = fixtures_dir();

    // Check that compile-fail fixtures exist and are loadable
    let fixture_files: Vec<_> = std::fs::read_dir(&fixtures)
        .expect("compile-fail fixtures directory must exist")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "rs"))
        .map(|entry| entry.path())
        .collect();

    if fixture_files.is_empty() {
        // No compile-fail fixtures yet; emit a clear message and pass
        eprintln!(
            "NOTE: No compile-fail fixtures found in {}",
            fixtures.display()
        );
        eprintln!("      This is expected until unsupported-primitive fixtures are added.");
        return;
    }

    for fixture in fixture_files {
        t.compile_fail(&fixture);
    }
}

#[test]
fn trybuild_pass_tests() {
    let t = trybuild::TestCases::new();
    let fixtures = fixtures_dir().join("pass");

    if !fixtures.exists() {
        eprintln!(
            "NOTE: No pass fixtures directory found at {}",
            fixtures.display()
        );
        return;
    }

    let fixture_files: Vec<_> = std::fs::read_dir(&fixtures)
        .expect("pass fixtures directory must exist")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "rs"))
        .map(|entry| entry.path())
        .collect();

    if fixture_files.is_empty() {
        eprintln!("NOTE: No pass fixtures found in {}", fixtures.display());
        return;
    }

    for fixture in fixture_files {
        t.pass(&fixture);
    }
}
