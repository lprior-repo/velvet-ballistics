//! Phase 0 Scaffold Tests — vb-blq
//!
//! These tests verify the existence and validity of project infrastructure
//! scaffolding files. Phase 0 deliverables do NOT exist yet, so ALL tests FAIL.
//!
//! Once the scaffold is created, these tests will PASS.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Workspace root is the crate root for these integration tests.
const WORKSPACE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

// =============================================================================
// Section 1: Config File Existence Tests
// =============================================================================

#[test]
fn deny_toml_exists() {
    let path = Path::new(WORKSPACE_ROOT).join("deny.toml");
    assert!(
        path.exists(),
        "deny.toml must exist at workspace root: {}",
        path.display()
    );
}

#[test]
fn cargo_vet_toml_exists() {
    let path = Path::new(WORKSPACE_ROOT).join("cargo-vet.toml");
    assert!(
        path.exists(),
        "cargo-vet.toml must exist at workspace root: {}",
        path.display()
    );
}

#[test]
fn geigerignore_exists() {
    let path = Path::new(WORKSPACE_ROOT).join(".geigerignore");
    assert!(
        path.exists(),
        ".geigerignore must exist at workspace root: {}",
        path.display()
    );
}

// =============================================================================
// Section 2: Config File Parsing Tests
// =============================================================================

#[test]
fn deny_toml_parses_as_valid_toml() {
    use toml::Table;

    let path = Path::new(WORKSPACE_ROOT).join("deny.toml");
    let contents = fs::read_to_string(&path).expect("deny.toml must be readable");

    let parsed: Table = toml::from_str(&contents).expect("deny.toml must be valid TOML");

    // Verify required sections exist
    assert!(
        parsed.contains_key("advisories"),
        "deny.toml must have [advisories] section"
    );
    assert!(
        parsed.contains_key("bans"),
        "deny.toml must have [bans] section"
    );
}

#[test]
fn cargo_vet_toml_parses_as_valid_toml() {
    use toml::Table;

    let path = Path::new(WORKSPACE_ROOT).join("cargo-vet.toml");
    let contents = fs::read_to_string(&path).expect("cargo-vet.toml must be readable");

    let parsed: Table = toml::from_str(&contents).expect("cargo-vet.toml must be valid TOML");

    // Basic structure check - should have some content
    assert!(!parsed.is_empty(), "cargo-vet.toml must not be empty");
}

#[test]
fn geigerignore_has_content() {
    let path = Path::new(WORKSPACE_ROOT).join(".geigerignore");
    let contents = fs::read_to_string(&path).expect(".geigerignore must be readable");

    assert!(
        !contents.trim().is_empty(),
        ".geigerignore must not be empty"
    );

    // Should contain paths to third-party crate sources
    assert!(
        contents.contains("crates/") || contents.contains("target/"),
        ".geigerignore should contain paths to third-party sources"
    );
}

// =============================================================================
// Section 3: deny.toml Content Validation
// =============================================================================

#[test]
fn deny_toml_bans_required_licenses() {
    let path = Path::new(WORKSPACE_ROOT).join("deny.toml");
    let contents = fs::read_to_string(&path).expect("deny.toml must be readable");

    let banned_patterns = ["GPL", "LGPL", "AGPL", "SSPL", "Commons Clause"];

    for pattern in banned_patterns {
        assert!(
            contents.contains(pattern),
            "deny.toml must ban '{}' license pattern",
            pattern
        );
    }
}

#[test]
fn deny_toml_allows_required_licenses() {
    let path = Path::new(WORKSPACE_ROOT).join("deny.toml");
    let contents = fs::read_to_string(&path).expect("deny.toml must be readable");

    let allowed_patterns = [
        "MIT",
        "Apache-2.0",
        "BSD-2-Clause",
        "BSD-3-Clause",
        "ISC",
        "Zlib",
    ];

    for pattern in allowed_patterns {
        assert!(
            contents.contains(pattern),
            "deny.toml must allow '{}' license pattern",
            pattern
        );
    }
}

