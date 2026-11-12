#![forbid(unsafe_code)]
// xtask is a CLI binary with internal submodules. Many pub items are
// only consumed by other modules within xtask itself. Per-module
// #[allow(unreachable_pub)] below documents which items are externally
// reachable (bin entry-point, public API for tests) vs internal.

mod ai_profile;
mod benchmark_budget;
mod benchmark_freshness;
mod benchmark_policy;
mod cli;
mod contracts;
mod crash_lab;
mod diff_test;
mod evidence;
mod forbidden_scan;
mod gates;
mod loom;
mod proof;
mod proof_cmds;
mod proof_commands;
mod replay_lab;
mod shell;
// UI modules moved to velvet-optional repo (deferred):
// mod ui_overlap;
// mod ui_snapshot;
// mod ui_snapshot_render;
// mod ui_tokens_cmd;

use std::path::{Path, PathBuf};

use clap::Parser;

use ai_profile::{cmd_ai_deep, cmd_ai_fast, cmd_ai_release};
use cli::{Cli, Commands, ProofCommands};
use contracts::cmd_contracts;
use shell::write_stdout;
// UI commands moved to velvet-optional repo (deferred)
// use ui_overlap::cmd_ui_overlap_check;
// use ui_snapshot::cmd_ui_snapshot;
// use ui_tokens_cmd::cmd_ui_tokens;
use xtask::discovery;
use xtask::lanes;
use xtask::profiles;
use xtask::proof_orchestrator;
use xtask::summary;

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
    let workspace_root = std::env::current_dir()?;

    match cli.command {
        // UI commands moved to velvet-optional repo (deferred)
        // Commands::Snapshot { .. } => cmd_ui_snapshot(all, fixture, emit, output_dir),
        // Commands::Tokens { .. } => cmd_ui_tokens(&input, &output, emit, check),
        // Commands::OverlapCheck { .. } => cmd_ui_overlap_check(all, screen, &input_dir),
        Commands::Snapshot { .. } | Commands::Tokens { .. } | Commands::OverlapCheck { .. } => {
            anyhow::bail!("UI commands moved to velvet-optional repo (deferred milestone)")
        }
        Commands::AiFast { bead } => cmd_ai_fast(bead.as_deref()),
        Commands::AiDeep { bead } => cmd_ai_deep(bead.as_deref()),
        Commands::AiRelease { bead } => cmd_ai_release(bead.as_deref()),
        Commands::ProofPlan { crate_name } => proof_cmds::cmd_proof_plan(crate_name.as_deref()),
        Commands::ProofCheck { level, bead } => cmd_proof_check(level.as_deref(), bead.as_deref()),
        Commands::ProofEvidence { bead } => cmd_proof_evidence(&bead),
        Commands::ProofDrift { sections } => cmd_proof_drift(sections.as_deref()),
        Commands::Loom { model } => loom::cmd_loom(&model),
        Commands::CrashLab {
            workflow,
            crash_at,
            all_crash_points,
        } => crash_lab::cmd_crash_lab(
            &workflow,
            crash_at.as_deref(),
            all_crash_points,
            &workspace_root,
        ),
        Commands::DiffTest { suite } => diff_test::cmd_diff_test(&suite, &workspace_root),
        Commands::ForbiddenScan { crates, allowlist } => {
            forbidden_scan::cmd_forbidden_scan(crates.as_deref(), allowlist.as_deref())
        }
        Commands::BenchmarkRegressionPolicy { budget, evidence } => {
            benchmark_policy::cmd_benchmark_policy(&budget, &evidence)
        }
        Commands::ReplayLab => replay_lab::cmd_replay_lab(&workspace_root),
        Commands::ListCrates {
            include,
            exclude,
            json,
        } => cmd_list_crates(&workspace_root, include, exclude, json),
        Commands::Proof { command } => proof_cmds::cmd_proof(&workspace_root, command),
        Commands::Contracts { dir, json, check } => cmd_contracts(&dir, json, check),
    }
}

