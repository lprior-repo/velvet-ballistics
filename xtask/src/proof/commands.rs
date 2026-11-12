#![allow(unreachable_pub)]
//! Command derivation for proof obligations.

use super::{FuzzField, ProofObligation};
use crate::proof_commands::{crash_lab_command, diff_test_command, nextest_command};

/// Derive the `--harness` filter from a `required: kani:` file reference.
/// The registry names the proof file (`kani_step_budget.rs`) or a
/// `file.rs::harness` pair; Kani matches filters against fully qualified
/// harness names, which never contain `.rs`.
pub(super) fn kani_harness_filter(reference: &str) -> &str {
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