// =============================================================================
// Section 4: Benchmark Scaffold Tests
// =============================================================================

#[test]
fn benches_velvet_ballastics_rs_exists() {
    let path = Path::new(WORKSPACE_ROOT).join("benches/velvet_ballastics.rs");
    assert!(
        path.exists(),
        "benches/velvet_ballastics.rs must exist: {}",
        path.display()
    );
}

#[test]
fn benches_velvet_ballastics_has_27_bench_functions() {
    let path = Path::new(WORKSPACE_ROOT).join("benches/velvet_ballastics.rs");
    let contents =
        fs::read_to_string(&path).expect("benches/velvet_ballastics.rs must be readable");

    // Count #[bench] attributes
    let bench_count = contents.matches("#[bench]").count();

    assert_eq!(
        bench_count, 27,
        "benches/velvet_ballastics.rs must have exactly 27 #[bench] functions, found {}",
        bench_count
    );
}

#[test]
fn benches_velvet_ballastics_uses_black_box() {
    let path = Path::new(WORKSPACE_ROOT).join("benches/velvet_ballastics.rs");
    let contents =
        fs::read_to_string(&path).expect("benches/velvet_ballastics.rs must be readable");

    assert!(
        contents.contains("black_box"),
        "Benchmarks must use criterion::black_box for input parameters"
    );
}

#[test]
fn benches_velvet_ballastics_compiles() {
    let manifest_path = Path::new(WORKSPACE_ROOT).join("Cargo.toml");
    let status = Command::new("cargo")
        .args(["check", "--benches", "--manifest-path"])
        .arg(&manifest_path)
        .current_dir(WORKSPACE_ROOT)
        .status()
        .expect("cargo check --benches must execute");

    assert!(
        status.success(),
        "benches/velvet_ballastics.rs must compile with cargo check --benches"
    );
}

// =============================================================================
// Section 5: Fuzz Target Scaffold Tests
// =============================================================================

#[test]
fn fuzz_fuzz_targets_rs_exists() {
    let path = Path::new(WORKSPACE_ROOT).join("fuzz/fuzz_targets.rs");
    assert!(
        path.exists(),
        "fuzz/fuzz_targets.rs must exist: {}",
        path.display()
    );
}

#[test]
fn fuzz_fuzz_targets_has_5_no_mangle_extern_c_functions() {
    let path = Path::new(WORKSPACE_ROOT).join("fuzz/fuzz_targets.rs");
    let contents = fs::read_to_string(&path).expect("fuzz/fuzz_targets.rs must be readable");

    // Count #[no_mangle] extern "C" function definitions
    // We look for the pattern that indicates a libfuzzer entry point.
    // Count #[no_mangle] attribute lines — each marks a fuzzer entry point.
    let no_mangle_extern_count = contents
        .lines()
        .filter(|line| line.contains("#[no_mangle]"))
        .count();

    assert_eq!(
        no_mangle_extern_count, 5,
        "fuzz/fuzz_targets.rs must have exactly 5 #[no_mangle] extern \"C\" functions, found {}",
        no_mangle_extern_count
    );
}

#[test]
fn fuzz_fuzz_targets_contains_llvm_fuzzer_test_one_input() {
    let path = Path::new(WORKSPACE_ROOT).join("fuzz/fuzz_targets.rs");
    let contents = fs::read_to_string(&path).expect("fuzz/fuzz_targets.rs must be readable");

    // Should contain LLVMFuzzerTestOneInput for each of the 5 targets
    let fuzzer_count = contents.matches("LLVMFuzzerTestOneInput").count();

    assert_eq!(
        fuzzer_count, 5,
        "fuzz/fuzz_targets.rs must have 5 LLVMFuzzerTestOneInput functions, found {}",
        fuzzer_count
    );
}

