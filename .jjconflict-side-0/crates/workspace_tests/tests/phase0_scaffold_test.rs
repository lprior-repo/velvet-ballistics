#![cfg(any())]
//! Phase 0 Scaffold Tests — vb-blq
//!
//! These tests verify the existence and validity of project infrastructure
//! scaffolding files. The scaffold should compile and carry enough metadata for
//! later phase-specific tests to prove real behavior and performance.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Workspace root is two directories above this integration-test package.
const WORKSPACE_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

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

fn require_file_contains(relative: &str, needle: &str, reason: &str) -> Result<(), String> {
    let contents = read_workspace_file(relative)?;
    ensure(
        contents.contains(needle),
        format!("{} must contain '{}' for {}", relative, needle, reason),
    )
}

fn require_valid_yaml_file(relative: &str, reason: &str) -> Result<(), String> {
    use saphyr::LoadableYamlNode;

    let contents = read_workspace_file(relative)?;
    saphyr::Yaml::load_from_str(&contents)
        .map_err(|error| format!("{} must be valid YAML for {}: {}", relative, reason, error))?;
    Ok(())
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
fn deny_toml_bans_gpl_license_pattern() -> Result<(), String> {
    require_file_contains("deny.toml", "GPL", "banned license policy")
}

#[test]
fn deny_toml_bans_lgpl_license_pattern() -> Result<(), String> {
    require_file_contains("deny.toml", "LGPL", "banned license policy")
}

#[test]
fn deny_toml_bans_agpl_license_pattern() -> Result<(), String> {
    require_file_contains("deny.toml", "AGPL", "banned license policy")
}

#[test]
fn deny_toml_bans_sspl_license_pattern() -> Result<(), String> {
    require_file_contains("deny.toml", "SSPL", "banned license policy")
}

#[test]
fn deny_toml_bans_commons_clause_license_pattern() -> Result<(), String> {
    require_file_contains("deny.toml", "Commons Clause", "banned license policy")
}

#[test]
fn deny_toml_allows_mit_license_pattern() -> Result<(), String> {
    require_file_contains("deny.toml", "MIT", "allowed license policy")
}

#[test]
fn deny_toml_allows_apache_2_license_pattern() -> Result<(), String> {
    require_file_contains("deny.toml", "Apache-2.0", "allowed license policy")
}

#[test]
fn deny_toml_allows_bsd_2_clause_license_pattern() -> Result<(), String> {
    require_file_contains("deny.toml", "BSD-2-Clause", "allowed license policy")
}

#[test]
fn deny_toml_allows_bsd_3_clause_license_pattern() -> Result<(), String> {
    require_file_contains("deny.toml", "BSD-3-Clause", "allowed license policy")
}

#[test]
fn deny_toml_allows_isc_license_pattern() -> Result<(), String> {
    require_file_contains("deny.toml", "ISC", "allowed license policy")
}

#[test]
fn deny_toml_allows_zlib_license_pattern() -> Result<(), String> {
    require_file_contains("deny.toml", "Zlib", "allowed license policy")
}

#[test]
fn benches_velvet_ballistics_rs_exists() -> Result<(), String> {
    require_workspace_path("benches/velvet_ballistics.rs")?;
    Ok(())
}

#[test]
fn benches_velvet_ballistics_has_yaml_parse_group() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "yaml_parse",
        "benchmark group",
    )
}

#[test]
fn benches_velvet_ballistics_has_compile_validate_group() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "compile_validate",
        "benchmark group",
    )
}

#[test]
fn benches_velvet_ballistics_has_expression_group() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "expression",
        "benchmark group",
    )
}

#[test]
fn benches_velvet_ballistics_has_runtime_core_group() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "runtime_core",
        "benchmark group",
    )
}

#[test]
fn benches_velvet_ballistics_has_storage_ipc_group() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "storage_ipc",
        "benchmark group",
    )
}

#[test]
fn benches_velvet_ballistics_has_generated_mode_group() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "generated_mode",
        "benchmark group",
    )
}

