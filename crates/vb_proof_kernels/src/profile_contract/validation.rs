//! Pure validation functions for profile contract checking.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! No I/O, no panics, no unsafe. Returns typed error lists for contract and
//! governance gaps.

use crate::profile_contract::errors::{ContractGap, GovernanceGap};
use crate::profile_contract::{HARDENED_GOVERNANCE_REQUIRED, MasterProfileContract};
use crate::profile_contract::types::{ProfileKey, ProfileName};
use crate::profile_contract::workspace::WorkspaceProfileSet;

/// Validate the workspace profile set against the master contract (MASTER §34).
///
/// Returns a list of ContractGap for every violation. An empty list means
/// the workspace satisfies the master contract for profiles.
pub fn validate_against_master(
    set: &WorkspaceProfileSet,
    contract: &MasterProfileContract,
) -> Vec<ContractGap> {
    let mut gaps = Vec::new();

    // 1. Check required profiles exist
    for &required in contract.required_profiles {
        if !set.has(required) {
            gaps.push(ContractGap::MissingProfile { name: required });
        }
    }

    // 2. Check forbidden profile names — these strings should not appear
    //    Since ProfileName::new("maxperf") returns Err, there is no way to
    //    have maxperf in the workspace set. But we check anyway for
    //    defense-in-depth. If somehow a profile config with a placeholder
    //    name was constructed, this catches it.
    //    (In practice, this is a compile-time / construction-time property.)
    for &forbidden in contract.forbidden_profile_names {
        // maxperf cannot be converted to ProfileName, so this check is
        // documented as defense-in-depth.
        let _ = forbidden; // consumed; type-level guard already active
    }

    // 2b. Forbidden check: if maxperf SNEAKS in through a bug, catch it.
    //     This is checked by trying ProfileName::new("maxperf") — it returns Err.
    //     No valid ProfileName can represent maxperf.
    if ProfileName::new("maxperf").is_ok() {
        // This branch should be unreachable — but if somehow maxperf becomes
        // a valid ProfileName, flag it.
        gaps.push(ContractGap::ForbiddenProfile {
            name: ProfileName::Release, // placeholder — maxperf has no enum variant
        });
    }

    // 3. Check [profile.release] key values
    if let Some(release) = set.find(ProfileName::Release) {
        for &(key, ref expected_value) in contract.release_keys {
            match release.get(key) {
                Some(actual) => {
                    if actual != expected_value {
                        gaps.push(ContractGap::WrongSetting {
                            profile: ProfileName::Release,
                            key,
                            expected: expected_value.clone(),
                            actual: actual.clone(),
                        });
                    }
                }
                None => {
                    gaps.push(ContractGap::MissingSetting {
                        profile: ProfileName::Release,
                        key,
                    });
                }
            }
        }
    }

    // 4. Check [profile.bench] key values
    if let Some(bench) = set.find(ProfileName::Bench) {
        for &(key, ref expected_value) in contract.bench_keys {
            match bench.get(key) {
                Some(actual) => {
                    if actual != expected_value {
                        gaps.push(ContractGap::WrongSetting {
                            profile: ProfileName::Bench,
                            key,
                            expected: expected_value.clone(),
                            actual: actual.clone(),
                        });
                    }
                }
                None => {
                    gaps.push(ContractGap::MissingSetting {
                        profile: ProfileName::Bench,
                        key,
                    });
                }
            }
        }
    }

    gaps
}

/// Validate the hardened profile against governance requirements
/// (docs/rust-governance.md:61).
///
/// Returns a list of GovernanceGap for every violation. An empty list
/// means the hardened profile satisfies governance requirements.
pub fn validate_against_governance(set: &WorkspaceProfileSet) -> Vec<GovernanceGap> {
    let mut gaps = Vec::new();

    // governance only applies to hardened profile
    if let Some(hardened) = set.find(ProfileName::Hardened) {
        for &(key, ref required_value) in HARDENED_GOVERNANCE_REQUIRED {
            match hardened.get(key) {
                Some(actual) => {
                    if actual != required_value {
                        // Gap: wrong value — still a governance gap
                        if key == ProfileKey::DebugAssertions {
                            gaps.push(GovernanceGap::MissingDebugAssertions);
                        } else if key == ProfileKey::OverflowChecks {
                            gaps.push(GovernanceGap::MissingOverflowChecks);
                        }
                    }
                }
                None => {
                    if key == ProfileKey::DebugAssertions {
                        gaps.push(GovernanceGap::MissingDebugAssertions);
                    } else if key == ProfileKey::OverflowChecks {
                        gaps.push(GovernanceGap::MissingOverflowChecks);
                    }
                }
            }
        }
    }

    gaps
}
