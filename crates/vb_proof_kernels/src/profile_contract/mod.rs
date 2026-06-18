//! Profile contract domain model for Cargo build profile validation.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! Pure core: no I/O, no unsafe, no panics.
//!
//! This module defines a pure local model for workspace Cargo.toml profile
//! validation. It is not a production-bound formal proof of the root Cargo.toml
//! parser or profile enforcement until a future pass adds reviewed production
//! bindings.
//!
//! ## Module layout (master.rs split — vb-esq9.1)
//!
//! The former `master.rs` (575 lines) was decomposed into five focused modules:
//!
//! - `contract_types` — Verus verified domain types (`ProfileName`, `ProfileKey`)
//! - `contract_lemmas` — Verus proof lemmas (disjointness, exhaustiveness, partition)
//! - `contract_witnesses` — Verus exec witness functions (spec/exec bridge)
//! - `contract_constants` — Production constants (`MasterProfileContract`, key arrays)
//! - `contract_tests` — Unit tests for contract constants

#![forbid(unsafe_code)]

pub mod binding;
pub mod config;
pub mod contract_constants;
pub mod contract_lemmas;
#[cfg(test)]
pub mod contract_tests;
pub mod contract_types;
pub mod contract_witnesses;
pub mod errors;
pub mod inheritance;
pub mod types;
pub mod validation;
pub mod workspace;

// Re-export key types for convenience
pub use binding::{BindingResult, MoonTaskProfileBinding, ProfileRefKind, bind_moon_task};
pub use config::ProfileConfig;
pub use contract_constants::{
    BENCH_REQUIRED_KEYS, HARDENED_GOVERNANCE_REQUIRED, MASTER_PROFILE_CONTRACT,
    MasterProfileContract, RELEASE_REQUIRED_KEYS,
};
pub use errors::{
    ContractGap, GovernanceGap, ProfileKeyError, ProfileNameError, ResolveError, SettingValueError,
};
pub use inheritance::resolve_inheritance;
pub use types::{
    DebugMode, LtoMode, PanicMode, ProfileKey, ProfileName, SettingValue, StrVal, StripMode,
};
pub use validation::{validate_against_governance, validate_against_master};
pub use workspace::WorkspaceProfileSet;

// Maximum inheritance chain depth (safety guard).
pub const MAX_INHERITANCE_DEPTH: u8 = 8;

// Retired by vb-dzibx: the Kani harness modules contain fixed master-profile
// shapes and are not production-bound proof evidence. Keep the source files for
// future repair, but do not compile/register them as active Kani harnesses.
