use super::*;

pub(super) fn cmd_proof(workspace_root: &Path, command: ProofCommands) -> anyhow::Result<()> {
    match command {
        ProofCommands::List { crate_name, json } => {
            cmd_proof_list(workspace_root, crate_name.as_deref(), json)
        }
        ProofCommands::Run {
            profile,
            jobs,
            exclude,
            include,
            fail_fast,
            keep_going,
            timeout,
            dry_run,
            json,
        } => {
            let cfg = ProofRunConfig {
                profile_str: profile,
                jobs_str: jobs,
                exclude,
                include,
                fail_fast,
                timeout,
                dry_run,
                json,
            };
            cmd_proof_run(workspace_root, &cfg, keep_going)
        }
        ProofCommands::Crate {
            crate_name,
            lanes,
            jobs,
            fail_fast,
            timeout,
            dry_run,
            json,
        } => {
            let cfg = ProofCrateConfig {
                lane_names: lanes,
                jobs_str: jobs,
                fail_fast,
                timeout,
                dry_run,
                json,
            };
            cmd_proof_crate(workspace_root, &crate_name, &cfg)
        }
        ProofCommands::Affected {
            base,
            jobs,
            fail_fast,
            timeout,
            dry_run,
            json,
        } => {
            let cfg = ProofAffectedConfig {
                jobs_str: jobs,
                fail_fast,
                timeout,
                dry_run,
                json,
            };
            cmd_proof_affected(workspace_root, &base, &cfg)
        }
    }
}

fn cmd_proof_list(
    workspace_root: &Path,
    crate_name: Option<&str>,
    json: bool,
) -> anyhow::Result<()> {
    let crates = discovery::discover_crates(workspace_root)?;
    let available_lanes = lanes::detect_available_lanes(workspace_root);

    let filtered: Vec<_> = match crate_name {
        Some(name) => crates.into_iter().filter(|c| c.name == name).collect(),
        None => crates,
    };

    if json {
        let output: Vec<serde_json::Value> = filtered
            .iter()
            .map(|c| {
                serde_json::json!({
                    "crate": c.name,
                    "lanes": available_lanes.iter().map(|l| &l.name).collect::<Vec<_>>(),
                })
            })
            .collect();
        write_stdout(format_args!("{}", serde_json::to_string_pretty(&output)?))?;
    } else {
        for c in &filtered {
            write_stdout(format_args!("{}:", c.name))?;
            for lane in &available_lanes {
                let req = if lane.required {
                    "required"
                } else {
                    "optional"
                };
                write_stdout(format_args!("  {} ({})", lane.name, req))?;
            }
        }
    }
    Ok(())
}

struct ProofRunConfig {
    profile_str: String,
    jobs_str: String,
    exclude: Option<Vec<String>>,
    include: Option<Vec<String>>,
    fail_fast: bool,
    timeout: u64,
    dry_run: bool,
    json: bool,
}

fn cmd_proof_run(
    workspace_root: &Path,
    cfg: &ProofRunConfig,
    keep_going: bool,
) -> anyhow::Result<()> {
    let Some(profile) = profiles::parse_profile(&cfg.profile_str) else {
        anyhow::bail!(
            "Unknown profile: {}. Use fast|standard|deep|proof|all",
            cfg.profile_str
        );
    };

    let max_jobs = resolve_jobs(&cfg.jobs_str);
    let _keep_going = keep_going;

    let config = proof_orchestrator::OrchestratorConfig {
        profile,
        max_jobs,
        timeout_secs: cfg.timeout,
        fail_fast: cfg.fail_fast,
        dry_run: cfg.dry_run,
        json_output: cfg.json,
        include: cfg.include.clone(),
        exclude: cfg.exclude.clone(),
    };

    let (exit_code, summary) = proof_orchestrator::run_proof(workspace_root, &config)?;
    let output = summary::format_summary(&summary, cfg.json);
    write_stdout(format_args!("{output}"))?;

    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

struct ProofCrateConfig {
    lane_names: Option<Vec<String>>,
    jobs_str: String,
    fail_fast: bool,
    timeout: u64,
    dry_run: bool,
    json: bool,
}

fn cmd_proof_crate(
    workspace_root: &Path,
    crate_name: &str,
    cfg: &ProofCrateConfig,
) -> anyhow::Result<()> {
    let lanes = cfg.lane_names.clone().unwrap_or_else(|| {
        lanes::detect_available_lanes(workspace_root)
            .into_iter()
            .map(|l| l.name)
            .collect()
    });

    let max_jobs = resolve_jobs(&cfg.jobs_str);

    let config = proof_orchestrator::OrchestratorConfig {
        profile: profiles::Profile::All,
        max_jobs,
        timeout_secs: cfg.timeout,
        fail_fast: cfg.fail_fast,
        dry_run: cfg.dry_run,
        json_output: cfg.json,
        include: None,
        exclude: None,
    };

    let (exit_code, summary) =
        proof_orchestrator::run_proof_for_crate(workspace_root, crate_name, &lanes, &config)?;
    let output = summary::format_summary(&summary, cfg.json);
    write_stdout(format_args!("{output}"))?;

    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

struct ProofAffectedConfig {
    jobs_str: String,
    fail_fast: bool,
    timeout: u64,
    dry_run: bool,
    json: bool,
}

fn cmd_proof_affected(
    workspace_root: &Path,
    base: &str,
    cfg: &ProofAffectedConfig,
) -> anyhow::Result<()> {
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", base, "HEAD"])
        .current_dir(workspace_root)
        .output()?;

    let changed_files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(String::from)
        .collect();

    let crates = discovery::discover_crates(workspace_root)?;
    let affected: Vec<_> = crates
        .into_iter()
        .filter(|c| {
            let crate_prefix = format!("crates/{}/", c.name);
            changed_files.iter().any(|f| f.starts_with(&crate_prefix))
        })
        .collect();

    if affected.is_empty() {
        write_stdout(format_args!("No affected crates changed since {base}"))?;
        return Ok(());
    }

    let max_jobs = resolve_jobs(&cfg.jobs_str);

    let config = proof_orchestrator::OrchestratorConfig {
        profile: profiles::Profile::Standard,
        max_jobs,
        timeout_secs: cfg.timeout,
        fail_fast: cfg.fail_fast,
        dry_run: cfg.dry_run,
        json_output: cfg.json,
        include: Some(affected.iter().map(|c| c.name.clone()).collect()),
        exclude: None,
    };

    let (exit_code, summary) = proof_orchestrator::run_proof(workspace_root, &config)?;
    let output = summary::format_summary(&summary, cfg.json);
    write_stdout(format_args!("{output}"))?;

    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

pub(super) fn cmd_proof_plan(crate_name: Option<&str>) -> anyhow::Result<()> {
    let obligations = proof::load_proof_obligations()
        .map_err(|e| anyhow::anyhow!("Failed to load proof obligations: {e}"))?;

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
            write_stdout(format_args!("    Command: {cmd}"))?;
        }
    }

    Ok(())
}
