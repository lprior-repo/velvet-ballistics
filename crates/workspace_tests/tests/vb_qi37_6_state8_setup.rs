#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> Result<PathBuf, String> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "workspace root must be two parents above workspace_tests".to_string())
}

fn read_file(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))
}

fn normalize_lines(source: &str) -> String {
    source
        .lines()
        .map(|line| line.trim().to_string())
        .collect::<Vec<String>>()
        .join("\n")
}

fn has_kani_registration(lib_source: &str) -> bool {
    let normalized = normalize_lines(lib_source);
    normalized.contains("#[cfg(kani)]\n#[path = \"verification/kani/kani_capability_harnesses.rs\"]\npub mod kani_capability_harnesses;")
}

fn has_named_proof(kani_source: &str) -> bool {
    let normalized = normalize_lines(kani_source);
    normalized.contains("#[kani::proof]\nfn capability_name_grants_harness()")
}

fn kani_module_path(root: &Path) -> PathBuf {
    root.join("crates/vb_core/src/verification/kani/kani_capability_harnesses.rs")
}

fn kani_setup_status(root: &Path) -> Result<String, String> {
    let lib_path = root.join("crates/vb_core/src/lib.rs");
    let lib_source = read_file(&lib_path)?;
    if !kani_module_path(root).is_file() {
        return Ok("KANI_FILE_MISSING".to_string());
    }
    if !has_kani_registration(&lib_source) {
        return Ok("KANI_MODULE_UNREGISTERED".to_string());
    }
    let kani_source = read_file(&kani_module_path(root))?;
    if !has_named_proof(&kani_source) {
        return Ok("NAMED_PROOF_MISSING".to_string());
    }
    Ok("KANI_SETUP_PRESENT".to_string())
}

fn write_lib(root: &Path, source: &str) -> Result<(), String> {
    let dir = root.join("crates/vb_core/src");
    fs::create_dir_all(&dir).map_err(|error| format!("mkdir {}: {error}", dir.display()))?;
    fs::write(dir.join("lib.rs"), source).map_err(|error| format!("write lib.rs: {error}"))?;
    Ok(())
}

fn write_kani(root: &Path, source: &str) -> Result<(), String> {
    let dir = root.join("crates/vb_core/src/verification/kani");
    fs::create_dir_all(&dir).map_err(|error| format!("mkdir {}: {error}", dir.display()))?;
    fs::write(dir.join("kani_capability_harnesses.rs"), source)
        .map_err(|error| format!("write kani: {error}"))?;
    Ok(())
}

#[test]
fn state8_kani_exact_grant_setup_detects_module_before_state11_execution() -> Result<(), String> {
    let root = workspace_root()?;
    let status = kani_setup_status(&root)?;
    assert_eq!(status, "KANI_SETUP_PRESENT");
    Ok(())
}

fn run_fixture(lib: &str, kani: Option<&str>, expected: &str) -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|error| format!("tempdir: {error}"))?;
    let root = dir.path();
    write_lib(root, lib)?;
    if let Some(kani_source) = kani {
        write_kani(root, kani_source)?;
    }
    let actual = kani_setup_status(root)?;
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn state8_kani_exact_grant_setup_fixture_table() -> Result<(), String> {
    let valid_lib = "#[cfg(kani)]\n#[path = \"verification/kani/kani_capability_harnesses.rs\"]\npub mod kani_capability_harnesses;\n";
    let valid_kani = "#[kani::proof]\nfn capability_name_grants_harness() {}\n";
    run_fixture(valid_lib, Some(valid_kani), "KANI_SETUP_PRESENT")?;
    run_fixture(valid_lib, None, "KANI_FILE_MISSING")?;
    run_fixture(
        "// no registration\n",
        Some(valid_kani),
        "KANI_MODULE_UNREGISTERED",
    )?;
    run_fixture(
        "#[cfg(kani)]\npub mod other;\n",
        Some(valid_kani),
        "KANI_MODULE_UNREGISTERED",
    )?;
    run_fixture(
        "#[cfg(kani)]\npub mod other;\npub mod kani_capability_harnesses;\n",
        Some(valid_kani),
        "KANI_MODULE_UNREGISTERED",
    )?;
    run_fixture(
        valid_lib,
        Some("#[kani::proof]\nfn some_other_harness() {}\n"),
        "NAMED_PROOF_MISSING",
    )?;
    run_fixture(
        valid_lib,
        Some("#[kani::proof]\nfn different_name() {}\n"),
        "NAMED_PROOF_MISSING",
    )?;
    Ok(())
}

#[test]
fn state8_fuzz_schema_setup_registers_required_capability_bins() -> Result<(), String> {
    // Given: cargo-fuzz execution is blocked unless both bins are registered.
    let root = workspace_root()?;
    let manifest_path = root.join("fuzz/Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path)
        .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?;

    // When: State 8 checks bin registration under autobins=false.
    let name_bin = manifest.contains("name = \"capability_name_schema\"");
    let contract_bin = manifest.contains("name = \"capability_contract_schema\"");
    let setup_status = match (name_bin, contract_bin) {
        (true, true) => "FUZZ_BINS_PRESENT",
        _ => "FUZZ_BINS_MISSING",
    };

    // Then: both State 11 fuzz routes are eligible for execution.
    assert_eq!(setup_status, "FUZZ_BINS_PRESENT");
    Ok(())
}
