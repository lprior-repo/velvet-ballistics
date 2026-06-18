//! Profile contract domain model for Cargo build profile validation.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! Pure core: no I/O, no unsafe, no panics.
//!
//! This module defines a pure local model for workspace Cargo.toml profile
//! validation. It is not a production-bound formal proof of the root Cargo.toml
//! parser or profile enforcement until a future pass adds reviewed production
//! bindings.

#![forbid(unsafe_code)]

pub mod binding;
pub mod config;
pub mod errors;
pub mod inheritance;
pub mod master;
pub mod types;
pub mod validation;
pub mod workspace;

// Re-export key types for convenience
pub use binding::{BindingResult, MoonTaskProfileBinding, ProfileRefKind, bind_moon_task};
pub use config::ProfileConfig;
pub use errors::{
    ContractGap, GovernanceGap, ProfileKeyError, ProfileNameError, ResolveError, SettingValueError,
};
pub use inheritance::resolve_inheritance;
pub use master::{MASTER_PROFILE_CONTRACT, MasterProfileContract};
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
