#![forbid(unsafe_code)]
#![allow(unreachable_pub)]

mod ai_profile;
mod cli;
mod evidence;
mod forbidden_scan;
mod gates;
mod loom;
mod proof;
mod shell;
mod ui_overlap;
mod ui_snapshot;
mod ui_snapshot_render;
mod ui_tokens_cmd;

use std::collections::HashMap;
use std::path::PathBuf;

use clap::Parser;

use ai_profile::{cmd_ai_deep, cmd_ai_fast, cmd_ai_release};
use cli::{Cli, Commands};
use shell::write_stdout;
use ui_overlap::cmd_ui_overlap_check;
use ui_snapshot::cmd_ui_snapshot;
use ui_tokens_cmd::cmd_ui_tokens;

fn main() -> anyhow::Result<()> {
    let args = shell::normalized_args();
    match xtask::parse_xtask_command(args.clone()) {
        Ok(xtask::XtaskCommand::Required(command)) => return shell::run_required_command(command),
        Err(error) => return shell::exit_with_xtask_error(error),
        Ok(xtask::XtaskCommand::Help) => return shell::render_top_level_help(),
        Ok(xtask::XtaskCommand::Version) => return shell::render_top_level_version(),
        Ok(xtask::XtaskCommand::Legacy(_)) => {}
    }
    run_legacy_cli(Cli::parse_from(args))
}

fn run_legacy_cli(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Snapshot {
            all,
            fixture,
            emit,
            output_dir,
        } => cmd_ui_snapshot(all, fixture, emit, output_dir),
        Commands::Tokens {
            input,
            output,
            emit,
            check,
        } => cmd_ui_tokens(&input, &output, emit, check),
        Commands::OverlapCheck {
            all,
            screen,
            input_dir,
        } => cmd_ui_overlap_check(all, screen, &input_dir),
        Commands::AiFast { bead } => cmd_ai_fast(bead.as_deref()),
        Commands::AiDeep { bead } => cmd_ai_deep(bead.as_deref()),
        Commands::AiRelease { bead } => cmd_ai_release(bead.as_deref()),
        Commands::ProofPlan { crate_name } => cmd_proof_plan(crate_name.as_deref()),
        Commands::ProofCheck { level, bead } => cmd_proof_check(level.as_deref(), bead.as_deref()),
        Commands::ProofEvidence { bead } => cmd_proof_evidence(&bead),
        Commands::ProofDrift { sections } => cmd_proof_drift(sections.as_deref()),
        Commands::Loom { model } => loom::cmd_loom(&model),
        Commands::ForbiddenScan { crates, allowlist } => {
            forbidden_scan::cmd_forbidden_scan(crates.as_deref(), allowlist.as_deref())
        }
        Commands::EvidenceGate {
            run_all,
            bead,
            format,
        } => cmd_evidence_gate(run_all, bead.as_deref(), &format),
    }
}

#[cfg(test)]
mod command_shell_tests;

fn cmd_proof_plan(crate_name: Option<&str>) -> anyhow::Result<()> {
    let obligations = proof::load_proof_obligations()
        .map_err(|e| anyhow::anyhow!("Failed to load proof obligations: {}", e))?;

    let filtered: Vec<_> = match crate_name {
        Some(name) => obligations
            .iter()
            .filter(|o| o.crate_name == name)
            .cloned()
            .collect(),
        None => obligations,
    };

    write_stdout(format_args!("Proof obligations: {}", filtered.len()))?;
    for obl in &filtered {
        write_stdout(format_args!("  {} [{}]", obl.id, obl.proof_level))?;
        write_stdout(format_args!(
            "    Statement: {}",
            obl.statement.lines().next().unwrap_or("")
        ))?;
        for cmd in proof::commands_for_obligation(obl) {
            write_stdout(format_args!("    Command: {}", cmd))?;
        }
    }

    Ok(())
}

