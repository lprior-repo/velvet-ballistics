//! Profile contract domain model for Cargo build profile validation.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! Pure core: no I/O, no unsafe, no panics.
//!
//! This module defines the formal specification for the workspace Cargo.toml
//! profile configuration. It ensures types match the master contract
//! (velvet-ballistics-MASTER.md §34:1375-1386) and governance
//! (docs/rust-governance.md:61).

#![forbid(unsafe_code)]

pub mod types;
pub mod config;
pub mod workspace;
pub mod master;
pub mod validation;
pub mod inheritance;
pub mod binding;
pub mod errors;

// Re-export key types for convenience
pub use types::{
    ProfileName, ProfileKey, SettingValue, StrVal, DebugMode,
    LtoMode, StripMode, PanicMode,
};
pub use config::ProfileConfig;
pub use workspace::WorkspaceProfileSet;
pub use master::{MasterProfileContract, MASTER_PROFILE_CONTRACT};
pub use validation::{validate_against_master, validate_against_governance};
pub use inheritance::resolve_inheritance;
pub use binding::{bind_moon_task, MoonTaskProfileBinding, ProfileRefKind, BindingResult};
pub use errors::{
    ContractGap, GovernanceGap, ResolveError,
    ProfileNameError, ProfileKeyError, SettingValueError,
};

// Maximum inheritance chain depth (safety guard).
pub const MAX_INHERITANCE_DEPTH: u8 = 8;

#[cfg(kani)]
pub mod kani;
