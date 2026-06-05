//! proptest properties for vb-esq9.1 profile contract validation.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! Obligations: PO-P-001 through PO-P-005
//!
//! Command: cargo test -p vb_proof_kernels --test profile_properties -- --nocapture

mod profile_property_cases;

use proptest::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use vb_proof_kernels::profile_contract::{
    ContractGap, MASTER_PROFILE_CONTRACT, ProfileKey, ProfileName, SettingValue, StrVal,
    binding::{BindingResult, MoonTaskProfileBinding, ProfileRefKind, bind_moon_task},
    resolve_inheritance, validate_against_governance, validate_against_master,
};

use profile_property_cases::{arb_correct_workspace, arb_workspace_profile_set};

proptest! {
    #[test]
    fn prop_forbidden_states_rejected_with_correct_errors(ws in arb_workspace_profile_set()) {
        assert_eq!(ProfileName::new("maxperf").is_err(), true);
        let gaps = validate_against_master(&ws, &MASTER_PROFILE_CONTRACT);
        if !ws.has(ProfileName::Release) {
            assert_eq!(has_missing(&gaps, ProfileName::Release), true);
        }
        if !ws.has(ProfileName::Bench) {
            assert_eq!(has_missing(&gaps, ProfileName::Bench), true);
        }
        if let Some(release) = ws.find(ProfileName::Release) {
            assert_release_lto_gap(release.get(ProfileKey::Lto), &gaps);
        }
        if let Some(hardened) = ws.find(ProfileName::Hardened) {
            let gov_gaps = validate_against_governance(&ws);
            if hardened.get(ProfileKey::DebugAssertions) != Some(&SettingValue::Bool(true)) {
                assert_eq!(gov_gaps.is_empty(), false);
            }
        }
    }
}

proptest! {
    #[test]
    fn prop_pure_core_functions_are_idempotent(ws in arb_workspace_profile_set()) {
        assert_eq!(validate_against_master(&ws, &MASTER_PROFILE_CONTRACT), validate_against_master(&ws, &MASTER_PROFILE_CONTRACT));
        assert_eq!(validate_against_governance(&ws), validate_against_governance(&ws));
        for config in &ws.profiles {
            assert_eq!(resolve_inheritance(config, &ws), resolve_inheritance(config, &ws));
        }
    }
}

proptest! {
    #[test]
    fn prop_inheritance_resolution_deterministic_and_override_consistent(ws in arb_workspace_profile_set()) {
        for config in &ws.profiles {
            let r1 = resolve_inheritance(config, &ws);
            let r2 = resolve_inheritance(config, &ws);
            assert_eq!(r1, r2);
            if let Ok(resolved) = r1 {
                assert_explicit_overrides_resolved(config.settings.as_ref(), &resolved);
                assert_no_duplicate_keys(&resolved);
            }
        }
    }
}

proptest! {
    #[test]
    fn prop_gap_detection_accurate_and_complete(correct_ws in arb_correct_workspace(), arbitrary_ws in arb_workspace_profile_set()) {
        assert_eq!(validate_against_master(&correct_ws, &MASTER_PROFILE_CONTRACT).is_empty(), true);
        assert_eq!(validate_against_governance(&correct_ws).is_empty(), true);
        let gaps = validate_against_master(&arbitrary_ws, &MASTER_PROFILE_CONTRACT);
        for gap in &gaps {
            assert_valid_gap_variant(gap);
        }
    }
}

proptest! {
    #[test]
    fn prop_moon_task_profile_binding_correct(ws in arb_workspace_profile_set()) {
        assert_binding_matches_workspace(&ws, "hardened-build", ProfileRefKind::Explicit(ProfileName::Hardened), ProfileName::Hardened);
        assert_binding_matches_workspace(&ws, "bench-build", ProfileRefKind::ImplicitBench, ProfileName::Bench);
        assert_deferred_scope_bindings(&ws);
    }
}

