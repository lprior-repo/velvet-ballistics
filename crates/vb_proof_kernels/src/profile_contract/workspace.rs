//! WorkspaceProfileSet — the collection of all profiles in root Cargo.toml.
//!
//! Bead: vb-esq9.1 | State: 5 (proof-writer)

use crate::profile_contract::config::ProfileConfig;
use crate::profile_contract::types::ProfileName;

/// All profiles in the workspace root Cargo.toml.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceProfileSet {
    /// Profiles keyed by name.
    pub profiles: Vec<ProfileConfig>,
}

impl WorkspaceProfileSet {
    /// Create an empty workspace profile set.
    pub fn new() -> Self {
        Self { profiles: Vec::new() }
    }

    /// Add a profile to the set.
    pub fn add(&mut self, config: ProfileConfig) {
        self.profiles.push(config);
    }

    /// Find a profile by name. Returns None if absent.
    pub fn find(&self, name: ProfileName) -> Option<&ProfileConfig> {
        self.profiles.iter().find(|p| p.name == name)
    }

    /// Check if a profile exists by name.
    pub fn has(&self, name: ProfileName) -> bool {
        self.find(name).is_some()
    }

    /// Count profiles in the set.
    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    /// Returns true if the set has no profiles.
    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }
}

impl Default for WorkspaceProfileSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(kani)]
impl kani::Arbitrary for WorkspaceProfileSet {
    fn any() -> Self {
        let num_profiles: u8 = kani::any();
        let num_profiles = (num_profiles % 7).max(1); // 1..=6 profiles, at least 1

        let mut ws = Self::new();
        for _ in 0..num_profiles {
            let config: ProfileConfig = kani::any();
            ws.add(config);
        }
        ws
    }
}
