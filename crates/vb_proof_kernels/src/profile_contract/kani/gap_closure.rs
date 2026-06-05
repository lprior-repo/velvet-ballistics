//! Kani harness: zero-gap verification after profile fix.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! Obligation: PO-K-007
//!
//! Verifies that after restoring all master-required profiles and fixing
//! the governance gap, validate_against_master returns empty Vec, and
//! validate_against_governance returns empty Vec.

use crate::profile_contract::{
    ProfileName, ProfileKey, SettingValue, StrVal, DebugMode,
    ProfileConfig, WorkspaceProfileSet,
    MASTER_PROFILE_CONTRACT,
    validate_against_master, validate_against_governance,
};

/// PO-K-007: Verify zero gaps after restoring all profiles.
///
/// Constructs the exact post-fix workspace configuration:
///   - [profile.release]: opt-level=3, lto="thin", codegen-units=1, strip="symbols"
///   - [profile.bench]: inherits="release", debug=true, lto="thin", codegen-units=1
///   - [profile.hardened]: debug-assertions=true, overflow-checks=true, plus other keys
///
/// Asserts that BOTH validate_against_master AND validate_against_governance
/// return empty vectors.
#[kani::proof]
#[kani::unwind(20)]
fn zero_gaps_after_fix() {
    let mut ws = WorkspaceProfileSet::new();

    // Release: all 4 master keys
    ws.add(ProfileConfig::new(ProfileName::Release, vec![
        (ProfileKey::OptLevel, SettingValue::U8(3)),
        (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
        (ProfileKey::CodegenUnits, SettingValue::U16(1)),
        (ProfileKey::Strip, SettingValue::String(StrVal::Symbols)),
    ]));

    // Bench: all 4 master keys
    ws.add(ProfileConfig::new(ProfileName::Bench, vec![
        (ProfileKey::Inherits, SettingValue::String(StrVal::Release)),
        (ProfileKey::Debug, SettingValue::Bool(true)),
        (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
        (ProfileKey::CodegenUnits, SettingValue::U16(1)),
    ]));

    // Hardened: existing keys + debug-assertions=true (the fix)
    ws.add(ProfileConfig::new(ProfileName::Hardened, vec![
        (ProfileKey::Inherits, SettingValue::String(StrVal::Release)),
        (ProfileKey::CodegenUnits, SettingValue::U16(1)),
        (ProfileKey::Debug, SettingValue::DebugMode(DebugMode::LineTablesOnly)),
        (ProfileKey::Lto, SettingValue::String(StrVal::Thin)),
        (ProfileKey::OverflowChecks, SettingValue::Bool(true)),
        (ProfileKey::Panic, SettingValue::String(StrVal::Abort)),
        (ProfileKey::Strip, SettingValue::String(StrVal::Symbols)),
        (ProfileKey::DebugAssertions, SettingValue::Bool(true)),  // THE FIX
    ]));

    // ----- Master contract validation -----
    let master_gaps = validate_against_master(&ws, &MASTER_PROFILE_CONTRACT);

    kani::assert(
        master_gaps.is_empty(),
        "Post-fix configuration must produce zero master contract gaps"
    );

    // Provide detailed diagnostics on any gap
    if !master_gaps.is_empty() {
        for gap in &master_gaps {
            // Kani will report the specific gap in a counterexample
            kani::cover!(gap != gap, "Gap found"); // vacuous but documents the branch
        }
    }

    // ----- Governance validation -----
    let governance_gaps = validate_against_governance(&ws);

    kani::assert(
        governance_gaps.is_empty(),
        "Post-fix configuration must produce zero governance gaps"
    );

    // Confirm the hardened profile has debug-assertions=true explicitly
    if let Some(hardened) = ws.find(ProfileName::Hardened) {
        let da = hardened.get(ProfileKey::DebugAssertions);
        kani::assert(
            da == Some(&SettingValue::Bool(true)),
            "Hardened must have debug-assertions=true in explicit settings"
        );
    }
}