// =============================================================================
// Section 6: Test Fixtures Tests
// =============================================================================

#[test]
fn tests_fixtures_directory_exists() {
    let path = Path::new(WORKSPACE_ROOT).join("tests/fixtures");
    assert!(
        path.exists() && path.is_dir(),
        "tests/fixtures/ directory must exist: {}",
        path.display()
    );
}

#[test]
fn tests_fixtures_valid_directory_exists() {
    let path = Path::new(WORKSPACE_ROOT).join("tests/fixtures/valid");
    assert!(
        path.exists() && path.is_dir(),
        "tests/fixtures/valid/ directory must exist: {}",
        path.display()
    );
}

#[test]
fn tests_fixtures_invalid_directory_exists() {
    let path = Path::new(WORKSPACE_ROOT).join("tests/fixtures/invalid");
    assert!(
        path.exists() && path.is_dir(),
        "tests/fixtures/invalid/ directory must exist: {}",
        path.display()
    );
}

#[test]
fn fixtures_valid_3step_choose_yaml_exists() {
    let path = Path::new(WORKSPACE_ROOT).join("tests/fixtures/valid/3step_choose.yaml");
    assert!(
        path.exists(),
        "tests/fixtures/valid/3step_choose.yaml must exist: {}",
        path.display()
    );
}

#[test]
fn fixtures_valid_minimal_yaml_exists() {
    let path = Path::new(WORKSPACE_ROOT).join("tests/fixtures/valid/minimal.yaml");
    assert!(
        path.exists(),
        "tests/fixtures/valid/minimal.yaml must exist: {}",
        path.display()
    );
}

#[test]
fn fixtures_valid_yaml_files_parse() {
    use serde_yaml::Value;

    let valid_dir = Path::new(WORKSPACE_ROOT).join("tests/fixtures/valid");

    // Check 3step_choose.yaml
    let path1 = valid_dir.join("3step_choose.yaml");
    let contents1 = fs::read_to_string(&path1).expect("3step_choose.yaml must be readable");
    let _: Value = serde_yaml::from_str(&contents1).expect("3step_choose.yaml must be valid YAML");

    // Check minimal.yaml
    let path2 = valid_dir.join("minimal.yaml");
    let contents2 = fs::read_to_string(&path2).expect("minimal.yaml must be readable");
    let _: Value = serde_yaml::from_str(&contents2).expect("minimal.yaml must be valid YAML");
}

#[test]
fn fixtures_invalid_yaml_files_exist() {
    let invalid_dir = Path::new(WORKSPACE_ROOT).join("tests/fixtures/invalid");

    let required_files = [
        "invalid_missing_when.yaml",
        "invalid_cyclic_dep.yaml",
        "invalid_invalid_step_type.yaml",
    ];

    for filename in required_files {
        let path = invalid_dir.join(filename);
        assert!(
            path.exists(),
            "tests/fixtures/invalid/{} must exist: {}",
            filename,
            path.display()
        );
    }
}

#[test]
fn fixtures_invalid_yaml_files_parse_as_valid_yaml() {
    use serde_yaml::Value;

    let invalid_dir = Path::new(WORKSPACE_ROOT).join("tests/fixtures/invalid");

    let invalid_files = [
        "invalid_missing_when.yaml",
        "invalid_cyclic_dep.yaml",
        "invalid_invalid_step_type.yaml",
    ];

    for filename in invalid_files {
        let path = invalid_dir.join(filename);
        let contents =
            fs::read_to_string(&path).unwrap_or_else(|_| panic!("{} must be readable", filename));

        // These files should parse as valid YAML structurally
        // (even if they fail semantic validation later)
        let _: Value = serde_yaml::from_str(&contents).unwrap_or_else(|_| {
            panic!(
                "{} must be valid YAML (even if semantically invalid)",
                filename
            )
        });
    }
}

// =============================================================================
// Section 7: Dependency Policy Document Tests
// =============================================================================

