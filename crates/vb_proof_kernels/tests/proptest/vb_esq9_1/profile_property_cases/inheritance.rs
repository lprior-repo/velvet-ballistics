//! Inheritance resolution profile properties.

use proptest::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use vb_proof_kernels::profile_contract::{ProfileKey, SettingValue, resolve_inheritance};

use super::strategies::arb_workspace_profile_set;

proptest! {
    #[test]
    fn prop_inheritance_resolution_deterministic_and_override_consistent(
        ws in arb_workspace_profile_set(),
    ) {
        for config in &ws.profiles {
            let r1 = resolve_inheritance(config, &ws);
            let r2 = resolve_inheritance(config, &ws);
            assert_eq!(r1, r2, "Resolution must be deterministic");

            if let Ok(resolved) = r1 {
                assert_explicit_overrides_resolved(config.settings.as_ref(), &resolved);
                assert_no_duplicate_keys(&resolved);
            }
        }
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
        assert!(
            found.is_some(),
            "Profile's explicit key {:?} must appear in resolved settings",
            key
        );
        if let Some((_, resolved_value)) = found {
            assert_eq!(
                resolved_value, *expected_value,
                "Explicit setting value for {:?} must match the last entry in settings",
                key
            );
        }
    }
}

fn assert_no_duplicate_keys(resolved: &[(ProfileKey, SettingValue)]) {
    let mut seen = BTreeSet::new();
    for (key, _) in resolved {
        assert!(
            seen.insert(*key),
            "Resolved settings must not contain duplicate keys"
        );
    }
}
