//! Kani harnesses: profile inheritance resolution correctness.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! Obligations: PO-K-005, PO-K-006, PO-K-008
//!
//! Verifier: cargo kani --harness <name> -p vb_proof_kernels

use crate::profile_contract::errors::ResolveError;
use crate::profile_contract::{
    DebugMode, MAX_INHERITANCE_DEPTH, ProfileConfig, ProfileKey, ProfileName, SettingValue, StrVal,
    WorkspaceProfileSet, resolve_inheritance,
};

#[path = "inheritance_depth.rs"]
mod inheritance_depth;

// Helper: look up a setting in a resolved profile by key
fn resolved_get(resolved: &[(ProfileKey, SettingValue)], key: ProfileKey) -> Option<&SettingValue> {
    resolved
        .iter()
        .find_map(|(k, v)| if *k == key { Some(v) } else { None })
}

// ---------------------------------------------------------------------------
// PO-K-005: Bench inherits release settings correctly
// ---------------------------------------------------------------------------

/// Verify that [profile.bench] inheriting from [profile.release] produces
/// the correct resolved settings:
///   - lto="thin" (inherited from release)
///   - codegen-units=1 (inherited from release)
///   - strip="symbols" (inherited from release)
///   - debug=true (explicit in bench, overrides release default)
///
/// Master reference: contract.md §5.2 (post-fix resolved settings table)
#[kani::proof]
#[kani::unwind(20)]
fn bench_inherits_release_correctly() {
    let mut ws = WorkspaceProfileSet::new();

    // Construct [profile.release] with master-specified values
    let release = ProfileConfig::new(
        ProfileName::Release,
        vec![
            (ProfileKey::OptLevel, SettingValue::U8(3)),
            (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
            (ProfileKey::CodegenUnits, SettingValue::U16(1)),
            (ProfileKey::Strip, SettingValue::String(StrVal::Symbols)),
        ],
    );

    // Construct [profile.bench] inheriting from release
    let bench = ProfileConfig::new(
        ProfileName::Bench,
        vec![
            (ProfileKey::Inherits, SettingValue::String(StrVal::Release)),
            (ProfileKey::Debug, SettingValue::Bool(true)),
            (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
            (ProfileKey::CodegenUnits, SettingValue::U16(1)),
        ],
    );

    ws.add(release);
    ws.add(bench);

    let bench_config = ws
        .find(ProfileName::Bench)
        .expect("Bench profile should exist in workspace");

    let resolved = resolve_inheritance(bench_config, &ws)
        .expect("Bench inheriting from release should resolve successfully");

    // Verify inherited keys from release
    let lto = resolved_get(&resolved, ProfileKey::Lto);
    kani::assert(
        lto == Some(&SettingValue::String(StrVal::Thin)),
        "Bench should inherit lto='thin' from release",
    );

    let cgu = resolved_get(&resolved, ProfileKey::CodegenUnits);
    kani::assert(
        cgu == Some(&SettingValue::U16(1)),
        "Bench should inherit codegen-units=1 from release",
    );

    let strip = resolved_get(&resolved, ProfileKey::Strip);
    kani::assert(
        strip == Some(&SettingValue::String(StrVal::Symbols)),
        "Bench should inherit strip='symbols' from release",
    );

    // Verify explicit bench overrides
    let debug = resolved_get(&resolved, ProfileKey::Debug);
    kani::assert(
        debug == Some(&SettingValue::Bool(true)),
        "Bench should have debug=true (explicit override)",
    );

    let opt_level = resolved_get(&resolved, ProfileKey::OptLevel);
    kani::assert(
        opt_level == Some(&SettingValue::U8(3)),
        "Bench should inherit opt-level=3 from release",
    );

    // Verify inherits source
    kani::assert(
        bench_config.inherits_from(ProfileName::Release),
        "Bench should inherit from release",
    );
}

// ---------------------------------------------------------------------------
// PO-K-006: Hardened inherits release settings with correct overrides
// ---------------------------------------------------------------------------

/// Verify that [profile.hardened] inheriting from [profile.release] with
/// explicit overrides produces correct resolved settings:
///   - debug-assertions=true (explicit in hardened)
///   - overflow-checks=true (explicit in hardened)
///   - panic="abort" (explicit in hardened)
///   - debug=line-tables-only (explicit in hardened)
///   - lto="thin" (inherited from custom release)
///   - codegen-units=1 (inherited from custom release)
///   - strip="symbols" (inherited from custom release)
///
/// Per contract.md §5.2.
#[kani::proof]
#[kani::unwind(20)]
fn hardened_inherits_release_with_overrides() {
    let mut ws = WorkspaceProfileSet::new();

    // Release: master-specified
    let release = ProfileConfig::new(
        ProfileName::Release,
        vec![
            (ProfileKey::OptLevel, SettingValue::U8(3)),
            (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
            (ProfileKey::CodegenUnits, SettingValue::U16(1)),
            (ProfileKey::Strip, SettingValue::String(StrVal::Symbols)),
        ],
    );

    // Hardened: inherits release, with governance overrides
    let hardened = ProfileConfig::new(
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
    );

    ws.add(release);
    ws.add(hardened);

    let hardened_config = ws
        .find(ProfileName::Hardened)
        .expect("Hardened profile should exist");
    let resolved = resolve_inheritance(hardened_config, &ws)
        .expect("Hardened inheriting from release should resolve successfully");

    // Override checks (hardened explicit keys take precedence)
    let debug_assertions = resolved_get(&resolved, ProfileKey::DebugAssertions);
    kani::assert(
        debug_assertions == Some(&SettingValue::Bool(true)),
        "Hardened should have debug-assertions=true (explicit)",
    );

    let overflow = resolved_get(&resolved, ProfileKey::OverflowChecks);
    kani::assert(
        overflow == Some(&SettingValue::Bool(true)),
        "Hardened should have overflow-checks=true (explicit)",
    );

    let panic = resolved_get(&resolved, ProfileKey::Panic);
    kani::assert(
        panic == Some(&SettingValue::String(StrVal::Abort)),
        "Hardened should have panic='abort' (explicit)",
    );

    let debug = resolved_get(&resolved, ProfileKey::Debug);
    kani::assert(
        debug == Some(&SettingValue::DebugMode(DebugMode::LineTablesOnly)),
        "Hardened should have debug=line-tables-only (explicit)",
    );

    // Inherited keys from custom release
    let lto = resolved_get(&resolved, ProfileKey::Lto);
    kani::assert(
        lto == Some(&SettingValue::String(StrVal::Thin)),
        "Hardened should inherit lto='thin' from custom release",
    );

    let cgu = resolved_get(&resolved, ProfileKey::CodegenUnits);
    kani::assert(
        cgu == Some(&SettingValue::U16(1)),
        "Hardened should have codegen-units=1",
    );

    let strip = resolved_get(&resolved, ProfileKey::Strip);
    kani::assert(
        strip == Some(&SettingValue::String(StrVal::Symbols)),
        "Hardened should have strip='symbols'",
    );
}
