/// Stable numeric decision code carried on the decision record.
/// 1xx allows, 2xx blocks with a recovery path, 3xx blocks outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CompatDecisionCode {
    /// Identical versions; live effects allowed.
    AllowIdentical = 100,
    /// Backward compatible; live effects allowed.
    AllowBackward = 101,
    /// Forward compatible; live effects allowed.
    AllowForward = 102,
    /// Migration required; dispatch blocked until migrated.
    BlockMigrationRequired = 200,
    /// Replay only; live effects forbidden, replay allowed.
    ReplayOnlyNoLiveEffects = 201,
    /// Rejected; nothing allowed.
    BlockRejected = 300,
}

impl CompatDecisionCode {
    /// Numeric code for diagnostics and wire stability.
    #[must_use]
    pub const fn code(self) -> u16 {
        match self {
            Self::AllowIdentical => 100,
            Self::AllowBackward => 101,
            Self::AllowForward => 102,
            Self::BlockMigrationRequired => 200,
            Self::ReplayOnlyNoLiveEffects => 201,
            Self::BlockRejected => 300,
        }
    }
}

/// Inputs to one classification. Bundled so no call site grows past the
/// argument-count limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompatInputs {
    /// Boundary the decision applies to.
    pub boundary: CompatBoundary,
    /// Version of the data or artifact at hand.
    pub held: SchemaVersion,
    /// Version the consumer understands.
    pub required: SchemaVersion,
    /// A migration path from the held major line to the required major
    /// line is declared for this boundary.
    pub migration_available: bool,
    /// The held major line is declared replayable without migration.
    pub replay_only: bool,
}

/// Digests carried on a decision record. Callers supply real digests;
/// this module never fabricates one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompatDigests {
    /// Digest of the deciding rule (domain-separated encoding of boundary
    /// plus rule; storage computes it with BLAKE3).
    pub rule_digest: [u8; 32],
    /// Digest of the held schema.
    pub old_schema_digest: [u8; 32],
    /// Digest of the required schema.
    pub new_schema_digest: [u8; 32],
}

/// Outcome of [`classify`]: class, deciding rule, and decision code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Classification {
    /// Compatibility class.
    pub class: CompatibilityClass,
    /// Rule that decided the class.
    pub rule: CompatRule,
    /// Numeric decision code for the class.
    pub code: CompatDecisionCode,
}

/// Pure §77.17 classification. Migration availability wins over a replay
/// allowance: when both are declared, the data must still migrate before
/// live use rather than silently falling back to replay-only.
#[must_use]
pub const fn classify(inputs: CompatInputs) -> Classification {
    if inputs.held.major == inputs.required.major
        && inputs.held.minor == inputs.required.minor
        && inputs.held.patch == inputs.required.patch
    {
        return Classification {
            class: CompatibilityClass::Identical,
            rule: CompatRule::IdenticalVersions,
            code: CompatDecisionCode::AllowIdentical,
        };
    }
    if inputs.held.major == inputs.required.major {
        if inputs.required.minor > inputs.held.minor
            || (inputs.required.minor == inputs.held.minor
                && inputs.required.patch > inputs.held.patch)
        {
            return Classification {
                class: CompatibilityClass::BackwardCompatible,
                rule: CompatRule::SameMajorNewerConsumer,
                code: CompatDecisionCode::AllowBackward,
            };
        }
        return Classification {
            class: CompatibilityClass::ForwardCompatible,
            rule: CompatRule::SameMajorOlderConsumer,
            code: CompatDecisionCode::AllowForward,
        };
    }
    if inputs.migration_available {
        return Classification {
            class: CompatibilityClass::MigrationRequired,
            rule: CompatRule::MajorChangeMigrationAvailable,
            code: CompatDecisionCode::BlockMigrationRequired,
        };
    }
    if inputs.replay_only {
        return Classification {
            class: CompatibilityClass::ReplayOnly,
            rule: CompatRule::MajorChangeReplayOnly,
            code: CompatDecisionCode::ReplayOnlyNoLiveEffects,
        };
    }
    Classification {
        class: CompatibilityClass::Rejected,
        rule: CompatRule::MajorChangeUnsupported,
        code: CompatDecisionCode::BlockRejected,
    }
}