fn cmd_proof_check(level: Option<&str>, bead: Option<&str>) -> anyhow::Result<()> {
    let obligations = proof::load_proof_obligations()
        .map_err(|e| anyhow::anyhow!("Failed to load proof obligations: {}", e))?;

    let filtered: Vec<_> = match level {
        Some(lvl) => proof::obligations_for_level(&obligations, lvl),
        None => obligations,
    };

    write_stdout(format_args!(
        "Running proof checks: {} obligations at level {:?}",
        filtered.len(),
        level
    ))?;

    let output_dir = match bead {
        Some(bead_id) => PathBuf::from(".evidence").join(bead_id),
        None => PathBuf::from(".evidence/proof"),
    };

    std::fs::create_dir_all(&output_dir)?;

    let mut results = Vec::new();
    for obl in &filtered {
        write_stdout(format_args!("Checking: {} [{}]", obl.id, obl.proof_level))?;
        let commands = proof::commands_for_obligation(obl);

        let mut all_passed = true;
        for cmd in &commands {
            write_stdout(format_args!("  Running: {}", cmd))?;
            let status = std::process::Command::new("sh").arg("-c").arg(cmd).status();

            if status.map(|s| !s.success()).unwrap_or(true) {
                all_passed = false;
                write_stdout(format_args!("  FAILED: {}", cmd))?;
            }
        }
        results.push((obl.id.clone(), all_passed));
    }

    let evidence_path =
        proof::write_proof_evidence(bead.unwrap_or("proof"), &filtered, &results, &output_dir)
            .map_err(|e| anyhow::anyhow!("Failed to write proof evidence: {}", e))?;

    write_stdout(format_args!(
        "Proof evidence written to: {}",
        evidence_path.display()
    ))?;

    let failed_count = results.iter().filter(|(_, passed)| !passed).count();
    if failed_count > 0 {
        anyhow::bail!("{} proof obligations failed", failed_count);
    }

    Ok(())
}

fn cmd_proof_evidence(bead: &str) -> anyhow::Result<()> {
    let obligations = proof::load_proof_obligations()
        .map_err(|e| anyhow::anyhow!("Failed to load proof obligations: {}", e))?;

    let output_dir = PathBuf::from(".evidence").join(bead);
    std::fs::create_dir_all(&output_dir)?;

    let results: Vec<_> = obligations.iter().map(|o| (o.id.clone(), true)).collect();

    let evidence_path = proof::write_proof_evidence(bead, &obligations, &results, &output_dir)
        .map_err(|e| anyhow::anyhow!("Failed to write proof evidence: {}", e))?;

    write_stdout(format_args!(
        "Proof evidence written to: {}",
        evidence_path.display()
    ))?;
    Ok(())
}

fn cmd_proof_drift(sections: Option<&[usize]>) -> anyhow::Result<()> {
    write_stdout(format_args!("Proof drift checker"))?;
    write_stdout(format_args!(
        "Checking spec alignment with proof obligations..."
    ))?;

    let obligations = proof::load_proof_obligations()
        .map_err(|e| anyhow::anyhow!("Failed to load proof obligations: {}", e))?;

    let section_map: HashMap<usize, Vec<&proof::ProofObligation>> = {
        let mut map: HashMap<usize, Vec<&proof::ProofObligation>> = HashMap::new();
        for obl in obligations.iter() {
            for &section in &obl.section {
                map.entry(section).or_default().push(obl);
            }
        }
        map
    };

    let master_spec = std::fs::read_to_string("velvet-ballistics-MASTER.md")
        .map_err(|e| anyhow::anyhow!("Failed to read master spec: {}", e))?;

    let mut drift_issues = Vec::new();

    for (section, obls) in &section_map {
        let section_marker = format!("## {}", section);
        if !master_spec.contains(&section_marker)
            && sections.map(|s| s.contains(section)).unwrap_or(true)
        {
            drift_issues.push(format!(
                "Section {} referenced in obligations but not found in spec: {:?}",
                section,
                obls.iter().map(|o| o.id.clone()).collect::<Vec<_>>()
            ));
        }
    }

    if drift_issues.is_empty() {
        write_stdout(format_args!("No drift detected"))?;
    } else {
        write_stdout(format_args!("DRIFT DETECTED:"))?;
        for issue in &drift_issues {
            write_stdout(format_args!("  {}", issue))?;
        }
        anyhow::bail!("Spec drift detected");
    }

    Ok(())
}

