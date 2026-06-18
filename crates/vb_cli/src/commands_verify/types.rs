//! Domain types for the verification pipeline.
//!
//! Exposes the result type, error taxonomy, and gate outcome bookkeeping
//! used by the verification pipeline and consumed by the CLI command layer.

#![forbid(unsafe_code)]

use crate::args::{DurabilityMode, VerifyProfile};
use crate::exit_code::CliExitCode;
use vb_core::workflow::WorkflowParts;

// ---------------------------------------------------------------------------
// Public result type
// ---------------------------------------------------------------------------

/// Structured result of a successful verification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VerifyOk {
    /// Hex-encoded canonical workflow digest.
    pub digest_hex: String,
    /// Hex-encoded postcard IR artifact digest.
    pub ir_digest_hex: String,
    /// Number of compiled workflow nodes.
    pub node_count: u16,
    /// Master §63 gate statuses in canonical order.
    pub checks: Vec<&'static str>,
    /// Non-fatal warnings produced during verification.
    pub warnings: Vec<String>,
    /// Durability mode the verify call was tagged with (e.g. strict, journaled, none).
    pub durability_mode: DurabilityMode,
}

impl VerifyOk {
    pub(crate) fn all_gates_closed(&self) -> bool {
        self.checks
            .iter()
            .all(|check| !is_deferred_gate_status(check))
    }

    pub(crate) fn passed_gates(&self) -> Vec<&'static str> {
        self.checks
            .iter()
            .copied()
            .filter(|check| !is_deferred_gate_status(check))
            .collect()
    }

    pub(crate) fn deferred_gates(&self) -> Vec<&'static str> {
        self.checks
            .iter()
            .copied()
            .filter_map(|check| {
                if is_deferred_gate_status(check) {
                    Some(canonical_gate_name(check))
                } else {
                    None
                }
            })
            .collect()
    }
}

fn is_deferred_gate_status(status: &str) -> bool {
    status.ends_with(":deferred")
}

fn canonical_gate_name(status: &'static str) -> &'static str {
    if let Some(name) = status.strip_suffix(":deferred") {
        name
    } else {
        status
    }
}

// ---------------------------------------------------------------------------
// Public error taxonomy
// ---------------------------------------------------------------------------

/// Structured error from the verification pipeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum VerifyError {
    /// YAML source could not be parsed.
    YamlParse(String),
    /// Compilation failed with one or more errors.
    Compile(Vec<String>),
    /// IR validation failed.
    IrValidation(String),
    /// Budget policy violation (fatal in full profile).
    BudgetPolicy(String),
    /// Storage operation failed.
    StorageError(String),
    /// Replay divergence detected.
    ReplayDivergence(String),
    /// Full verification cannot pass while any canonical gate remains deferred.
    DeferredGates(VerifyOk),
}

/// Map a [`VerifyError`] to a [`CliExitCode`].
pub(crate) fn exit_code_for_error(err: &VerifyError) -> CliExitCode {
    match err {
        VerifyError::YamlParse(_) => CliExitCode::ValidationFailed,
        VerifyError::Compile(_) => CliExitCode::ValidationFailed,
        VerifyError::IrValidation(_) => CliExitCode::VerificationFailed,
        VerifyError::BudgetPolicy(_) => CliExitCode::VerificationFailed,
        VerifyError::StorageError(_) => CliExitCode::StorageError,
        VerifyError::ReplayDivergence(_) => CliExitCode::ReplayDivergence,
        VerifyError::DeferredGates(_) => CliExitCode::VerificationFailed,
    }
}

// ---------------------------------------------------------------------------
// Internal gate outcome tracking
// ---------------------------------------------------------------------------

/// Canonical verification gate outcomes reported by the CLI layer.
///
/// Gates backed directly by the local parse/compile/validate pipeline are
/// emitted as bare gate names. Gates that still rely on external registries,
/// runtime admission, or release evidence stay suffixed with `:deferred` so
/// the output stays faithful to master §63 without inventing replacement names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VerificationGateOutcomes {
    pub(crate) bounded: bool,
    pub(crate) budgets: bool,
    contracts: bool,
    taint: bool,
    idempotency: bool,
    durability: bool,
    capabilities: bool,
    evidence: bool,
}

impl VerificationGateOutcomes {
    pub(crate) const fn baseline_success() -> Self {
        Self {
            bounded: false,
            budgets: false,
            contracts: false,
            taint: false,
            idempotency: false,
            durability: false,
            capabilities: false,
            evidence: false,
        }
    }

    pub(crate) fn to_checks(self) -> [&'static str; 15] {
        [
            "profile",
            "shape",
            "names",
            "references",
            "expressions",
            "CFG",
            if self.bounded {
                "bounded"
            } else {
                "bounded:deferred"
            },
            if self.budgets {
                "budgets"
            } else {
                "budgets:deferred"
            },
            if self.contracts {
                "contracts"
            } else {
                "contracts:deferred"
            },
            if self.taint {
                "taint"
            } else {
                "taint:deferred"
            },
            if self.idempotency {
                "idempotency"
            } else {
                "idempotency:deferred"
            },
            if self.durability {
                "durability"
            } else {
                "durability:deferred"
            },
            if self.capabilities {
                "capabilities"
            } else {
                "capabilities:deferred"
            },
            "results",
            if self.evidence {
                "evidence"
            } else {
                "evidence:deferred"
            },
        ]
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

pub(crate) fn workflow_digest_hex(digest: vb_core::WorkflowDigest) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn bytes_digest_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes)
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
