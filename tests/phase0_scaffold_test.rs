//! Phase 0 Scaffold Tests — vb-blq
//!
//! These tests verify the existence and validity of project infrastructure
//! scaffolding files. The scaffold should compile and carry enough metadata for
//! later phase-specific tests to prove real behavior and performance.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Workspace root is the crate root for these integration tests.
const WORKSPACE_ROOT: &str = env!("CARGO_MANIFEST_DIR");

fn workspace_path(relative: &str) -> PathBuf {
    PathBuf::from(WORKSPACE_ROOT).join(relative)
}

fn ensure(condition: bool, message: String) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message) }
}

fn read_workspace_file(relative: &str) -> Result<String, String> {
    fs::read_to_string(workspace_path(relative))
        .map_err(|error| format!("{} must be readable: {}", relative, error))
}

fn require_workspace_path(relative: &str) -> Result<PathBuf, String> {
    let path = workspace_path(relative);
    ensure(
        path.exists(),
        format!(
            "{} must exist at workspace root: {}",
            relative,
            path.display()
        ),
    )?;
    Ok(path)
}

fn require_workspace_dir(relative: &str) -> Result<PathBuf, String> {
    let path = workspace_path(relative);
    ensure(
        path.exists() && path.is_dir(),
        format!("{} directory must exist: {}", relative, path.display()),
    )?;
    Ok(path)
}

#[test]
fn deny_toml_exists() -> Result<(), String> {
    require_workspace_path("deny.toml")?;
    Ok(())
}

#[test]
fn cargo_vet_toml_exists() -> Result<(), String> {
    require_workspace_path("cargo-vet.toml")?;
    Ok(())
}

#[test]
fn geigerignore_exists() -> Result<(), String> {
    require_workspace_path(".geigerignore")?;
    Ok(())
}

#[test]
fn deny_toml_parses_as_valid_toml() -> Result<(), String> {
    use toml::Table;

    let contents = read_workspace_file("deny.toml")?;
    let parsed: Table = toml::from_str(&contents)
        .map_err(|error| format!("deny.toml must be valid TOML: {}", error))?;

    ensure(
        parsed.contains_key("advisories"),
        "deny.toml must have [advisories] section".to_string(),
    )?;
    ensure(
        parsed.contains_key("bans"),
        "deny.toml must have [bans] section".to_string(),
    )
}

#[test]
fn cargo_vet_toml_parses_as_valid_toml() -> Result<(), String> {
    use toml::Table;

    let contents = read_workspace_file("cargo-vet.toml")?;
    let parsed: Table = toml::from_str(&contents)
        .map_err(|error| format!("cargo-vet.toml must be valid TOML: {}", error))?;

    ensure(
        !parsed.is_empty(),
        "cargo-vet.toml must not be empty".to_string(),
    )
}

#[test]
fn geigerignore_has_content() -> Result<(), String> {
    let contents = read_workspace_file(".geigerignore")?;

    ensure(
        !contents.trim().is_empty(),
        ".geigerignore must not be empty".to_string(),
    )?;
    ensure(
        contents.contains("crates/") || contents.contains("target/"),
        ".geigerignore should contain paths to third-party sources".to_string(),
    )
}

#[test]
fn deny_toml_bans_required_licenses() -> Result<(), String> {
    let contents = read_workspace_file("deny.toml")?;

    for pattern in ["GPL", "LGPL", "AGPL", "SSPL", "Commons Clause"] {
        ensure(
            contents.contains(pattern),
            format!("deny.toml must ban '{}' license pattern", pattern),
        )?;
    }

    Ok(())
}

#[test]
fn deny_toml_allows_required_licenses() -> Result<(), String> {
    let contents = read_workspace_file("deny.toml")?;

    for pattern in [
        "MIT",
        "Apache-2.0",
        "BSD-2-Clause",
        "BSD-3-Clause",
        "ISC",
        "Zlib",
    ] {
        ensure(
            contents.contains(pattern),
            format!("deny.toml must allow '{}' license pattern", pattern),
        )?;
    }

    Ok(())
}

#[test]
fn benches_velvet_ballastics_rs_exists() -> Result<(), String> {
    require_workspace_path("benches/velvet_ballastics.rs")?;
    Ok(())
}

#[test]
fn benches_velvet_ballastics_has_required_criterion_groups() -> Result<(), String> {
    let contents = read_workspace_file("benches/velvet_ballastics.rs")?;

    for group in [
        "yaml_parse",
        "compile_validate",
        "expression",
        "runtime_core",
        "storage_ipc",
        "generated_mode",
    ] {
        ensure(
            contents.contains(group),
            format!("benches/velvet_ballastics.rs must define {group} benchmark group"),
        )?;
    }

    Ok(())
}

