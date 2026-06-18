//! Production constants and the master profile contract struct.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! Non-Verus compilation kernel: required-key arrays and the compile-time
//! contract constant. Derived from velvet-ballistics-MASTER.md §34:1375-1386.
//! Governance requirements from docs/rust-governance.md:61.

use crate::profile_contract::types::{ProfileKey, ProfileName, SettingValue, StrVal};

// ── Required keys per profile ────────────────────────────────────────

/// The required key-value pairs that `[profile.release]` must declare.
/// Master reference: velvet-ballistics-MASTER.md lines 1375-1379.
pub const RELEASE_REQUIRED_KEYS: &[(ProfileKey, SettingValue)] = &[
    (ProfileKey::OptLevel, SettingValue::U8(3)),
    (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
    (ProfileKey::CodegenUnits, SettingValue::U16(1)),
    (ProfileKey::Strip, SettingValue::String(StrVal::Symbols)),
];

/// The required key-value pairs that `[profile.bench]` must declare.
/// Master reference: velvet-ballistics-MASTER.md lines 1381-1385.
pub const BENCH_REQUIRED_KEYS: &[(ProfileKey, SettingValue)] = &[
    (ProfileKey::Inherits, SettingValue::String(StrVal::Release)),
    (ProfileKey::Debug, SettingValue::Bool(true)),
    (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
    (ProfileKey::CodegenUnits, SettingValue::U16(1)),
];

/// The governance-required settings for hardened (docs/rust-governance.md:61):
///   - debug-assertions = true
///   - overflow-checks = true
pub const HARDENED_GOVERNANCE_REQUIRED: &[(ProfileKey, SettingValue)] = &[
    (ProfileKey::DebugAssertions, SettingValue::Bool(true)),
    (ProfileKey::OverflowChecks, SettingValue::Bool(true)),
];

// ── Master contract struct ───────────────────────────────────────────

/// The master profile contract — compile-time constant.
/// Derived from velvet-ballistics-MASTER.md §34:1375-1386.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterProfileContract {
    /// Profiles that MUST exist in root Cargo.toml.
    pub required_profiles: &'static [ProfileName],
    /// Profile name strings that MUST NOT appear (only "maxperf").
    pub forbidden_profile_names: &'static [&'static str],
    /// Key-value pairs required in `[profile.release]`.
    pub release_keys: &'static [(ProfileKey, SettingValue)],
    /// Key-value pairs required in `[profile.bench]`.
    pub bench_keys: &'static [(ProfileKey, SettingValue)],
}

/// Compile-time constant matching velvet-ballistics-MASTER.md §34:1375-1386.
pub const MASTER_PROFILE_CONTRACT: MasterProfileContract = MasterProfileContract {
    required_profiles: &[ProfileName::Release, ProfileName::Bench],
    forbidden_profile_names: &["maxperf"],
    release_keys: RELEASE_REQUIRED_KEYS,
    bench_keys: BENCH_REQUIRED_KEYS,
};
