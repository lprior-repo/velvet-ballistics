#![forbid(unsafe_code)]

//! MASTER §77.17 version compatibility algebra.
//!
//! Pure value types: no I/O, no allocation in the classification path, no
//! hashing. The deciding-rule digest is supplied by the caller (storage
//! computes it with BLAKE3 over the domain-separated rule encoding); this
//! crate has no hash dependency in production code, so it never invents
//! a digest here.
//!
//! Direction convention: `held` is the version of the data or artifact at
//! hand; `required` is the version the consumer understands. A newer
//! consumer reading older data within one major line is backward
//! compatible; an older consumer reading newer data within one major line
//! is forward compatible under the additive-only assumption documented on
//! [`CompatibilityClass::ForwardCompatible`]. A major-line change needs a
//! migration path, a replay-only allowance, or it is rejected outright.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Semver-shaped schema version. Single-number wire versions (for example
/// journal schema versions) map to `major` with minor and patch zero via
/// [`SchemaVersion::from_major`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SchemaVersion {
    /// Major line. A change here breaks wire reading without migration.
    pub major: u16,
    /// Minor line. Additive change within a major line.
    pub minor: u16,
    /// Patch line. No structural change.
    pub patch: u16,
}

impl SchemaVersion {
    /// Builds a version triple.
    #[must_use]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Builds a version from a single wire number (minor and patch zero).
    #[must_use]
    pub const fn from_major(major: u16) -> Self {
        Self {
            major,
            minor: 0,
            patch: 0,
        }
    }
}

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

/// The twelve MASTER §77.17 boundaries. Input and output schemas share one
/// boundary, matching the spec's "input/output schemas" item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum CompatBoundary {
    /// Compiled workflow IR versions.
    IrVersion = 0,
    /// Action ABI versions.
    ActionAbi = 1,
    /// Input and output schema versions.
    InputOutputSchema = 2,
    /// Policy input schema versions.
    PolicyInputSchema = 3,
    /// Signal schema versions.
    SignalSchema = 4,
    /// Journal event versions.
    JournalEventVersion = 5,
    /// Snapshot versions.
    SnapshotVersion = 6,
    /// Debug map versions.
    DebugMapVersion = 7,
    /// Resource contract versions.
    ResourceContract = 8,
    /// Incident bundle versions.
    IncidentBundle = 9,
    /// SDK ABI versions.
    SdkAbi = 10,
    /// Artifact envelope versions.
    ArtifactEnvelope = 11,
}

impl CompatBoundary {
    /// All twelve boundaries, in spec order.
    pub const ALL: [CompatBoundary; 12] = [
        CompatBoundary::IrVersion,
        CompatBoundary::ActionAbi,
        CompatBoundary::InputOutputSchema,
        CompatBoundary::PolicyInputSchema,
        CompatBoundary::SignalSchema,
        CompatBoundary::JournalEventVersion,
        CompatBoundary::SnapshotVersion,
        CompatBoundary::DebugMapVersion,
        CompatBoundary::ResourceContract,
        CompatBoundary::IncidentBundle,
        CompatBoundary::SdkAbi,
        CompatBoundary::ArtifactEnvelope,
    ];

    /// Stable name used in rule-digest encoding and diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::IrVersion => "ir_version",
            Self::ActionAbi => "action_abi",
            Self::InputOutputSchema => "input_output_schema",
            Self::PolicyInputSchema => "policy_input_schema",
            Self::SignalSchema => "signal_schema",
            Self::JournalEventVersion => "journal_event_version",
            Self::SnapshotVersion => "snapshot_version",
            Self::DebugMapVersion => "debug_map_version",
            Self::ResourceContract => "resource_contract",
            Self::IncidentBundle => "incident_bundle",
            Self::SdkAbi => "sdk_abi",
            Self::ArtifactEnvelope => "artifact_envelope",
        }
    }
}

/// The rule that decided a classification. This is what the decision
/// record's rule digest covers, distinct from the decision code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum CompatRule {
    /// Held and required versions are equal.
    IdenticalVersions = 0,
    /// Same major line; the consumer is newer than the data.
    SameMajorNewerConsumer = 1,
    /// Same major line; the consumer is older than the data.
    SameMajorOlderConsumer = 2,
    /// Major lines differ and a migration path is declared.
    MajorChangeMigrationAvailable = 3,
    /// Major lines differ with no migration path but a replay allowance.
    MajorChangeReplayOnly = 4,
    /// Major lines differ with neither migration nor replay allowance.
    MajorChangeUnsupported = 5,
}

