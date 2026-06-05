//! Error types for the Cargo profile contract domain.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! Maps to error taxonomy: C001-C009 (contract gaps), R001-R003 (resolve errors).

use crate::profile_contract::types::{ProfileKey, ProfileName, SettingValue};

// ---------------------------------------------------------------------------
// Construction errors
// ---------------------------------------------------------------------------

/// Error constructing a ProfileName from a raw string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileNameError {
    /// Master-forbidden profile name "maxperf".
    Forbidden,
    /// Unrecognized profile name.
    Unknown(String),
}

/// Error parsing a TOML profile key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileKeyError {
    Unknown(String),
}

/// Error constructing a SettingValue for a ProfileKey.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingValueError {
    InvalidOptLevel(u8),
    InvalidCodegenUnits(u16),
    InvalidLto,
    InvalidStrip,
    InvalidPanic,
    InvalidDebug,
    InvalidInherits,
}

// ---------------------------------------------------------------------------
// Contract validation errors
// ---------------------------------------------------------------------------

/// A single discrepancy between the workspace profile set and the master contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractGap {
    /// A required profile is absent from root Cargo.toml. (C001, C002)
    MissingProfile { name: ProfileName },

    /// A forbidden profile (maxperf) is present. (C004)
    ForbiddenProfile { name: ProfileName },

    /// A key-value pair does not match the master specification. (C005-C007)
    WrongSetting {
        profile: ProfileName,
        key: ProfileKey,
        expected: SettingValue,
        actual: SettingValue,
    },

    /// A required key is absent from the profile. (C003, C008)
    MissingSetting {
        profile: ProfileName,
        key: ProfileKey,
    },
}

/// A governance requirement discrepancy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovernanceGap {
    /// hardened profile is missing debug-assertions=true. (C009)
    MissingDebugAssertions,

    /// hardened profile is missing overflow-checks=true.
    MissingOverflowChecks,
}

/// Error during inheritance resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// Circular inheritance chain detected. (R001)
    InheritCycle,

    /// A profile's `inherits` target does not exist. (R002)
    InheritTargetMissing {
        profile: ProfileName,
        parent: ProfileName,
    },

    /// Inheritance depth exceeded MAX_INHERITANCE_DEPTH. (R003)
    InheritanceDepthExceeded { depth: u8 },
}

/// Result of binding a Moon task profile reference to a workspace profile set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingResult {
    /// Profile exists and satisfies master contract.
    ExistsAndValid,
    /// Profile exists but has contract gaps.
    ExistsButGapped,
    /// Profile does not exist in the workspace.
    Missing,
    /// Profile is maxperf — intentionally absent, task is deferred.
    DeferredScope,
}