#[test]
fn benches_velvet_ballastics_has_required_metadata_fields() -> Result<(), String> {
    let contents = read_workspace_file("benches/velvet_ballastics.rs")?;

    for field in [
        "profile=bench",
        "tool=criterion-0.8",
        "durability=",
        "latency=",
        "allocations=",
        "fixture_digest=",
    ] {
        ensure(
            contents.contains(field),
            format!("benches/velvet_ballastics.rs metadata must include {field}"),
        )?;
    }

    Ok(())
}

#[test]
fn benches_velvet_ballastics_has_master_traceable_benchmark_ids() -> Result<(), String> {
    let contents = read_workspace_file("benches/velvet_ballastics.rs")?;

    for id in [
        "bench_engine_step_once_save_const_single_transition",
        "bench_engine_run_save_chain_10_steps",
        "bench_engine_run_save_chain_1000_steps",
        "bench_engine_choose_true_branch",
        "bench_engine_choose_false_branch",
        "bench_engine_finish_no_observability",
        "bench_engine_numeric_slots_read_write_i64",
        "bench_memory_ingress_try_submit_capacity_1024",
        "bench_memory_ingress_submit_recv_single_thread",
        "bench_memory_ingress_backpressure_full_queue",
        "bench_fjall_append_run_accepted_no_persist",
        "bench_replay_ordered_journal_1000_events",
    ] {
        ensure(
            contents.contains(id),
            format!("benches/velvet_ballastics.rs must include benchmark id {id}"),
        )?;
    }

    Ok(())
}

#[test]
fn benches_velvet_ballastics_uses_black_box() -> Result<(), String> {
    let contents = read_workspace_file("benches/velvet_ballastics.rs")?;

    ensure(
        contents.contains("black_box"),
        "Benchmarks must use criterion::black_box for input parameters".to_string(),
    )
}

#[test]
fn benches_velvet_ballastics_compiles() -> Result<(), String> {
    let manifest_path = workspace_path("Cargo.toml");
    let status = Command::new("cargo")
        .args(["check", "--benches", "--all-features", "--manifest-path"])
        .arg(&manifest_path)
        .current_dir(WORKSPACE_ROOT)
        .status()
        .map_err(|error| {
            format!(
                "cargo check --benches --all-features must execute: {}",
                error
            )
        })?;

    ensure(
        status.success(),
        "benches/velvet_ballastics.rs must compile with cargo check --benches --all-features"
            .to_string(),
    )
}

#[test]
fn fuzz_fuzz_targets_rs_exists() -> Result<(), String> {
    require_workspace_path("fuzz/fuzz_targets.rs")?;
    Ok(())
}

#[test]
fn fuzz_fuzz_targets_has_5_no_mangle_extern_c_functions() -> Result<(), String> {
    let contents = read_workspace_file("fuzz/fuzz_targets.rs")?;
    let no_mangle_extern_count = contents
        .lines()
        .filter(|line| line.contains("#[no_mangle]") || line.contains("#[unsafe(no_mangle)]"))
        .count();

    ensure(
        no_mangle_extern_count == 5,
        format!(
            "fuzz/fuzz_targets.rs must have exactly 5 #[no_mangle] extern \"C\" functions, found {}",
            no_mangle_extern_count
        ),
    )
}

#[test]
fn fuzz_fuzz_targets_contains_llvm_fuzzer_test_one_input() -> Result<(), String> {
    let contents = read_workspace_file("fuzz/fuzz_targets.rs")?;
    let fuzzer_count = contents.matches("LLVMFuzzerTestOneInput").count();

    ensure(
        fuzzer_count == 5,
        format!(
            "fuzz/fuzz_targets.rs must have 5 LLVMFuzzerTestOneInput functions, found {}",
            fuzzer_count
        ),
    )
}

#[test]
fn tests_fixtures_directory_exists() -> Result<(), String> {
    require_workspace_dir("tests/fixtures")?;
    Ok(())
}

#[test]
fn tests_fixtures_valid_directory_exists() -> Result<(), String> {
    require_workspace_dir("tests/fixtures/valid")?;
    Ok(())
}

#[test]
fn tests_fixtures_invalid_directory_exists() -> Result<(), String> {
    require_workspace_dir("tests/fixtures/invalid")?;
    Ok(())
}

#[test]
fn fixtures_valid_3step_choose_yaml_exists() -> Result<(), String> {
    require_workspace_path("tests/fixtures/valid/3step_choose.yaml")?;
    Ok(())
}

#[test]
fn fixtures_valid_minimal_yaml_exists() -> Result<(), String> {
    require_workspace_path("tests/fixtures/valid/minimal.yaml")?;
    Ok(())
}

