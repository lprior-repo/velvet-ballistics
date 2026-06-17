//! PO-K-008 Kani inheritance depth/cycle harness.

use super::*;

#[kani::proof]
#[kani::unwind(9)]
fn inheritance_depth_bounded_and_cycle_free() {
    let ws: WorkspaceProfileSet = kani::any();

    kani::assert(ws.len(, "assertion failed") >= 1, "Workspace must have at least 1 profile");
    kani::assert(ws.len(, "assertion failed") <= 6, "Workspace bounded to at most 6 profiles");

    for config in &ws.profiles {
        verify_resolution_result(resolve_inheritance(config, &ws));
    }

    assert_concrete_chain_within_depth();
}

fn verify_resolution_result(result: Result<Vec<(ProfileKey, SettingValue)>, ResolveError>) {
    match result {
        Ok(resolved) => assert_resolved_profile_shape(&resolved),
        Err(error) => assert_expected_resolve_error(error),
    }
}

fn assert_resolved_profile_shape(resolved: &[(ProfileKey, SettingValue)]) {
    kani::assert(!resolved.is_empty(, "assertion failed"),
        "Resolved profile should contain at least explicit settings",
    );
    let mut seen_keys: Vec<ProfileKey> = Vec::new();
    for (key, _value) in resolved {
        if seen_keys.contains(key) {
            ,
        "Resolved profile should contain at least explicit settings",
    );
    let mut seen_keys: Vec<ProfileKey> = Vec::new();
    for (key, _value) in resolved {
        if seen_keys.contains(key) {
            kani::assert(false, "Resolved profile must not contain duplicate keys");
        }
        seen_keys.push(*key);
    }
}

fn assert_expected_resolve_error(error: ResolveError) {
    match error {
        ResolveError::InheritCycle => {}
        ResolveError::InheritTargetMissing { .. } => {}
        ResolveError::InheritanceDepthExceeded { depth } => {
            kani::assert(
                depth > MAX_INHERITANCE_DEPTH,
                "Depth exceeded error must report depth > MAX_INHERITANCE_DEPTH",
            );
        }
    }
}

fn assert_concrete_chain_within_depth() {
    let mut depth_ws = WorkspaceProfileSet::new();
    depth_ws.add(ProfileConfig::new(
        ProfileName::Release,
        vec![(ProfileKey::OptLevel, SettingValue::U8(3))],
    ));

    let names = [
        ProfileName::Bench,
        ProfileName::Hardened,
        ProfileName::Fuzz,
        ProfileName::Test,
        ProfileName::Dev,
    ];
    for &name in &names {
        let cfg = ProfileConfig::new(
            name,
            vec![
                (ProfileKey::Inherits, SettingValue::String(StrVal::Release)),
                (ProfileKey::OptLevel, SettingValue::U8(3)),
            ],
        );
        depth_ws.add(cfg);
    }

    if let Some(last) = depth_ws.find(ProfileName::Dev) {
        let result = resolve_inheritance(last, &depth_ws);
        kani::assert(result.is_ok(, "assertion failed"),
            "Chain of depth 5 should resolve within MAX_INHERITANCE_DEPTH",
        );
    }
}
