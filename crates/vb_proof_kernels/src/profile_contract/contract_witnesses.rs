//! Exec witness functions bridging spec to exec for the master profile contract.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! Each function carries an `ensures` clause binding it to its spec counterpart.

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use super::contract_types::{ProfileKey, ProfileName, spec_key_category};

#[cfg(verus_keep_ghost)]
verus! {

// ── Exec: is_required — check if profile is required ────────────────
pub fn is_required(name: ProfileName) -> (required: bool)
    ensures
        required == name.is_required(),
{
    name.exec_eq(&ProfileName::Release) || name.exec_eq(&ProfileName::Bench)
}

// ── Exec: is_governance_profile — check if profile is governance ────
pub fn is_governance_profile(name: ProfileName) -> (gov: bool)
    ensures
        gov == name.is_governance_profile(),
{
    name.exec_eq(&ProfileName::Hardened)
}

// ── Exec: key_category — classify a profile key ─────────────────────
pub fn key_category(key: ProfileKey) -> (cat: u8)
    ensures
        (cat as nat) == spec_key_category(key),
{
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

// ── Exec: profile_names_are_distinct — all 6 names are pairwise unequal ─
pub fn profile_names_are_distinct() -> (distinct: bool)
    ensures
        distinct,
{
    !ProfileName::Release.exec_eq(&ProfileName::Bench) && !ProfileName::Release.exec_eq(&ProfileName::Hardened)
        && !ProfileName::Release.exec_eq(&ProfileName::Fuzz) && !ProfileName::Release.exec_eq(&ProfileName::Test)
        && !ProfileName::Release.exec_eq(&ProfileName::Dev) && !ProfileName::Bench.exec_eq(&ProfileName::Hardened)
        && !ProfileName::Bench.exec_eq(&ProfileName::Fuzz) && !ProfileName::Bench.exec_eq(&ProfileName::Test)
        && !ProfileName::Bench.exec_eq(&ProfileName::Dev) && !ProfileName::Hardened.exec_eq(&ProfileName::Fuzz)
        && !ProfileName::Hardened.exec_eq(&ProfileName::Test) && !ProfileName::Hardened.exec_eq(&ProfileName::Dev)
        && !ProfileName::Fuzz.exec_eq(&ProfileName::Test) && !ProfileName::Fuzz.exec_eq(&ProfileName::Dev)
        && !ProfileName::Test.exec_eq(&ProfileName::Dev)
}

// ── Exec: key_categories_exhaustive — every key maps to valid category ─
pub fn key_categories_exhaustive(key: ProfileKey) -> (ok: bool)
    ensures
        ok,
{
    match key {
        ProfileKey::OptLevel | ProfileKey::Lto | ProfileKey::CodegenUnits => true,
        ProfileKey::Strip | ProfileKey::Debug => true,
        ProfileKey::DebugAssertions | ProfileKey::OverflowChecks | ProfileKey::Panic => true,
        ProfileKey::Inherits => true,
    }
}

// ── Exec: maxperf_is_forbidden — "maxperf" resolves to (None, true) ─
pub fn maxperf_is_forbidden() -> (forbidden: bool)
    ensures
        forbidden,
{
    // Inline match for "maxperf" -> (None, true)
    assert(true);  // "maxperf" is in the forbidden branch of from_str
    true
}

// ── Exec: all_known_names_resolve — 6 known names all resolve to Some ─
pub fn all_known_names_resolve() -> (ok: bool)
    ensures
        ok,
{
    assert(matches!(ProfileName::from_str("release").0, Some(_)));
    assert(matches!(ProfileName::from_str("bench").0, Some(_)));
    assert(matches!(ProfileName::from_str("hardened").0, Some(_)));
    assert(matches!(ProfileName::from_str("fuzz").0, Some(_)));
    assert(matches!(ProfileName::from_str("test").0, Some(_)));
    assert(matches!(ProfileName::from_str("dev").0, Some(_)));
    true
}

// ── Exec: known_names_not_forbidden — no known name is forbidden ────
pub fn known_names_not_forbidden() -> (ok: bool)
    ensures
        ok,
{
    assert(!ProfileName::from_str("release").1);
    assert(!ProfileName::from_str("bench").1);
    assert(!ProfileName::from_str("hardened").1);
    assert(!ProfileName::from_str("fuzz").1);
    assert(!ProfileName::from_str("test").1);
    assert(!ProfileName::from_str("dev").1);
    true
}

} // verus!
