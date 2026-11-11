#![forbid(unsafe_code)]

//! MASTER §77.17 compatibility gates for the storage layer.
//!
//! The algebra itself lives in this module. This module owns the
//! storage-side pieces: the BLAKE3 deciding-rule digest, journaling of
//! decisions that affect admission or live-effect dispatch, and the two
//! gate seams — admission and live dispatch.
//!
//! Gate split, by design:
//! - Admission stores the artifact and journals the decision. It blocks
//!   [`CompatibilityClass::MigrationRequired`] and
//!   [`CompatibilityClass::Rejected`]; a
//!   [`CompatibilityClass::ReplayOnly`] artifact may be admitted (stored
//!   for replay) because admission alone mints no live-effect authority.
//! - Live dispatch re-checks the journaled decision and blocks
//!   [`CompatibilityClass::ReplayOnly`] too: a replay-only decision never
//!   authorizes a new live effect.
//! - Every decision is journaled before enforcement, including blocked
//!   ones, so a refusal is itself on the durable record.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    EventSeq, constants::CURRENT_SCHEMA_VERSION, error::JournalError, events::JournalEvent,
    journal::FjallJournal,
};
use vb_core::RunId;

include!("compat/version.rs");

include!("compat/class.rs");

include!("compat/boundary.rs");

include!("compat/rule.rs");

include!("compat/decision.rs");

include!("compat/gates.rs");

/// Inputs for artifact IR version classification with an explicit required
/// version. Used when the caller specifies the target version rather than
/// using the current baseline.
#[must_use]
pub const fn artifact_ir_version_inputs_for(
    held: SchemaVersion,
    required: SchemaVersion,
) -> CompatInputs {
    CompatInputs {
        boundary: CompatBoundary::IrVersion,
        held,
        required,
        migration_available: false,
        replay_only: false,
    }
}

#[cfg(test)]
// Only deny-level lints are allowed here: the forbid-level lints
// (expect_used, panic, unwrap_used) cannot be allowed back, so the tests
// below use `?` instead of `expect`/`unwrap`.
#[allow(clippy::indexing_slicing)]
mod tests;
