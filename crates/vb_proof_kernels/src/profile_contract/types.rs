//! Core value types for the Cargo profile contract domain.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! These types make illegal states unrepresentable by design.

use crate::profile_contract::errors::{ProfileNameError, ProfileKeyError, SettingValueError};

// ---------------------------------------------------------------------------
// ProfileName — validated profile identifier
// ---------------------------------------------------------------------------

/// A validated Cargo build profile name.
///
/// Only 6 discriminants are valid. `Maxperf` is excluded by master contract
/// (velvet-ballistics-MASTER.md §34:1388). Use `ProfileName::new()` to construct;
/// stringly-typed "maxperf" will be rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProfileName {
    Release,
    Bench,
    Hardened,
    Fuzz,
    Test,
    Dev,
}

impl ProfileName {
    /// Construct a ProfileName from a string.
    ///
    /// Returns `Err(ProfileNameError::Forbidden("maxperf"))` for the master-forbidden
    /// name. Returns `Err(ProfileNameError::Unknown(_))` for unrecognized names.
    pub fn new(name: &str) -> Result<Self, ProfileNameError> {
        match name {
            "release" => Ok(Self::Release),
            "bench" => Ok(Self::Bench),
            "hardened" => Ok(Self::Hardened),
            "fuzz" => Ok(Self::Fuzz),
            "test" => Ok(Self::Test),
            "dev" => Ok(Self::Dev),
            "maxperf" => Err(ProfileNameError::Forbidden),
            other => Err(ProfileNameError::Unknown(other.to_string())),
        }
    }

    /// Return the TOML profile section name for this variant.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Release => "release",
            Self::Bench => "bench",
            Self::Hardened => "hardened",
            Self::Fuzz => "fuzz",
            Self::Test => "test",
            Self::Dev => "dev",
        }
    }
}

// ---------------------------------------------------------------------------
// ProfileKey — configuration dimension within a profile
// ---------------------------------------------------------------------------

/// A specific configuration dimension within a Cargo profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProfileKey {
    OptLevel,
    Lto,
    CodegenUnits,
    Strip,
    Debug,
    DebugAssertions,
    OverflowChecks,
    Panic,
    Inherits,
}

impl ProfileKey {
    /// Parse a TOML profile key string.
    pub fn from_toml_key(key: &str) -> Result<Self, ProfileKeyError> {
        match key {
            "opt-level" => Ok(Self::OptLevel),
            "lto" => Ok(Self::Lto),
            "codegen-units" => Ok(Self::CodegenUnits),
            "strip" => Ok(Self::Strip),
            "debug" => Ok(Self::Debug),
            "debug-assertions" => Ok(Self::DebugAssertions),
            "overflow-checks" => Ok(Self::OverflowChecks),
            "panic" => Ok(Self::Panic),
            "inherits" => Ok(Self::Inherits),
            other => Err(ProfileKeyError::Unknown(other.to_string())),
        }
    }

    /// Known profile keys for enumeration in Kani harnesses.
    pub const ALL_KEYS: &'static [ProfileKey] = &[
        Self::OptLevel,
        Self::Lto,
        Self::CodegenUnits,
        Self::Strip,
        Self::Debug,
        Self::DebugAssertions,
        Self::OverflowChecks,
        Self::Panic,
        Self::Inherits,
    ];
}

// ---------------------------------------------------------------------------
// SettingValue — typed wrapper for profile setting values
// ---------------------------------------------------------------------------

/// TOML value types used in profile settings.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SettingValue {
    Bool(bool),
    String(StrVal),
    U8(u8),
    U16(u16),
    DebugMode(DebugMode),
}

/// Interned string values for SettingValue::String.
///
/// Only known-setting strings are represented; unknown strings
/// are represented as `Other`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StrVal {
    Thin,          // "thin"
    Fat,           // "fat"
    Off,           // "off"
    True,          // "true"  (for inherits = "release")
    False,         // "false"
    None_,         // "none"
    Symbols,       // "symbols"
    Debuginfo,     // "debuginfo"
    Release,       // "release" (for inherits)
    Unwind,        // "unwind"
    Abort,         // "abort"
    Other,         // catch-all for unknown string values
}

impl StrVal {
    pub fn from_str(s: &str) -> Self {
        match s {
            "thin" => Self::Thin,
            "fat" => Self::Fat,
            "off" => Self::Off,
            "true" => Self::True,
            "false" => Self::False,
            "none" => Self::None_,
            "symbols" => Self::Symbols,
            "debuginfo" => Self::Debuginfo,
            "release" => Self::Release,
            "unwind" => Self::Unwind,
            "abort" => Self::Abort,
            _ => Self::Other,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Thin => "thin",
            Self::Fat => "fat",
            Self::Off => "off",
            Self::True => "true",
            Self::False => "false",
            Self::None_ => "none",
            Self::Symbols => "symbols",
            Self::Debuginfo => "debuginfo",
            Self::Release => "release",
            Self::Unwind => "unwind",
            Self::Abort => "abort",
            Self::Other => "<other>",
        }
    }
}

// ---------------------------------------------------------------------------
// Setting value enums for constrained domains
// ---------------------------------------------------------------------------

/// The `debug` profile setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DebugMode {
    False,
    True,
    LineTablesOnly,
}

/// The `lto` profile setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LtoMode {
    False,
    Thin,
    Fat,
    Off,
}

/// The `strip` profile setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StripMode {
    None,
    Symbols,
    Debuginfo,
}

/// The `panic` profile setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanicMode {
    Unwind,
    Abort,
}

// ---------------------------------------------------------------------------
// SettingValue constructor (with domain-level validation)
// ---------------------------------------------------------------------------

impl SettingValue {
    /// Construct a SettingValue for a given key.
    ///
    /// Returns Err for domain-invalid value-key combinations:
    /// - opt-level != 3 for release profile
    /// - codegen-units != 1 for release/bench
    /// - lto value not in {false, thin, fat, off}
    pub fn for_key(
        key: ProfileKey,
        value: SettingValue,
    ) -> Result<Self, SettingValueError> {
        match (key, &value) {
            (ProfileKey::OptLevel, SettingValue::U8(v)) if *v != 3 => {
                Err(SettingValueError::InvalidOptLevel(*v))
            }
            (ProfileKey::CodegenUnits, SettingValue::U16(v)) if *v != 1 => {
                Err(SettingValueError::InvalidCodegenUnits(*v))
            }
            (ProfileKey::Lto, SettingValue::String(s)) => {
                match s {
                    StrVal::Thin | StrVal::Fat | StrVal::Off | StrVal::False => Ok(value),
                    _ => Err(SettingValueError::InvalidLto),
                }
            }
            (ProfileKey::Strip, SettingValue::String(s)) => {
                match s {
                    StrVal::None_ | StrVal::Symbols | StrVal::Debuginfo => Ok(value),
                    _ => Err(SettingValueError::InvalidStrip),
                }
            }
            (ProfileKey::Panic, SettingValue::String(s)) => {
                match s {
                    StrVal::Unwind | StrVal::Abort => Ok(value),
                    _ => Err(SettingValueError::InvalidPanic),
                }
            }
            _ => Ok(value),
        }
    }
}
