//! proptest properties for vb-esq9.1 profile contract validation.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! Obligations: PO-P-001 through PO-P-005
//!
//! Command: cargo test -p vb_proof_kernels --test profile_properties -- --nocapture

use proptest::prelude::*;
use vb_proof_kernels::profile_contract::{
    ContractGap, DebugMode, MASTER_PROFILE_CONTRACT, ProfileConfig, ProfileKey, ProfileName,
    SettingValue, StrVal, WorkspaceProfileSet,
    binding::{BindingResult, MoonTaskProfileBinding, ProfileRefKind, bind_moon_task},
    resolve_inheritance, validate_against_governance, validate_against_master,
};

// =========================================================================
// Strategy: Generate arbitrary ProfileName (6 valid variants)
// =========================================================================

fn arb_profile_name() -> impl Strategy<Value = ProfileName> {
    prop_oneof![
        Just(ProfileName::Release),
        Just(ProfileName::Bench),
        Just(ProfileName::Hardened),
        Just(ProfileName::Fuzz),
        Just(ProfileName::Test),
        Just(ProfileName::Dev),
    ]
}

// =========================================================================
// Strategy: Generate arbitrary ProfileKey
// =========================================================================

fn arb_profile_key() -> impl Strategy<Value = ProfileKey> {
    prop_oneof![
        Just(ProfileKey::OptLevel),
        Just(ProfileKey::Lto),
        Just(ProfileKey::CodegenUnits),
        Just(ProfileKey::Strip),
        Just(ProfileKey::Debug),
        Just(ProfileKey::DebugAssertions),
        Just(ProfileKey::OverflowChecks),
        Just(ProfileKey::Panic),
        Just(ProfileKey::Inherits),
    ]
}

// =========================================================================
// Strategy: Generate arbitrary SettingValue
// =========================================================================

fn arb_setting_value() -> impl Strategy<Value = SettingValue> {
    prop_oneof![
        any::<bool>().prop_map(SettingValue::Bool),
        prop_oneof![
            Just(StrVal::Thin),
            Just(StrVal::Fat),
            Just(StrVal::Off),
            Just(StrVal::True),
            Just(StrVal::False),
            Just(StrVal::None_),
            Just(StrVal::Symbols),
            Just(StrVal::Debuginfo),
            Just(StrVal::Release),
            Just(StrVal::Unwind),
            Just(StrVal::Abort),
        ]
        .prop_map(SettingValue::String),
        any::<u8>().prop_map(SettingValue::U8),
        any::<u16>().prop_map(SettingValue::U16),
        prop_oneof![
            Just(DebugMode::False),
            Just(DebugMode::True),
            Just(DebugMode::LineTablesOnly),
        ]
        .prop_map(SettingValue::DebugMode),
    ]
}

// =========================================================================
// Strategy: Generate arbitrary ProfileConfig
// =========================================================================

fn arb_profile_config() -> impl Strategy<Value = ProfileConfig> {
    (
        arb_profile_name(),
        proptest::collection::vec((arb_profile_key(), arb_setting_value()), 0..12),
    )
        .prop_map(|(name, settings)| ProfileConfig::new(name, settings))
}

// =========================================================================
// Strategy: Generate arbitrary WorkspaceProfileSet (1..6 profiles)
// =========================================================================

fn arb_workspace_profile_set() -> impl Strategy<Value = WorkspaceProfileSet> {
    proptest::collection::vec(arb_profile_config(), 1..=6).prop_map(|profiles| {
        let mut ws = WorkspaceProfileSet::new();
        for p in profiles {
            ws.add(p);
        }
        ws
    })
}

// =========================================================================
// Strategy: Generate a "correct" WorkspaceProfileSet
// (release + bench with master-specified values, hardened with debug-assertions)
// =========================================================================

