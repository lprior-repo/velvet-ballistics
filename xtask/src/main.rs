#![forbid(unsafe_code)]
#![allow(unreachable_pub)]

mod ai_profile;
mod cli;
mod evidence;
mod gates;
mod shell;
mod ui_overlap;
mod ui_snapshot;
mod ui_snapshot_render;
mod ui_tokens_cmd;

use clap::Parser;

use ai_profile::{cmd_ai_deep, cmd_ai_fast, cmd_ai_release};
use cli::{Cli, Commands};
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
    }
}

#[cfg(test)]
mod command_shell_tests;
