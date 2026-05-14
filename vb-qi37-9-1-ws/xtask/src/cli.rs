use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Velvet Ballistics xtask commands")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    #[command(name = "ui-snapshot")]
    Snapshot {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        fixture: Option<String>,
        #[arg(long)]
        emit: Option<String>,
        #[arg(long, default_value = "tests/ui_snapshots")]
        output_dir: String,
    },
    #[command(name = "ui-tokens")]
    Tokens {
        #[arg(long, default_value = "design/tokens/velvet_ui_tokens.toml")]
        input: String,
        #[arg(long, default_value = "crates/vb_ui/src/theme/tokens_generated.rs")]
        output: String,
        #[arg(long)]
        emit: Option<String>,
        #[arg(long)]
        check: bool,
    },
    #[command(name = "ui-overlap-check")]
    OverlapCheck {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        screen: Option<String>,
        #[arg(long, default_value = "tests/ui_snapshots")]
        input_dir: String,
    },
    // ========================================================================
    // Section 77 Command-Center Gates (POST-001/002/003/007)
    // ========================================================================
    #[command(name = "ai-fast")]
    AiFast {
        /// Bead ID to scope evidence output to .evidence/<bead-id>/
        #[arg(long)]
        bead: Option<String>,
    },
    #[command(name = "ai-deep")]
    AiDeep {
        /// Bead ID to scope evidence output to .evidence/<bead-id>/
        #[arg(long)]
        bead: Option<String>,
    },
    #[command(name = "ai-release")]
    AiRelease {
        /// Bead ID to scope evidence output to .evidence/<bead-id>/
        #[arg(long)]
        bead: Option<String>,
    },
    #[command(name = "proof-plan")]
    ProofPlan {
        #[arg(long)]
        crate_name: Option<String>,
    },
    #[command(name = "proof-check")]
    ProofCheck {
        #[arg(long)]
        level: Option<String>,
        #[arg(long)]
        bead: Option<String>,
    },
    #[command(name = "proof-evidence")]
    ProofEvidence {
        #[arg(long)]
        bead: String,
    },
    #[command(name = "proof-drift")]
    ProofDrift {
        #[arg(long, value_delimiter = ',')]
        sections: Option<Vec<usize>>,
    },
    #[command(name = "loom")]
    Loom {
        /// Loom model name to run (e.g., bounded_queue, journal_writer_queue)
        #[arg(long)]
        model: String,
    },
}
