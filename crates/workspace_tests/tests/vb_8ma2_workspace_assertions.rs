#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const EXTRA_MEMBER_MANIFESTS: [(&str, &str); 15] = [
    ("crates/vb_boundary_inventory", "vb_boundary_inventory"),
    ("crates/vb_yaml", "vb_yaml"),
    ("crates/vb_validate", "vb_validate"),
    ("crates/vb_expr", "vb_expr"),
    ("crates/vb_compile", "vb_compile"),
    ("crates/vb_doc", "vb_doc"),
    ("crates/vb_codegen", "vb_codegen"),
    ("crates/vb_ui_makepad", "vb_ui_makepad"),
    ("crates/vb_ui_model", "vb_ui_model"),
    ("crates/vb_ui_snapshot", "vb_ui_snapshot"),
    ("crates/vb_proof_kernels", "vb_proof_kernels"),
    ("crates/vb_cli", "velvet-ballastics"),
    (
        "crates/workspace_tests",
        "velvet-ballastics-workspace-tests",
    ),
    ("crates/vb_benchmark", "vb_benchmark"),
    ("xtask", "xtask"),
];

fn repo_root() -> Result<PathBuf, std::env::VarError> {
    std::env::var("CARGO_MANIFEST_DIR").map(|dir| Path::new(&dir).join("../.."))
}

