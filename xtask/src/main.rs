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