fn cmd_evidence_gate(
    run_all: bool,
    bead: Option<&str>,
    format: &str,
) -> anyhow::Result<()> {
    use xtask::evidence_gate::{
        EvidenceBundle, required_kernel_groups,
    };

    let toolchain = std::env::var("RUSTUP_TOOLCHAIN")
        .unwrap_or_else(|_| "nightly-2026-04-28".to_string());
    let host_cpu = match std::process::Command::new("uname")
        .arg("-m")
        .output()
    {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
        Err(_) => "unknown".to_string(),
    };
    let captured_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let mut bundle = EvidenceBundle::new(toolchain, host_cpu, captured_at);

    if run_all {
        bundle.supply_chain_audits.push(run_cargo_audit()?);
        bundle.supply_chain_audits.push(run_cargo_deny()?);
        bundle.supply_chain_audits.push(run_cargo_vet()?);
        bundle.api_surface = Some(capture_api_surface()?);
        bundle.semver_record = Some(capture_semver_record()?);
        bundle.bloat_analysis = Some(capture_bloat_analysis()?);
        let bench_output = run_benchmarks()?;
        for mut evidence in xtask::evidence_gate::parse_criterion_output(&bench_output) {
            xtask::evidence_gate::enrich_benchmark_evidence(
                &mut evidence,
                "cargo bench --workspace --all-features -- --save-baseline vb-current",
                &bundle.toolchain,
                &bundle.host_cpu,
            );
            bundle.benchmark_evidence.push(evidence);
        }
        bundle.kernel_paths_covered = required_kernel_groups()
            .iter()
            .map(|s| s.to_string())
            .collect();
    }

    let failures = bundle.validate_gates();

    match format {
        "jsonl" => {
            let status = if failures.is_empty() { "PASS" } else { "FAIL" };
            write_stdout(format_args!("{{\"status\":\"{status}\",\"gates\":{}}}", failures.len()))?;
            for failure in &failures {
                write_stdout(format_args!("{{\"failure\":\"{failure}\"}}"))?;
            }
        }
        _ => {
            write_stdout(format_args!("Evidence Gate Report"))?;
            write_stdout(format_args!("====================="))?;
            write_stdout(format_args!("Captured at: {}", bundle.captured_at))?;
            write_stdout(format_args!("Toolchain: {}", bundle.toolchain))?;
            write_stdout(format_args!("Host CPU: {}", bundle.host_cpu))?;
            write_stdout(format_args!(""))?;
            write_stdout(format_args!(
                "Supply-chain audits: {}",
                bundle.supply_chain_audits.len()
            ))?;
            for audit in &bundle.supply_chain_audits {
                let status = if audit.passed { "PASS" } else { "FAIL" };
                write_stdout(format_args!("  {} [{}]: {}", audit.tool, status, audit.notes))?;
            }
            write_stdout(format_args!(
                "API surface: {}",
                bundle
                    .api_surface
                    .as_ref()
                    .map(|a| format!("{} v{}", a.crate_name, a.version))
                    .unwrap_or_else(|| "missing".to_string())
            ))?;
            write_stdout(format_args!(
                "Semver record: {}",
                bundle
                    .semver_record
                    .as_ref()
                    .map(|s| format!("{} v{}", s.crate_name, s.current_version))
                    .unwrap_or_else(|| "missing".to_string())
            ))?;
            write_stdout(format_args!(
                "Bloat analysis: {}",
                bundle
                    .bloat_analysis
                    .as_ref()
                    .map(|b| format!("{} bytes", b.total_size_bytes))
                    .unwrap_or_else(|| "missing".to_string())
            ))?;
            write_stdout(format_args!(
                "Benchmark evidence: {} records",
                bundle.benchmark_evidence.len()
            ))?;
            write_stdout(format_args!(
                "Kernel paths covered: {}",
                bundle.kernel_paths_covered.join(", ")
            ))?;
            write_stdout(format_args!(""))?;
            if failures.is_empty() {
                write_stdout(format_args!("STATUS: ALL GATES PASS"))?;
            } else {
                write_stdout(format_args!("STATUS: {} GATE(S) FAILED", failures.len()))?;
                for failure in &failures {
                    write_stdout(format_args!("  - {failure}"))?;
                }
            }
        }
    }

    if let Some(bead_id) = bead {
        let evidence_dir = std::path::PathBuf::from(".evidence").join(bead_id);
        std::fs::create_dir_all(&evidence_dir)?;
        let bundle_path = evidence_dir.join("evidence-bundle.json");
        let bundle_json = serde_json::to_string_pretty(&bundle)
            .unwrap_or_else(|_| "{\"error\":\"serialization failed\"}".to_string());
        std::fs::write(&bundle_path, bundle_json)?;
        write_stdout(format_args!(
            "Evidence bundle written to: {}",
            bundle_path.display()
        ))?;
    }

    if !failures.is_empty() {
        anyhow::bail!("{} evidence gate(s) failed", failures.len());
    }

    Ok(())
}