#[test]
fn benches_velvet_ballistics_metadata_has_profile() -> Result<(), String> {
    require_file_contains("benches/velvet_ballistics.rs", "profile=bench", "metadata")
}

#[test]
fn benches_velvet_ballistics_metadata_has_criterion_tool() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "tool=criterion-0.8",
        "metadata",
    )
}

#[test]
fn benches_velvet_ballistics_metadata_has_durability() -> Result<(), String> {
    require_file_contains("benches/velvet_ballistics.rs", "durability=", "metadata")
}

#[test]
fn benches_velvet_ballistics_metadata_has_latency() -> Result<(), String> {
    require_file_contains("benches/velvet_ballistics.rs", "latency=", "metadata")
}

#[test]
fn benches_velvet_ballistics_metadata_has_allocations() -> Result<(), String> {
    require_file_contains("benches/velvet_ballistics.rs", "allocations=", "metadata")
}

#[test]
fn benches_velvet_ballistics_metadata_has_fixture_digest() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "fixture_digest=",
        "metadata",
    )
}

#[test]
fn benches_velvet_ballistics_has_step_once_benchmark_id() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "bench_engine_step_once_save_const_single_transition",
        "master traceable benchmark id",
    )
}

#[test]
fn benches_velvet_ballistics_has_run_chain_10_benchmark_id() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "bench_engine_run_save_chain_10_steps",
        "master traceable benchmark id",
    )
}

#[test]
fn benches_velvet_ballistics_has_run_chain_1000_benchmark_id() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "bench_engine_run_save_chain_1000_steps",
        "master traceable benchmark id",
    )
}

#[test]
fn benches_velvet_ballistics_has_choose_true_benchmark_id() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "bench_engine_choose_true_branch",
        "master traceable benchmark id",
    )
}

#[test]
fn benches_velvet_ballistics_has_choose_false_benchmark_id() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "bench_engine_choose_false_branch",
        "master traceable benchmark id",
    )
}

#[test]
fn benches_velvet_ballistics_has_finish_benchmark_id() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "bench_engine_finish_no_observability",
        "master traceable benchmark id",
    )
}

#[test]
fn benches_velvet_ballistics_has_numeric_slots_benchmark_id() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "bench_engine_numeric_slots_read_write_i64",
        "master traceable benchmark id",
    )
}

#[test]
fn benches_velvet_ballistics_has_ingress_capacity_benchmark_id() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "bench_memory_ingress_try_submit_capacity_1024",
        "master traceable benchmark id",
    )
}

#[test]
fn benches_velvet_ballistics_has_ingress_submit_recv_benchmark_id() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "bench_memory_ingress_submit_recv_single_thread",
        "master traceable benchmark id",
    )
}

#[test]
fn benches_velvet_ballistics_has_ingress_backpressure_benchmark_id() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "bench_memory_ingress_backpressure_full_queue",
        "master traceable benchmark id",
    )
}

#[test]
fn benches_velvet_ballistics_has_fjall_append_benchmark_id() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "bench_fjall_append_run_accepted_no_persist",
        "master traceable benchmark id",
    )
}

#[test]
fn benches_velvet_ballistics_has_replay_journal_benchmark_id() -> Result<(), String> {
    require_file_contains(
        "benches/velvet_ballistics.rs",
        "bench_replay_ordered_journal_1000_events",
        "master traceable benchmark id",
    )
}

#[test]
fn benches_velvet_ballistics_uses_black_box() -> Result<(), String> {
    let contents = read_workspace_file("benches/velvet_ballistics.rs")?;

    ensure(
        contents.contains("black_box"),
        "Benchmarks must use criterion::black_box for input parameters".to_string(),
    )
}

#[test]
fn benches_velvet_ballistics_compiles() -> Result<(), String> {
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
        "benches/velvet_ballistics.rs must compile with cargo check --benches --all-features"
            .to_string(),
    )
}

