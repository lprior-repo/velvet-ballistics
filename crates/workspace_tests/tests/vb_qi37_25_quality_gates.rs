#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use vb_cli::naming_scan::{
    AllowlistPolicy, CanonicalEntry, CanonicalNameKind, CanonicalSpellingTable, LegacyAllowRule,
    LineNumber, NamingFinding, NamingScanError, RawScanConfig, RepoPath, ScanConfig, ScanInput,
    SpellingClass, scan_file, validate_scan_config,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const MEMBERS: [(&str, &str); 19] = [
    ("crates/vb_boundary_inventory", "vb_boundary_inventory"),
    ("crates/vb_core", "vb_core"),
    ("crates/vb_yaml", "vb_yaml"),
    ("crates/vb_validate", "vb_validate"),
    ("crates/vb_expr", "vb_expr"),
    ("crates/vb_compile", "vb_compile"),
    ("crates/vb_storage", "vb_storage"),
    ("crates/vb_runtime", "vb_runtime"),
    ("crates/vb_doc", "vb_doc"),
    ("crates/vb_ipc", "vb_ipc"),
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
    for (member, package_name) in MEMBERS {
        write_manifest(root, member, package_name)?;
    }
    Ok(dir)
}

fn write_manifest(root: &Path, member: &str, package_name: &str) -> Result<(), std::io::Error> {
    let mut manifest =
        format!("[package]\nname = \"{package_name}\"\nedition = \"2024\"\n\n[dependencies]\n");
    if member == "crates/vb_cli" {
        manifest.push_str("\n[lib]\nname = \"vb_cli\"\npath = \"src/lib.rs\"\n\n[[bin]]\nname = \"velvet-ballastics\"\npath = \"src/main.rs\"\n");
    }
    if member == "crates/vb_core" {
        manifest.push_str("\n[features]\ndefault = []\ngenerated = []\nbench = []\nvolatile = []\ntest-util = []\n");
    }
    if member == "crates/vb_validate" {
        manifest.push_str("\n[features]\ndefault = []\nverus = []\n");
    }
    if member == "crates/vb_ui_snapshot" {
        manifest.push_str("\n[features]\ndefault = [\"std\"]\nstd = []\n");
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

#[test]
fn package_name_drift_reports_exact_member_and_expected_name() -> TestResult {
    let dir = workspace()?;
    write_manifest(dir.path(), "crates/vb_cli", "velvet-ballistics")?;
    let output = run_assertions(dir.path())?;
    assert!(!output.status.success());
    assert_eq!(
        stderr(&output),
        "crates/vb_cli/Cargo.toml: package.name expected \"velvet-ballastics\", got Some(\"velvet-ballistics\")\n"
    );
    Ok(())
}

#[test]
fn binary_alias_reports_exact_allowed_binary_set() -> TestResult {
    let dir = workspace()?;
    let manifest = "[package]\nname = \"velvet-ballastics\"\nedition = \"2024\"\n\n[dependencies]\n\n[[bin]]\nname = \"vb\"\npath = \"src/main.rs\"\n";
    write_file(&dir.path().join("crates/vb_cli/Cargo.toml"), manifest)?;
    let output = run_assertions(dir.path())?;
    assert!(!output.status.success());
    assert_eq!(
        stderr(&output),
        "crates/vb_cli/Cargo.toml: bin names missing [\"velvet-ballastics\"]\ncrates/vb_cli/Cargo.toml: bin names unexpected [\"vb\"]\n"
    );
    Ok(())
}

#[test]
fn feature_drift_reports_exact_expected_feature_set() -> TestResult {
    let dir = workspace()?;
    let manifest = "[package]\nname = \"vb_core\"\nedition = \"2024\"\n\n[features]\ndefault = []\ngenerated = []\nbench = []\njson = []\n";
    write_file(&dir.path().join("crates/vb_core/Cargo.toml"), manifest)?;
    let output = run_assertions(dir.path())?;
    assert!(!output.status.success());
    assert_eq!(
        stderr(&output),
        "crates/vb_core/Cargo.toml: features missing [\"test-util\", \"volatile\"]\ncrates/vb_core/Cargo.toml: features unexpected [\"json\"]\ncrates/vb_core/Cargo.toml: forbidden feature names [\"json\"]\n"
    );
    Ok(())
}

fn scan_config() -> ScanConfig {
    ScanConfig {
        canonical_table: CanonicalSpellingTable {
            product: "velvet-ballastics".to_string(),
            binary: "velvet-ballastics".to_string(),
            package: "velvet-ballastics".to_string(),
            bead_rig: "velvet-ballastics".to_string(),
            crate_module: "vb_cli".to_string(),
            bead_database: "vb_cli".to_string(),
            language_version: "velvet-ballastics/v1".to_string(),
        },
        allowlist_policy: AllowlistPolicy::Exact(vec![
            LegacyAllowRule::RepositoryPath {
                path: "https://github.com/priorlewis43/velvet-ballistics".to_string(),
            },
            LegacyAllowRule::MasterFilename {
                filename: "velvet-ballistics-MASTER.md".to_string(),
            },
            LegacyAllowRule::MigrationReference {
                label: "MIGRATION-REFERENCE".to_string(),
                artifact: "external-preexisting-artifact".to_string(),
                legacy_text: "velvet-ballistics".to_string(),
            },
        ]),
        scan_patterns: vec!["velvet-ballistics".to_string()],
        excluded_path_rules: vec![".git/**".to_string()],
        config_fingerprint: "vb-qi37.25-test".to_string(),
        report_destination: None,
    }
}

#[test]
fn spelling_gate_rejects_legacy_spelling_outside_exact_allowlist() {
    let result = scan_file(
        ScanInput::Text {
            path: RepoPath::new("docs/new.md"),
            contents: "new velvet-ballistics mention\n".to_string(),
        },
        &scan_config(),
    );
    assert_eq!(
        result,
        Ok(vec![NamingFinding {
            path: RepoPath::new("docs/new.md"),
            line: LineNumber::new(1),
            column: vb_cli::naming_scan::ColumnNumber::new(5),
            spelling_class: SpellingClass::LegacyProjectSpelling,
            remediation: "velvet-ballastics".to_string(),
        }])
    );
}

#[test]
fn broad_substring_allowlist_is_configuration_error() {
    let raw = RawScanConfig {
        canonical_entries: vec![
            CanonicalEntry::new(CanonicalNameKind::Product, "velvet-ballastics"),
            CanonicalEntry::new(CanonicalNameKind::Binary, "velvet-ballastics"),
            CanonicalEntry::new(CanonicalNameKind::Package, "velvet-ballastics"),
            CanonicalEntry::new(CanonicalNameKind::BeadRig, "velvet-ballastics"),
            CanonicalEntry::new(CanonicalNameKind::CrateModule, "vb_cli"),
            CanonicalEntry::new(CanonicalNameKind::BeadDatabase, "vb_cli"),
            CanonicalEntry::new(CanonicalNameKind::LanguageVersion, "velvet-ballastics/v1"),
        ],
        legacy_allowlist: vec![LegacyAllowRule::Substring {
            needle: "velvet-ballistics".to_string(),
        }],
        scan_patterns: vec!["velvet-ballistics".to_string()],
        excluded_path_rules: Vec::new(),
        workspace_root: PathBuf::from("."),
        report_destination: None,
    };
    assert_eq!(
        validate_scan_config(raw),
        Err(NamingScanError::InvalidConfiguration {
            reason: "substring allowlist rule: velvet-ballistics".to_string(),
        })
    );
}