/// Typed incompatibility failures, one variant per blocking class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum CompatError {
    /// Dispatch is blocked until the data migrates to the required line.
    #[error("compatibility blocked at {boundary:?}: migration required")]
    MigrationRequired {
        /// Boundary the decision applies to.
        boundary: CompatBoundary,
    },
    /// A replay-only decision never authorizes a new live effect.
    #[error(
        "compatibility blocked at {boundary:?}: replay-only data cannot authorize live effects"
    )]
    ReplayOnlyLiveEffect {
        /// Boundary the decision applies to.
        boundary: CompatBoundary,
    },
    /// The versions are incompatible with no migration or replay path.
    #[error("compatibility blocked at {boundary:?}: versions rejected")]
    Rejected {
        /// Boundary the decision applies to.
        boundary: CompatBoundary,
    },
}

/// Journaled §77.17 decision record. Every decision that affects admission,
/// replay, or live-effect dispatch is persisted with class, deciding-rule
/// digest, old/new schema digests, and decision code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompatDecision {
    /// Boundary the decision applies to.
    pub boundary: CompatBoundary,
    /// Compatibility class.
    pub class: CompatibilityClass,
    /// Rule that decided the class.
    pub rule: CompatRule,
    /// Digest of the deciding rule.
    pub rule_digest: [u8; 32],
    /// Digest of the held schema.
    pub old_schema_digest: [u8; 32],
    /// Digest of the required schema.
    pub new_schema_digest: [u8; 32],
    /// Numeric decision code for the class.
    pub decision_code: CompatDecisionCode,
}

impl CompatDecision {
    /// Classifies and builds the decision record from supplied digests.
    #[must_use]
    pub const fn decide(inputs: CompatInputs, digests: CompatDigests) -> Self {
        let classification = classify(inputs);
        Self {
            boundary: inputs.boundary,
            class: classification.class,
            rule: classification.rule,
            rule_digest: digests.rule_digest,
            old_schema_digest: digests.old_schema_digest,
            new_schema_digest: digests.new_schema_digest,
            decision_code: classification.code,
        }
    }

    /// Authorizes a new live effect, or fails with the typed blocking error.
    /// Replay-only decisions always fail here, by construction.
    pub const fn authorize_live_effect(self) -> Result<(), CompatError> {
        if self.class.authorizes_live_effect() {
            return Ok(());
        }
        match self.class {
            CompatibilityClass::MigrationRequired => Err(CompatError::MigrationRequired {
                boundary: self.boundary,
            }),
            CompatibilityClass::ReplayOnly => Err(CompatError::ReplayOnlyLiveEffect {
                boundary: self.boundary,
            }),
            CompatibilityClass::Rejected => Err(CompatError::Rejected {
                boundary: self.boundary,
            }),
            CompatibilityClass::Identical
            | CompatibilityClass::BackwardCompatible
            | CompatibilityClass::ForwardCompatible => Ok(()),
        }
    }

    /// Authorizes replay of the data. Only rejected data is unreadable;
    /// migration-required data replays as-is, and replay mints no new
    /// live-effect authority (see [`CompatDecision::authorize_live_effect`]).
    pub const fn authorize_replay(self) -> Result<(), CompatError> {
        if self.class.authorizes_replay() {
            Ok(())
        } else {
            Err(CompatError::Rejected {
                boundary: self.boundary,
            })
        }
    }

    /// True when the decision must block dispatch until resolved.
    #[must_use]
    pub const fn blocks_dispatch(self) -> bool {
        self.class.blocks_dispatch()
    }
}
