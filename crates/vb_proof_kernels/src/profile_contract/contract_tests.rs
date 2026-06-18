//! Unit tests for the master profile contract constants.

use super::contract_constants::{
    BENCH_REQUIRED_KEYS, HARDENED_GOVERNANCE_REQUIRED, MASTER_PROFILE_CONTRACT,
    RELEASE_REQUIRED_KEYS,
};
use crate::profile_contract::types::ProfileName;

#[test]
fn test_release_required_keys_count() {
    assert_eq!(RELEASE_REQUIRED_KEYS.len(), 4);
}

#[test]
fn test_bench_required_keys_count() {
    assert_eq!(BENCH_REQUIRED_KEYS.len(), 4);
}

#[test]
fn test_hardened_gov_required_count() {
    assert_eq!(HARDENED_GOVERNANCE_REQUIRED.len(), 2);
}

#[test]
fn test_master_contract_required_profiles() {
    assert_eq!(MASTER_PROFILE_CONTRACT.required_profiles.len(), 2);
    assert_eq!(
        MASTER_PROFILE_CONTRACT.required_profiles[0],
        ProfileName::Release
    );
    assert_eq!(
        MASTER_PROFILE_CONTRACT.required_profiles[1],
        ProfileName::Bench
    );
}

#[test]
fn test_master_contract_forbidden() {
    assert_eq!(
        MASTER_PROFILE_CONTRACT.forbidden_profile_names,
        &["maxperf"]
    );
}

#[test]
fn test_master_contract_release_keys_match() {
    assert!(MASTER_PROFILE_CONTRACT.release_keys == RELEASE_REQUIRED_KEYS);
}

#[test]
fn test_master_contract_bench_keys_match() {
    assert!(MASTER_PROFILE_CONTRACT.bench_keys == BENCH_REQUIRED_KEYS);
}
