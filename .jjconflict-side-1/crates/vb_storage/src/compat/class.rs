/// MASTER §77.17 compatibility classes, in strictness order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum CompatibilityClass {
    /// Held and required versions are equal.
    Identical = 0,
    /// Consumer is newer than the data within one major line; the consumer
    /// reads the older shape.
    BackwardCompatible = 1,
    /// Consumer is older than the data within one major line; the data
    /// adds only fields the older reader ignores.
    ForwardCompatible = 2,
    /// Major lines differ and a migration path exists; use is blocked
    /// until the data is migrated.
    MigrationRequired = 3,
    /// Major lines differ with no migration path, but the data stays
    /// readable for replay. It never authorizes new live effects.
    ReplayOnly = 4,
    /// No reading is safe: major lines differ with neither a migration
    /// path nor a replay allowance.
    Rejected = 5,
}

impl CompatibilityClass {
    /// True only for classes that may authorize a new live effect.
    #[must_use]
    pub const fn authorizes_live_effect(self) -> bool {
        matches!(
            self,
            Self::Identical | Self::BackwardCompatible | Self::ForwardCompatible
        )
    }

    /// True for every class except [`CompatibilityClass::Rejected`].
    /// Replay reads historical data as-is; it mints no new authority.
    #[must_use]
    pub const fn authorizes_replay(self) -> bool {
        !matches!(self, Self::Rejected)
    }

    /// True for classes that must block dispatch until resolved.
    #[must_use]
    pub const fn blocks_dispatch(self) -> bool {
        matches!(self, Self::MigrationRequired | Self::Rejected)
    }
}