impl CompatRule {
    /// Stable name used in rule-digest encoding and diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::IdenticalVersions => "identical_versions",
            Self::SameMajorNewerConsumer => "same_major_newer_consumer",
            Self::SameMajorOlderConsumer => "same_major_older_consumer",
            Self::MajorChangeMigrationAvailable => "major_change_migration_available",
            Self::MajorChangeReplayOnly => "major_change_replay_only",
            Self::MajorChangeUnsupported => "major_change_unsupported",
        }
    }
}

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

#[cfg(test)]
// No allow block: the forbid-level lints (expect_used, panic, unwrap_used)
// cannot be allowed back, so the tests below use `?` instead.
mod tests {
    use super::*;

    fn inputs(
        held: SchemaVersion,
        required: SchemaVersion,
        migration_available: bool,
        replay_only: bool,
    ) -> CompatInputs {
        CompatInputs {
            boundary: CompatBoundary::IrVersion,
            held,
            required,
            migration_available,
            replay_only,
        }
    }

    fn v(major: u16, minor: u16, patch: u16) -> SchemaVersion {
        SchemaVersion::new(major, minor, patch)
    }

    #[test]
    fn six_class_table() {
        let cases: [(
            SchemaVersion,
            SchemaVersion,
            bool,
            bool,
            CompatibilityClass,
            CompatRule,
            CompatDecisionCode,
        ); 8] = [
            (
                v(1, 0, 0),
                v(1, 0, 0),
                false,
                false,
                CompatibilityClass::Identical,
                CompatRule::IdenticalVersions,
                CompatDecisionCode::AllowIdentical,
            ),
            (
                v(1, 0, 0),
                v(1, 2, 0),
                false,
                false,
                CompatibilityClass::BackwardCompatible,
                CompatRule::SameMajorNewerConsumer,
                CompatDecisionCode::AllowBackward,
            ),
            (
                v(1, 0, 1),
                v(1, 0, 0),
                false,
                false,
                CompatibilityClass::ForwardCompatible,
                CompatRule::SameMajorOlderConsumer,
                CompatDecisionCode::AllowForward,
            ),
            (
                v(1, 2, 0),
                v(1, 0, 0),
                false,
                false,
                CompatibilityClass::ForwardCompatible,
                CompatRule::SameMajorOlderConsumer,
                CompatDecisionCode::AllowForward,
            ),
            (
                v(1, 0, 0),
                v(2, 0, 0),
                true,
                false,
                CompatibilityClass::MigrationRequired,
                CompatRule::MajorChangeMigrationAvailable,
                CompatDecisionCode::BlockMigrationRequired,
            ),
            (
                v(1, 0, 0),
                v(2, 0, 0),
                false,
                true,
                CompatibilityClass::ReplayOnly,
                CompatRule::MajorChangeReplayOnly,
                CompatDecisionCode::ReplayOnlyNoLiveEffects,
            ),
            (
                v(1, 0, 0),
                v(2, 0, 0),
                false,
                false,
                CompatibilityClass::Rejected,
                CompatRule::MajorChangeUnsupported,
                CompatDecisionCode::BlockRejected,
            ),
            (
                v(1, 0, 0),
                v(2, 0, 0),
                true,
                true,
                CompatibilityClass::MigrationRequired,
                CompatRule::MajorChangeMigrationAvailable,
                CompatDecisionCode::BlockMigrationRequired,
            ),
        ];
        for (held, required, migration, replay, class, rule, code) in cases {
            let got = classify(inputs(held, required, migration, replay));
            assert_eq!(got.class, class, "held {held:?} required {required:?}");
            assert_eq!(got.rule, rule, "held {held:?} required {required:?}");
            assert_eq!(got.code, code, "held {held:?} required {required:?}");
        }
    }

    #[test]
    fn decision_codes_are_stable_numbers() {
        assert_eq!(CompatDecisionCode::AllowIdentical.code(), 100);
        assert_eq!(CompatDecisionCode::AllowBackward.code(), 101);
        assert_eq!(CompatDecisionCode::AllowForward.code(), 102);
        assert_eq!(CompatDecisionCode::BlockMigrationRequired.code(), 200);
        assert_eq!(CompatDecisionCode::ReplayOnlyNoLiveEffects.code(), 201);
        assert_eq!(CompatDecisionCode::BlockRejected.code(), 300);
    }

