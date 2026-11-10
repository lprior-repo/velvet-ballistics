#![allow(unreachable_pub)]
#![forbid(unsafe_code)]

use super::{
    ContractError, ContractFile, ContractKind, DiscoveryReport, ReportSummary, SemverCmp,
    VersionViolation, compare_semver, parse_schema_version, parse_vet_exit_code, run_cue_vet,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Walk `contracts_dir` for `.cue` files and validate each one.
///
/// For each file:
/// 1. Run cue vet and collect errors.
/// 2. Attempt to extract kind and schema_version from the file content.
/// 3. Validate extracted values.
#[allow(clippy::as_conversions)]
pub fn discover_contracts(contracts_dir: &Path) -> Result<DiscoveryReport, String> {
    if !contracts_dir.exists() {
        return Err(format!(
            "contracts directory does not exist: {}",
            contracts_dir.display()
        ));
    }

    if !contracts_dir.is_dir() {
        return Err(format!(
            "contracts path is not a directory: {}",
            contracts_dir.display()
        ));
    }

    // Collect all discoverable contract .cue files recursively.
    let mut cue_files: Vec<PathBuf> = Vec::new();
    collect_cue_files(contracts_dir, contracts_dir, &mut cue_files)
        .map_err(|e| format!("Failed to walk contracts directory: {e}"))?;

    // Sort for deterministic output (INV-005).
    cue_files.sort();
    let contract_files: Vec<PathBuf> = cue_files
        .into_iter()
        .filter(|path| is_contract_discovery_candidate(path))
        .collect();

    let mut files = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut summary = ReportSummary::new();
    summary.total = u32::try_from(contract_files.len())
        .map_err(|_| String::from("too many contract files to summarize"))?;

    for file_rel in &contract_files {
        // Build absolute path for file I/O; keep relative path for reports.
        let file_abs = contracts_dir.join(file_rel);
        let result = validate_single_file(&file_abs, contracts_dir);
        match result {
            Ok(contract_file) => {
                files.push(contract_file);
                #[allow(clippy::arithmetic_side_effects)]
                {
                    summary.valid += 1;
                }
            }
            Err((_file_path, validation_errors)) => {
                #[allow(clippy::arithmetic_side_effects)]
                {
                    summary.invalid += 1;
                }
                for err in &validation_errors {
                    let key = err.to_string();
                    #[allow(clippy::arithmetic_side_effects)]
                    summary
                        .errors_by_kind
                        .entry(key)
                        .and_modify(|c| *c += 1)
                        .or_insert(1);
                }
                for err in &validation_errors {
                    errors.push(err.to_string());
                }
            }
        }
    }

    // Sort files by path (INV-005).
    files.sort_by(|a, b| a.path.cmp(&b.path));

    // Monotonicity gate: schema versions must be non-decreasing across files.
    let mut version_violations = Vec::new();
    #[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
    for i in 1..files.len() {
        let prev_ver = &files[i - 1].schema_version;
        let curr_ver = &files[i].schema_version;
        match compare_semver(prev_ver, curr_ver) {
            Ok(SemverCmp::Greater) => {
                // Monotonicity breach: previous file has a higher version.
                let breach = ContractError::VersionMonotonicityBreach {
                    file: files[i].path.display().to_string(),
                    expected: files[i - 1].schema_version.clone(),
                    actual: curr_ver.clone(),
                };
                errors.push(breach.to_string());
                version_violations.push(VersionViolation {
                    file: files[i].path.clone(),
                    expected: files[i - 1].schema_version.clone(),
                    actual: curr_ver.clone(),
                    detail: format!(
                        "version {} < previous version {} (must be non-decreasing)",
                        curr_ver, prev_ver
                    ),
                });
            }
            Err(_) => {
                // Invalid semver in one of the files — treat as error.
                let err_msg = format!("INVALID_VERSION: {}", prev_ver);
                if !errors.contains(&err_msg) {
                    errors.push(err_msg);
                }
            }
            _ => {} // Equal or Less — monotonicity OK
        }
    }

    // Deduplicate version violations by file path.
    let mut version_violations_map: BTreeMap<PathBuf, VersionViolation> = BTreeMap::new();
    for v in version_violations {
        version_violations_map.entry(v.file.clone()).or_insert(v);
    }
    let mut version_violations_vec: Vec<_> = version_violations_map.into_values().collect();
    version_violations_vec.sort_by(|a, b| a.file.cmp(&b.file));

    summary.version_violations = version_violations_vec;

    // Sort errors for determinism.
    errors.sort();

    Ok(DiscoveryReport {
        files,
        errors,
        summary,
    })
}

/// Recursively collect all .cue file paths under root, relative to base.
fn collect_cue_files(base: &Path, current: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !current.is_dir() {
        return Ok(());
    }
    let mut entries: Vec<_> = current.read_dir()?.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            collect_cue_files(base, &path, out)?;
        } else if path.extension().map(|ext| ext == "cue").unwrap_or(false) {
            let relative = path.strip_prefix(base).unwrap_or(&path);
            out.push(relative.to_path_buf());
        }
    }
    Ok(())
}

