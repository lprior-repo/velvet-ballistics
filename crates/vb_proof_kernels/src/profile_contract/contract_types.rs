//! Verified domain types for the master profile contract.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//!
//! Verus verified layer: `ProfileName` (6 variants), `ProfileKey` (9 variants),
//! and their spec/exec method pairs. These types are local-only mirrors of the
//! profile contract and are not bound to the executable profile validation
//! modules or root Cargo.toml parsing.

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {

// ── ProfileName enum (6 valid variants) ────────────────────────────
#[derive(Clone, Copy)]
pub enum ProfileName {
    Release,
    Bench,
    Hardened,
    Fuzz,
    Test,
    Dev,
}

impl ProfileName {
    /// Spec-mode equality for ProfileName (avoids external PartialEq derive).
    pub open spec fn spec_eq(&self, other: &ProfileName) -> bool {
        matches!((self, other),
            (ProfileName::Release, ProfileName::Release)
            | (ProfileName::Bench, ProfileName::Bench)
            | (ProfileName::Hardened, ProfileName::Hardened)
            | (ProfileName::Fuzz, ProfileName::Fuzz)
            | (ProfileName::Test, ProfileName::Test)
            | (ProfileName::Dev, ProfileName::Dev)
        )
    }

    /// Exec-mode equality for ProfileName (avoids external PartialEq derive).
    pub exec fn exec_eq(&self, other: &ProfileName) -> (result: bool)
        ensures
            result == self.spec_eq(other),
    {
        matches!((self, other),
            (ProfileName::Release, ProfileName::Release)
            | (ProfileName::Bench, ProfileName::Bench)
            | (ProfileName::Hardened, ProfileName::Hardened)
            | (ProfileName::Fuzz, ProfileName::Fuzz)
            | (ProfileName::Test, ProfileName::Test)
            | (ProfileName::Dev, ProfileName::Dev)
        )
    }

    /// Spec: is the profile required by the master contract?
    pub open spec fn is_required(&self) -> bool {
        matches!(self, ProfileName::Release | ProfileName::Bench)
    }

    /// Spec: is the profile a governance-enforced profile?
    pub open spec fn is_governance_profile(&self) -> bool {
        matches!(self, ProfileName::Hardened)
    }

    /// Spec: parse a string into a known profile, with forbidden-flag.
    /// Returns (Some(name), false) if known, (None, true) if forbidden.
    pub open spec fn from_str(name: &str) -> (Option<ProfileName>, bool) {
        match name {
            "release" => (Some(ProfileName::Release), false),
            "bench" => (Some(ProfileName::Bench), false),
            "hardened" => (Some(ProfileName::Hardened), false),
            "fuzz" => (Some(ProfileName::Fuzz), false),
            "test" => (Some(ProfileName::Test), false),
            "dev" => (Some(ProfileName::Dev), false),
            "maxperf" => (None, true),  // forbidden
            _ => (None, false),  // unknown but not forbidden
        }
    }
}

// ── ProfileKey enum (9 variants) ───────────────────────────────────
#[derive(Clone, Copy, PartialEq)]
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

// ── Spec: key belongs to a category ────────────────────────────────
/// 0 = optimization, 1 = debug, 2 = safety, 3 = inheritance.
pub open spec fn spec_key_category(key: ProfileKey) -> nat {
    match key {
        ProfileKey::OptLevel => 0,
        ProfileKey::Lto => 0,
        ProfileKey::CodegenUnits => 0,
        ProfileKey::Strip => 1,
        ProfileKey::Debug => 1,
        ProfileKey::DebugAssertions => 2,
        ProfileKey::OverflowChecks => 2,
        ProfileKey::Panic => 2,
        ProfileKey::Inherits => 3,
    }
}

} // verus!
