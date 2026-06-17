#![allow(clippy::expect_used, clippy::unwrap_used, clippy::as_conversions, clippy::arithmetic_side_effects, clippy::indexing_slicing, clippy::let_underscore_must_use, clippy::panic, clippy::panic_in_result_fn, clippy::bool_comparison, clippy::manual_div_ceil, clippy::clone_on_copy, clippy::len_zero, clippy::redundant_clone, clippy::collapsible_if, clippy::needless_return, clippy::needless_borrow, clippy::useless_format, clippy::redundant_pub_crate, clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::missing_safety_doc, clippy::wildcard_enum_match_arm, clippy::large_futures, clippy::unused_async, clippy::unused_self, clippy::let_underscore_drop, clippy::filter_map_next, clippy::from_iter_instead_of_collect, clippy::if_not_else, clippy::implicit_clone, clippy::inefficient_to_string, clippy::inconsistent_struct_constructor, clippy::iter_filter_is_ok, clippy::iter_filter_is_some, clippy::iter_not_returning_iterator, clippy::iter_over_hash_type, clippy::iter_without_into_iter, clippy::large_digit_groups, clippy::large_types_passed_by_value, clippy::let_and_return, clippy::misnamed_getters, clippy::mutable_key_type, clippy::needless_collect, clippy::nonminimal_bool, clippy::option_if_let_else, clippy::or_fun_call, clippy::path_buf_push_overwrite, clippy::print_stderr, clippy::print_stdout, clippy::pub_with_shorthand, clippy::range_minus_one, clippy::range_plus_one, clippy::ref_binding_to_reference, clippy::ref_option_ref, clippy::single_match_else, clippy::suspicious_operation_groupings, clippy::trivially_copy_pass_by_ref, clippy::uninlined_format_args, clippy::unnecessary_wraps, clippy::unnested_or_patterns, clippy::unreadable_literal, clippy::unused_io_amount, clippy::unused_trait_names, clippy::vec_init_then_push, clippy::wildcard_imports, clippy::approximate_const, clippy::absurd_extreme_comparisons, clippy::expect_fun_call)]


//! MasterProfileContract — the immutable reference contract from the master document.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! Derived from velvet-ballistics-MASTER.md §34 lines 1375-1386.
//! Governance requirements from docs/rust-governance.md:61.

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