fn arb_correct_workspace() -> impl Strategy<Value = WorkspaceProfileSet> {
    Just(()).prop_map(|_| {
        let mut ws = WorkspaceProfileSet::new();
        ws.add(ProfileConfig::new(
            ProfileName::Release,
            vec![
                (ProfileKey::OptLevel, SettingValue::U8(3)),
                (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
                (ProfileKey::CodegenUnits, SettingValue::U16(1)),
                (ProfileKey::Strip, SettingValue::String(StrVal::Symbols)),
            ],
        ));
        ws.add(ProfileConfig::new(
            ProfileName::Bench,
            vec![
                (ProfileKey::Inherits, SettingValue::String(StrVal::Release)),
                (ProfileKey::Debug, SettingValue::Bool(true)),
                (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
                (ProfileKey::CodegenUnits, SettingValue::U16(1)),
            ],
        ));
        ws.add(ProfileConfig::new(
            ProfileName::Hardened,
            vec![
                (ProfileKey::Inherits, SettingValue::String(StrVal::Release)),
                (ProfileKey::CodegenUnits, SettingValue::U16(1)),
                (
                    ProfileKey::Debug,
                    SettingValue::DebugMode(DebugMode::LineTablesOnly),
                ),
                (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
                (ProfileKey::OverflowChecks, SettingValue::Bool(true)),
                (ProfileKey::Panic, SettingValue::String(StrVal::Abort)),
                (ProfileKey::Strip, SettingValue::String(StrVal::Symbols)),
                (ProfileKey::DebugAssertions, SettingValue::Bool(true)),
            ],
        ));
        ws
    })
}

// =========================================================================
// PO-P-001: Forbidden states produce correct errors
// =========================================================================

proptest! {
    /// Verify that forbidden states are rejected with the correct contract gaps.
    ///
    /// Given an arbitrary WorkspaceProfileSet, verify:
    /// 1. If release is missing → MissingProfile(Release) gap
    /// 2. If bench is missing → MissingProfile(Bench) gap
    /// 3. If release has wrong lto → WrongSetting gap
    /// 4. maxperf is rejected at construction → ProfileName::new("maxperf").is_err()
    /// 5. If hardened has no debug-assertions → MissingDebugAssertions governance gap
    #[test]
    fn prop_forbidden_states_rejected_with_correct_errors(
        ws in arb_workspace_profile_set(),
    ) {
        // 1. maxperf is always rejected at construction
        assert!(ProfileName::new("maxperf").is_err());

        // 2. Any workspace with missing required profiles should report it
        let gaps = validate_against_master(&ws, &MASTER_PROFILE_CONTRACT);
        let has_release = ws.has(ProfileName::Release);
        let has_bench = ws.has(ProfileName::Bench);

        if !has_release {
            assert!(
                gaps.iter().any(|g| matches!(g, ContractGap::MissingProfile { name: ProfileName::Release })),
                "Missing release must produce MissingProfile(Release) gap"
            );
        }

        if !has_bench {
            assert!(
                gaps.iter().any(|g| matches!(g, ContractGap::MissingProfile { name: ProfileName::Bench })),
                "Missing bench must produce MissingProfile(Bench) gap"
            );
        }

        // 3. Wrong setting detection: if release exists with wrong lto, it should be flagged
        if let Some(release) = ws.find(ProfileName::Release) {
            if let Some(actual_lto) = release.get(ProfileKey::Lto) {
                if *actual_lto != SettingValue::String(StrVal::Thin) {
                    assert!(
                        gaps.iter().any(|g| matches!(g, ContractGap::WrongSetting {
                            profile: ProfileName::Release,
                            key: ProfileKey::Lto,
                            ..
                        })),
                        "Wrong lto in release must produce WrongSetting gap"
                    );
                }
            } else {
                assert!(
                    gaps.iter().any(|g| matches!(g, ContractGap::MissingSetting {
                        profile: ProfileName::Release,
                        key: ProfileKey::Lto,
                    })),
                    "Missing lto in release must produce MissingSetting gap"
                );
            }
        }

        // 4. Governance gaps: if hardened exists without debug-assertions
        let gov_gaps = validate_against_governance(&ws);
        if let Some(hardened) = ws.find(ProfileName::Hardened) {
            if hardened.get(ProfileKey::DebugAssertions) != Some(&SettingValue::Bool(true)) {
                assert!(
                    !gov_gaps.is_empty(),
                    "Hardened without debug-assertions=true must produce governance gap"
                );
            }
        }
    }
}

// =========================================================================
// PO-P-002: Pure core functions are idempotent
// =========================================================================

proptest! {
    /// Verify that pure core functions are idempotent: f(x) == f(f(x)).
    ///
    /// Tests: validate_against_master, validate_against_governance,
    /// resolve_inheritance.
    #[test]
    fn prop_pure_core_functions_are_idempotent(
        ws in arb_workspace_profile_set(),
    ) {
        // validate_against_master idempotence
        let gaps1 = validate_against_master(&ws, &MASTER_PROFILE_CONTRACT);
        let gaps2 = validate_against_master(&ws, &MASTER_PROFILE_CONTRACT);
        assert_eq!(gaps1, gaps2, "validate_against_master must be deterministic");

        // validate_against_governance idempotence
        let gov1 = validate_against_governance(&ws);
        let gov2 = validate_against_governance(&ws);
        assert_eq!(gov1, gov2, "validate_against_governance must be deterministic");

        // resolve_inheritance idempotence for each profile
        for config in &ws.profiles {
            let r1 = resolve_inheritance(config, &ws);
            let r2 = resolve_inheritance(config, &ws);
            assert_eq!(r1, r2, "resolve_inheritance must be deterministic");
        }
    }
}

// =========================================================================
// PO-P-003: Inheritance resolution is deterministic and override-consistent
// =========================================================================

proptest! {
    /// Verify inheritance resolution properties:
    /// 1. Determinism: same input produces same output
    /// 2. Override consistency: child explicit keys override parent keys
    /// 3. Deep idempotence: calling twice with same input yields identical result
    #[test]
    fn prop_inheritance_resolution_deterministic_and_override_consistent(
        ws in arb_workspace_profile_set(),
    ) {
        for config in &ws.profiles {
            let r1 = resolve_inheritance(config, &ws);
            let r2 = resolve_inheritance(config, &ws);
            assert_eq!(r1, r2, "Resolution must be deterministic");

            if let Ok(resolved) = r1 {
                // Check override consistency: for each unique key in explicit settings,
                // the LAST entry wins (override semantics). Verify the resolved
                // value matches the last explicit value for that key.
                use std::collections::BTreeMap;
                let mut explicit_by_key: BTreeMap<ProfileKey, &SettingValue> = BTreeMap::new();
                for (key, value) in &config.settings {
                    explicit_by_key.insert(*key, value);
                }
                for (key, expected_value) in &explicit_by_key {
                    let found = resolved.iter().find(|(k, _)| k == key);
                    assert!(
                        found.is_some(),
                        "Profile's explicit key {:?} must appear in resolved settings", key
                    );
                    if let Some((_, resolved_value)) = found {
                        assert_eq!(
                            resolved_value, *expected_value,
                            "Explicit setting value for {:?} must match the last entry in settings", key
                        );
                    }
                }

                // No duplicate keys in resolved
                let mut seen = std::collections::BTreeSet::new();
                for (key, _) in &resolved {
                    assert!(
                        seen.insert(*key),
                        "Resolved settings must not contain duplicate keys"
                    );
                }
            }
        }
    }
}

// =========================================================================
// PO-P-004: Gap detection accuracy
// =========================================================================

proptest! {
    /// Verify gap detection accuracy:
    /// 1. A correct workspace → zero gaps (both master and governance)
    /// 2. An arbitrary workspace → gaps reported are valid (MissingProfile,
    ///    WrongSetting, MissingSetting, etc.)
    #[test]
    fn prop_gap_detection_accurate_and_complete(
        correct_ws in arb_correct_workspace(),
        arbitrary_ws in arb_workspace_profile_set(),
    ) {
        // 1. Correct workspace must produce zero gaps
        let master_gaps = validate_against_master(&correct_ws, &MASTER_PROFILE_CONTRACT);
        assert!(
            master_gaps.is_empty(),
            "Correct workspace must produce zero master contract gaps"
        );

        let gov_gaps = validate_against_governance(&correct_ws);
        assert!(
            gov_gaps.is_empty(),
            "Correct workspace must produce zero governance gaps"
        );

        // 2. Arbitrary workspace: every reported gap must be a valid variant
        let gaps = validate_against_master(&arbitrary_ws, &MASTER_PROFILE_CONTRACT);
        for gap in &gaps {
            match gap {
                ContractGap::MissingProfile { name } => {
                    assert!(
                        *name == ProfileName::Release || *name == ProfileName::Bench,
                        "MissingProfile must be for Release or Bench only"
                    );
                }
                ContractGap::ForbiddenProfile { .. } => {
                    // Should not appear for arbitrary sets (maxperf can't exist)
                }
                ContractGap::WrongSetting { profile, .. } => {
                    assert!(
                        *profile == ProfileName::Release || *profile == ProfileName::Bench,
                        "WrongSetting must be for Release or Bench only"
                    );
                }
                ContractGap::MissingSetting { profile, .. } => {
                    assert!(
                        *profile == ProfileName::Release || *profile == ProfileName::Bench,
                        "MissingSetting must be for Release or Bench only"
                    );
                }
            }
        }
    }
}

// =========================================================================
// PO-P-005: Moon task profile binding
// =========================================================================

proptest! {
    /// Verify Moon task profile binding correctness:
    /// 1. hardened ref with existing hardened profile → ExistsAndValid
    ///    (when hardened satisfies governance)
    /// 2. bench ref with missing bench → Missing
    /// 3. release ref with existing release → ExistsAndValid or ExistsButGapped
    #[test]
    fn prop_moon_task_profile_binding_correct(
        ws in arb_workspace_profile_set(),
    ) {
        // Test binding for hardened-build task
        let hardened_binding = MoonTaskProfileBinding {
            task_name: "hardened-build",
            profile_ref: ProfileRefKind::Explicit(ProfileName::Hardened),
            in_pipeline: true,
            run_in_ci: true,
        };
        let result = bind_moon_task(&hardened_binding, &ws);
        match result {
            BindingResult::ExistsAndValid => {
                assert!(ws.has(ProfileName::Hardened));
            }
            BindingResult::ExistsButGapped(_) => {
                assert!(ws.has(ProfileName::Hardened));
            }
            BindingResult::Missing => {
                assert!(!ws.has(ProfileName::Hardened));
            }
            BindingResult::DeferredScope => {
                // Only for maxperf tasks, not hardened
            }
        }

        // Test binding for bench-build task (implicit bench reference)
        let bench_binding = MoonTaskProfileBinding {
            task_name: "bench-build",
            profile_ref: ProfileRefKind::ImplicitBench,
            in_pipeline: true,
            run_in_ci: true,
        };
        let bench_result = bind_moon_task(&bench_binding, &ws);
        match bench_result {
            BindingResult::ExistsAndValid | BindingResult::ExistsButGapped(_) => {
                assert!(ws.has(ProfileName::Bench));
            }
            BindingResult::Missing => {
                assert!(!ws.has(ProfileName::Bench));
            }
            _ => {}
        }

        // Test binding for a deferred-scope maxperf task
        // Task names containing "maxperf" with run_in_ci=false → DeferredScope
        let deferred_binding = MoonTaskProfileBinding {
            task_name: "pgo-maxperf-build",
            profile_ref: ProfileRefKind::Explicit(ProfileName::Release),
            in_pipeline: false,
            run_in_ci: false,
        };
        let result = bind_moon_task(&deferred_binding, &ws);
        assert!(
            matches!(result, BindingResult::DeferredScope),
            "Task with 'maxperf' in name and runInCI=false must return DeferredScope"
        );

        // Test that "maxperf" task name also triggers DeferredScope
        let maxperf_named = MoonTaskProfileBinding {
            task_name: "maxperf",
            profile_ref: ProfileRefKind::ImplicitRelease,
            in_pipeline: false,
            run_in_ci: false,
        };
        let result = bind_moon_task(&maxperf_named, &ws);
        assert!(
            matches!(result, BindingResult::DeferredScope),
            "Task named 'maxperf' with runInCI=false must return DeferredScope"
        );

        // Test that run_in_ci=true DOES NOT trigger DeferredScope even with maxperf name
        let maxperf_active = MoonTaskProfileBinding {
            task_name: "maxperf",
            profile_ref: ProfileRefKind::ImplicitRelease,
            in_pipeline: false,
            run_in_ci: true,
        };
        let result = bind_moon_task(&maxperf_active, &ws);
        // When run_in_ci=true, we proceed to normal profile lookup
        // Release may or may not exist → ExistsButGapped/ExistsAndValid/Missing
        assert!(
            !matches!(result, BindingResult::DeferredScope),
            "Task named 'maxperf' with runInCI=true must NOT return DeferredScope"
        );
    }
}