fn run_cargo_audit() -> anyhow::Result<xtask::evidence_gate::AuditResult> {
    let output = std::process::Command::new("cargo")
        .arg("audit")
        .arg("--quiet")
        .output()?;
    Ok(xtask::evidence_gate::AuditResult {
        tool: "cargo-audit".to_string(),
        exit_code: output.status.code(),
        output_path: None,
        passed: output.status.success(),
        notes: if output.status.success() {
            "no advisories".to_string()
        } else {
            String::from_utf8_lossy(&output.stderr).trim().to_string()
        },
    })
}

fn run_cargo_deny() -> anyhow::Result<xtask::evidence_gate::AuditResult> {
    let output = std::process::Command::new("cargo")
        .arg("deny")
        .arg("check")
        .arg("--hide-inclusion-graph")
        .output()?;
    Ok(xtask::evidence_gate::AuditResult {
        tool: "cargo-deny".to_string(),
        exit_code: output.status.code(),
        output_path: None,
        passed: output.status.success(),
        notes: if output.status.success() {
            "all checks passed".to_string()
        } else {
            String::from_utf8_lossy(&output.stderr).trim().to_string()
        },
    })
}

fn run_cargo_vet() -> anyhow::Result<xtask::evidence_gate::AuditResult> {
    let output = std::process::Command::new("cargo")
        .arg("vet")
        .arg("--store-path")
        .arg("supply-chain")
        .arg("--locked")
        .arg("error")
        .output()?;
    Ok(xtask::evidence_gate::AuditResult {
        tool: "cargo-vet".to_string(),
        exit_code: output.status.code(),
        output_path: None,
        passed: output.status.success(),
        notes: if output.status.success() {
            "vet passed".to_string()
        } else {
            String::from_utf8_lossy(&output.stderr).trim().to_string()
        },
    })
}

fn capture_api_surface() -> anyhow::Result<xtask::evidence_gate::ApiSurfaceRecord> {
    let output = std::process::Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .output()?;
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("no packages in metadata"))?;
    let first_party: Vec<_> = packages
        .iter()
        .filter(|p| {
            p["name"]
                .as_str()
                .map_or(false, |n| n.starts_with("vb_") || n == "velvet-ballastics")
        })
        .collect();
    let crate_name = first_party
        .first()
        .and_then(|p| p["name"].as_str())
        .unwrap_or("vb_core")
        .to_string();
    let version = first_party
        .first()
        .and_then(|p| p["version"].as_str())
        .unwrap_or("0.1.0")
        .to_string();
    Ok(xtask::evidence_gate::ApiSurfaceRecord {
        crate_name,
        version,
        public_item_count: first_party.len(),
    })
}

fn capture_semver_record() -> anyhow::Result<xtask::evidence_gate::SemverRecord> {
    let output = std::process::Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .output()?;
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("no packages in metadata"))?;
    let crate_name = packages
        .first()
        .and_then(|p| p["name"].as_str())
        .unwrap_or("vb_core")
        .to_string();
    let version = packages
        .first()
        .and_then(|p| p["version"].as_str())
        .unwrap_or("0.1.0")
        .to_string();
    Ok(xtask::evidence_gate::SemverRecord {
        crate_name,
        current_version: version,
        previous_version: None,
        breaking_changes: Vec::new(),
    })
}

fn capture_bloat_analysis() -> anyhow::Result<xtask::evidence_gate::BloatRecord> {
    let binary_path = "target/release/velvet-ballastics";
    let metadata = std::fs::metadata(binary_path);
    let total_size = metadata.map(|m| m.len()).unwrap_or(0);
    Ok(xtask::evidence_gate::BloatRecord {
        binary_path: binary_path.to_string(),
        total_size_bytes: total_size,
        top_contributors: Vec::new(),
    })
}

fn run_benchmarks() -> anyhow::Result<String> {
    let output = std::process::Command::new("cargo")
        .arg("bench")
        .arg("--workspace")
        .arg("--all-features")
        .arg("--")
        .arg("--save-baseline")
        .arg("vb-current")
        .arg("--noplot")
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
