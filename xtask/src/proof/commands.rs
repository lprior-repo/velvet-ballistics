#[cfg(test)]
use super::types::RequiredProof;
use super::types::{FuzzField, ObligationStatus, ProofEvidence, ProofObligation};
use crate::proof_commands::{crash_lab_command, diff_test_command, nextest_command};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct ProofObligationsFile {
    obligations: Vec<ProofObligation>,
}

pub fn load_proof_obligations() -> Result<Vec<ProofObligation>, String> {
    let path = PathBuf::from("contracts/proof_obligations.yaml");
    if !path.exists() {
        return Err(format!(
            "Proof obligations file not found: {}",
            path.display()
        ));
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let file: ProofObligationsFile = serde_saphyr::from_str(&content).map_err(|e| e.to_string())?;
    Ok(file.obligations)
}

#[allow(dead_code)]
pub fn obligations_for_files(
    obligations: &[ProofObligation],
    changed_files: &[&str],
) -> Vec<ProofObligation> {
    let changed_set: std::collections::HashSet<_> = changed_files.iter().collect();
    obligations
        .iter()
        .filter(|obl| obl.files.iter().any(|f| changed_set.contains(&f.as_str())))
        .cloned()
        .collect()
}

pub fn obligations_for_level(obligations: &[ProofObligation], level: &str) -> Vec<ProofObligation> {
    obligations
        .iter()
        .filter(|obl| obl.proof_level == level)
        .cloned()
        .collect()
}

/// Derive the `--harness` filter from a `required: kani:` file reference.
/// The registry names the proof file (`kani_step_budget.rs`) or a
/// `file.rs::harness` pair; Kani matches filters against fully qualified
/// harness names, which never contain `.rs`.
fn kani_harness_filter(reference: &str) -> &str {
    let bare = match reference.rsplit("::").next() {
        Some(part) => part,
        None => reference,
    };
    match bare.strip_suffix(".rs") {
        Some(stem) => stem,
        None => bare,
    }
}

pub fn commands_for_obligation(obl: &ProofObligation) -> Vec<String> {
    let mut cmds = Vec::new();

    if let Some(ref kani) = obl.required.kani {
        cmds.push(format!(
            "cargo kani --package {} --harness {}",
            obl.crate_name,
            kani_harness_filter(kani)
        ));
    }
    if let Some(ref verus) = obl.required.verus {
        cmds.push(format!("verus {}", verus));
    }
    for test in &obl.required.tests {
        cmds.push(nextest_command(&obl.crate_name, test));
    }
    for test in &obl.required.property_tests {
        cmds.push(nextest_command(&obl.crate_name, test));
    }
    for test in &obl.required.differential_tests {
        cmds.push(nextest_command(&obl.crate_name, test));
    }
    if let Some(ref fuzz) = obl.required.fuzz {
        match fuzz {
            FuzzField::Single(s) => cmds.push(format!("cargo fuzz run {}", s)),
            FuzzField::Multiple(v) => {
                for f in v {
                    cmds.push(format!("cargo fuzz run {}", f));
                }
            }
        }
    }
    if let Some(ref tla) = obl.required.tla {
        cmds.push(format!("tla2tools {}", tla));
    }
    if let Some(ref loom) = obl.required.loom {
        cmds.push(format!("cargo xtask loom --model {}", loom));
    }
    for entry in &obl.required.crash_lab {
        cmds.push(crash_lab_command(entry));
    }
    for entry in &obl.required.diff_test {
        cmds.push(diff_test_command(entry));
    }

    cmds
}

#[allow(dead_code)]
pub fn proof_levels() -> Vec<(&'static str, &'static str)> {
    vec![
        ("L0", "Mechanical scan / lint / CI evidence"),
        ("L1", "Unit + property + differential tests"),
        ("L2", "Fuzz / mutation / crash-lab evidence"),
        ("L3", "Bounded model checking: Kani / Loom"),
        ("L4", "Deductive Rust proof: Verus / Creusot"),
        ("L5", "Proof assistant model: Lean via Aeneas"),
        ("L6", "Operational evidence: benchmark, replay, recovery"),
    ]
}

pub fn write_proof_evidence(
    bead_id: &str,
    obligations: &[ProofObligation],
    results: &[(String, bool)],
    output_dir: &Path,
) -> Result<PathBuf, String> {
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let obligations_status: Vec<ObligationStatus> = results
        .iter()
        .map(|(id, passed)| {
            let Some(obl) = obligations.iter().find(|o| o.id == *id) else {
                return Err(format!("Obligation not found: {id}"));
            };
            Ok(ObligationStatus {
                id: id.clone(),
                status: if *passed {
                    "pass".to_string()
                } else {
                    "fail".to_string()
                },
                level: obl.proof_level.clone(),
                commands: commands_for_obligation(obl),
                log: None,
            })
        })
        .collect::<Result<Vec<ObligationStatus>, String>>()?;

    let evidence = ProofEvidence {
        kind: "ProofEvidence".to_string(),
        bead: bead_id.to_string(),
        commit,
        obligations: obligations_status,
        remaining_assumptions: vec![
            "Fjall fsync correctness treated as external dependency".to_string(),
        ],
        verified: HashMap::new(),
    };

    let yaml = serde_saphyr::to_string(&evidence).map_err(|e| e.to_string())?;

    let path = output_dir.join("proof.yaml");
    std::fs::write(&path, &yaml).map_err(|e| e.to_string())?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kani_harness_filter_strips_rs_suffix() {
        assert_eq!(
            kani_harness_filter("kani_step_budget.rs"),
            "kani_step_budget"
        );
        assert_eq!(
            kani_harness_filter("kani_taint_lattice.rs"),
            "kani_taint_lattice"
        );
    }

    #[test]
    fn kani_harness_filter_takes_harness_after_separator() {
        assert_eq!(
            kani_harness_filter("kani_digest_repeat.rs::kani_repeat_max_attempts_consumed"),
            "kani_repeat_max_attempts_consumed"
        );
    }

    #[test]
    fn kani_harness_filter_leaves_bare_names_unchanged() {
        assert_eq!(kani_harness_filter("kani_step_budget"), "kani_step_budget");
        assert_eq!(kani_harness_filter(""), "");
    }

    #[test]
    fn commands_for_obligation_derives_kani_filter_from_file_reference() {
        let obligation = ProofObligation {
            id: "VB-CORE-BUDGET-003".to_string(),
            section: vec![47],
            statement: "Step budget bound".to_string(),
            proof_level: "L3".to_string(),
            crate_name: "vb_core".to_string(),
            files: vec!["crates/vb_core/src/budget.rs".to_string()],
            required: RequiredProof {
                verus: None,
                kani: Some("kani_step_budget.rs".to_string()),
                tests: vec![],
                fuzz: None,
                tla: None,
                loom: None,
                property_tests: vec![],
                differential_tests: vec![],
                crash_lab: vec![],
                diff_test: vec![],
            },
            commands: vec![],
        };
        assert_eq!(
            commands_for_obligation(&obligation),
            vec!["cargo kani --package vb_core --harness kani_step_budget".to_string()]
        );
    }

    #[test]
    fn commands_for_obligation_names_package_for_vb_compile_kani() {
        let obligation = ProofObligation {
            id: "VB-COMPILE-DIGEST-REPEAT-001".to_string(),
            section: vec![47],
            statement: "Repeat max attempts changes digest".to_string(),
            proof_level: "L3".to_string(),
            crate_name: "vb_compile".to_string(),
            files: vec!["crates/vb_compile/src/digest.rs".to_string()],
            required: RequiredProof {
                verus: None,
                kani: Some("kani_digest_repeat.rs::kani_repeat_max_attempts_consumed".to_string()),
                tests: vec![],
                fuzz: None,
                tla: None,
                loom: None,
                property_tests: vec![],
                differential_tests: vec![],
                crash_lab: vec![],
                diff_test: vec![],
            },
            commands: vec![],
        };
        assert_eq!(
            commands_for_obligation(&obligation),
            vec![
                "cargo kani --package vb_compile --harness kani_repeat_max_attempts_consumed"
                    .to_string()
            ]
        );
    }
}