#[test]
fn dependency_policy_md_exists() {
    let path = Path::new(WORKSPACE_ROOT).join("docs/dependency-policy.md");
    assert!(
        path.exists(),
        "docs/dependency-policy.md must exist: {}",
        path.display()
    );
}

#[test]
fn dependency_policy_lists_allowed_licenses() {
    let path = Path::new(WORKSPACE_ROOT).join("docs/dependency-policy.md");
    let contents = fs::read_to_string(&path).expect("docs/dependency-policy.md must be readable");

    let allowed_count = ["MIT", "Apache", "Zlib", "BSD"]
        .iter()
        .filter(|l| contents.contains(*l))
        .count();

    assert!(
        allowed_count >= 4,
        "docs/dependency-policy.md must list at least 4 allowed license types (MIT/Apache/Zlib/BSD variants), found {}",
        allowed_count
    );
}

#[test]
fn dependency_policy_lists_banned_licenses() {
    let path = Path::new(WORKSPACE_ROOT).join("docs/dependency-policy.md");
    let contents = fs::read_to_string(&path).expect("docs/dependency-policy.md must be readable");

    let banned_count = ["GPL", "LGPL", "AGPL", "SSPL", "Commons Clause"]
        .iter()
        .filter(|b| contents.contains(*b))
        .count();

    assert!(
        banned_count >= 5,
        "docs/dependency-policy.md must list all 5 banned license types (GPL/LGPL/AGPL/SSPL/Commons Clause), found {}",
        banned_count
    );
}

#[test]
fn dependency_policy_has_exception_process_section() {
    let path = Path::new(WORKSPACE_ROOT).join("docs/dependency-policy.md");
    let contents = fs::read_to_string(&path).expect("docs/dependency-policy.md must be readable");

    assert!(
        contents.to_lowercase().contains("exception"),
        "docs/dependency-policy.md must have an 'Exception Process' section"
    );
}

// =============================================================================
// Section 8: CI Workflow Tests
// =============================================================================

#[test]
fn ci_workflow_yaml_is_valid_yaml() {
    use serde_yaml::Value;

    let path = Path::new(WORKSPACE_ROOT).join(".github/workflows/ci.yml");
    let contents = fs::read_to_string(&path).expect(".github/workflows/ci.yml must be readable");

    let _: Value =
        serde_yaml::from_str(&contents).expect(".github/workflows/ci.yml must be valid YAML");
}

#[test]
fn ci_workflow_has_required_steps() {
    let path = Path::new(WORKSPACE_ROOT).join(".github/workflows/ci.yml");
    let contents = fs::read_to_string(&path).expect(".github/workflows/ci.yml must be readable");

    let required_steps = ["geiger", "vet", "bench", "fuzz"];

    for step in required_steps {
        assert!(
            contents.contains(step),
            ".github/workflows/ci.yml must contain a step for '{}'",
            step
        );
    }
}

// =============================================================================
// Section 9: Tool Availability Tests (这些测试检查工具是否可用)
// =============================================================================

#[test]
fn cargo_deny_is_available() {
    let output = Command::new("cargo")
        .args(["deny", "--version"])
        .current_dir(WORKSPACE_ROOT)
        .output();

    assert!(
        output.is_ok(),
        "cargo-deny must be installed (run: cargo install cargo-deny)"
    );
}

#[test]
fn cargo_vet_is_available() {
    let output = Command::new("cargo")
        .args(["vet", "--version"])
        .current_dir(WORKSPACE_ROOT)
        .output();

    assert!(
        output.is_ok(),
        "cargo-vet must be installed (run: cargo install cargo-vet)"
    );
}

#[test]
fn cargo_geiger_is_available() {
    let output = Command::new("cargo")
        .args(["geiger", "--version"])
        .current_dir(WORKSPACE_ROOT)
        .output();

    assert!(
        output.is_ok(),
        "cargo-geiger must be installed (run: cargo install cargo-geiger)"
    );
}
