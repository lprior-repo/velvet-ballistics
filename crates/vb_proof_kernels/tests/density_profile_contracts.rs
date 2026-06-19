#![forbid(unsafe_code)]

use vb_proof_kernels::profile_contract::{
    BindingResult, ContractGap, GovernanceGap, MASTER_PROFILE_CONTRACT, MoonTaskProfileBinding,
    ProfileConfig, ProfileKey, ProfileKeyError, ProfileName, ProfileNameError, ProfileRefKind,
    ResolveError, SettingValue, SettingValueError, StrVal, WorkspaceProfileSet, bind_moon_task,
    resolve_inheritance, validate_against_governance, validate_against_master,
};

macro_rules! ktest {
    ($(#[$attr:meta])* $name:ident, $body:block) => {
        $(#[$attr])*
        fn $name() $body
    };
}

fn release_profile() -> ProfileConfig {
    ProfileConfig::new(
        ProfileName::Release,
        vec![
            (ProfileKey::OptLevel, SettingValue::U8(3)),
            (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
            (ProfileKey::CodegenUnits, SettingValue::U16(1)),
            (ProfileKey::Strip, SettingValue::String(StrVal::Symbols)),
        ],
    )
}

fn bench_profile() -> ProfileConfig {
    ProfileConfig::new(
        ProfileName::Bench,
        vec![
            (ProfileKey::Inherits, SettingValue::String(StrVal::Release)),
            (ProfileKey::Debug, SettingValue::Bool(true)),
            (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
            (ProfileKey::CodegenUnits, SettingValue::U16(1)),
        ],
    )
}

fn hardened_profile(debug_assertions: bool, overflow_checks: bool) -> ProfileConfig {
    ProfileConfig::new(
        ProfileName::Hardened,
        vec![
            (
                ProfileKey::DebugAssertions,
                SettingValue::Bool(debug_assertions),
            ),
            (
                ProfileKey::OverflowChecks,
                SettingValue::Bool(overflow_checks),
            ),
        ],
    )
}

fn correct_workspace() -> WorkspaceProfileSet {
    let mut set = WorkspaceProfileSet::new();
    set.add(release_profile());
    set.add(bench_profile());
    set
}

ktest!(
    #[test]
    profile_name_accepts_release,
    {
        assert_eq!(ProfileName::new("release"), Ok(ProfileName::Release));
    }
);

ktest!(
    #[test]
    profile_name_accepts_bench,
    {
        assert_eq!(ProfileName::new("bench"), Ok(ProfileName::Bench));
    }
);

ktest!(
    #[test]
    profile_name_accepts_hardened,
    {
        assert_eq!(ProfileName::new("hardened"), Ok(ProfileName::Hardened));
    }
);

ktest!(
    #[test]
    profile_name_rejects_maxperf,
    {
        assert_eq!(
            ProfileName::new("maxperf"),
            Err(ProfileNameError::Forbidden)
        );
    }
);

ktest!(
    #[test]
    profile_name_reports_unknown_name,
    {
        assert_eq!(
            ProfileName::new("release-fast"),
            Err(ProfileNameError::Unknown(String::from("release-fast")))
        );
    }
);

ktest!(
    #[test]
    profile_name_release_as_str_is_release,
    {
        assert_eq!(ProfileName::Release.as_str(), "release");
    }
);

ktest!(
    #[test]
    profile_name_dev_as_str_is_dev,
    {
        assert_eq!(ProfileName::Dev.as_str(), "dev");
    }
);

ktest!(
    #[test]
    profile_key_parses_opt_level,
    {
        assert_eq!(
            ProfileKey::from_toml_key("opt-level"),
            Ok(ProfileKey::OptLevel)
        );
    }
);

ktest!(
    #[test]
    profile_key_parses_debug_assertions,
    {
        assert_eq!(
            ProfileKey::from_toml_key("debug-assertions"),
            Ok(ProfileKey::DebugAssertions)
        );
    }
);

ktest!(
    #[test]
    profile_key_reports_unknown_key,
    {
        assert_eq!(
            ProfileKey::from_toml_key("codegen"),
            Err(ProfileKeyError::Unknown(String::from("codegen")))
        );
    }
);

ktest!(
    #[test]
    profile_key_all_keys_covers_nine_variants,
    {
        assert_eq!(ProfileKey::ALL_KEYS.len(), 9);
    }
);

ktest!(
    #[test]
    strval_parse_thin,
    {
        assert_eq!(StrVal::parse("thin"), StrVal::Thin);
    }
);

ktest!(
    #[test]
    strval_parse_unknown_as_other,
    {
        assert_eq!(StrVal::parse("local"), StrVal::Other);
    }
);

ktest!(
    #[test]
    strval_abort_as_str,
    {
        assert_eq!(StrVal::Abort.as_str(), "abort");
    }
);

ktest!(
    #[test]
    setting_value_accepts_release_opt_level,
    {
        assert_eq!(
            SettingValue::for_key(ProfileKey::OptLevel, SettingValue::U8(3)),
            Ok(SettingValue::U8(3))
        );
    }
);

ktest!(
    #[test]
    setting_value_rejects_noncontract_opt_level,
    {
        assert_eq!(
            SettingValue::for_key(ProfileKey::OptLevel, SettingValue::U8(2)),
            Err(SettingValueError::InvalidOptLevel(2))
        );
    }
);

ktest!(
    #[test]
    setting_value_accepts_single_codegen_unit,
    {
        assert_eq!(
            SettingValue::for_key(ProfileKey::CodegenUnits, SettingValue::U16(1)),
            Ok(SettingValue::U16(1))
        );
    }
);

ktest!(
    #[test]
    setting_value_rejects_multiple_codegen_units,
    {
        assert_eq!(
            SettingValue::for_key(ProfileKey::CodegenUnits, SettingValue::U16(2)),
            Err(SettingValueError::InvalidCodegenUnits(2))
        );
    }
);

ktest!(
    #[test]
    setting_value_accepts_lto_thin,
    {
        assert_eq!(
            SettingValue::for_key(ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
            Ok(SettingValue::String(StrVal::Thin))
        );
    }
);

ktest!(
    #[test]
    setting_value_rejects_lto_release_string,
    {
        assert_eq!(
            SettingValue::for_key(ProfileKey::Lto, SettingValue::String(StrVal::Release)),
            Err(SettingValueError::InvalidLto)
        );
    }
);

ktest!(
    #[test]
    setting_value_accepts_strip_symbols,
    {
        assert_eq!(
            SettingValue::for_key(ProfileKey::Strip, SettingValue::String(StrVal::Symbols)),
            Ok(SettingValue::String(StrVal::Symbols))
        );
    }
);

ktest!(
    #[test]
    setting_value_rejects_strip_thin,
    {
        assert_eq!(
            SettingValue::for_key(ProfileKey::Strip, SettingValue::String(StrVal::Thin)),
            Err(SettingValueError::InvalidStrip)
        );
    }
);

ktest!(
    #[test]
    setting_value_accepts_panic_unwind,
    {
        assert_eq!(
            SettingValue::for_key(ProfileKey::Panic, SettingValue::String(StrVal::Unwind)),
            Ok(SettingValue::String(StrVal::Unwind))
        );
    }
);

ktest!(
    #[test]
    setting_value_rejects_panic_thin,
    {
        assert_eq!(
            SettingValue::for_key(ProfileKey::Panic, SettingValue::String(StrVal::Thin)),
            Err(SettingValueError::InvalidPanic)
        );
    }
);

ktest!(
    #[test]
    profile_config_records_release_inheritance,
    {
        assert!(bench_profile().inherits_from(ProfileName::Release));
    }
);

ktest!(
    #[test]
    profile_config_ignores_nonrelease_inherits_string,
    {
        let config = ProfileConfig::new(
            ProfileName::Bench,
            vec![(ProfileKey::Inherits, SettingValue::String(StrVal::True))],
        );
        assert!(!config.inherits_from(ProfileName::Release));
    }
);

ktest!(
    #[test]
    profile_config_get_returns_matching_setting,
    {
        assert_eq!(
            release_profile().get(ProfileKey::Lto),
            Some(&SettingValue::String(StrVal::Thin))
        );
    }
);

ktest!(
    #[test]
    profile_config_get_returns_none_for_absent_setting,
    {
        assert_eq!(release_profile().get(ProfileKey::Panic), None);
    }
);

ktest!(
    #[test]
    workspace_new_is_empty,
    {
        let set = WorkspaceProfileSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }
);

ktest!(
    #[test]
    workspace_add_find_has_and_len_track_profile,
    {
        let mut set = WorkspaceProfileSet::new();
        set.add(release_profile());
        assert_eq!(set.len(), 1);
        assert!(set.has(ProfileName::Release));
        assert_eq!(
            set.find(ProfileName::Release).map(|p| p.name),
            Some(ProfileName::Release)
        );
    }
);

ktest!(
    #[test]
    master_validation_reports_missing_required_profiles,
    {
        let gaps = validate_against_master(&WorkspaceProfileSet::new(), &MASTER_PROFILE_CONTRACT);
        assert!(gaps.contains(&ContractGap::MissingProfile {
            name: ProfileName::Release
        }));
        assert!(gaps.contains(&ContractGap::MissingProfile {
            name: ProfileName::Bench
        }));
    }
);

ktest!(
    #[test]
    master_validation_accepts_correct_workspace,
    {
        assert!(validate_against_master(&correct_workspace(), &MASTER_PROFILE_CONTRACT).is_empty());
    }
);

ktest!(
    #[test]
    master_validation_reports_wrong_release_lto,
    {
        let mut set = WorkspaceProfileSet::new();
        set.add(ProfileConfig::new(
            ProfileName::Release,
            vec![
                (ProfileKey::OptLevel, SettingValue::U8(3)),
                (ProfileKey::Lto, SettingValue::String(StrVal::Fat)),
                (ProfileKey::CodegenUnits, SettingValue::U16(1)),
                (ProfileKey::Strip, SettingValue::String(StrVal::Symbols)),
            ],
        ));
        set.add(bench_profile());
        assert!(
            validate_against_master(&set, &MASTER_PROFILE_CONTRACT)
                .iter()
                .any(|gap| matches!(
                    gap,
                    ContractGap::WrongSetting {
                        profile: ProfileName::Release,
                        key: ProfileKey::Lto,
                        ..
                    }
                ))
        );
    }
);

ktest!(
    #[test]
    governance_validation_reports_missing_debug_assertions,
    {
        let mut set = WorkspaceProfileSet::new();
        set.add(hardened_profile(false, true));
        assert_eq!(
            validate_against_governance(&set),
            vec![GovernanceGap::MissingDebugAssertions]
        );
    }
);

ktest!(
    #[test]
    governance_validation_accepts_hardened_profile,
    {
        let mut set = WorkspaceProfileSet::new();
        set.add(hardened_profile(true, true));
        assert!(validate_against_governance(&set).is_empty());
    }
);

ktest!(
    #[test]
    inheritance_resolution_includes_parent_setting,
    {
        let mut set = WorkspaceProfileSet::new();
        set.add(release_profile());
        let child = bench_profile();
        set.add(child.clone());
        assert!(matches!(
            resolve_inheritance(&child, &set),
            Ok(settings) if settings.contains(&(ProfileKey::Strip, SettingValue::String(StrVal::Symbols)))
        ));
    }
);

ktest!(
    #[test]
    inheritance_resolution_child_overrides_parent_lto,
    {
        let mut set = WorkspaceProfileSet::new();
        set.add(release_profile());
        let child = ProfileConfig::new(
            ProfileName::Bench,
            vec![
                (ProfileKey::Inherits, SettingValue::String(StrVal::Release)),
                (ProfileKey::Lto, SettingValue::String(StrVal::Fat)),
            ],
        );
        set.add(child.clone());
        assert!(matches!(
            resolve_inheritance(&child, &set),
            Ok(settings) if settings.contains(&(ProfileKey::Lto, SettingValue::String(StrVal::Fat)))
        ));
    }
);

ktest!(
    #[test]
    inheritance_resolution_reports_missing_parent,
    {
        let child = bench_profile();
        assert_eq!(
            resolve_inheritance(&child, &WorkspaceProfileSet::new()),
            Err(ResolveError::InheritTargetMissing {
                profile: ProfileName::Bench,
                parent: ProfileName::Release,
            })
        );
    }
);

ktest!(
    #[test]
    moon_binding_implicit_release_missing_when_workspace_empty,
    {
        let binding = MoonTaskProfileBinding {
            task_name: "release-build",
            profile_ref: ProfileRefKind::ImplicitRelease,
            in_pipeline: true,
            run_in_ci: true,
        };
        assert_eq!(
            bind_moon_task(&binding, &WorkspaceProfileSet::new()),
            BindingResult::Missing
        );
    }
);

ktest!(
    #[test]
    moon_binding_implicit_bench_accepts_correct_workspace,
    {
        let binding = MoonTaskProfileBinding {
            task_name: "bench-build",
            profile_ref: ProfileRefKind::ImplicitBench,
            in_pipeline: true,
            run_in_ci: true,
        };
        assert_eq!(
            bind_moon_task(&binding, &correct_workspace()),
            BindingResult::ExistsAndValid
        );
    }
);

ktest!(
    #[test]
    moon_binding_defers_non_ci_maxperf_task,
    {
        let binding = MoonTaskProfileBinding {
            task_name: "pgo-maxperf-build",
            profile_ref: ProfileRefKind::Explicit(ProfileName::Release),
            in_pipeline: false,
            run_in_ci: false,
        };
        assert_eq!(
            bind_moon_task(&binding, &correct_workspace()),
            BindingResult::DeferredScope
        );
    }
);