    #[test]
    fn twelve_boundaries_enumerated() {
        assert_eq!(CompatBoundary::ALL.len(), 12);
        for boundary in CompatBoundary::ALL {
            assert!(!boundary.name().is_empty());
        }
        assert_ne!(
            CompatBoundary::InputOutputSchema,
            CompatBoundary::PolicyInputSchema
        );
    }

    #[test]
    fn live_effect_authorization() {
        let digests = CompatDigests {
            rule_digest: [7u8; 32],
            old_schema_digest: [1u8; 32],
            new_schema_digest: [2u8; 32],
        };
        let allowed = [
            (v(1, 0, 0), v(1, 0, 0), false, false),
            (v(1, 0, 0), v(1, 1, 0), false, false),
            (v(1, 1, 0), v(1, 0, 0), false, false),
        ];
        for (held, required, migration, replay) in allowed {
            let decision =
                CompatDecision::decide(inputs(held, required, migration, replay), digests);
            assert!(decision.authorize_live_effect().is_ok());
            assert!(decision.authorize_replay().is_ok());
            assert!(!decision.blocks_dispatch());
        }
        let blocked: [(SchemaVersion, SchemaVersion, bool, bool, CompatError); 3] = [
            (
                v(1, 0, 0),
                v(2, 0, 0),
                true,
                false,
                CompatError::MigrationRequired {
                    boundary: CompatBoundary::IrVersion,
                },
            ),
            (
                v(1, 0, 0),
                v(2, 0, 0),
                false,
                true,
                CompatError::ReplayOnlyLiveEffect {
                    boundary: CompatBoundary::IrVersion,
                },
            ),
            (
                v(1, 0, 0),
                v(2, 0, 0),
                false,
                false,
                CompatError::Rejected {
                    boundary: CompatBoundary::IrVersion,
                },
            ),
        ];
        for (held, required, migration, replay, expected) in blocked {
            let decision =
                CompatDecision::decide(inputs(held, required, migration, replay), digests);
            assert_eq!(decision.authorize_live_effect(), Err(expected));
            // ReplayOnly blocks live effects, not dispatch as such: replay
            // dispatch proceeds, so blocks_dispatch is false for it.
            assert_eq!(
                decision.blocks_dispatch(),
                decision.class != CompatibilityClass::ReplayOnly
            );
        }
    }

    #[test]
    fn replay_only_never_authorizes_live_effects_but_allows_replay() {
        let decision = CompatDecision::decide(
            inputs(v(1, 0, 0), v(2, 0, 0), false, true),
            CompatDigests {
                rule_digest: [7u8; 32],
                old_schema_digest: [1u8; 32],
                new_schema_digest: [2u8; 32],
            },
        );
        assert_eq!(decision.class, CompatibilityClass::ReplayOnly);
        assert_eq!(
            decision.authorize_live_effect(),
            Err(CompatError::ReplayOnlyLiveEffect {
                boundary: CompatBoundary::IrVersion,
            })
        );
        assert!(decision.authorize_replay().is_ok());
    }

    #[test]
    fn rejected_blocks_replay_too() {
        let decision = CompatDecision::decide(
            inputs(v(1, 0, 0), v(2, 0, 0), false, false),
            CompatDigests {
                rule_digest: [7u8; 32],
                old_schema_digest: [1u8; 32],
                new_schema_digest: [2u8; 32],
            },
        );
        assert_eq!(
            decision.authorize_replay(),
            Err(CompatError::Rejected {
                boundary: CompatBoundary::IrVersion,
            })
        );
    }

    #[test]
    fn migration_required_blocks_dispatch_but_replays() {
        let decision = CompatDecision::decide(
            inputs(v(1, 0, 0), v(2, 0, 0), true, false),
            CompatDigests {
                rule_digest: [7u8; 32],
                old_schema_digest: [1u8; 32],
                new_schema_digest: [2u8; 32],
            },
        );
        assert!(decision.blocks_dispatch());
        assert_eq!(
            decision.authorize_live_effect(),
            Err(CompatError::MigrationRequired {
                boundary: CompatBoundary::IrVersion,
            })
        );
        assert!(decision.authorize_replay().is_ok());
    }

    #[test]
    fn decision_record_round_trips_through_postcard() -> Result<(), postcard::Error> {
        let decision = CompatDecision::decide(
            inputs(v(1, 0, 0), v(2, 0, 0), false, true),
            CompatDigests {
                rule_digest: [7u8; 32],
                old_schema_digest: [1u8; 32],
                new_schema_digest: [2u8; 32],
            },
        );
        let bytes = postcard::to_allocvec(&decision)?;
        let decoded: CompatDecision = postcard::from_bytes(&bytes)?;
        assert_eq!(decoded, decision);
        Ok(())
    }