#[test]
fn pgo_workload_minimal_save_fixture_compiles() -> Result<(), String> {
    let relative = "tests/fixtures/pgo/minimal_save.yaml";
    let bytes = fs::read(workspace_path(relative))
        .map_err(|error| format!("{} must be readable: {}", relative, error))?;
    vb_compile::compile_workflow(&bytes)
        .map(|_workflow| ())
        .map_err(|errors| {
            let details = errors
                .0
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{} must compile as a PGO workload: {}", relative, details)
        })?;
    Ok(())
}

#[test]
fn pgo_workload_choose_true_fixture_compiles() -> Result<(), String> {
    let relative = "tests/fixtures/pgo/choose_true.yaml";
    let bytes = fs::read(workspace_path(relative))
        .map_err(|error| format!("{} must be readable: {}", relative, error))?;
    vb_compile::compile_workflow(&bytes)
        .map(|_workflow| ())
        .map_err(|errors| {
            let details = errors
                .0
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{} must compile as a PGO workload: {}", relative, details)
        })?;
    Ok(())
}

#[test]
fn pgo_choose_true_workload_fixture_compiles() -> Result<(), String> {
    assert_pgo_workload_fixture_compiles("tests/fixtures/pgo/choose_true.yaml")
}

fn assert_pgo_workload_fixture_compiles(relative: &str) -> Result<(), String> {
    let bytes = fs::read(workspace_path(relative))
        .map_err(|error| format!("{} must be readable: {}", relative, error))?;
    vb_compile::compile_workflow(&bytes)
        .map(|_workflow| ())
        .map_err(|errors| {
            let details = errors
                .0
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{} must compile as a PGO workload: {}", relative, details)
        })
}

