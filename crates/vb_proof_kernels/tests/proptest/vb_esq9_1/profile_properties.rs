//! vb-esq9.1 inline proptest properties.
//!
//! This module is loaded as part of the profile_properties test binary
//! via tests/proptest/vb_esq9_1/mod.rs.
//!
//! All proptest properties here are unique to this module; the modularized
//! versions in profile_property_cases/ are loaded as a sibling module.

use proptest::prelude::*;
use vb_proof_kernels::profile_contract::{
    ContractGap, MASTER_PROFILE_CONTRACT, ProfileKey, ProfileName, SettingValue, StrVal,
    binding::{BindingResult, MoonTaskProfileBinding, ProfileRefKind, bind_moon_task},
    validate_against_governance, validate_against_master,
};

use super::profile_property_cases::strategies::{arb_correct_workspace, arb_workspace_profile_set};

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
        if let Some(hardened) = ws.find(ProfileName::Hardened)
            && hardened.get(ProfileKey::DebugAssertions) != Some(&SettingValue::Bool(true))
        {
            assert!(
                !gov_gaps.is_empty(),
                "Hardened without debug-assertions=true must produce governance gap"
            );
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
