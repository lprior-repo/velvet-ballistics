//! Shared generators for profile contract proptests.

use proptest::prelude::*;
use vb_proof_kernels::profile_contract::{
    DebugMode, ProfileConfig, ProfileKey, ProfileName, SettingValue, StrVal, WorkspaceProfileSet,
};

pub(crate) fn arb_profile_name() -> impl Strategy<Value = ProfileName> {
    prop_oneof![
        Just(ProfileName::Release),
        Just(ProfileName::Bench),
        Just(ProfileName::Hardened),
        Just(ProfileName::Fuzz),
        Just(ProfileName::Test),
        Just(ProfileName::Dev),
    ]
}

pub(crate) fn arb_profile_key() -> impl Strategy<Value = ProfileKey> {
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

pub(crate) fn arb_setting_value() -> impl Strategy<Value = SettingValue> {
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

pub(crate) fn arb_profile_config() -> impl Strategy<Value = ProfileConfig> {
    (
        arb_profile_name(),
        proptest::collection::vec((arb_profile_key(), arb_setting_value()), 0..12),
    )
        .prop_map(|(name, settings)| ProfileConfig::new(name, settings))
}

pub(crate) fn arb_workspace_profile_set() -> impl Strategy<Value = WorkspaceProfileSet> {
    proptest::collection::vec(arb_profile_config(), 1..=6).prop_map(|profiles| {
        let mut ws = WorkspaceProfileSet::new();
        for profile in profiles {
            ws.add(profile);
        }
        ws
    })
}

pub(crate) fn arb_correct_workspace() -> impl Strategy<Value = WorkspaceProfileSet> {
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