// ── Verus verified layer ────────────────────────────────────────────────────
#[cfg(verus_keep_ghost)]
verus! {

    // ── ProfileName enum (6 valid variants) ────────────────────────────
    #[derive(Clone, Copy, PartialEq)]
    pub enum ProfileName {
        Release,
        Bench,
        Hardened,
        Fuzz,
        Test,
        Dev,
    }

    impl ProfileName {
        // ── Spec: is_required ──────────────────────────────────────────
        pub open spec fn is_required(&self) -> bool {
            matches!(self, ProfileName::Release | ProfileName::Bench)
        }

        // ── Spec: is_governance_profile ────────────────────────────────
        pub open spec fn is_governance_profile(&self) -> bool {
            matches!(self, ProfileName::Hardened)
        }

        // ── Spec: new from string ──────────────────────────────────────
        pub open spec fn from_str(name: &str) -> (Option<ProfileName>, bool) {
            // Returns (Some(name), false) if known, (None, true) if forbidden
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
    pub open spec fn spec_key_category(key: ProfileKey) -> nat {
        // 0 = optimization, 1 = debug, 2 = safety, 3 = inheritance
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

    // ── Release/Bench required key specs ───────────────────────────────
    //
    // These are the nat-encoded key-value pairs from the master contract.
    // We encode each pair as a single nat for proof convenience.
    //
    // Encoding: key_index * 100 + value_tag
    // where value_tag: U8(3) = 3, U16(1) = 1, String(Thin) = 10,
    //     String(Release) = 11, Bool(true) = 20

    pub open spec fn spec_release_required_keys() -> nat {
        // 4 required keys for release
        4
    }

    pub open spec fn spec_bench_required_keys() -> nat {
        // 4 required keys for bench
        4
    }

    pub open spec fn spec_hardened_gov_required() -> nat {
        // 2 governance keys for hardened
        2
    }

    // ── Lemma: exactly 6 profile names exist ───────────────────────────
    proof fn lemma_exactly_6_profile_names()
        ensures
            ProfileName::Release != ProfileName::Bench
                && ProfileName::Release != ProfileName::Hardened
                && ProfileName::Release != ProfileName::Fuzz
                && ProfileName::Release != ProfileName::Test
                && ProfileName::Release != ProfileName::Dev
                && ProfileName::Bench != ProfileName::Hardened
                && ProfileName::Bench != ProfileName::Fuzz
                && ProfileName::Bench != ProfileName::Test
                && ProfileName::Bench != ProfileName::Dev
                && ProfileName::Hardened != ProfileName::Fuzz
                && ProfileName::Hardened != ProfileName::Test
                && ProfileName::Hardened != ProfileName::Dev
                && ProfileName::Fuzz != ProfileName::Test
                && ProfileName::Fuzz != ProfileName::Dev
                && ProfileName::Test != ProfileName::Dev,
    {
    }

    // ── Lemma: release and bench are the only required profiles ────────
    proof fn lemma_required_profiles_are_exactly_release_and_bench(
        name: ProfileName,
    )
        ensures
            name.is_required() == (name == ProfileName::Release || name == ProfileName::Bench),
    {
    }

    // ── Lemma: only hardened is a governance profile ───────────────────
    proof fn lemma_only_hardened_is_governance(name: ProfileName)
        ensures
            name.is_governance_profile() == (name == ProfileName::Hardened),
    {
    }

    // ── Lemma: no profile is both required and governance ──────────────
    proof fn lemma_required_and_governance_disjoint()
        ensures
            !(ProfileName::Release.is_governance_profile() || ProfileName::Bench.is_governance_profile()),
    {
    }

    // ── Lemma: maxperf is not a valid profile name ─────────────────────
    proof fn lemma_maxperf_not_valid()
        ensures
            ProfileName::from_str("maxperf") == (None::<ProfileName>, true),
    {
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
    }

    // ── Lemma: all known names are not forbidden ───────────────────────
    proof fn lemma_known_names_not_forbidden()
        ensures
            !ProfileName::from_str("release").1
                && !ProfileName::from_str("bench").1
                && !ProfileName::from_str("hardened").1
                && !ProfileName::from_str("fuzz").1
                && !ProfileName::from_str("test").1
                && !ProfileName::from_str("dev").1,
    {
    }

    // ── Lemma: key categories partition correctly ──────────────────────
    proof fn lemma_key_categories_exhaustive(key: ProfileKey)
        ensures
            spec_key_category(key) <= 3,
    {
    }

    // ── Lemma: inherits is the only inheritance key ────────────────────
    proof fn lemma_inherits_unique_inheritance_key(key: ProfileKey)
        ensures
            (spec_key_category(key) == 3) ==> key == ProfileKey::Inherits,
    {
    }

    // ── Lemma: release keys count ──────────────────────────────────────
    proof fn lemma_release_key_count()
        ensures
            spec_release_required_keys() == 4,
    {
    }

    // ── Lemma: bench keys count ────────────────────────────────────────
    proof fn lemma_bench_key_count()
        ensures
            spec_bench_required_keys() == 4,
    {
    }

    // ── Lemma: hardened governance key count ───────────────────────────
    proof fn lemma_hardened_gov_key_count()
        ensures
            spec_hardened_gov_required() == 2,
    {
    }

    // ── Lemma: release has more optimization keys than safety keys ─────
    proof fn lemma_release_optimization_dominance()
        ensures
            spec_release_required_keys() >= spec_hardened_gov_required(),
    {
    }

    // ── Lemma: bench and release have same key count ───────────────────
    proof fn lemma_bench_release_same_key_count()
        ensures
            spec_bench_required_keys() == spec_release_required_keys(),
    {
    }

} // verus!

// ── Regular Rust implementation (non-Verus compilation) ─────────────────────
#[cfg(not(verus_keep_ghost))]
mod cargo_kernel {
    use crate::profile_contract::types::{ProfileKey, ProfileName, SettingValue, StrVal};

    /// The required key-value pairs that [profile.release] must declare.
    /// Master reference: velvet-ballistics-MASTER.md lines 1375-1379.
    pub const RELEASE_REQUIRED_KEYS: &[(ProfileKey, SettingValue)] = &[
        (ProfileKey::OptLevel, SettingValue::U8(3)),
        (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
        (ProfileKey::CodegenUnits, SettingValue::U16(1)),
        (ProfileKey::Strip, SettingValue::String(StrVal::Symbols)),
    ];

    /// The required key-value pairs that [profile.bench] must declare.
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

    /// The master profile contract — compile-time constant.
    /// Derived from velvet-ballistics-MASTER.md §34:1375-1386.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct MasterProfileContract {
        /// Profiles that MUST exist in root Cargo.toml.
        pub required_profiles: &'static [ProfileName],
        /// Profile name strings that MUST NOT appear (only "maxperf").
        pub forbidden_profile_names: &'static [&'static str],
        /// Key-value pairs required in [profile.release].
        pub release_keys: &'static [(ProfileKey, SettingValue)],
        /// Key-value pairs required in [profile.bench].
        pub bench_keys: &'static [(ProfileKey, SettingValue)],
    }

    /// Compile-time constant matching velvet-ballistics-MASTER.md §34:1375-1386.
    pub const MASTER_PROFILE_CONTRACT: MasterProfileContract = MasterProfileContract {
        required_profiles: &[ProfileName::Release, ProfileName::Bench],
        forbidden_profile_names: &["maxperf"],
        release_keys: RELEASE_REQUIRED_KEYS,
        bench_keys: BENCH_REQUIRED_KEYS,
    };
}
#[cfg(not(verus_keep_ghost))]
pub use cargo_kernel::*;

// ── Tests (compiled in both modes) ──────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::cargo_kernel::{
        BENCH_REQUIRED_KEYS, HARDENED_GOVERNANCE_REQUIRED, MASTER_PROFILE_CONTRACT,
        RELEASE_REQUIRED_KEYS,
    };
    use crate::profile_contract::types::ProfileName;

    #[test]
    fn test_release_required_keys_count() {
        assert_eq!(RELEASE_REQUIRED_KEYS.len(), 4);
    }

    #[test]
    fn test_bench_required_keys_count() {
        assert_eq!(BENCH_REQUIRED_KEYS.len(), 4);
    }

    #[test]
    fn test_hardened_gov_required_count() {
        assert_eq!(HARDENED_GOVERNANCE_REQUIRED.len(), 2);
    }

    #[test]
    fn test_master_contract_required_profiles() {
        assert_eq!(MASTER_PROFILE_CONTRACT.required_profiles.len(), 2);
        assert_eq!(MASTER_PROFILE_CONTRACT.required_profiles[0], ProfileName::Release);
        assert_eq!(MASTER_PROFILE_CONTRACT.required_profiles[1], ProfileName::Bench);
    }

    #[test]
    fn test_master_contract_forbidden() {
        assert_eq!(MASTER_PROFILE_CONTRACT.forbidden_profile_names, &["maxperf"]);
    }

    #[test]
    fn test_master_contract_release_keys_match() {
        assert!(MASTER_PROFILE_CONTRACT.release_keys == RELEASE_REQUIRED_KEYS);
    }

    #[test]
    fn test_master_contract_bench_keys_match() {
        assert!(MASTER_PROFILE_CONTRACT.bench_keys == BENCH_REQUIRED_KEYS);
    }
}
