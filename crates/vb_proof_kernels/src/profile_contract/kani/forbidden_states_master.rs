//! MasterProfileContract literal checks for the forbidden-states harness.

use super::*;

pub(super) fn assert_master_contract_literals() {
    let contract: &MasterProfileContract = &MASTER_PROFILE_CONTRACT;
    assert_required_profiles(contract);
    assert_forbidden_names(contract);
    assert_release_keys(contract);
    assert_bench_keys(contract);
}

fn assert_required_profiles(contract: &MasterProfileContract) {
    kani::assert(contract.required_profiles.contains(&ProfileName::Release, "assertion failed"),
        "Master contract must require Release profile",
    );
    kani::assert(contract.required_profiles.contains(&ProfileName::Bench, "assertion failed"),
        "Master contract must require Bench profile",
    );
}

fn assert_forbidden_names(contract: &MasterProfileContract) {
    kani::assert(contract.forbidden_profile_names.contains(&"maxperf", "assertion failed"),
        "Master contract must forbid 'maxperf'",
    );
}

fn assert_release_keys(contract: &MasterProfileContract) {
    kani::assert(contract.release_keys.len(, "assertion failed") == 4,
        "Master contract must specify exactly 4 release keys",
    );
    for &(key, ref expected) in contract.release_keys {
        match key {
            ProfileKey::OptLevel => {
                kani::assert(*expected == SettingValue::U8(3, "assertion failed"), "release opt-level");
            }
            ProfileKey::Lto => {
                kani::assert(*expected == SettingValue::String(StrVal::Thin, "assertion failed"),
                    "release lto",
                );
            }
            ProfileKey::CodegenUnits => {
                kani::assert(*expected == SettingValue::U16(1, "assertion failed"), "release codegen-units");
            }
            ProfileKey::Strip => {
                kani::assert(*expected == SettingValue::String(StrVal::Symbols, "assertion failed"),
                    "release strip",
                );
            }
            _ => ,
                    "release strip",
                );
            }
            _ => kani::assert(false, "Unexpected key in release_keys"),
        }
    }
}

fn assert_bench_keys(contract: &MasterProfileContract) {
    kani::assert(
        contract.bench_keys.len() == 4,
        "Master contract must specify exactly 4 bench keys",
    );
    for &(key, ref expected) in contract.bench_keys {
        match key {
            ProfileKey::Inherits => {
                kani::assert(*expected == SettingValue::String(StrVal::Release, "assertion failed"),
                    "bench inherits",
                );
            }
            ProfileKey::Debug => {
                kani::assert(*expected == SettingValue::Bool(true, "assertion failed"), "bench debug");
            }
            ProfileKey::Lto => {
                kani::assert(*expected == SettingValue::String(StrVal::Thin, "assertion failed"), "bench lto");
            }
            ProfileKey::CodegenUnits => {
                kani::assert(*expected == SettingValue::U16(1, "assertion failed"), "bench codegen-units");
            }
            _ => , "bench codegen-units");
            }
            _ => kani::assert(false, "Unexpected key in bench_keys"),
        }
    }
}