#[test]
fn cargo_maxperf_profile_is_release_inheriting_fat_lto() -> Result<(), String> {
    let contents = read_workspace_file("Cargo.toml")?;
    let parsed: toml::Value = toml::from_str(&contents)
        .map_err(|error| format!("Cargo.toml must parse as TOML: {}", error))?;
    let maxperf = parsed
        .get("profile")
        .and_then(|profile| profile.get("maxperf"))
        .and_then(toml::Value::as_table)
        .ok_or_else(|| "Cargo.toml must define [profile.maxperf]".to_string())?;

    ensure(
        maxperf.get("inherits").and_then(toml::Value::as_str) == Some("release"),
        "[profile.maxperf] must inherit from release".to_string(),
    )?;
    ensure(
        maxperf.get("lto").and_then(toml::Value::as_str) == Some("fat"),
        "[profile.maxperf] must enable fat LTO".to_string(),
    )?;
    ensure(
        maxperf
            .get("codegen-units")
            .and_then(toml::Value::as_integer)
            == Some(1),
        "[profile.maxperf] must use one codegen unit".to_string(),
    )?;
    ensure(
        maxperf.get("debug").and_then(toml::Value::as_bool) == Some(false),
        "[profile.maxperf] must disable debug info".to_string(),
    )?;
    ensure(
        maxperf
            .get("debug-assertions")
            .and_then(toml::Value::as_bool)
            != Some(true),
        "[profile.maxperf] must not enable debug assertions".to_string(),
    )?;
    ensure(
        maxperf
            .get("overflow-checks")
            .and_then(toml::Value::as_bool)
            != Some(true),
        "[profile.maxperf] must not enable overflow checks".to_string(),
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
fn fixtures_valid_3step_choose_yaml_parses() -> Result<(), String> {
    require_valid_yaml_file("tests/fixtures/valid/3step_choose.yaml", "valid fixture")
}

#[test]
fn fixtures_valid_minimal_yaml_parses() -> Result<(), String> {
    require_valid_yaml_file("tests/fixtures/valid/minimal.yaml", "valid fixture")
}

#[test]
fn fixtures_invalid_missing_when_yaml_exists() -> Result<(), String> {
    require_workspace_path("tests/fixtures/invalid/invalid_missing_when.yaml")?;
    Ok(())
}

#[test]
fn fixtures_invalid_cyclic_dep_yaml_exists() -> Result<(), String> {
    require_workspace_path("tests/fixtures/invalid/invalid_cyclic_dep.yaml")?;
    Ok(())
}

#[test]
fn fixtures_invalid_step_type_yaml_exists() -> Result<(), String> {
    require_workspace_path("tests/fixtures/invalid/invalid_invalid_step_type.yaml")?;
    Ok(())
}

#[test]
fn fixtures_invalid_missing_when_yaml_parses() -> Result<(), String> {
    require_valid_yaml_file(
        "tests/fixtures/invalid/invalid_missing_when.yaml",
        "semantically invalid fixture",
    )
}

#[test]
fn fixtures_invalid_cyclic_dep_yaml_parses() -> Result<(), String> {
    require_valid_yaml_file(
        "tests/fixtures/invalid/invalid_cyclic_dep.yaml",
        "semantically invalid fixture",
    )
}

#[test]
fn fixtures_invalid_step_type_yaml_parses() -> Result<(), String> {
    require_valid_yaml_file(
        "tests/fixtures/invalid/invalid_invalid_step_type.yaml",
        "semantically invalid fixture",
    )
}

#[test]
fn dependency_policy_md_exists() -> Result<(), String> {
    require_workspace_path("docs/dependency-policy.md")?;
    Ok(())
}

#[test]
fn dependency_policy_lists_mit_allowed_license() -> Result<(), String> {
    require_file_contains("docs/dependency-policy.md", "MIT", "allowed license policy")
}

#[test]
fn dependency_policy_lists_apache_allowed_license() -> Result<(), String> {
    require_file_contains(
        "docs/dependency-policy.md",
        "Apache",
        "allowed license policy",
    )
}

#[test]
fn dependency_policy_lists_zlib_allowed_license() -> Result<(), String> {
    require_file_contains(
        "docs/dependency-policy.md",
        "Zlib",
        "allowed license policy",
    )
}

#[test]
fn dependency_policy_lists_bsd_allowed_license() -> Result<(), String> {
    require_file_contains("docs/dependency-policy.md", "BSD", "allowed license policy")
}

#[test]
fn dependency_policy_lists_gpl_banned_license() -> Result<(), String> {
    require_file_contains("docs/dependency-policy.md", "GPL", "banned license policy")
}

#[test]
fn dependency_policy_lists_lgpl_banned_license() -> Result<(), String> {
    require_file_contains("docs/dependency-policy.md", "LGPL", "banned license policy")
}

#[test]
fn dependency_policy_lists_agpl_banned_license() -> Result<(), String> {
    require_file_contains("docs/dependency-policy.md", "AGPL", "banned license policy")
}

#[test]
fn dependency_policy_lists_sspl_banned_license() -> Result<(), String> {
    require_file_contains("docs/dependency-policy.md", "SSPL", "banned license policy")
}

#[test]
fn dependency_policy_lists_commons_clause_banned_license() -> Result<(), String> {
    require_file_contains(
        "docs/dependency-policy.md",
        "Commons Clause",
        "banned license policy",
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
fn github_actions_workflows_are_removed() -> Result<(), String> {
    let workflow_dir = workspace_path(".github/workflows");

    ensure(
        !workflow_dir.exists(),
        ".github/workflows must not exist; CI is intentionally driven by local Moon gates"
            .to_string(),
    )
}

#[test]
fn moon_tasks_preserve_local_ci_gate_coverage() -> Result<(), String> {
    let contents = read_workspace_file(".moon/tasks/all.yml")?;

    ensure(
        contents.contains("supply-chain"),
        ".moon/tasks/all.yml must keep the local supply-chain gate".to_string(),
    )?;
    ensure(
        contents.contains("bench-build"),
        ".moon/tasks/all.yml must keep the local benchmark build gate".to_string(),
    )?;
    ensure(
        contents.contains("fuzz-smoke"),
        ".moon/tasks/all.yml must keep the local fuzz smoke gate".to_string(),
    )?;
    ensure(
        !contents.contains(".github/**/*"),
        ".moon/tasks/all.yml must not depend on removed GitHub Actions files".to_string(),
    )
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