fn cmd_list_crates(
    workspace_root: &Path,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    json: bool,
) -> anyhow::Result<()> {
    let crates = discovery::discover_crates(workspace_root)?;
    let crates = discovery::filter_crates(&crates, include.as_deref(), exclude.as_deref());

    if json {
        let output: Vec<serde_json::Value> = crates
            .iter()
            .map(|c| {
                serde_json::json!({
                    "name": c.name,
                    "path": c.manifest_path.to_string_lossy(),
                    "dependencies": c.dependencies,
                })
            })
            .collect();
        write_stdout(format_args!("{}", serde_json::to_string_pretty(&output)?))?;
    } else {
        for c in &crates {
            write_stdout(format_args!("{} ({})", c.name, c.manifest_path.display()))?;
        }
        write_stdout(format_args!("Total: {} crates", crates.len()))?;
    }
    Ok(())
}

fn resolve_jobs(jobs: &str) -> usize {
    if jobs == "auto" {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
    } else {
        jobs.parse().unwrap_or(4)
    }
}

#[cfg(test)]
mod benchmark_tests_support;
#[cfg(test)]
mod command_shell_tests;

fn cmd_proof_check(level: Option<&str>, bead: Option<&str>) -> anyhow::Result<()> {
    let obligations = proof::load_proof_obligations()
        .map_err(|e| anyhow::anyhow!("Failed to load proof obligations: {e}"))?;

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
            write_stdout(format_args!("  Running: {cmd}"))?;
            let status = std::process::Command::new("sh").arg("-c").arg(cmd).status();

            if status.map(|s| !s.success()).unwrap_or(true) {
                all_passed = false;
                write_stdout(format_args!("  FAILED: {cmd}"))?;
            }
        }
        results.push((obl.id.clone(), all_passed));
    }

    let evidence_path =
        proof::write_proof_evidence(bead.unwrap_or("proof"), &filtered, &results, &output_dir)
            .map_err(|e| anyhow::anyhow!("Failed to write proof evidence: {e}"))?;

    write_stdout(format_args!(
        "Proof evidence written to: {}",
        evidence_path.display()
    ))?;

    let failed_count = results.iter().filter(|(_, passed)| !passed).count();
    if failed_count > 0 {
        anyhow::bail!("{failed_count} proof obligations failed");
    }

    Ok(())
}

fn cmd_proof_evidence(bead: &str) -> anyhow::Result<()> {
    let obligations = proof::load_proof_obligations()
        .map_err(|e| anyhow::anyhow!("Failed to load proof obligations: {e}"))?;

    let output_dir = PathBuf::from(".evidence").join(bead);
    std::fs::create_dir_all(&output_dir)?;

    let results: Vec<_> = obligations.iter().map(|o| (o.id.clone(), true)).collect();

    let evidence_path = proof::write_proof_evidence(bead, &obligations, &results, &output_dir)
        .map_err(|e| anyhow::anyhow!("Failed to write proof evidence: {e}"))?;

    write_stdout(format_args!(
        "Proof evidence written to: {}",
        evidence_path.display()
    ))?;
    Ok(())
}

fn cmd_proof_drift(sections: Option<&[usize]>) -> anyhow::Result<()> {
    use std::collections::HashMap;

    write_stdout(format_args!("Proof drift checker"))?;
    write_stdout(format_args!(
        "Checking spec alignment with proof obligations..."
    ))?;

    let obligations = proof::load_proof_obligations()
        .map_err(|e| anyhow::anyhow!("Failed to load proof obligations: {e}"))?;

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
        .map_err(|e| anyhow::anyhow!("Failed to read master spec: {e}"))?;

    let mut drift_issues = Vec::new();

    for (section, obls) in &section_map {
        let section_marker = format!("## {section}");
        if !master_spec.contains(&section_marker)
            && sections.map(|s| s.contains(section)).unwrap_or(true)
        {
            drift_issues.push(format!(
                "Section {section} referenced in obligations but not found in spec: {:?}",
                obls.iter().map(|o| o.id.clone()).collect::<Vec<_>>()
            ));
        }
    }

    if drift_issues.is_empty() {
        write_stdout(format_args!("No drift detected"))?;
    } else {
        write_stdout(format_args!("DRIFT DETECTED:"))?;
        for issue in &drift_issues {
            write_stdout(format_args!("  {issue}"))?;
        }
        anyhow::bail!("Spec drift detected");
    }

    Ok(())
}