pub(crate) fn is_contract_discovery_candidate(path: &Path) -> bool {
    match path.file_name().and_then(|name| name.to_str()) {
        Some("manifest.cue") => false,
        Some(_) | None => true,
    }
}

/// Validate a single .cue file: cue vet + field extraction.
///
/// `file_path` must be an absolute path (used for file I/O).
/// `contracts_dir` is used as the working directory for `cue vet`
/// so that relative paths are resolved correctly.
fn validate_single_file(
    file_path: &Path,
    contracts_dir: &Path,
) -> Result<ContractFile, (PathBuf, Vec<ContractError>)> {
    let mut vet_errors: Vec<ContractError> = Vec::new();

    // Run cue vet.
    // Use the relative path (relative to contracts_dir) for cue, with
    // contracts_dir as CWD, so cue resolves the path correctly.
    let relative_path = file_path.strip_prefix(contracts_dir).unwrap_or(file_path);
    let (exit_code, _stderr) = match run_cue_vet(relative_path, Some(contracts_dir)) {
        Ok(result) => result,
        Err(_e) => {
            vet_errors.push(ContractError::CueVetFailed {
                file: relative_path.to_string_lossy().to_string(),
            });
            // Continue with field parsing even if cue not available.
            (0, String::new())
        }
    };

    if parse_vet_exit_code(exit_code).is_err() {
        vet_errors.push(ContractError::CueVetFailed {
            file: relative_path.to_string_lossy().to_string(),
        });
    }

    // Parse file content for kind and schema_version.
    let content = match std::fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(_) => {
            vet_errors.push(ContractError::CueVetFailed {
                file: relative_path.to_string_lossy().to_string(),
            });
            return Err((relative_path.to_path_buf(), vet_errors));
        }
    };

    // Extract kind from the CUE content.
    let kind_str = extract_kind(&content);

    // Extract schema_version from the CUE content.
    let schema_version = extract_schema_version(&content);

    // Validate kind.
    let kind = match &kind_str {
        Some(k) => match ContractKind::parse(k) {
            Ok(kind) => kind,
            Err(unrecognised) => {
                vet_errors.push(ContractError::InvalidKind {
                    kind: unrecognised.clone(),
                });
                return Err((relative_path.to_path_buf(), vet_errors));
            }
        },
        None => {
            vet_errors.push(ContractError::MissingSchemaVersion);
            return Err((relative_path.to_path_buf(), vet_errors));
        }
    };

    // Validate schema_version.
    let sv = match parse_schema_version(&schema_version) {
        Ok(sv) => sv,
        Err(e) => {
            vet_errors.push(e);
            return Err((relative_path.to_path_buf(), vet_errors));
        }
    };

    Ok(ContractFile {
        path: relative_path.to_path_buf(),
        schema_version: sv,
        kind,
        vet_errors: Vec::new(),
    })
}

/// Extract the kind value from CUE content by scanning for the `kind:` field.
pub(crate) fn extract_kind(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(after_colon) = trimmed.strip_prefix("kind:") {
            let val = after_colon.trim();
            let val = val.trim_matches('"');
            return Some(val.to_string());
        }
    }
    None
}

/// Extract the schema_version value from CUE content.
pub(crate) fn extract_schema_version(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(after_colon) = trimmed.strip_prefix("schema_version:") {
            let val = after_colon.trim();
            let val = val.trim_matches('"');
            return val.to_string();
        }
    }
    String::new()
}
