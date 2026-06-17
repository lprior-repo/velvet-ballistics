//! Kani harnesses: master profile key-value contract validation.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! Obligations: PO-K-001, PO-K-002, PO-K-003, PO-K-004
//!
//! Verifier: cargo kani --harness <name> -p vb_proof_kernels

use crate::profile_contract::{
    ContractGap, MASTER_PROFILE_CONTRACT, ProfileConfig, ProfileKey, ProfileName, SettingValue,
    StrVal, WorkspaceProfileSet, validate_against_governance, validate_against_master,
};

// ---------------------------------------------------------------------------
// PO-K-001: [profile.release] has all 4 master-required keys with correct values
// ---------------------------------------------------------------------------

/// Verify that a WorkspaceProfileSet containing the master-specified
/// [profile.release] produces zero ContractGap for the release profile.
///
/// Master reference: velvet-ballistics-MASTER.md §34:1375-1379
/// Required keys: opt-level=3, lto="thin", codegen-units=1, strip="symbols"
#[kani::proof]
#[kani::unwind(20)]
fn release_profile_master_keys() {
    let mut ws = WorkspaceProfileSet::new();

    let release_settings = vec![
        (ProfileKey::OptLevel, SettingValue::U8(3)),
        (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
        (ProfileKey::CodegenUnits, SettingValue::U16(1)),
        (ProfileKey::Strip, SettingValue::String(StrVal::Symbols)),
    ];

    let release = ProfileConfig::new(ProfileName::Release, release_settings);
    ws.add(release);

    let bench_settings = vec![
        (ProfileKey::Inherits, SettingValue::String(StrVal::Release)),
        (ProfileKey::Debug, SettingValue::Bool(true)),
        (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
        (ProfileKey::CodegenUnits, SettingValue::U16(1)),
    ];
    ws.add(ProfileConfig::new(ProfileName::Bench, bench_settings));

    let gaps = validate_against_master(&ws, &MASTER_PROFILE_CONTRACT);

    // Check for any release-related gaps using explicit loop (no alloc)
    let mut has_release_gap = false;
    for gap in &gaps {
        match gap {
            ContractGap::MissingSetting { profile, .. } if *profile == ProfileName::Release => {
                has_release_gap = true;
            }
            ContractGap::WrongSetting { profile, .. } if *profile == ProfileName::Release => {
                has_release_gap = true;
            }
            ContractGap::MissingProfile { name } if *name == ProfileName::Release => {
                has_release_gap = true;
            }
            _ => {}
        }
    }

    //! Kani harnesses: master profile key-value contract validation.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! Obligations: PO-K-001, PO-K-002, PO-K-003, PO-K-004
//!
//! Verifier: cargo kani --harness <name> -p vb_proof_kernels

use crate::profile_contract::{
    ContractGap, MASTER_PROFILE_CONTRACT, ProfileConfig, ProfileKey, ProfileName, SettingValue,
    StrVal, WorkspaceProfileSet, validate_against_governance, validate_against_master,
};

// ---------------------------------------------------------------------------
// PO-K-001: [profile.release] has all 4 master-required keys with correct values
// ---------------------------------------------------------------------------

/// Verify that a WorkspaceProfileSet containing the master-specified
/// [profile.release] produces zero ContractGap for the release profile.
///
/// Master reference: velvet-ballistics-MASTER.md §34:1375-1379
/// Required keys: opt-level=3, lto="thin", codegen-units=1, strip="symbols"
#[kani::proof]
#[kani::unwind(20)]
fn release_profile_master_keys() {
    let mut ws = WorkspaceProfileSet::new();

    let release_settings = vec![
        (ProfileKey::OptLevel, SettingValue::U8(3)),
        (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
        (ProfileKey::CodegenUnits, SettingValue::U16(1)),
        (ProfileKey::Strip, SettingValue::String(StrVal::Symbols)),
    ];

    let release = ProfileConfig::new(ProfileName::Release, release_settings);
    ws.add(release);

    let bench_settings = vec![
        (ProfileKey::Inherits, SettingValue::String(StrVal::Release)),
        (ProfileKey::Debug, SettingValue::Bool(true)),
        (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
        (ProfileKey::CodegenUnits, SettingValue::U16(1)),
    ];
    ws.add(ProfileConfig::new(ProfileName::Bench, bench_settings));

    let gaps = validate_against_master(&ws, &MASTER_PROFILE_CONTRACT);

    // Check for any release-related gaps using explicit loop (no alloc)
    let mut has_release_gap = false;
    for gap in &gaps {
        match gap {
            ContractGap::MissingSetting { profile, .. } if *profile == ProfileName::Release => {
                has_release_gap = true;
            }
            ContractGap::WrongSetting { profile, .. } if *profile == ProfileName::Release => {
                has_release_gap = true;
            }
            ContractGap::MissingProfile { name } if *name == ProfileName::Release => {
                has_release_gap = true;
            }
            _ => {}
        }
    }

    kani::assert(!has_release_gap, "assertion failed");

    kani::assert(gaps.is_empty(), "assertion failed");
}

// ---------------------------------------------------------------------------
// PO-K-002: [profile.bench] has all 4 master-required keys with correct values
// ---------------------------------------------------------------------------

/// Verify that a WorkspaceProfileSet containing the master-specified
/// [profile.bench] produces zero bench ContractGap.
#[kani::proof]
#[kani::unwind(20)]
fn bench_profile_master_keys() {
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

    let gaps = validate_against_master(&ws, &MASTER_PROFILE_CONTRACT);

    let mut has_bench_gap = false;
    for gap in &gaps {
        match gap {
            ContractGap::MissingSetting { profile, .. } if *profile == ProfileName::Bench => {
                has_bench_gap = true;
            }
            ContractGap::WrongSetting { profile, .. } if *profile == ProfileName::Bench => {
                has_bench_gap = true;
            }
            ContractGap::MissingProfile { name } if *name == ProfileName::Bench => {
                has_bench_gap = true;
            }
            _ => {}
        }
    }

    , "assertion failed");
}

// ---------------------------------------------------------------------------
// PO-K-002: [profile.bench] has all 4 master-required keys with correct values
// ---------------------------------------------------------------------------

/// Verify that a WorkspaceProfileSet containing the master-specified
/// [profile.bench] produces zero bench ContractGap.
#[kani::proof]
#[kani::unwind(20)]
fn bench_profile_master_keys() {
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

    let gaps = validate_against_master(&ws, &MASTER_PROFILE_CONTRACT);

    let mut has_bench_gap = false;
    for gap in &gaps {
        match gap {
            ContractGap::MissingSetting { profile, .. } if *profile == ProfileName::Bench => {
                has_bench_gap = true;
            }
            ContractGap::WrongSetting { profile, .. } if *profile == ProfileName::Bench => {
                has_bench_gap = true;
            }
            ContractGap::MissingProfile { name } if *name == ProfileName::Bench => {
                has_bench_gap = true;
            }
            _ => {}
        }
    }

    kani::assert(!has_bench_gap, "assertion failed");
    kani::assert(gaps.is_empty(), "assertion failed");
}

// ---------------------------------------------------------------------------
// PO-K-003: [profile.hardened] has debug-assertions=true (governance)
// ---------------------------------------------------------------------------

#[kani::proof]
#[kani::unwind(20)]
fn hardened_debug_assertions_enabled() {
    let mut ws = WorkspaceProfileSet::new();

    ws.add(ProfileConfig::new(
        ProfileName::Hardened,
        vec![
            (ProfileKey::DebugAssertions, SettingValue::Bool(true)),
            (ProfileKey::OverflowChecks, SettingValue::Bool(true)),
        ],
    ));

    let gaps = validate_against_governance(&ws);

    kani::assert(gaps.is_empty(, "assertion failed"), "assertion failed");
}

// ---------------------------------------------------------------------------
// PO-K-004: [profile.maxperf] is rejected at construction
// ---------------------------------------------------------------------------

#[kani::proof]
#[kani::unwind(20)]
fn maxperf_rejected_by_construction() {
    // 1. Direct check
    let result = ProfileName::new("maxperf");
    kani::assert(result.is_err(), "assertion failed");

    // 2. Exhaustive: all valid ProfileName variants are not maxperf
    let name: ProfileName = kani::any();
    // All 6 valid variants are non-maxperf — this is a structural guarantee
    , "assertion failed");

    // 2. Exhaustive: all valid ProfileName variants are not maxperf
    let name: ProfileName = kani::any();
    // All 6 valid variants are non-maxperf — this is a structural guarantee
    kani::assert(
        name == ProfileName::Release
            || name == ProfileName::Bench
            || name == ProfileName::Hardened
            || name == ProfileName::Fuzz
            || name == ProfileName::Test
            || name == ProfileName::Dev,
    );

    // 3. Defense-in-depth: arbitrary workspace never has ForbiddenProfile gap
    //    because maxperf can't be constructed
    let ws: WorkspaceProfileSet = kani::any();
    let gaps = validate_against_master(&ws, &MASTER_PROFILE_CONTRACT);

    let mut has_forbidden = false;
    for gap in &gaps {
        if matches!(gap, ContractGap::ForbiddenProfile { .. }) {
            has_forbidden = true;
        }
    }
    
        name == ProfileName::Release
            || name == ProfileName::Bench
            || name == ProfileName::Hardened
            || name == ProfileName::Fuzz
            || name == ProfileName::Test
            || name == ProfileName::Dev,
    );

    // 3. Defense-in-depth: arbitrary workspace never has ForbiddenProfile gap
    //    because maxperf can't be constructed
    let ws: WorkspaceProfileSet = kani::any();
    let gaps = validate_against_master(&ws, &MASTER_PROFILE_CONTRACT);

    let mut has_forbidden = false;
    for gap in &gaps {
        if matches!(gap, ContractGap::ForbiddenProfile { .. }) {
            has_forbidden = true;
        }
    }
    kani::assert(!has_forbidden, "assertion failed");
}
