//! Profile inheritance resolution.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)
//! Resolves inherited settings following Cargo's `inherits` semantics.
//! Depth-bounded to MAX_INHERITANCE_DEPTH (8). Cycle detection via visited set.

use crate::profile_contract::config::ProfileConfig;
use crate::profile_contract::types::{ProfileName, ProfileKey, SettingValue};
use crate::profile_contract::workspace::WorkspaceProfileSet;
use crate::profile_contract::errors::ResolveError;
use crate::profile_contract::MAX_INHERITANCE_DEPTH;

/// Resolved profile settings map.
pub type ResolvedProfile = Vec<(ProfileKey, SettingValue)>;

/// Resolve a profile's effective settings by walking its inheritance chain.
///
/// Algorithm:
/// 1. Start with the child profile's explicit settings.
/// 2. If `inherits` is set, resolve the parent first.
/// 3. Apply child's explicit settings OVER parent (override semantics).
/// 4. Bounds: depth <= MAX_INHERITANCE_DEPTH (8), cycle detection via visited set.
///
/// Cargo built-in defaults are NOT modeled here; only the explicit profile
/// settings in the workspace are considered for inheritance resolution.
/// Keys not set by any profile in the chain are absent from the resolved map.
pub fn resolve_inheritance(
    profile: &ProfileConfig,
    all: &WorkspaceProfileSet,
) -> Result<ResolvedProfile, ResolveError> {
    resolve_inner(profile, all, 0, &mut Vec::new())
}

/// Recursive inner resolution with depth guard and cycle detection.
fn resolve_inner(
    profile: &ProfileConfig,
    all: &WorkspaceProfileSet,
    depth: u8,
    visited: &mut Vec<ProfileName>,
) -> Result<ResolvedProfile, ResolveError> {
    // Depth guard
    if depth > MAX_INHERITANCE_DEPTH {
        return Err(ResolveError::InheritanceDepthExceeded { depth });
    }

    // Cycle detection
    if visited.contains(&profile.name) {
        return Err(ResolveError::InheritCycle);
    }
    visited.push(profile.name);

    // Start with an empty resolved map
    let mut resolved: ResolvedProfile = Vec::new();

    // 1. Resolve parent chain first (base)
    if let Some(parent_name) = profile.inherits {
        // Find parent in workspace
        let parent = all.find(parent_name).ok_or_else(|| {
            ResolveError::InheritTargetMissing {
                profile: profile.name,
                parent: parent_name,
            }
        })?;

        let parent_resolved = resolve_inner(parent, all, depth + 1, visited)?;

        // Apply parent settings as base
        for (k, v) in &parent_resolved {
            // Only add if not already in resolved (dedup)
            if !resolved.iter().any(|(rk, _)| rk == k) {
                resolved.push((*k, v.clone()));
            }
        }
    }

    // 2. Apply child's explicit settings (override parent)
    for (k, v) in &profile.settings {
        // Remove existing entry for this key (override)
        resolved.retain(|(rk, _)| rk != k);
        resolved.push((*k, v.clone()));
    }

    // Pop visited for this recursion frame
    visited.pop();

    Ok(resolved)
}
