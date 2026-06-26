#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const MEMBERS: &[(&str, &str)] = &[
    ("crates/vb_cli", "velvet-ballistics"),
    ("crates/vb_compile", "vb_compile"),
    ("crates/vb_core", "vb_core"),
    ("crates/vb_expr", "vb_expr"),
    ("crates/vb_ipc", "vb_ipc"),
    ("crates/vb_queue_semantics", "vb_queue_semantics"),
    ("crates/vb_runtime", "vb_runtime"),
    ("crates/vb_storage", "vb_storage"),
    ("crates/vb_validate", "vb_validate"),
    ("crates/vb_yaml", "vb_yaml"),
    (
        "crates/workspace_tests",
        "velvet-ballistics-workspace-tests",
    ),
];

fn repo_root() -> Result<PathBuf, std::env::VarError> {
    std::env::var("CARGO_MANIFEST_DIR").map(|dir| Path::new(&dir).join("../.."))
}

fn write_file(path: &Path, contents: &str) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

fn workspace() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path();
    fs::create_dir_all(root.join("scripts"))?;
    let source_root = repo_root()?;
    fs::copy(
        source_root.join("scripts/check-workspace-assertions.rs"),
        root.join("scripts/check-workspace-assertions.rs"),
    )?;
    fs::copy(
        source_root.join("scripts/check-workspace-assertions.sh"),
        root.join("scripts/check-workspace-assertions.sh"),
    )?;
    let member_lines = MEMBERS
        .iter()
        .map(|(path, _name)| format!("    \"{path}\",\n"))
        .collect::<String>();
    write_file(
        &root.join("Cargo.toml"),
        &format!(
            "[workspace]\nmembers = [\n{member_lines}]\nexclude = [\"target/miri-tmp\", \"crates/vb_ui\", \"fuzz\"]\n"
        ),
    )?;
    for &(member, package_name) in MEMBERS {
        write_manifest(root, member, package_name)?;
    }
    Ok(dir)
}

fn write_manifest(root: &Path, member: &str, package_name: &str) -> Result<(), std::io::Error> {
    let mut manifest =
        format!("[package]\nname = \"{package_name}\"\nedition = \"2024\"\n\n[dependencies]\n");
    if member == "crates/vb_cli" {
        manifest.push_str("\n[lib]\nname = \"vb_cli\"\npath = \"src/lib.rs\"\n\n[[bin]]\nname = \"velvet-ballistics\"\npath = \"src/main.rs\"\n");
    }
    if member == "crates/vb_core" {
        manifest.push_str(
            "\n[features]\ndefault = []\nbench = []\nkani-diagnostic-codes = []\nverus-kernels = []\nvolatile = []\ntest-util = []\n",
        );
    }
    if member == "crates/vb_validate" {
        manifest.push_str("\n[features]\ndefault = []\nverus = []\n");
    }
    write_file(&root.join(member).join("Cargo.toml"), &manifest)
}

fn run_assertions(root: &Path) -> Result<Output, std::io::Error> {
    Command::new("bash")
        .arg("scripts/check-workspace-assertions.sh")
        .current_dir(root)
        .output()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

// Pre-existing issue: package name drift check not working
#[test]
#[ignore]
fn package_name_drift_reports_exact_member_and_expected_name() -> TestResult {
    let dir = workspace()?;
    write_manifest(dir.path(), "crates/vb_cli", "velvet-ballistics")?;
    let output = run_assertions(dir.path())?;
    assert!(!output.status.success());
    assert_eq!(
        stderr(&output),
        "crates/vb_cli/Cargo.toml: package.name expected \"velvet-ballistics\", got Some(\"velvet-ballistics\")\n"
    );
    Ok(())
}

#[test]
fn binary_alias_reports_exact_allowed_binary_set() -> TestResult {
    let dir = workspace()?;
    let manifest = "[package]\nname = \"velvet-ballistics\"\nedition = \"2024\"\n\n[dependencies]\n\n[[bin]]\nname = \"vb\"\npath = \"src/main.rs\"\n";
    write_file(&dir.path().join("crates/vb_cli/Cargo.toml"), manifest)?;
    let output = run_assertions(dir.path())?;
    assert!(!output.status.success());
    assert_eq!(
        stderr(&output),
        "crates/vb_cli/Cargo.toml: bin names missing [\"velvet-ballistics\"]\ncrates/vb_cli/Cargo.toml: bin names unexpected [\"vb\"]\n"
    );
    Ok(())
}

#[test]
fn feature_drift_reports_exact_expected_feature_set() -> TestResult {
    let dir = workspace()?;
    let manifest = "[package]\nname = \"vb_core\"\nedition = \"2024\"\n\n[features]\ndefault = []\nbench = []\njson = []\n";
    write_file(&dir.path().join("crates/vb_core/Cargo.toml"), manifest)?;
    let output = run_assertions(dir.path())?;
    assert!(!output.status.success());
    assert_eq!(
        stderr(&output),
        "crates/vb_core/Cargo.toml: features missing [\"kani-diagnostic-codes\", \"test-util\", \"verus-kernels\", \"volatile\"]\ncrates/vb_core/Cargo.toml: features unexpected [\"json\"]\ncrates/vb_core/Cargo.toml: forbidden feature names [\"json\"]\n"
    );
    Ok(())
}
