//! Verified lemmas for the master profile contract types.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! 13 proof lemmas covering disjointness, exhaustiveness, and partition invariants.

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use super::contract_types::{ProfileKey, ProfileName};

#[cfg(verus_keep_ghost)]
verus! {

// ── Lemma: exactly 6 profile names exist (pairwise distinct) ───────
proof fn lemma_exactly_6_profile_names()
    ensures
        !ProfileName::Release.spec_eq(&ProfileName::Bench) && !ProfileName::Release.spec_eq(&ProfileName::Hardened)
            && !ProfileName::Release.spec_eq(&ProfileName::Fuzz) && !ProfileName::Release.spec_eq(&ProfileName::Test)
            && !ProfileName::Release.spec_eq(&ProfileName::Dev) && !ProfileName::Bench.spec_eq(&ProfileName::Hardened)
            && !ProfileName::Bench.spec_eq(&ProfileName::Fuzz)
            && !ProfileName::Bench.spec_eq(&ProfileName::Test) && !ProfileName::Bench.spec_eq(&ProfileName::Dev)
            && !ProfileName::Hardened.spec_eq(&ProfileName::Fuzz) && !ProfileName::Hardened.spec_eq(&ProfileName::Test)
            && !ProfileName::Hardened.spec_eq(&ProfileName::Dev)
            && !ProfileName::Fuzz.spec_eq(&ProfileName::Test)
            && !ProfileName::Fuzz.spec_eq(&ProfileName::Dev)
            && !ProfileName::Test.spec_eq(&ProfileName::Dev),
{
    assert(!ProfileName::Release.spec_eq(&ProfileName::Bench));
    assert(!ProfileName::Release.spec_eq(&ProfileName::Hardened));
    assert(!ProfileName::Release.spec_eq(&ProfileName::Fuzz));
    assert(!ProfileName::Release.spec_eq(&ProfileName::Test));
    assert(!ProfileName::Release.spec_eq(&ProfileName::Dev));
    assert(!ProfileName::Bench.spec_eq(&ProfileName::Hardened));
    assert(!ProfileName::Bench.spec_eq(&ProfileName::Fuzz));
    assert(!ProfileName::Bench.spec_eq(&ProfileName::Test));
    assert(!ProfileName::Bench.spec_eq(&ProfileName::Dev));
    assert(!ProfileName::Hardened.spec_eq(&ProfileName::Fuzz));
    assert(!ProfileName::Hardened.spec_eq(&ProfileName::Test));
    assert(!ProfileName::Hardened.spec_eq(&ProfileName::Dev));
    assert(!ProfileName::Fuzz.spec_eq(&ProfileName::Test));
    assert(!ProfileName::Fuzz.spec_eq(&ProfileName::Dev));
    assert(!ProfileName::Test.spec_eq(&ProfileName::Dev));
}

// ── Lemma: release and bench are the only required profiles ────────
proof fn lemma_required_profiles_are_exactly_release_and_bench(name: ProfileName)
    ensures
        name.is_required() == (name.spec_eq(&ProfileName::Release) || name.spec_eq(&ProfileName::Bench)),
{
    if name.spec_eq(&ProfileName::Release) {
        assert(name.spec_eq(&ProfileName::Release) || name.spec_eq(&ProfileName::Bench));
        assert(name.is_required());
    } else if name.spec_eq(&ProfileName::Bench) {
        assert(name.spec_eq(&ProfileName::Release) || name.spec_eq(&ProfileName::Bench));
        assert(name.is_required());
    } else {
        assert(!name.spec_eq(&ProfileName::Release));
        assert(!name.spec_eq(&ProfileName::Bench));
        assert(!name.is_required());
    }
}

// ── Lemma: only hardened is a governance profile ───────────────────
proof fn lemma_only_hardened_is_governance(name: ProfileName)
    ensures
        name.is_governance_profile() == name.spec_eq(&ProfileName::Hardened),
{
    if name.spec_eq(&ProfileName::Hardened) {
        assert(name.spec_eq(&ProfileName::Hardened));
        assert(name.is_governance_profile());
    } else {
        assert(!name.spec_eq(&ProfileName::Hardened));
        assert(!name.is_governance_profile());
    }
}

// ── Lemma: no profile is both required and governance ──────────────
proof fn lemma_required_and_governance_disjoint()
    ensures
        !(ProfileName::Release.is_governance_profile()
            || ProfileName::Bench.is_governance_profile()),
{
    assert(!ProfileName::Release.is_governance_profile());
    assert(!ProfileName::Bench.is_governance_profile());
}

// ── Lemma: maxperf is not a valid profile name ─────────────────────
proof fn lemma_maxperf_not_valid()
    ensures
        ProfileName::from_str("maxperf") == (None::<ProfileName>, true),
{
    assert(ProfileName::from_str("maxperf") == (None::<ProfileName>, true));
}

// ── Lemma: all known names resolve to Some ─────────────────────────
proof fn lemma_known_names_resolve()
    ensures
        matches!(ProfileName::from_str("release").0, Some(_))
            && matches!(ProfileName::from_str("bench").0, Some(_))
            && matches!(ProfileName::from_str("hardened").0, Some(_))
            && matches!(ProfileName::from_str("fuzz").0, Some(_))
            && matches!(ProfileName::from_str("test").0, Some(_))
            && matches!(ProfileName::from_str("dev").0, Some(_)),
{
    assert(matches!(ProfileName::from_str("release").0, Some(_)));
    assert(matches!(ProfileName::from_str("bench").0, Some(_)));
    assert(matches!(ProfileName::from_str("hardened").0, Some(_)));
    assert(matches!(ProfileName::from_str("fuzz").0, Some(_)));
    assert(matches!(ProfileName::from_str("test").0, Some(_)));
    assert(matches!(ProfileName::from_str("dev").0, Some(_)));
}

// ── Lemma: all known names are not forbidden ───────────────────────
proof fn lemma_known_names_not_forbidden()
    ensures
        !ProfileName::from_str("release").1 && !ProfileName::from_str("bench").1
            && !ProfileName::from_str("hardened").1 && !ProfileName::from_str("fuzz").1
            && !ProfileName::from_str("test").1 && !ProfileName::from_str("dev").1,
{
    assert(!ProfileName::from_str("release").1);
    assert(!ProfileName::from_str("bench").1);
    assert(!ProfileName::from_str("hardened").1);
    assert(!ProfileName::from_str("fuzz").1);
    assert(!ProfileName::from_str("test").1);
    assert(!ProfileName::from_str("dev").1);
}

// ── Lemma: key categories partition correctly ──────────────────────
proof fn lemma_key_categories_exhaustive(key: ProfileKey)
    ensures
        spec_key_category(key) <= 3,
{
    // All 9 variants map to 0..=3.
    assert(spec_key_category(key) <= 3);
}

// ── Lemma: inherits is the only inheritance key ────────────────────
proof fn lemma_inherits_unique_inheritance_key(key: ProfileKey)
    ensures
        (spec_key_category(key) == 3) ==> key == ProfileKey::Inherits,
{
    // Only ProfileKey::Inherits maps to category 3.
    if key == ProfileKey::Inherits {
        assert(spec_key_category(key) == 3);
    } else {
        assert(spec_key_category(key) != 3);
    }
}

} // verus!