fn copy_assertion_scripts(root: &Path) -> Result<(), std::io::Error> {
    let scripts = root.join("scripts");
    fs::create_dir_all(&scripts)?;
    let source_root = repo_root().map_err(std::io::Error::other)?;
    fs::copy(
        source_root.join("scripts/check-workspace-assertions.rs"),
        scripts.join("check-workspace-assertions.rs"),
    )?;
    fs::copy(
        source_root.join("scripts/check-workspace-assertions.sh"),
        scripts.join("check-workspace-assertions.sh"),
    )?;
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

fn write_manifest(root: &Path, extra_member: Option<&str>) -> Result<(), std::io::Error> {
    let extra = extra_member.map_or(String::new(), |member| format!("    \"{member}\",\n"));
    write_file(
        &root.join("Cargo.toml"),
        &format!(
            r#"[workspace]
members = [
    "crates/vb_boundary_inventory",
    "crates/vb_core",
    "crates/vb_yaml",
    "crates/vb_validate",
    "crates/vb_expr",
    "crates/vb_compile",
    "crates/vb_storage",
    "crates/vb_runtime",
    "crates/vb_doc",
    "crates/vb_ipc",
    "crates/vb_codegen",
    "crates/vb_ui_makepad",
    "crates/vb_ui_model",
    "crates/vb_ui_snapshot",
    "crates/vb_proof_kernels",
    "crates/vb_cli",
    "crates/workspace_tests",
    "crates/vb_benchmark",
    "xtask",
{extra}]
exclude = ["target/miri-tmp", "crates/vb_ui", "fuzz"]
"#
        ),
    )
}

fn write_boundary_crates(root: &Path, forbidden_dep: Option<&str>) -> Result<(), std::io::Error> {
    let dependency = forbidden_dep.map(|name| format!("{name} = {{ path = \"../{name}\" }}\n"));
    write_boundary_crates_with_dependency(root, dependency.as_deref())?;
    write_extra_member_manifests(root)
}

fn write_boundary_crates_with_dependency(
    root: &Path,
    dependency_line: Option<&str>,
) -> Result<(), std::io::Error> {
    for crate_name in ["vb_core", "vb_runtime", "vb_storage", "vb_ipc"] {
        let dependency = if crate_name == "vb_core" {
            dependency_line.unwrap_or_default().to_owned()
        } else {
            String::new()
        };
        write_file(
            &root.join(format!("crates/{crate_name}/Cargo.toml")),
            &format!(
                r#"[package]
name = "{crate_name}"
edition = "2024"

[dependencies]
{dependency}
[features]
default = []
generated = []
bench = []
volatile = []
test-util = []
"#
            ),
        )?;
    }
    Ok(())
}

fn write_extra_member_manifests(root: &Path) -> Result<(), std::io::Error> {
    for (member, package_name) in EXTRA_MEMBER_MANIFESTS {
        let mut manifest =
            format!("[package]\nname = \"{package_name}\"\nedition = \"2024\"\n\n[dependencies]\n");
        if member == "crates/vb_cli" {
            manifest.push_str("\n[lib]\nname = \"vb_cli\"\npath = \"src/lib.rs\"\n\n[[bin]]\nname = \"velvet-ballastics\"\npath = \"src/main.rs\"\n");
        }
        if member == "crates/vb_validate" {
            manifest.push_str("\n[features]\ndefault = []\nverus = []\n");
        }
        if member == "crates/vb_ui_snapshot" {
            manifest.push_str("\n[features]\ndefault = [\"std\"]\nstd = []\n");
        }
        write_file(&root.join(member).join("Cargo.toml"), &manifest)?;
    }
    Ok(())
}

fn write_generated_source(root: &Path, contents: &str) -> Result<(), std::io::Error> {
    write_file(
        &root.join("crates/vb_codegen/src/generated/workflow.rs"),
        contents,
    )
}

fn workspace_with(
    forbidden_dep: Option<&str>,
    extra_member: Option<&str>,
) -> Result<tempfile::TempDir, std::io::Error> {
    let dir = tempfile::tempdir()?;
    copy_assertion_scripts(dir.path())?;
    write_manifest(dir.path(), extra_member)?;
    write_boundary_crates(dir.path(), forbidden_dep)?;
    Ok(dir)
}

fn workspace_with_dependency_line(
    dependency_line: &str,
) -> Result<tempfile::TempDir, std::io::Error> {
    let dir = tempfile::tempdir()?;
    copy_assertion_scripts(dir.path())?;
    write_manifest(dir.path(), None)?;
    write_boundary_crates_with_dependency(dir.path(), Some(dependency_line))?;
    Ok(dir)
}

fn run_assertions(root: &Path) -> Result<Output, std::io::Error> {
    Command::new("bash")
        .arg("scripts/check-workspace-assertions.sh")
        .current_dir(root)
        .output()
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn valid_workspace_passes_sharpened_assertions() -> TestResult {
    let workspace = workspace_with(None, None)?;
    let output = run_assertions(workspace.path())?;
    assert!(output.status.success(), "{}", stderr_text(&output));
    Ok(())
}

#[test]
fn forbidden_ui_dependency_fails_target_crate() -> TestResult {
    let workspace = workspace_with(Some("vb_ui_makepad"), None)?;
    let output = run_assertions(workspace.path())?;
    let stderr = stderr_text(&output);
    assert!(!output.status.success());
    assert!(stderr.contains("crates/vb_core/Cargo.toml"), "{stderr}");
    assert!(stderr.contains("forbidden UI dependency"), "{stderr}");
    assert!(stderr.contains("vb_ui_makepad"), "{stderr}");
    Ok(())
}

#[test]
fn unexpected_workspace_member_fails_exact_gate() -> TestResult {
    let workspace = workspace_with(None, Some("crates/vb_surprise"))?;
    let output = run_assertions(workspace.path())?;
    let stderr = stderr_text(&output);
    assert!(!output.status.success());
    assert!(stderr.contains("workspace.members unexpected"), "{stderr}");
    assert!(stderr.contains("crates/vb_surprise"), "{stderr}");
    Ok(())
}

#[test]
fn forbidden_runtime_format_dependency_fails_target_crate() -> TestResult {
    let workspace = workspace_with(Some("serde_json"), None)?;
    let output = run_assertions(workspace.path())?;
    let stderr = stderr_text(&output);
    assert!(!output.status.success());
    assert!(
        stderr.contains("forbidden runtime format dependency"),
        "{stderr}"
    );
    assert!(stderr.contains("serde_json"), "{stderr}");
    Ok(())
}

#[test]
fn renamed_forbidden_ui_dependency_fails_target_crate() -> TestResult {
    let workspace = workspace_with_dependency_line("ui = { package = \"vb_ui_makepad\" }\n")?;
    let output = run_assertions(workspace.path())?;
    let stderr = stderr_text(&output);
    assert!(!output.status.success());
    assert!(stderr.contains("crates/vb_core/Cargo.toml"), "{stderr}");
    assert!(stderr.contains("forbidden UI dependency"), "{stderr}");
    assert!(stderr.contains("vb_ui_makepad"), "{stderr}");
    Ok(())
}

#[test]
fn renamed_forbidden_runtime_format_dependency_fails_target_crate() -> TestResult {
    let workspace = workspace_with_dependency_line("fmt = { package = \"serde_json\" }\n")?;
    let output = run_assertions(workspace.path())?;
    let stderr = stderr_text(&output);
    assert!(!output.status.success());
    assert!(
        stderr.contains("forbidden runtime format dependency"),
        "{stderr}"
    );
    assert!(stderr.contains("serde_json"), "{stderr}");
    Ok(())
}

#[test]
fn path_aliased_forbidden_ui_dependency_fails_target_crate() -> TestResult {
    let workspace = workspace_with_dependency_line("ui = { path = \"../vb_ui_makepad\" }\n")?;
    let output = run_assertions(workspace.path())?;
    let stderr = stderr_text(&output);
    assert!(!output.status.success());
    assert!(stderr.contains("forbidden UI dependency"), "{stderr}");
    assert!(stderr.contains("vb_ui_makepad"), "{stderr}");
    Ok(())
}

#[test]
fn generated_boundary_forbidden_token_fails_target_source() -> TestResult {
    let workspace = workspace_with(None, None)?;
    write_generated_source(
        workspace.path(),
        "pub const FORMAT: &str = \"serde_json\";\n",
    )?;
    let output = run_assertions(workspace.path())?;
    let stderr = stderr_text(&output);
    assert!(!output.status.success());
    assert!(
        stderr.contains("crates/vb_codegen/src/generated/workflow.rs"),
        "{stderr}"
    );
    assert!(
        stderr.contains("forbidden generated boundary token"),
        "{stderr}"
    );
    assert!(stderr.contains("serde_json"), "{stderr}");
    Ok(())
}