fn has_missing(gaps: &[ContractGap], name: ProfileName) -> bool {
    gaps.iter()
        .any(|gap| matches!(gap, ContractGap::MissingProfile { name: found } if *found == name))
}

fn assert_release_lto_gap(actual_lto: Option<&SettingValue>, gaps: &[ContractGap]) {
    match actual_lto {
        Some(actual) if *actual != SettingValue::String(StrVal::Thin) => assert_eq!(
            gaps.iter().any(|gap| matches!(
                gap,
                ContractGap::WrongSetting {
                    profile: ProfileName::Release,
                    key: ProfileKey::Lto,
                    ..
                }
            )),
            true
        ),
        None => assert_eq!(
            gaps.iter().any(|gap| matches!(
                gap,
                ContractGap::MissingSetting {
                    profile: ProfileName::Release,
                    key: ProfileKey::Lto,
                }
            )),
            true
        ),
        Some(_) => {}
    }
}

fn assert_explicit_overrides_resolved(
    settings: &[(ProfileKey, SettingValue)],
    resolved: &[(ProfileKey, SettingValue)],
) {
    let mut explicit_by_key: BTreeMap<ProfileKey, &SettingValue> = BTreeMap::new();
    for (key, value) in settings {
        explicit_by_key.insert(*key, value);
    }
    for (key, expected_value) in &explicit_by_key {
        let found = resolved
            .iter()
            .find(|(resolved_key, _)| resolved_key == key);
        assert_eq!(found.is_some(), true);
        if let Some((_, resolved_value)) = found {
            assert_eq!(resolved_value, *expected_value);
        }
    }
}

fn assert_no_duplicate_keys(resolved: &[(ProfileKey, SettingValue)]) {
    let mut seen = BTreeSet::new();
    for (key, _) in resolved {
        assert_eq!(seen.insert(*key), true);
    }
}

fn assert_valid_gap_variant(gap: &ContractGap) {
    match gap {
        ContractGap::MissingProfile { name } => assert_eq!(is_master_profile(*name), true),
        ContractGap::ForbiddenProfile { .. } => {}
        ContractGap::WrongSetting { profile, .. } => assert_eq!(is_master_profile(*profile), true),
        ContractGap::MissingSetting { profile, .. } => {
            assert_eq!(is_master_profile(*profile), true)
        }
    }
}

fn is_master_profile(profile: ProfileName) -> bool {
    profile == ProfileName::Release || profile == ProfileName::Bench
}

fn assert_binding_matches_workspace(
    ws: &vb_proof_kernels::profile_contract::WorkspaceProfileSet,
    task_name: &'static str,
    profile_ref: ProfileRefKind,
    expected: ProfileName,
) {
    let binding = MoonTaskProfileBinding {
        task_name,
        profile_ref,
        in_pipeline: true,
        run_in_ci: true,
    };
    match bind_moon_task(&binding, ws) {
        BindingResult::ExistsAndValid | BindingResult::ExistsButGapped(_) => {
            assert_eq!(ws.has(expected), true)
        }
        BindingResult::Missing => assert_eq!(ws.has(expected), false),
        BindingResult::DeferredScope => {}
    }
}

fn assert_deferred_scope_bindings(ws: &vb_proof_kernels::profile_contract::WorkspaceProfileSet) {
    let deferred_binding = MoonTaskProfileBinding {
        task_name: "pgo-maxperf-build",
        profile_ref: ProfileRefKind::Explicit(ProfileName::Release),
        in_pipeline: false,
        run_in_ci: false,
    };
    assert_eq!(
        matches!(
            bind_moon_task(&deferred_binding, ws),
            BindingResult::DeferredScope
        ),
        true
    );
    let maxperf_active = MoonTaskProfileBinding {
        task_name: "maxperf",
        profile_ref: ProfileRefKind::ImplicitRelease,
        in_pipeline: false,
        run_in_ci: true,
    };
    assert_eq!(
        matches!(
            bind_moon_task(&maxperf_active, ws),
            BindingResult::DeferredScope
        ),
        false
    );
}
