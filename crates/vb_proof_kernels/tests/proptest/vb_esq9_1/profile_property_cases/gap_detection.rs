//! Forbidden-state and gap-detection profile properties.

use proptest::prelude::*;
use vb_proof_kernels::profile_contract::{
    ContractGap, MASTER_PROFILE_CONTRACT, ProfileKey, ProfileName, SettingValue, StrVal,
    validate_against_governance, validate_against_master,
};

use super::strategies::{arb_correct_workspace, arb_workspace_profile_set};

proptest! {
    #[test]
    fn prop_forbidden_states_rejected_with_correct_errors(ws in arb_workspace_profile_set()) {
        assert!(ProfileName::new("maxperf").is_err());
        let gaps = validate_against_master(&ws, &MASTER_PROFILE_CONTRACT);
        let has_release = ws.has(ProfileName::Release);
        let has_bench = ws.has(ProfileName::Bench);

        if !has_release {
            assert!(
                gaps.iter().any(|gap| matches!(gap, ContractGap::MissingProfile { name: ProfileName::Release })),
                "Missing release must produce MissingProfile(Release) gap"
            );
        }
        if !has_bench {
            assert!(
                gaps.iter().any(|gap| matches!(gap, ContractGap::MissingProfile { name: ProfileName::Bench })),
                "Missing bench must produce MissingProfile(Bench) gap"
            );
        }

        if let Some(release) = ws.find(ProfileName::Release) {
            assert_release_lto_gap(release.get(ProfileKey::Lto), &gaps);
        }
        let gov_gaps = validate_against_governance(&ws);
        if let Some(hardened) = ws.find(ProfileName::Hardened)
            && hardened.get(ProfileKey::DebugAssertions) != Some(&SettingValue::Bool(true))
        {
            assert!(!gov_gaps.is_empty(), "Hardened without debug-assertions=true must produce governance gap");
        }
    }

    #[test]
    fn prop_gap_detection_accurate_and_complete(
        correct_ws in arb_correct_workspace(),
        arbitrary_ws in arb_workspace_profile_set(),
    ) {
        let master_gaps = validate_against_master(&correct_ws, &MASTER_PROFILE_CONTRACT);
        assert!(master_gaps.is_empty(), "Correct workspace must produce zero master contract gaps");
        let gov_gaps = validate_against_governance(&correct_ws);
        assert!(gov_gaps.is_empty(), "Correct workspace must produce zero governance gaps");

        let gaps = validate_against_master(&arbitrary_ws, &MASTER_PROFILE_CONTRACT);
        for gap in &gaps {
            assert_valid_gap_variant(gap);
        }
    }
}

fn assert_release_lto_gap(actual_lto: Option<&SettingValue>, gaps: &[ContractGap]) {
    if let Some(actual) = actual_lto {
        if *actual != SettingValue::String(StrVal::Thin) {
            assert!(
                gaps.iter().any(|gap| matches!(
                    gap,
                    ContractGap::WrongSetting {
                        profile: ProfileName::Release,
                        key: ProfileKey::Lto,
                        ..
                    }
                )),
                "Wrong lto in release must produce WrongSetting gap"
            );
        }
    } else {
        assert!(
            gaps.iter().any(|gap| matches!(
                gap,
                ContractGap::MissingSetting {
                    profile: ProfileName::Release,
                    key: ProfileKey::Lto,
                }
            )),
            "Missing lto in release must produce MissingSetting gap"
        );
    }
}

fn assert_valid_gap_variant(gap: &ContractGap) {
    match gap {
        ContractGap::MissingProfile { name } => {
            assert!(*name == ProfileName::Release || *name == ProfileName::Bench);
        }
        ContractGap::ForbiddenProfile { .. } => {}
        ContractGap::WrongSetting { profile, .. } => {
            assert!(*profile == ProfileName::Release || *profile == ProfileName::Bench);
        }
        ContractGap::MissingSetting { profile, .. } => {
            assert!(*profile == ProfileName::Release || *profile == ProfileName::Bench);
        }
    }
}