#[test]
fn fixtures_valid_yaml_files_parse() -> Result<(), String> {
    use saphyr::LoadableYamlNode;

    for relative in [
        "tests/fixtures/valid/3step_choose.yaml",
        "tests/fixtures/valid/minimal.yaml",
    ] {
        let contents = read_workspace_file(relative)?;
        let _ = saphyr::Yaml::load_from_str(&contents)
            .map_err(|error| format!("{} must be valid YAML: {}", relative, error))?;
    }

    Ok(())
}

#[test]
fn fixtures_invalid_yaml_files_exist() -> Result<(), String> {
    for filename in [
        "invalid_missing_when.yaml",
        "invalid_cyclic_dep.yaml",
        "invalid_invalid_step_type.yaml",
    ] {
        let relative = format!("tests/fixtures/invalid/{}", filename);
        require_workspace_path(&relative)?;
    }

    Ok(())
}

#[test]
fn fixtures_invalid_yaml_files_parse_as_valid_yaml() -> Result<(), String> {
    use saphyr::LoadableYamlNode;

    for filename in [
        "invalid_missing_when.yaml",
        "invalid_cyclic_dep.yaml",
        "invalid_invalid_step_type.yaml",
    ] {
        let relative = format!("tests/fixtures/invalid/{}", filename);
        let contents = read_workspace_file(&relative)?;
        let _ = saphyr::Yaml::load_from_str(&contents).map_err(|error| {
            format!(
                "{} must be valid YAML even if semantically invalid: {}",
                filename, error
            )
        })?;
    }

    Ok(())
}

#[test]
fn dependency_policy_md_exists() -> Result<(), String> {
    require_workspace_path("docs/dependency-policy.md")?;
    Ok(())
}

#[test]
fn dependency_policy_lists_allowed_licenses() -> Result<(), String> {
    let contents = read_workspace_file("docs/dependency-policy.md")?;
    let allowed_count = ["MIT", "Apache", "Zlib", "BSD"]
        .iter()
        .filter(|license| contents.contains(**license))
        .count();

    ensure(
        allowed_count >= 4,
        format!(
            "docs/dependency-policy.md must list at least 4 allowed license types, found {}",
            allowed_count
        ),
    )
}

#[test]
fn dependency_policy_lists_banned_licenses() -> Result<(), String> {
    let contents = read_workspace_file("docs/dependency-policy.md")?;
    let banned_count = ["GPL", "LGPL", "AGPL", "SSPL", "Commons Clause"]
        .iter()
        .filter(|license| contents.contains(**license))
        .count();

    ensure(
        banned_count >= 5,
        format!(
            "docs/dependency-policy.md must list all 5 banned license types, found {}",
            banned_count
        ),
    )
}

#[test]
fn dependency_policy_has_exception_process_section() -> Result<(), String> {
    let contents = read_workspace_file("docs/dependency-policy.md")?;

    ensure(
        contents.to_lowercase().contains("exception"),
        "docs/dependency-policy.md must have an 'Exception Process' section".to_string(),
    )
}

#[test]
fn ci_workflow_yaml_is_valid_yaml() -> Result<(), String> {
    use saphyr::LoadableYamlNode;

    let contents = read_workspace_file(".github/workflows/ci.yml")?;
    let _ = saphyr::Yaml::load_from_str(&contents)
        .map_err(|error| format!(".github/workflows/ci.yml must be valid YAML: {}", error))?;
    Ok(())
}

#[test]
fn ci_workflow_has_required_steps() -> Result<(), String> {
    let contents = read_workspace_file(".github/workflows/ci.yml")?;

    for step in ["geiger", "vet", "bench", "fuzz"] {
        ensure(
            contents.contains(step),
            format!(
                ".github/workflows/ci.yml must contain a step for '{}'",
                step
            ),
        )?;
    }

    Ok(())
}

#[test]
fn cargo_deny_is_available() -> Result<(), String> {
    let output = Command::new("cargo")
        .args(["deny", "--version"])
        .current_dir(WORKSPACE_ROOT)
        .output();

    ensure(
        output.is_ok(),
        "cargo-deny must be installed (run: cargo install cargo-deny)".to_string(),
    )
}

#[test]
fn cargo_vet_is_available() -> Result<(), String> {
    let output = Command::new("cargo")
        .args(["vet", "--version"])
        .current_dir(WORKSPACE_ROOT)
        .output();

    ensure(
        output.is_ok(),
        "cargo-vet must be installed (run: cargo install cargo-vet)".to_string(),
    )
}

#[test]
fn cargo_geiger_is_available() -> Result<(), String> {
    let output = Command::new("cargo")
        .args(["geiger", "--version"])
        .current_dir(WORKSPACE_ROOT)
        .output();

    ensure(
        output.is_ok(),
        "cargo-geiger must be installed (run: cargo install cargo-geiger)".to_string(),
    )
}
