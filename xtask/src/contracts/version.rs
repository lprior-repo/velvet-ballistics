#![allow(unreachable_pub)]
#![forbid(unsafe_code)]

use super::ContractError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Parse a `schema_version` string into a validated version.
/// Returns the original string on success (validation already confirmed format).
pub fn parse_schema_version(s: &str) -> Result<String, ContractError> {
    if s.is_empty() {
        return Err(ContractError::MissingSchemaVersion);
    }
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 3 {
        return Err(ContractError::InvalidVersion {
            version: s.to_string(),
        });
    }
    for part in &parts {
        if part.is_empty() {
            return Err(ContractError::InvalidVersion {
                version: s.to_string(),
            });
        }
        if part.len() > 1 && part.starts_with('0') {
            return Err(ContractError::InvalidVersion {
                version: s.to_string(),
            });
        }
        if part.parse::<u32>().is_err() {
            return Err(ContractError::InvalidVersion {
                version: s.to_string(),
            });
        }
    }
    Ok(s.to_string())
}

/// Comparison result for two semver strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemverCmp {
    Equal,
    Less,
    Greater,
}

/// Compare two semver strings. Both must be valid "X.Y.Z" format.
pub fn compare_semver(a: &str, b: &str) -> Result<SemverCmp, String> {
    let pa: Vec<u32> = a.split('.').filter_map(|p| p.parse().ok()).collect();
    let pb: Vec<u32> = b.split('.').filter_map(|p| p.parse().ok()).collect();
    if pa.len() != 3 || pb.len() != 3 {
        return Err(format!("Invalid semver format: a='{a}', b='{b}'"));
    }
    for (a_v, b_v) in pa.iter().zip(pb.iter()) {
        if a_v < b_v {
            return Ok(SemverCmp::Less);
        }
        if a_v > b_v {
            return Ok(SemverCmp::Greater);
        }
    }
    Ok(SemverCmp::Equal)
}

/// Parse a `cue vet` exit code. Returns Ok(()) for exit 0, Err for non-zero.
pub fn parse_vet_exit_code(code: i32) -> Result<(), String> {
    if code == 0 {
        Ok(())
    } else {
        Err(format!("cue vet exited with code {code}"))
    }
}

/// Run `cue vet` on a single file and return (exit_code, stderr_output).
/// Returns an error if the cue binary is not found.
/// If `cwd` is provided, cue runs with that directory as its working directory
/// so that relative file paths are resolved correctly.
pub fn run_cue_vet(file: &Path, cwd: Option<&Path>) -> Result<(i32, String), String> {
    let mut cmd = std::process::Command::new("cue");
    cmd.args(["vet", file.to_string_lossy().as_ref()]);
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run cue vet: {e}"))?;
    let code = output.status.code().unwrap_or(1);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok((code, stderr))
}