    /// Independent adversarial corpus: hand-written literal versions only,
    /// no round-trip construction, no generators. Covers the numeric
    /// edges (zero, `u16::MAX`), patch-only direction inside one major
    /// line, flag precedence, and unknown-future fail-closed behavior.
    #[test]
    fn adversarial_version_corpus() {
        // (held, required, migration, replay, class).
        let max = u16::MAX;
        let cases: [(SchemaVersion, SchemaVersion, bool, bool, CompatibilityClass); 12] = [
            // Zero line against itself is identical, not rejected.
            (
                v(0, 0, 0),
                v(0, 0, 0),
                false,
                false,
                CompatibilityClass::Identical,
            ),
            // Absent (0.0.0) against any declared line with no path fails closed.
            (
                v(0, 0, 0),
                v(1, 0, 0),
                false,
                false,
                CompatibilityClass::Rejected,
            ),
            (
                v(0, 0, 0),
                v(1, 0, 0),
                true,
                true,
                CompatibilityClass::MigrationRequired,
            ),
            // Patch-only direction inside one major line.
            (
                v(2, 4, 0),
                v(2, 4, 7),
                false,
                false,
                CompatibilityClass::BackwardCompatible,
            ),
            (
                v(2, 4, 7),
                v(2, 4, 0),
                false,
                false,
                CompatibilityClass::ForwardCompatible,
            ),
            // Minor-only direction ignores the patch entirely.
            (
                v(2, 4, 9),
                v(2, 5, 0),
                false,
                false,
                CompatibilityClass::BackwardCompatible,
            ),
            (
                v(2, 5, 0),
                v(2, 4, 9),
                false,
                false,
                CompatibilityClass::ForwardCompatible,
            ),
            // Extreme majors: unknown future fails closed, known past
            // migrates explicitly.
            (
                v(1, 0, 0),
                v(max, 0, 0),
                false,
                false,
                CompatibilityClass::Rejected,
            ),
            (
                v(max, max, max),
                v(1, 0, 0),
                true,
                false,
                CompatibilityClass::MigrationRequired,
            ),
            (
                v(max, max, max),
                v(1, 0, 0),
                false,
                true,
                CompatibilityClass::ReplayOnly,
            ),
            (
                v(max, max, max),
                v(1, 0, 0),
                false,
                false,
                CompatibilityClass::Rejected,
            ),
            // Both flags set on a major change: migration wins over replay,
            // so data is never silently replayed past a needed migration.
            (
                v(3, 0, 0),
                v(4, 0, 0),
                true,
                true,
                CompatibilityClass::MigrationRequired,
            ),
        ];
        for (held, required, migration, replay, class) in cases {
            let got = classify(inputs(held, required, migration, replay));
            assert_eq!(got.class, class, "held {held:?} required {required:?}");
        }
    }

    /// Unknown-future fail-closed holds at every boundary, not just the
    /// IR one: a held major line no consumer understands, with no
    /// declared path, is `Rejected` everywhere and blocks replay too.
    #[test]
    fn unknown_future_fails_closed_at_all_boundaries() {
        assert_eq!(CompatBoundary::ALL.len(), 12);
        for boundary in CompatBoundary::ALL {
            let decision = CompatDecision::decide(
                CompatInputs {
                    boundary,
                    held: SchemaVersion::new(9, 0, 0),
                    required: SchemaVersion::new(1, 0, 0),
                    migration_available: false,
                    replay_only: false,
                },
                CompatDigests {
                    rule_digest: [0u8; 32],
                    old_schema_digest: [0u8; 32],
                    new_schema_digest: [0u8; 32],
                },
            );
            assert_eq!(decision.class, CompatibilityClass::Rejected, "{boundary:?}");
            assert_eq!(
                decision.rule,
                CompatRule::MajorChangeUnsupported,
                "{boundary:?}"
            );
            assert_eq!(
                decision.decision_code,
                CompatDecisionCode::BlockRejected,
                "{boundary:?}"
            );
            assert!(decision.blocks_dispatch(), "{boundary:?}");
            assert!(decision.authorize_live_effect().is_err(), "{boundary:?}");
            assert!(decision.authorize_replay().is_err(), "{boundary:?}");
        }
    }
}
