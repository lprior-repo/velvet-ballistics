#![allow(unreachable_pub)]
//! Proof system commands for velvet-ballistics.
//!
//! Provides:
//! - `proof-plan --changed` - maps changed files to proof obligations
//! - `proof-check --changed` - runs required proof checks
//! - `proof-check --level L3` - runs only specific proof level
//! - `proof-evidence --bead <id>` - writes evidence bundle
//! - `proof-drift` - checks spec/code alignment

pub mod commands;
pub mod types;

pub use commands::{
    commands_for_obligation, load_proof_obligations, obligations_for_level, write_proof_evidence,
};
pub use types::ProofObligation;
#[cfg(test)]
pub use types::RequiredProof;
