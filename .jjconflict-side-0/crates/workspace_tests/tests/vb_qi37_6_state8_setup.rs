#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

fn workspace_root() -> Result<PathBuf, String> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| String::from("workspace root must be two parents above workspace_tests"))
}

#[test]
fn state8_kani_exact_grant_setup_detects_module_before_state11_execution() -> Result<(), String> {
    // Given: State 8 owns Kani setup and State 11 owns Kani execution.
    let root = workspace_root()?;
    let flat_module = root.join("crates/vb_core/src/kani.rs");
    let nested_module = root.join("crates/vb_core/src/kani/mod.rs");

    // When: the approved setup predicate is evaluated.
    let setup_status = if flat_module.is_file() || nested_module.is_file() {
        "KANI_SETUP_PRESENT"
    } else {
        "KANI_SETUP_MISSING"
    };

    // Then: State 8 must leave a setup-present marker, not claim harness PASS.
    assert_eq!(setup_status, "KANI_SETUP_PRESENT");
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
